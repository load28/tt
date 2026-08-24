//! TypeScript target lowering from validated Core IR.
//!
//! This module is intentionally independent of `ast`: source text enters
//! only through HIR nodes and the source map. Every tt surface reaches this
//! module through a shared Core primitive.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};

use super::rope::{Flat, Rope};
use crate::analysis::SemanticFile;
use crate::core_ir::*;
use crate::evaluation_ir::{
    LoweringPlan, PlannedEvaluationInput, PlannedEvaluationStep, ValueTarget,
};
use crate::hir::ids::Idx;
use crate::hir::{self, ArmBodyKind, BindingMode, ExprId, NodeId};
use crate::program_syntax::{
    ConditionalBranch, EvaluationFrequency, EvaluationInputMode, EvaluationOwner, HostContinuation,
    HostEvaluationOperation, HostExit, HostOwnerKind, SourceSpan,
};
use crate::scanner::{at, ident_end, is_ident_start, scan_type_end, skip_ws_comments};
use crate::{AnchorKind, ImportRewrite, SourceKind, StdImports};

/// The one host-lowering failure the *input* can cause: the TypeScript in
/// the file does not parse, so there is no TypeScript owner model to lower
/// tt values against.
///
/// It is not an internal error and not codegen's to report — the phase that
/// owns diagnostics turns it into a located one ([`crate::verify::in_source`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceNotTypeScript {
    /// The syntax substrate's message.
    pub(crate) message: String,
    /// The source byte the parse stopped at.
    pub(crate) source: usize,
}

/// Builds the host-lowering plan for a file, ahead of emission.
///
/// Emission is infallible by contract (`docs/design/compiler-architecture.md`),
/// so the fallible half — projecting the file to TypeScript, joining every tt
/// value to its owner, and planning the rewrites — is this separate step, run
/// by the phase that can report. Every failure but
/// [`SourceNotTypeScript`] is a broken compiler invariant and fails the
/// build immediately (`docs/design/program-lowering.md` §11).
pub(crate) fn lowering_plan(
    semantic: &SemanticFile,
    core: &CoreFile,
    source: &str,
    source_kind: SourceKind,
) -> Result<LoweringPlan, SourceNotTypeScript> {
    if !core.requires_host_lowering() {
        return Ok(LoweringPlan::default());
    }
    let syntax =
        match crate::program_syntax::ProgramSyntax::build(semantic, core, source, source_kind) {
            Ok(syntax) => syntax,
            Err(crate::program_syntax::ProgramSyntaxError::SourceNotTypeScript {
                message,
                source,
            }) => {
                return Err(SourceNotTypeScript { message, source });
            }
            Err(error) => {
                panic!("internal compiler error: TypeScript owner construction failed: {error:?}")
            }
        };
    let evaluation =
        crate::evaluation_ir::EvaluationFile::build(&syntax, core).unwrap_or_else(|error| {
            panic!("internal compiler error: Evaluation IR construction failed: {error:?}")
        });
    Ok(evaluation.lowering_plan().unwrap_or_else(|error| {
        panic!("internal compiler error: owner lowering plan failed: {error:?}")
    }))
}

pub(crate) fn emit_with_map<'a>(
    semantic: &'a SemanticFile,
    core: &'a CoreFile,
    source: &'a str,
    lowering_plan: &LoweringPlan,
    rewrite_imports: ImportRewrite,
    std_imports: StdImports<'a>,
) -> Flat {
    let target = TargetRewritePlan::build(lowering_plan, core);
    let emitter = Emitter {
        semantic,
        core,
        source,
        rewrite_imports,
        std_imports,
        owner_slot_rewrites: target.owner_slots,
        compose_rewrites: target.composes,
        source_replacements: target.source_replacements,
        arrow_return_rewrites: target.arrow_returns,
        slot_exprs: target.slot_exprs,
        value_slots: target.value_slots,
        scheduled_slots: target.scheduled_slots,
        value_exits: target.value_exits,
        expression_boundary_name: target.expression_boundary_name,
        used_expression_boundary: Cell::new(false),
        used_pipe: Cell::new(false),
        used_flow: Cell::new(false),
    };
    let mut output = emitter.emit_body(core.root);
    if emitter.used_pipe.get() {
        if !output.ends_with_newline() {
            output.push_lit("\n");
        }
        output.push_lit("function $tt_ap<A, B>(v: A, f: (v: A) => B): B { return f(v); }\n");
    }
    if emitter.used_flow.get() {
        if !output.ends_with_newline() {
            output.push_lit("\n");
        }
        output.push_lit(
            "function $tt_fl<A extends unknown[], B, C>(f: (...a: A) => B, g: (b: B) => C): (...a: A) => C { return (...a: A) => g(f(...a)); }\n",
        );
    }
    if emitter.used_expression_boundary.get() {
        if !output.ends_with_newline() {
            output.push_lit("\n");
        }
        output.push_lit(format!(
            "function {}<T>(run: () => T): T {{ return run(); }}\n",
            emitter.expression_boundary_name
        ));
    }
    output.flatten(source.len())
}

struct TargetRewritePlan {
    owner_slots: Vec<OwnerSlotRewrite>,
    composes: Vec<ComposeRewrite>,
    source_replacements: Vec<SourceReplacement>,
    arrow_returns: Vec<ArrowReturnRewrite>,
    slot_exprs: HashMap<ExprId, String>,
    value_slots: HashMap<ExprId, String>,
    scheduled_slots: HashMap<crate::evaluation_ir::ValueSlotId, String>,
    value_exits: HashMap<ExprId, Vec<HostExit>>,
    expression_boundary_name: String,
}

#[derive(Debug, Clone)]
struct OwnerSlotRewrite {
    owner: SourceSpan,
    expr: ExprId,
    slot: String,
}

#[derive(Debug, Clone)]
struct ArrowReturnRewrite {
    expr: ExprId,
    slot: String,
}

#[derive(Debug, Clone)]
struct ComposeRewrite {
    owner: SourceSpan,
    owner_kind: HostOwnerKind,
    values: Vec<ComposeValue>,
}

#[derive(Debug, Clone)]
struct ComposeValue {
    expr: ExprId,
    slot: String,
    steps: Vec<PlannedEvaluationStep>,
}

#[derive(Debug, Clone)]
struct SourceReplacement {
    source: SourceSpan,
    slot: String,
}

#[derive(Debug, Clone)]
struct LocalSourceEdit {
    span: SourceSpan,
    text: String,
}

impl TargetRewritePlan {
    fn build(lowering: &LoweringPlan, core: &CoreFile) -> Self {
        let owner_slots: Vec<_> = lowering
            .owners()
            .filter(|rewrite| rewrite.values.len() == 1)
            .filter_map(|rewrite| {
                let value = &rewrite.values[0];
                let ValueTarget::Slot(slot) = value.target;
                (matches!(
                    value.context.continuation,
                    HostContinuation::Initialize
                        | HostContinuation::Return
                        | HostContinuation::Discard
                ) && value.schedule.steps().is_empty()
                    && !matches!(
                        value.context.owner,
                        EvaluationOwner::ParameterInitializer | EvaluationOwner::ClassInitializer
                    )
                    && can_structure_value_expr(core, value.expr))
                .then(|| OwnerSlotRewrite {
                    owner: rewrite.owner.span,
                    expr: value.expr,
                    slot: lowering.slot_name(slot).to_owned(),
                })
            })
            .collect();
        let arrow_returns: Vec<_> = lowering
            .owners()
            .filter(|rewrite| rewrite.values.len() == 1)
            .filter_map(|rewrite| {
                let value = &rewrite.values[0];
                let ValueTarget::Slot(slot) = value.target;
                (value.context.continuation == HostContinuation::ArrowReturn
                    && value.schedule.steps().is_empty()
                    && can_structure_value_expr(core, value.expr))
                .then(|| ArrowReturnRewrite {
                    expr: value.expr,
                    slot: lowering.slot_name(slot).to_owned(),
                })
            })
            .collect();
        let composes: Vec<_> = lowering
            .owners()
            .filter_map(|rewrite| {
                let can_compose = !rewrite.values.is_empty()
                    && rewrite.values.iter().all(|value| {
                        value.context.continuation == HostContinuation::Compose
                            && matches!(
                                value.context.owner,
                                EvaluationOwner::Module
                                    | EvaluationOwner::FunctionBody
                                    | EvaluationOwner::StaticBlock
                            )
                            && (!value.schedule.steps().is_empty()
                                || value.context.frequency == EvaluationFrequency::Once)
                            && compose_schedule_is_structurable(value.schedule.steps())
                            && can_structure_value_expr(core, value.expr)
                    });
                can_compose.then(|| ComposeRewrite {
                    owner: rewrite.owner.span,
                    owner_kind: rewrite.owner.kind,
                    values: rewrite
                        .values
                        .iter()
                        .map(|value| {
                            let ValueTarget::Slot(slot) = value.target;
                            ComposeValue {
                                expr: value.expr,
                                slot: lowering.slot_name(slot).to_owned(),
                                steps: value.schedule.steps().to_vec(),
                            }
                        })
                        .collect(),
                })
            })
            .collect();
        let source_replacements = composes
            .iter()
            .flat_map(|rewrite| &rewrite.values)
            .flat_map(|value| &value.steps)
            .flat_map(|step| &step.inputs)
            .filter_map(|input| match input {
                PlannedEvaluationInput::Source { source, target, .. } => Some(SourceReplacement {
                    source: *source,
                    slot: lowering.slot_name(*target).to_owned(),
                }),
                PlannedEvaluationInput::Slot { .. } => None,
            })
            .collect();
        let slot_exprs = owner_slots
            .iter()
            .map(|rewrite| (rewrite.expr, rewrite.slot.clone()))
            .chain(
                composes
                    .iter()
                    .flat_map(|rewrite| &rewrite.values)
                    .map(|value| (value.expr, value.slot.clone())),
            )
            .collect();
        let value_slots = lowering
            .value_slot_names()
            .map(|(expr, name)| (expr, name.to_owned()))
            .collect();
        let scheduled_slots = lowering
            .slots()
            .map(|(slot, name)| (slot, name.to_owned()))
            .collect();
        let value_exits = lowering
            .owners()
            .flat_map(|owner| &owner.values)
            .filter(|value| !value.exits.is_empty())
            .map(|value| (value.expr, value.exits.clone()))
            .collect();
        let expression_boundary_name = lowering.expression_boundary_name().to_owned();
        Self {
            owner_slots,
            composes,
            source_replacements,
            arrow_returns,
            slot_exprs,
            value_slots,
            scheduled_slots,
            value_exits,
            expression_boundary_name,
        }
    }
}

fn compose_schedule_is_structurable(steps: &[PlannedEvaluationStep]) -> bool {
    steps.iter().all(|step| {
        step.inputs.iter().all(|input| match input {
            PlannedEvaluationInput::Source { mode, receiver, .. } => match mode {
                EvaluationInputMode::Value | EvaluationInputMode::DirectReference => true,
                EvaluationInputMode::MemberReference => {
                    receiver.is_some()
                        && !matches!(
                            step.operation,
                            HostEvaluationOperation::Conditional(
                                ConditionalBranch::OptionalCallArgument(_)
                            )
                        )
                }
            },
            PlannedEvaluationInput::Slot { mode, .. } => {
                *mode != EvaluationInputMode::MemberReference
            }
        })
    })
}

fn can_structure_value_expr(core: &CoreFile, expr: ExprId) -> bool {
    match &core.exprs[expr.index()] {
        Expr::Decision(decision) => decision
            .arms
            .iter()
            .all(|arm| matches!(arm.action, ArmAction::Yield { .. })),
        Expr::ResultRegion(_) => true,
        Expr::Sequence(body) => {
            body_value_expr(core, *body).is_some_and(|inner| can_structure_value_expr(core, inner))
        }
        Expr::Apply(apply) => apply.head.is_some_and(|head| {
            can_structure_value_expr(core, head)
                || apply
                    .steps
                    .iter()
                    .any(|step| can_structure_value_expr(core, step.value))
        }),
        Expr::Opaque(_) | Expr::Template(_) => false,
    }
}

fn body_value_expr(core: &CoreFile, body: hir::BodyId) -> Option<ExprId> {
    sequence_value(&core.bodies[body.index()].statements).map(|(_, expr)| expr)
}

fn sequence_value(statements: &[Statement]) -> Option<(usize, ExprId)> {
    let (index, expr) = statements
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, statement)| match statement {
            Statement::Expr(expr) => Some((index, *expr)),
            _ => None,
        })?;
    statements[index + 1..]
        .iter()
        .all(|statement| matches!(statement, Statement::Opaque(_)))
        .then_some((index, expr))
}

struct Emitter<'a> {
    semantic: &'a SemanticFile,
    core: &'a CoreFile,
    source: &'a str,
    rewrite_imports: ImportRewrite,
    std_imports: StdImports<'a>,
    owner_slot_rewrites: Vec<OwnerSlotRewrite>,
    compose_rewrites: Vec<ComposeRewrite>,
    source_replacements: Vec<SourceReplacement>,
    arrow_return_rewrites: Vec<ArrowReturnRewrite>,
    slot_exprs: HashMap<ExprId, String>,
    value_slots: HashMap<ExprId, String>,
    scheduled_slots: HashMap<crate::evaluation_ir::ValueSlotId, String>,
    value_exits: HashMap<ExprId, Vec<HostExit>>,
    expression_boundary_name: String,
    used_expression_boundary: Cell<bool>,
    used_pipe: Cell<bool>,
    used_flow: Cell<bool>,
}

#[derive(Clone)]
struct ValueContinuation<'name> {
    destination: ValueDestination<'name>,
    wrappers: Vec<ValueWrapper>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ValueDestination<'name> {
    Expression,
    Assign(&'name str),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ValueWrapper {
    ResultOk,
}

struct ArmEmissionContext<'context, 'name> {
    /// Indentation depth of the arm's own line inside the lowering.
    depth: u16,
    chain: bool,
    continuation: &'context ValueContinuation<'name>,
    exits: &'context [HostExit],
    exit_label: Option<&'context str>,
}

impl<'name> ValueContinuation<'name> {
    fn expression() -> Self {
        Self {
            destination: ValueDestination::Expression,
            wrappers: Vec::new(),
        }
    }

    fn assign(target: &'name str) -> Self {
        Self {
            destination: ValueDestination::Assign(target),
            wrappers: Vec::new(),
        }
    }

    fn wrap_result_ok(&self) -> Self {
        let mut continuation = self.clone();
        continuation.wrappers.push(ValueWrapper::ResultOk);
        continuation
    }

    fn assigns(&self) -> bool {
        matches!(self.destination, ValueDestination::Assign(_))
    }

    fn is_expression(&self) -> bool {
        self.destination == ValueDestination::Expression
    }

    fn is_unwrapped_assignment_to(&self, target: &str) -> bool {
        self.wrappers.is_empty()
            && matches!(self.destination, ValueDestination::Assign(destination) if destination == target)
    }

    fn assignment_target(&self) -> Option<&str> {
        match self.destination {
            ValueDestination::Expression => None,
            ValueDestination::Assign(target) => Some(target),
        }
    }

    /// The text an early exit's `return ` becomes. `grouped` says whether
    /// the value it returns has to keep its parentheses ([`push_grouped`]).
    fn assignment_prefix(&self, grouped: bool) -> String {
        let target = self.assignment_target().unwrap_or_else(|| {
            panic!("internal compiler error: expression continuation cannot rewrite an exit")
        });
        let mut prefix = format!("{target} = ");
        for wrapper in &self.wrappers {
            match wrapper {
                ValueWrapper::ResultOk => {
                    prefix.push_str("{ kind: \"Ok\" as const, value: ");
                }
            }
        }
        if grouped {
            prefix.push('(');
        }
        prefix
    }

    fn assignment_suffix(&self, grouped: bool) -> String {
        let mut suffix = String::new();
        if grouped {
            suffix.push(')');
        }
        for _ in self.wrappers.iter().rev() {
            suffix.push_str(" }");
        }
        suffix
    }
}

fn decision_has_block_arm(decision: &Decision) -> bool {
    decision.arms.iter().any(|arm| {
        matches!(
            arm.action,
            ArmAction::Yield {
                kind: ArmBodyKind::Block,
                ..
            }
        )
    })
}

fn exit_label(target: &str) -> String {
    format!("$tt_y_{target}")
}

impl<'a> Emitter<'a> {
    fn span(&self, node: NodeId) -> hir::Span {
        self.semantic
            .hir
            .source_map
            .node_span(node)
            .unwrap_or_else(|| panic!("internal compiler error: target node has no source span"))
    }

    fn source_node(&self, node: NodeId) -> (&'a str, usize) {
        let span = self.span(node);
        (&self.source[span.start..span.end], span.start)
    }

    fn source_span(&self, span: hir::Span) -> (&'a str, usize) {
        (&self.source[span.start..span.end], span.start)
    }

    fn source_rope(&self, node: NodeId) -> Rope<'a> {
        let span = self.span(node);
        self.source_range_rope(span)
    }

    fn source_range_rope(&self, span: hir::Span) -> Rope<'a> {
        let mut rope = Rope::new();
        let mut insertions = self
            .owner_slot_rewrites
            .iter()
            .filter(|rewrite| span.start <= rewrite.owner.start && rewrite.owner.start < span.end)
            .peekable();
        let mut compose_insertions = self
            .compose_rewrites
            .iter()
            .filter(|rewrite| span.start <= rewrite.owner.start && rewrite.owner.start < span.end)
            .peekable();
        let mut compose_endings = self
            .compose_rewrites
            .iter()
            .filter(|rewrite| {
                rewrite.owner_kind == HostOwnerKind::ArrowExpression
                    && span.start < rewrite.owner.end
                    && rewrite.owner.end <= span.end
            })
            .peekable();
        let mut cursor = span.start;
        while cursor < span.end {
            while let Some(rewrite) = compose_endings.next_if(|rewrite| rewrite.owner.end == cursor)
            {
                rope.append(self.emit_compose_suffix(rewrite));
            }
            while let Some(rewrite) = insertions.next_if(|rewrite| rewrite.owner.start == cursor) {
                rope.append(self.emit_owner_slot_rewrite(rewrite));
            }
            while let Some(rewrite) =
                compose_insertions.next_if(|rewrite| rewrite.owner.start == cursor)
            {
                rope.append(self.emit_compose_rewrite(rewrite));
            }
            if let Some(replacement) = self.source_replacements.iter().find(|replacement| {
                replacement.source.start <= cursor && cursor < replacement.source.end
            }) {
                if cursor == replacement.source.start {
                    rope.push_lit(replacement.slot.clone());
                }
                cursor = replacement.source.end.min(span.end);
                continue;
            }
            let next_insertion = insertions
                .peek()
                .map_or(span.end, |rewrite| rewrite.owner.start);
            let next_compose = compose_insertions
                .peek()
                .map_or(span.end, |rewrite| rewrite.owner.start);
            let next_compose_end = compose_endings
                .peek()
                .map_or(span.end, |rewrite| rewrite.owner.end);
            let next_replacement = self
                .source_replacements
                .iter()
                .filter(|replacement| {
                    cursor < replacement.source.start && replacement.source.start < span.end
                })
                .map(|replacement| replacement.source.start)
                .min()
                .unwrap_or(span.end);
            let next = next_insertion
                .min(next_compose)
                .min(next_compose_end)
                .min(next_replacement)
                .min(span.end);
            if cursor < next {
                rope.push_src(&self.source[cursor..next], cursor);
                cursor = next;
            }
        }
        while let Some(rewrite) = compose_endings.next_if(|rewrite| rewrite.owner.end == span.end) {
            rope.append(self.emit_compose_suffix(rewrite));
        }
        rope
    }

    fn source_rope_with_edits(&self, node: NodeId, edits: &[LocalSourceEdit]) -> Rope<'a> {
        let span = self.span(node);
        let mut cursor = span.start;
        let mut out = Rope::new();
        for edit in edits
            .iter()
            .filter(|edit| span.start <= edit.span.start && edit.span.end <= span.end)
        {
            if cursor < edit.span.start {
                out.append(self.source_range_rope(hir::Span::new(cursor, edit.span.start)));
            }
            out.push_lit(edit.text.clone());
            cursor = edit.span.end;
        }
        if cursor < span.end {
            out.append(self.source_range_rope(hir::Span::new(cursor, span.end)));
        }
        out
    }

    fn emit_body(&self, body: hir::BodyId) -> Rope<'a> {
        self.emit_statements(&self.core.bodies[body.index()].statements)
    }

    fn emit_body_with_exits(
        &self,
        body: hir::BodyId,
        exits: &[HostExit],
        continuation: &ValueContinuation<'_>,
        label: &str,
    ) -> Rope<'a> {
        let mut edits = Vec::new();
        for exit in exits {
            match exit.argument {
                Some(argument) => {
                    let grouped =
                        grouping_required(self.source[argument.start..argument.end].trim());
                    edits.push(LocalSourceEdit {
                        span: SourceSpan {
                            start: exit.statement.start,
                            end: argument.start,
                        },
                        text: continuation.assignment_prefix(grouped),
                    });
                    edits.push(LocalSourceEdit {
                        span: SourceSpan {
                            start: argument.end,
                            end: exit.statement.end,
                        },
                        text: format!(
                            "{}; break {label};",
                            continuation.assignment_suffix(grouped)
                        ),
                    });
                }
                None => edits.push(LocalSourceEdit {
                    span: exit.statement,
                    text: format!(
                        "{}undefined{}; break {label};",
                        continuation.assignment_prefix(false),
                        continuation.assignment_suffix(false)
                    ),
                }),
            }
        }
        edits.sort_unstable_by_key(|edit| edit.span.start);
        self.emit_statements_with_edits(&self.core.bodies[body.index()].statements, &edits)
    }

    fn emit_statements(&self, statements: &[Statement]) -> Rope<'a> {
        self.emit_statements_with_edits(statements, &[])
    }

    fn emit_statements_with_edits(
        &self,
        statements: &[Statement],
        edits: &[LocalSourceEdit],
    ) -> Rope<'a> {
        let mut out = Rope::new();
        for statement in statements {
            match statement {
                Statement::Opaque(node) => out.append(self.source_rope_with_edits(*node, edits)),
                Statement::Adt(adt) => out.append(emit_adt(adt)),
                Statement::Import(import) => self.emit_import(import, &mut out),
                Statement::Propagate(propagate) => {
                    let span = self.span(propagate.node);
                    out.anchored(
                        AnchorKind::Try,
                        span.start,
                        span.end,
                        span.end,
                        self.emit_propagate(propagate),
                    );
                }
                Statement::Decision(decision) => self.emit_statement_decision(decision, &mut out),
                Statement::Expr(expr) => out.append(self.emit_expr(*expr)),
            }
        }
        out
    }

    fn emit_sequence_continued(
        &self,
        body: hir::BodyId,
        continuation: &ValueContinuation<'_>,
    ) -> Option<Rope<'a>> {
        let statements = &self.core.bodies[body.index()].statements;
        let (value_index, value) = sequence_value(statements)?;
        let mut out = self.emit_statements(&statements[..value_index]);
        out.append(self.emit_continued_expr(value, continuation)?);
        out.append(self.emit_statements(&statements[value_index + 1..]));
        Some(out)
    }

    fn emit_expr(&self, expr: ExprId) -> Rope<'a> {
        if let Some(rewrite) = self
            .arrow_return_rewrites
            .iter()
            .find(|rewrite| rewrite.expr == expr)
        {
            return self.emit_arrow_return_rewrite(rewrite);
        }
        if let Some(slot) = self.slot_exprs.get(&expr) {
            let (kind, start, end, extent) = self.value_anchor(expr);
            let mut generated = Rope::new();
            generated.push_lit(slot.clone());
            let mut out = Rope::new();
            out.anchored(kind, start, end, extent, generated);
            return out;
        }
        match &self.core.exprs[expr.index()] {
            Expr::Opaque(node) => self.source_rope(*node),
            Expr::Sequence(body) => self.emit_body(*body),
            Expr::Decision(decision) => {
                let head = self.span(decision.head);
                let extent = self.span(decision.extent);
                let inner =
                    self.emit_value_decision(decision, &ValueContinuation::expression(), &[]);
                let mut out = Rope::new();
                out.anchored(AnchorKind::Match, head.start, head.end, extent.end, inner);
                out
            }
            Expr::Apply(apply) => self.emit_apply(apply),
            Expr::ResultRegion(region) => self.emit_result_region(region),
            Expr::Template(template) => self.emit_template(template),
        }
    }

    fn emit_owner_slot_rewrite(&self, rewrite: &OwnerSlotRewrite) -> Rope<'a> {
        let anchored = self
            .emit_continued_expr(rewrite.expr, &ValueContinuation::assign(&rewrite.slot))
            .unwrap_or_else(|| {
                panic!("internal compiler error: initializer rewrite is not structurally emit-able")
            });
        let mut out = Rope::new();
        out.push_lit(format!("let {};", rewrite.slot));
        out.push_break(0);
        out.append(anchored);
        out.push_break(0);
        Rope::scoped(out)
    }

    fn emit_compose_rewrite(&self, rewrite: &ComposeRewrite) -> Rope<'a> {
        let mut out = Rope::new();
        if rewrite.owner_kind == HostOwnerKind::ArrowExpression {
            out.push_lit("{");
            out.push_break(0);
        }
        for value in &rewrite.values {
            out.push_lit(format!("let {};", value.slot));
            out.push_break(0);
        }
        let mut captured = HashSet::new();
        for value in &rewrite.values {
            let mut action = self
                .emit_continued_expr(value.expr, &ValueContinuation::assign(&value.slot))
                .unwrap_or_else(|| {
                    panic!("internal compiler error: compose value is not structurally emit-able")
                });
            for step in &value.steps {
                action = self.emit_scheduled_step(step, action, &mut captured);
            }
            out.append(action);
        }
        out.push_break(0);
        if rewrite.owner_kind == HostOwnerKind::ArrowExpression {
            out.push_lit("return ");
        }
        Rope::scoped(out)
    }

    fn emit_compose_suffix(&self, rewrite: &ComposeRewrite) -> Rope<'a> {
        debug_assert_eq!(rewrite.owner_kind, HostOwnerKind::ArrowExpression);
        let mut out = Rope::new();
        out.push_lit(";");
        out.push_break(0);
        out.push_lit("}");
        Rope::scoped(out)
    }

    fn emit_scheduled_step(
        &self,
        step: &PlannedEvaluationStep,
        action: Rope<'a>,
        captured: &mut HashSet<crate::evaluation_ir::ValueSlotId>,
    ) -> Rope<'a> {
        let mut prefix = Rope::new();
        for input in &step.inputs {
            let PlannedEvaluationInput::Source {
                source,
                mode,
                target,
                receiver,
            } = input
            else {
                continue;
            };
            if !captured.insert(*target) {
                continue;
            }
            if *mode == EvaluationInputMode::MemberReference {
                let (receiver_source, receiver_target) = receiver.unwrap_or_else(|| {
                    panic!("internal compiler error: member reference has no receiver")
                });
                let receiver_name = self.value_slot_name(receiver_target);
                prefix.push_lit(format!("const {receiver_name} = ("));
                prefix.push_src(
                    &self.source[receiver_source.start..receiver_source.end],
                    receiver_source.start,
                );
                prefix.push_lit(");");
                prefix.push_break(0);
                let optional_reference = matches!(
                    step.operation,
                    HostEvaluationOperation::Conditional(ConditionalBranch::OptionalCallArgument(
                        _
                    ))
                );
                prefix.push_lit(format!(
                    "{} {} = (",
                    if optional_reference { "let" } else { "const" },
                    self.value_slot_name(*target)
                ));
                if source.start < receiver_source.start {
                    prefix.push_src(
                        &self.source[source.start..receiver_source.start],
                        source.start,
                    );
                }
                prefix.push_lit(receiver_name.to_owned());
                if receiver_source.end < source.end {
                    prefix.push_src(
                        &self.source[receiver_source.end..source.end],
                        receiver_source.end,
                    );
                }
                if optional_reference {
                    let target_name = self.value_slot_name(*target);
                    prefix.push_lit(");");
                    prefix.push_break(0);
                    prefix.push_lit(format!("if ({target_name} != null) {{"));
                    prefix.push_break(1);
                    prefix.push_lit(format!(
                        "{target_name} = {target_name}.bind({receiver_name});"
                    ));
                    prefix.push_break(0);
                    prefix.push_lit("}");
                    prefix.push_break(0);
                } else {
                    prefix.push_lit(format!(").bind({receiver_name});"));
                    prefix.push_break(0);
                }
            } else {
                prefix.push_lit(format!("const {} = (", self.value_slot_name(*target)));
                prefix.push_src(&self.source[source.start..source.end], source.start);
                prefix.push_lit(");");
                prefix.push_break(0);
            }
        }
        match step.operation {
            HostEvaluationOperation::Eager(_) | HostEvaluationOperation::Suspend(_) => {
                prefix.append(action);
                prefix
            }
            HostEvaluationOperation::Reference(_) => {
                prefix.append(action);
                prefix
            }
            HostEvaluationOperation::Conditional(branch) => {
                let input = step.inputs.first().unwrap_or_else(|| {
                    panic!("internal compiler error: conditional schedule has no condition")
                });
                let condition = match input {
                    PlannedEvaluationInput::Source { target, .. }
                    | PlannedEvaluationInput::Slot { slot: target, .. } => {
                        self.value_slot_name(*target)
                    }
                };
                prefix.push_lit("if (");
                match branch {
                    ConditionalBranch::LogicalAndRight | ConditionalBranch::Consequent => {
                        prefix.push_lit(condition.to_owned());
                    }
                    ConditionalBranch::OptionalCallArgument(_) => {
                        prefix.push_lit(format!("{condition} != null"));
                    }
                    ConditionalBranch::LogicalOrRight | ConditionalBranch::Alternate => {
                        prefix.push_lit(format!("!({condition})"));
                    }
                    ConditionalBranch::NullishRight => {
                        prefix.push_lit(format!("{condition} == null"));
                    }
                }
                prefix.push_lit(") {");
                prefix.push_break(1);
                prefix.append(Rope::indented(1, action));
                prefix.push_break(0);
                prefix.push_lit("}");
                prefix.push_break(0);
                prefix
            }
        }
    }

    fn value_slot_name(&self, slot: crate::evaluation_ir::ValueSlotId) -> &str {
        self.scheduled_slots
            .get(&slot)
            .map(String::as_str)
            .unwrap_or_else(|| {
                panic!("internal compiler error: scheduled value slot has no generated name")
            })
    }

    fn structured_value_slot(&self, expr: ExprId) -> Option<&String> {
        self.value_slots.get(&expr).or_else(|| {
            let Expr::Sequence(body) = &self.core.exprs[expr.index()] else {
                return None;
            };
            body_value_expr(self.core, *body).and_then(|value| self.structured_value_slot(value))
        })
    }

    fn value_anchor(&self, expr: ExprId) -> (AnchorKind, usize, usize, usize) {
        match &self.core.exprs[expr.index()] {
            Expr::Decision(decision) => {
                let head = self.span(decision.head);
                let extent = self.span(decision.extent);
                (AnchorKind::Match, head.start, head.end, extent.end)
            }
            Expr::ResultRegion(region) => {
                let (start, end) = self.result_bind_anchor(region);
                (AnchorKind::ResultBind, start, end, end)
            }
            Expr::Sequence(body) => {
                let value = body_value_expr(self.core, *body).unwrap_or_else(|| {
                    panic!("internal compiler error: slotted sequence has no value")
                });
                self.value_anchor(value)
            }
            Expr::Apply(apply) => {
                let start = self.span(apply.node).start;
                let end = apply.steps.last().map_or_else(
                    || self.span(apply.node).end,
                    |step| self.span(step.node).end,
                );
                (AnchorKind::Pipe, start, end, end)
            }
            Expr::Opaque(_) | Expr::Template(_) => {
                panic!("internal compiler error: unstructured expression owns a join slot")
            }
        }
    }

    fn result_bind_anchor(&self, region: &ResultRegion) -> (usize, usize) {
        region
            .items
            .iter()
            .rev()
            .find_map(|item| {
                let ResultRegionItem::Propagate(propagate) = item else {
                    return None;
                };
                let binding = propagate.binding?;
                let span = self.span(propagate.node);
                let binding_span = self.span(binding.node);
                Some((
                    binding_span.start,
                    trimmed_end(self.source, binding_span.start, span.end),
                ))
            })
            .unwrap_or_else(|| {
                panic!("internal compiler error: result region has no propagation binding")
            })
    }

    fn emit_arrow_return_rewrite(&self, rewrite: &ArrowReturnRewrite) -> Rope<'a> {
        let anchored = self
            .emit_continued_expr(rewrite.expr, &ValueContinuation::assign(&rewrite.slot))
            .unwrap_or_else(|| {
                panic!(
                    "internal compiler error: arrow return rewrite is not structurally emit-able"
                )
            });
        let mut out = Rope::new();
        out.push_lit("{");
        out.push_break(1);
        out.push_lit(format!("let {};", rewrite.slot));
        out.push_break(1);
        out.append(Rope::indented(1, anchored));
        out.push_break(1);
        out.push_lit(format!("return {};", rewrite.slot));
        out.push_break(0);
        out.push_lit("}");
        Rope::scoped(out)
    }

    fn emit_propagate(&self, propagate: &Propagate) -> Rope<'a> {
        let temp = temp_name(propagate.temporary);
        let mut out = self.emit_propagate_input(propagate.value, &temp);
        out.push_lit(format!(
            " if ({temp}.{} !== \"{}\") return {temp};",
            propagate.layout.discriminant_field, propagate.layout.success_tag
        ));
        if let Some(binding) = propagate.binding {
            out.push_lit(format!(" {} ", binding_keyword(binding.mode)));
            out.append(self.source_rope(binding.node));
            out.push_lit(format!(" = {temp}.{};", propagate.layout.payload_field));
        }
        out
    }

    fn emit_propagate_input(&self, value: ExprId, temp: &str) -> Rope<'a> {
        let mut out = Rope::new();
        if can_structure_value_expr(self.core, value) && self.can_inline_continued_expr(value) {
            let slot = self.structured_value_slot(value).unwrap_or_else(|| {
                panic!("internal compiler error: structured propagation value has no slot")
            });
            out.push_lit(format!("let {slot};"));
            out.push_break(0);
            out.append(
                self.emit_continued_expr(value, &ValueContinuation::assign(slot))
                    .unwrap_or_else(|| {
                        panic!("internal compiler error: propagation value was not emitted")
                    }),
            );
            out.push_break(0);
            out.push_lit(format!("const {temp} = {slot};"));
        } else {
            out.push_lit(format!("const {temp} = "));
            push_grouped(&mut out, self.emit_expr(value).trim());
            out.push_lit(";");
        }
        out
    }

    fn can_inline_continued_expr(&self, expr: ExprId) -> bool {
        let Expr::Sequence(body) = &self.core.exprs[expr.index()] else {
            return true;
        };
        let statements = &self.core.bodies[body.index()].statements;
        let Some((value_index, _)) = sequence_value(statements) else {
            return false;
        };
        statements
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != value_index)
            .all(|(_, statement)| match statement {
                Statement::Opaque(node) => {
                    let span = self.span(*node);
                    self.source[span.start..span.end].trim().is_empty()
                }
                _ => false,
            })
    }

    fn emit_result_region(&self, region: &ResultRegion) -> Rope<'a> {
        let mut out = Rope::new();
        self.used_expression_boundary.set(true);
        out.push_lit(if region.is_async {
            format!("(await {}(async () => {{", self.expression_boundary_name)
        } else {
            format!("{}(() => {{", self.expression_boundary_name)
        });
        for item in &region.items {
            match item {
                ResultRegionItem::Statements(body) => {
                    out.append(guard_line_comment(self.emit_body(*body), 0));
                }
                ResultRegionItem::Propagate(propagate) => {
                    let span = self.span(propagate.node);
                    let binding = propagate.binding.unwrap_or_else(|| {
                        panic!("internal compiler error: result propagation has no binding")
                    });
                    let binding_span = self.span(binding.node);
                    let one = self.emit_propagate(propagate);
                    out.anchored(
                        AnchorKind::ResultBind,
                        binding_span.start,
                        trimmed_end(self.source, binding_span.start, span.end),
                        trimmed_end(self.source, binding_span.start, span.end),
                        one,
                    );
                }
            }
        }
        out.push_lit("return { kind: \"Ok\" as const, value: ");
        push_grouped(
            &mut out,
            guard_line_comment(self.emit_expr(region.value).trim(), 0),
        );
        out.push_lit(if region.is_async { " }; }))" } else { " }; })" });
        out
    }

    fn emit_result_region_continued(
        &self,
        region: &ResultRegion,
        continuation: &ValueContinuation<'_>,
    ) -> Rope<'a> {
        let mut out = Rope::new();
        // The block's own source follows, leading whitespace included, so
        // the opener does not break the line the region body starts on.
        out.push_lit(if continuation.assigns() { "do {" } else { "{" });
        for item in &region.items {
            match item {
                ResultRegionItem::Statements(body) => {
                    out.append(guard_line_comment(self.emit_body(*body), 0));
                }
                ResultRegionItem::Propagate(propagate) => {
                    let span = self.span(propagate.node);
                    let binding = propagate.binding.unwrap_or_else(|| {
                        panic!("internal compiler error: result propagation has no binding")
                    });
                    let binding_span = self.span(binding.node);
                    let lowered = self.emit_region_propagate(propagate, continuation);
                    out.anchored(
                        AnchorKind::ResultBind,
                        binding_span.start,
                        trimmed_end(self.source, binding_span.start, span.end),
                        trimmed_end(self.source, binding_span.start, span.end),
                        lowered,
                    );
                }
            }
        }
        let success = continuation.wrap_result_ok();
        if let Some(structured) = self.emit_continued_expr(region.value, &success) {
            out.append(structured);
            if continuation.assigns() {
                out.push_lit(" break;");
            }
        } else {
            out.append(self.emit_value_delivery(
                guard_line_comment(self.emit_expr(region.value).trim(), 0),
                None,
                &success,
            ));
        }
        out.push_break(0);
        out.push_lit(if continuation.assigns() {
            "} while (false);"
        } else {
            "}"
        });
        let (binding_start, binding_end) = self.result_bind_anchor(region);
        let mut anchored = Rope::new();
        anchored.anchored(
            AnchorKind::ResultBind,
            binding_start,
            binding_end,
            binding_end,
            out,
        );
        anchored
    }

    fn emit_region_propagate(
        &self,
        propagate: &Propagate,
        continuation: &ValueContinuation<'_>,
    ) -> Rope<'a> {
        let temp = temp_name(propagate.temporary);
        let mut out = self.emit_propagate_input(propagate.value, &temp);
        out.push_lit(format!(
            " if ({temp}.{} !== \"{}\") {{ ",
            propagate.layout.discriminant_field, propagate.layout.success_tag
        ));
        let mut value = Rope::new();
        value.push_lit(temp.clone());
        out.append(self.emit_value_delivery(value, None, continuation));
        out.push_lit(" }");
        if let Some(binding) = propagate.binding {
            out.push_lit(format!(" {} ", binding_keyword(binding.mode)));
            out.append(self.source_rope(binding.node));
            out.push_lit(format!(" = {temp}.{};", propagate.layout.payload_field));
        }
        out
    }

    fn emit_apply(&self, apply: &Apply) -> Rope<'a> {
        let inner = match apply.head {
            Some(head) => {
                self.used_pipe.set(true);
                let mut acc = guard_line_comment(self.emit_expr(head).trim(), 0);
                for step in &apply.steps {
                    let body = guard_line_comment(self.emit_expr(step.value).trim(), 0);
                    let mut next = Rope::new();
                    match step.mode {
                        ApplyMode::Postfix => {
                            push_receiver(&mut next, acc);
                            next.append(body);
                        }
                        ApplyMode::Call => {
                            next.push_lit("$tt_ap(");
                            push_grouped(&mut next, acc);
                            next.push_lit(", ");
                            push_grouped(&mut next, body);
                            next.push_lit(")");
                        }
                    }
                    acc = next;
                }
                acc
            }
            None => self.emit_flow(apply),
        };
        let start = self.span(apply.node).start;
        let end = apply.steps.last().map_or_else(
            || self.span(apply.node).end,
            |step| self.span(step.node).end,
        );
        let mut out = Rope::new();
        out.anchored(AnchorKind::Pipe, start, end, end, inner);
        out
    }

    fn emit_flow(&self, apply: &Apply) -> Rope<'a> {
        let mut steps = apply.steps.iter();
        let first = steps
            .next()
            .unwrap_or_else(|| panic!("internal compiler error: flow has no step"));
        let mut acc = Rope::new();
        push_grouped(
            &mut acc,
            guard_line_comment(self.emit_expr(first.value).trim(), 0),
        );
        for step in steps {
            self.used_flow.set(true);
            let body = guard_line_comment(self.emit_expr(step.value).trim(), 0);
            let mut next = Rope::new();
            next.push_lit("$tt_fl(");
            next.append(acc);
            match step.mode {
                ApplyMode::Postfix => {
                    next.push_lit(", (($tt_v) => ($tt_v)");
                    next.append(body);
                    next.push_lit("))");
                }
                ApplyMode::Call => {
                    next.push_lit(", ");
                    push_grouped(&mut next, body);
                    next.push_lit(")");
                }
            }
            acc = next;
        }
        acc
    }

    fn emit_template(&self, template: &Template) -> Rope<'a> {
        let mut out = Rope::new();
        for part in &template.parts {
            match part {
                TemplatePart::Raw(node) => out.append(self.source_rope(*node)),
                TemplatePart::Interpolation(expr) => {
                    out.push_lit("${");
                    out.append(self.emit_expr(*expr));
                    out.push_lit("}");
                }
            }
        }
        out
    }

    fn emit_import(&self, import: &Import, out: &mut Rope<'a>) {
        let (specifier, at) = self.source_node(import.specifier);
        if let hir::ImportKind::Std(module) = import.kind {
            match self.std_imports.get(module) {
                Some(path) => {
                    let quote = &specifier[..1];
                    out.push_lit(format!("{quote}{path}{quote}"));
                }
                None => out.push_src(specifier, at),
            }
            return;
        }
        match self.rewrite_imports {
            ImportRewrite::Off => out.push_src(specifier, at),
            ImportRewrite::Js => {
                let hir::ImportKind::Relative(kind) = import.kind else {
                    unreachable!("standard-library imports returned above")
                };
                let extension = if kind.is_tsx() { "jsx" } else { "js" };
                let suffix_len = if kind.is_tsx() { 5 } else { 4 };
                out.push_src(&specifier[..specifier.len() - suffix_len], at);
                out.push_lit(format!(".{extension}{}", &specifier[specifier.len() - 1..]));
            }
            ImportRewrite::Ts => {
                let hir::ImportKind::Relative(kind) = import.kind else {
                    unreachable!("standard-library imports returned above")
                };
                let extension = kind.output_extension();
                let suffix_len = if kind.is_tsx() { 5 } else { 4 };
                out.push_src(&specifier[..specifier.len() - suffix_len], at);
                out.push_lit(format!(".{extension}{}", &specifier[specifier.len() - 1..]));
            }
        }
    }

    fn emit_statement_decision(&self, decision: &Decision, out: &mut Rope<'a>) {
        let span = self.span(decision.head);
        let (kind, inner) = match &decision.kind {
            DecisionKind::LetElse { binding_mode, .. } => (
                AnchorKind::LetElse,
                self.emit_let_else(decision, *binding_mode),
            ),
            DecisionKind::IfLet => (AnchorKind::IfLet, self.emit_if_let(decision)),
            DecisionKind::Match { .. } => {
                panic!("internal compiler error: expression decision in statement position")
            }
        };
        out.anchored(kind, span.start, span.end, span.end, inner);
    }

    fn emit_let_else(&self, decision: &Decision, mode: BindingMode) -> Rope<'a> {
        let subject = &decision.subjects[0];
        let temp = temp_name(subject.temporary);
        let arm = &decision.arms[0];
        let mut out = Rope::new();
        out.push_lit(format!("const {temp} = "));
        push_grouped(&mut out, self.emit_expr(subject.value).trim());
        out.push_lit("; if (");
        let DecisionKind::LetElse {
            direct_variants, ..
        } = &decision.kind
        else {
            panic!("internal compiler error: let-else has wrong Core decision kind")
        };
        if let Some(variants) = direct_variants {
            for (index, constructor) in variants.iter().enumerate() {
                if index > 0 {
                    out.push_lit(" && ");
                }
                out.push_lit(format!(
                    "{temp}.kind !== \"{}\"",
                    self.constructor_name(constructor)
                ));
            }
        } else {
            out.push_lit("!(");
            out.append(self.emit_condition(&arm.pattern, decision));
            out.push_lit(")");
        }
        out.push_lit(") { ");
        let MissAction::Execute(body) = decision.miss else {
            panic!("internal compiler error: let-else has no else body")
        };
        let body = self.emit_body(body).trim();
        let newline = if body.last_line_has_line_comment() {
            "\n"
        } else {
            ""
        };
        out.append(body);
        out.push_lit(format!("{newline} }}"));
        let mut recovery = BindingRecovery::new(self, &arm.pattern);
        out.append(self.emit_bindings(&arm.pattern, decision, Some(mode), &mut recovery));
        out
    }

    fn emit_if_let(&self, decision: &Decision) -> Rope<'a> {
        let subject = &decision.subjects[0];
        let temp = temp_name(subject.temporary);
        let arm = &decision.arms[0];
        let mut out = Rope::new();
        out.push_lit(format!("{{ const {temp} = "));
        push_grouped(&mut out, self.emit_expr(subject.value).trim());
        out.push_lit("; if (");
        out.append(self.emit_condition(&arm.pattern, decision));
        out.push_lit(") { ");
        let mut recovery = BindingRecovery::new(self, &arm.pattern);
        out.append(self.emit_bindings(&arm.pattern, decision, None, &mut recovery));
        let ArmAction::Execute(body) = arm.action else {
            panic!("internal compiler error: if-let has no then body")
        };
        out.append(guard_line_comment(self.emit_body(body).trim(), 0));
        out.push_lit(" }");
        match &decision.miss {
            MissAction::Execute(body) => {
                out.push_lit(" else { ");
                out.append(guard_line_comment(self.emit_body(*body).trim(), 0));
                out.push_lit(" }");
            }
            MissAction::Decision(inner) => {
                out.push_lit(" else ");
                out.append(self.emit_if_let(inner));
            }
            MissAction::Nothing => {}
            MissAction::ThrowUnexpected(_) => {
                panic!("internal compiler error: if-let has match miss action")
            }
        }
        out.push_lit(" }");
        out
    }

    fn emit_value_decision(
        &self,
        decision: &Decision,
        continuation: &ValueContinuation<'_>,
        exits: &[HostExit],
    ) -> Rope<'a> {
        let DecisionKind::Match { dispatch, .. } = decision.kind else {
            panic!("internal compiler error: value decision is not a match")
        };
        let mut out = Rope::new();
        let label = continuation
            .assignment_target()
            .filter(|_| decision_has_block_arm(decision))
            .map(exit_label);
        if let Some(label) = &label {
            out.push_lit(format!("{label}: "));
        }
        if continuation.is_expression() {
            self.used_expression_boundary.set(true);
            out.push_lit(if decision.is_async {
                format!("(await {}(async () => {{", self.expression_boundary_name)
            } else {
                format!("{}(() => {{", self.expression_boundary_name)
            });
        } else {
            out.push_lit("{");
        }
        for subject in &decision.subjects {
            out.push_break(1);
            out.push_lit("const ");
            out.push_mark(self.span(decision.head).start);
            out.push_lit(format!("{} = ", temp_name(subject.temporary)));
            push_grouped(&mut out, self.emit_expr(subject.value).trim());
            out.push_lit(";");
        }
        out.append(Rope::indented(
            1,
            match dispatch {
                MatchDispatch::Conditional => {
                    self.emit_if_chain(decision, continuation, exits, label.as_deref())
                }
                MatchDispatch::VariantSwitch | MatchDispatch::LiteralSwitch => {
                    self.emit_switch(decision, continuation, exits, label.as_deref())
                }
            },
        ));
        out.push_break(0);
        out.push_lit(if continuation.is_expression() {
            if decision.is_async { "}))" } else { "})" }
        } else {
            "}"
        });
        out
    }

    fn emit_continued_expr(
        &self,
        expr: ExprId,
        continuation: &ValueContinuation<'_>,
    ) -> Option<Rope<'a>> {
        if !can_structure_value_expr(self.core, expr) {
            return None;
        }
        match &self.core.exprs[expr.index()] {
            Expr::Decision(decision) => {
                let head = self.span(decision.head);
                let extent = self.span(decision.extent);
                let exits = self
                    .value_exits
                    .get(&expr)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let lowered = self.emit_value_decision(decision, continuation, exits);
                let mut out = Rope::new();
                out.anchored(AnchorKind::Match, head.start, head.end, extent.end, lowered);
                Some(out)
            }
            Expr::ResultRegion(region) => {
                Some(self.emit_result_region_continued(region, continuation))
            }
            Expr::Sequence(body) => self.emit_sequence_continued(*body, continuation),
            Expr::Apply(apply) => self.emit_apply_continued(expr, apply, continuation),
            Expr::Opaque(_) | Expr::Template(_) => None,
        }
    }

    fn emit_apply_continued(
        &self,
        expr: ExprId,
        apply: &Apply,
        continuation: &ValueContinuation<'_>,
    ) -> Option<Rope<'a>> {
        if !continuation.assigns() {
            return None;
        }
        let head = apply.head?;
        let accumulator = self.value_slots.get(&expr).unwrap_or_else(|| {
            panic!("internal compiler error: structured apply has no value slot")
        });
        let accumulator_is_host_slot = self
            .slot_exprs
            .get(&expr)
            .is_some_and(|slot| slot == accumulator);
        let mut inner = Rope::new();
        inner.push_lit("do {");
        if !accumulator_is_host_slot {
            inner.push_break(1);
            inner.push_lit(format!("let {accumulator};"));
        }
        inner.push_break(1);
        if can_structure_value_expr(self.core, head) {
            inner.append(Rope::indented(
                1,
                self.emit_continued_expr(head, &ValueContinuation::assign(accumulator))
                    .unwrap_or_else(|| {
                        panic!("internal compiler error: structured apply head was not emitted")
                    }),
            ));
        } else {
            inner.push_lit(format!("{accumulator} = "));
            push_grouped(
                &mut inner,
                guard_line_comment(self.emit_expr(head).trim(), 1),
            );
            inner.push_lit(";");
        }
        self.used_pipe.set(true);
        for step in &apply.steps {
            let step_value = if can_structure_value_expr(self.core, step.value) {
                let slot = self.structured_value_slot(step.value).unwrap_or_else(|| {
                    panic!("internal compiler error: structured apply step has no value slot")
                });
                inner.push_break(1);
                inner.push_lit(format!("let {slot};"));
                inner.push_break(1);
                inner.append(Rope::indented(
                    1,
                    self.emit_continued_expr(step.value, &ValueContinuation::assign(slot))
                        .unwrap_or_else(|| {
                            panic!("internal compiler error: structured apply step was not emitted")
                        }),
                ));
                let mut value = Rope::new();
                value.push_lit(slot.clone());
                value
            } else {
                guard_line_comment(self.emit_expr(step.value).trim(), 1)
            };
            inner.push_break(1);
            match step.mode {
                ApplyMode::Postfix => {
                    inner.push_lit(format!("{accumulator} = {accumulator}"));
                    inner.append(step_value);
                    inner.push_lit(";");
                }
                ApplyMode::Call => {
                    inner.push_lit(format!("{accumulator} = $tt_ap({accumulator}, "));
                    push_grouped(&mut inner, step_value);
                    inner.push_lit(");");
                }
            }
        }
        inner.push_break(1);
        if continuation.is_unwrapped_assignment_to(accumulator) {
            inner.push_lit("break;");
        } else {
            let mut value = Rope::new();
            value.push_lit(accumulator.clone());
            inner.append(self.emit_value_delivery(value, None, continuation));
        }
        inner.push_break(0);
        inner.push_lit("} while (false);");
        let start = self.span(apply.node).start;
        let end = apply.steps.last().map_or_else(
            || self.span(apply.node).end,
            |step| self.span(step.node).end,
        );
        let mut out = Rope::new();
        out.anchored(AnchorKind::Pipe, start, end, end, inner);
        Some(out)
    }

    fn emit_switch(
        &self,
        decision: &Decision,
        continuation: &ValueContinuation<'_>,
        exits: &[HostExit],
        exit_label: Option<&str>,
    ) -> Rope<'a> {
        let DecisionKind::Match { dispatch, .. } = decision.kind else {
            panic!("internal compiler error: switch decision is not a match")
        };
        let literal = dispatch == MatchDispatch::LiteralSwitch;
        let temp = temp_name(decision.subjects[0].temporary);
        let mut out = Rope::new();
        out.push_break(0);
        out.push_lit(if literal {
            format!("switch ({temp}) {{")
        } else {
            format!("switch ({temp}.kind) {{")
        });
        let mut wildcard = false;
        for arm in &decision.arms {
            out.push_break(1);
            if matches!(arm.pattern, PatternPlan::Any) {
                wildcard = true;
                out.push_lit("default");
            } else {
                let alternatives = pattern_alternatives(&arm.pattern);
                for (index, alternative) in alternatives.iter().enumerate() {
                    out.push_lit(if index == 0 { "case " } else { ": case " });
                    if pattern_has_literal_test(alternative) {
                        out.append(self.literal_label(alternative));
                    } else {
                        out.push_lit(format!("\"{}\"", self.variant_label(alternative)));
                    }
                }
            }
            let mut recovery = BindingRecovery::new(self, &arm.pattern);
            out.push_lit(": { ");
            out.append(self.emit_bindings(&arm.pattern, decision, None, &mut recovery));
            self.emit_arm_action(
                arm,
                &ArmEmissionContext {
                    depth: 1,
                    chain: false,
                    continuation,
                    exits,
                    exit_label,
                },
                &mut out,
            );
        }
        if !wildcard {
            out.push_break(1);
            out.push_lit(unexpected_switch(literal));
        }
        out.push_break(0);
        out.push_lit("}");
        out
    }

    fn emit_if_chain(
        &self,
        decision: &Decision,
        continuation: &ValueContinuation<'_>,
        exits: &[HostExit],
        exit_label: Option<&str>,
    ) -> Rope<'a> {
        let DecisionKind::Match { needs_label, .. } = decision.kind else {
            panic!("internal compiler error: conditional decision is not a match")
        };
        let mut out = Rope::new();
        let mut depth = 0;
        if continuation.assigns() {
            out.push_break(depth);
            out.push_lit("do {");
            depth += 1;
        }
        if needs_label {
            out.push_break(depth);
            out.push_lit("$tt_b: {");
            depth += 1;
        }
        let mut unconditional = false;
        for arm in &decision.arms {
            let is_any = !pattern_has_test(&arm.pattern);
            out.push_break(depth);
            if is_any && arm.guard.is_none() {
                unconditional = true;
            } else {
                out.push_lit("if (");
                out.append(self.emit_condition(&arm.pattern, decision));
                out.push_lit(") { ");
                let mut recovery = BindingRecovery::new(self, &arm.pattern);
                out.append(self.emit_bindings(&arm.pattern, decision, None, &mut recovery));
            }
            self.emit_arm_action(
                arm,
                &ArmEmissionContext {
                    depth,
                    chain: true,
                    continuation,
                    exits,
                    exit_label,
                },
                &mut out,
            );
            if !is_any || arm.guard.is_some() {
                out.push_lit(" }");
            }
        }
        if !unconditional {
            out.push_break(depth);
            out.push_lit(self.unexpected_throw(decision));
        }
        if needs_label {
            depth -= 1;
            out.push_break(depth);
            out.push_lit("}");
        }
        if continuation.assigns() {
            depth -= 1;
            out.push_break(depth);
            out.push_lit("} while (false);");
        }
        out
    }

    fn emit_arm_action(
        &self,
        arm: &DecisionArm,
        context: &ArmEmissionContext<'_, '_>,
        out: &mut Rope<'a>,
    ) {
        let depth = context.depth;
        let chain = context.chain;
        let continuation = context.continuation;
        let exits = context.exits;
        let exit_label = context.exit_label;
        let ArmAction::Yield { body, kind } = arm.action else {
            panic!("internal compiler error: match arm does not yield")
        };
        let body_expr = match self.core.bodies[body.index()].statements.as_slice() {
            [Statement::Expr(expr)] => Some(*expr),
            _ => None,
        };
        let body = if kind == ArmBodyKind::Block && continuation.assigns() {
            let label = exit_label.unwrap_or_else(|| {
                panic!("internal compiler error: statement arm has no exit label")
            });
            self.emit_body_with_exits(body, exits, continuation, label)
                .trim()
        } else {
            self.emit_body(body).trim()
        };
        let mut action = Rope::new();
        match kind {
            ArmBodyKind::Expression => {
                if let Some(structured) = body_expr.and_then(|expr| {
                    (!continuation.is_expression())
                        .then(|| self.emit_continued_expr(expr, continuation))
                        .flatten()
                }) {
                    action.append(structured);
                    if continuation.assigns() {
                        action.push_lit(" break;");
                    }
                } else {
                    let close = body.last_line_has_line_comment().then_some(depth);
                    action.append(self.emit_value_delivery(body, close, continuation));
                }
            }
            ArmBodyKind::Block if chain => {
                action.push_lit("{ ");
                action.append(body);
                if continuation.assigns() {
                    action.push_break(depth + 1);
                    action.push_lit(format!(
                        "{} = undefined;",
                        continuation.assignment_target().unwrap()
                    ));
                }
                action.push_break(depth + 1);
                action.push_lit("break $tt_b;");
                action.push_break(depth);
                action.push_lit("}");
            }
            ArmBodyKind::Block => {
                action.append(body);
                if continuation.assigns() {
                    action.push_break(depth + 1);
                    action.push_lit(format!(
                        "{} = undefined;",
                        continuation.assignment_target().unwrap()
                    ));
                }
                action.push_break(depth + 1);
                action.push_lit("break;");
            }
        }
        if let Some(guard) = arm.guard {
            let guard = self.emit_expr(guard).trim();
            let guarded = guard.last_line_has_line_comment();
            out.push_lit("if (");
            out.append(guard);
            if guarded {
                out.push_break(depth);
            }
            out.push_lit(") ");
            if continuation.assigns() {
                out.push_lit("{ ");
            }
        }
        out.append(action);
        if arm.guard.is_some() && continuation.assigns() {
            out.push_lit(" }");
        }
        if !chain {
            out.push_lit(" }");
        }
    }

    /// Delivers one value to its continuation. `close` breaks the line
    /// before the closing `);` at that depth — the delivered body ends with
    /// a `//` comment that would otherwise swallow it.
    fn emit_value_delivery(
        &self,
        body: Rope<'a>,
        close: Option<u16>,
        continuation: &ValueContinuation<'_>,
    ) -> Rope<'a> {
        let mut value = body;
        for wrapper in continuation.wrappers.iter().rev() {
            match wrapper {
                ValueWrapper::ResultOk => {
                    let mut wrapped = Rope::new();
                    wrapped.push_lit("{ kind: \"Ok\" as const, value: ");
                    push_grouped(&mut wrapped, value);
                    wrapped.push_lit(" }");
                    value = wrapped;
                }
            }
        }
        let grouped = needs_grouping(&value);
        let mut out = Rope::new();
        match continuation.destination {
            ValueDestination::Expression => out.push_lit("return "),
            ValueDestination::Assign(target) => out.push_lit(format!("{target} = ")),
        }
        if grouped {
            out.push_lit("(");
        }
        out.append(value);
        if let Some(depth) = close {
            out.push_break(depth);
        }
        if grouped {
            out.push_lit(")");
        }
        out.push_lit(";");
        if continuation.assigns() {
            out.push_lit(" break;");
        }
        out
    }

    fn emit_condition(&self, plan: &PatternPlan, decision: &Decision) -> Rope<'a> {
        match plan {
            PatternPlan::Any | PatternPlan::Bind(_) => Rope::new(),
            PatternPlan::Test(test) => self.emit_test(test, decision),
            PatternPlan::AllOf(parts) => {
                let mut out = Rope::new();
                let tests = parts
                    .iter()
                    .filter(|part| pattern_has_test(part))
                    .collect::<Vec<_>>();
                for (index, part) in tests.iter().enumerate() {
                    if index > 0 {
                        out.push_lit(" && ");
                    }
                    let parenthesize = matches!(part, PatternPlan::AnyOf(_));
                    if parenthesize {
                        out.push_lit("(");
                    }
                    out.append(self.emit_condition(part, decision));
                    if parenthesize {
                        out.push_lit(")");
                    }
                }
                out
            }
            PatternPlan::AnyOf(parts) => {
                let mut out = Rope::new();
                for (index, part) in parts.iter().enumerate() {
                    if index > 0 {
                        out.push_lit(" || ");
                    }
                    out.append(self.emit_condition(part, decision));
                }
                out
            }
        }
    }

    fn emit_test(&self, test: &Test, decision: &Decision) -> Rope<'a> {
        match test {
            Test::Variant { place, constructor } => {
                let mut out = self.emit_place(place, decision, Some(constructor_node(constructor)));
                out.push_lit(format!(
                    ".kind === \"{}\"",
                    self.constructor_name(constructor)
                ));
                out
            }
            Test::Literal { place, pattern } => {
                let mut out = self.emit_place(place, decision, None);
                out.push_lit(" === ");
                let span = self
                    .semantic
                    .hir
                    .source_map
                    .pattern_span(*pattern)
                    .unwrap_or_else(|| panic!("internal compiler error: literal has no span"));
                let (literal, at) = self.source_span(span);
                out.push_src(literal, at);
                out
            }
        }
    }

    fn emit_place(
        &self,
        place: &Place,
        decision: &Decision,
        payload_for: Option<NodeId>,
    ) -> Rope<'a> {
        let mut out = Rope::new();
        out.push_lit(temp_name(decision.subjects[place.subject].temporary));
        for (index, field) in place.fields.iter().enumerate() {
            out.push_lit(".");
            if index + 1 == place.fields.len()
                && let Some(node) = payload_for
            {
                out.push_payload_mark(self.span(node).start);
            }
            out.push_lit(self.field_name(field));
        }
        out
    }

    fn emit_bindings(
        &self,
        plan: &PatternPlan,
        decision: &Decision,
        declaration: Option<BindingMode>,
        recovery: &mut BindingRecovery,
    ) -> Rope<'a> {
        let selected = if let PatternPlan::AnyOf(parts) = plan {
            parts.first().unwrap_or(plan)
        } else {
            plan
        };
        let mut groups: Vec<BindingGroup<'_>> = Vec::new();
        collect_binding_groups(
            selected,
            !matches!(plan, PatternPlan::AnyOf(_)),
            &mut groups,
        );
        let mut out = Rope::new();
        for (group_index, (receiver, bindings)) in groups.into_iter().enumerate() {
            if group_index == 0 {
                if let Some(mode) = declaration {
                    out.push_lit(format!(" {} {{ ", binding_keyword(mode)));
                } else {
                    out.push_lit("const { ");
                }
            } else {
                out.push_lit("const { ");
            }
            for (index, (binding, mapped)) in bindings.iter().enumerate() {
                if index > 0 {
                    out.push_lit(", ");
                }
                self.emit_binding(binding, *mapped, recovery, &mut out);
            }
            out.push_lit(" } = ");
            out.append(self.emit_place(&receiver, decision, None));
            out.push_lit(if declaration.is_some() { ";" } else { "; " });
        }
        out
    }

    fn emit_binding(
        &self,
        binding: &Bind,
        mapped: bool,
        recovery: &mut BindingRecovery,
        out: &mut Rope<'a>,
    ) {
        let field = binding
            .source
            .fields
            .last()
            .unwrap_or_else(|| panic!("internal compiler error: binding has no source field"));
        let field_node = field_node(field);
        let field_text = self.field_name(field);
        if mapped {
            let span = self.span(field_node);
            let (text, at) = self.source_span(span);
            out.push_src(text, at);
        } else {
            out.push_lit(field_text);
        }
        if let Some(replacement) = recovery.replacement(self, binding) {
            out.push_lit(format!(": {replacement}"));
        } else if binding.binding != field_node {
            out.push_lit(": ");
            if mapped {
                out.append(self.source_rope(binding.binding));
            } else {
                out.push_lit(self.source_node(binding.binding).0.to_owned());
            }
        }
    }

    fn constructor_name(&self, constructor: &Constructor) -> String {
        self.source_node(constructor_node(constructor)).0.to_owned()
    }

    fn field_name(&self, field: &FieldAccess) -> String {
        self.source_node(field_node(field)).0.to_owned()
    }

    fn literal_label(&self, plan: &PatternPlan) -> Rope<'a> {
        let PatternPlan::Test(Test::Literal { pattern, .. }) = plan else {
            panic!("internal compiler error: switch literal alternative is not literal")
        };
        let span = self.semantic.hir.source_map.pattern_span(*pattern).unwrap();
        let (text, at) = self.source_span(span);
        let mut out = Rope::new();
        out.push_src(text, at);
        out
    }

    fn variant_label(&self, plan: &PatternPlan) -> String {
        let PatternPlan::AllOf(parts) = plan else {
            panic!("internal compiler error: switch variant alternative is not constructor")
        };
        let constructor = parts.iter().find_map(|part| match part {
            PatternPlan::Test(Test::Variant { constructor, .. }) => Some(constructor),
            _ => None,
        });
        self.constructor_name(constructor.unwrap())
    }

    fn unexpected_throw(&self, decision: &Decision) -> String {
        match decision.miss {
            MissAction::ThrowUnexpected(UnexpectedKind::Tuple) => {
                let temps = decision
                    .subjects
                    .iter()
                    .map(|subject| temp_name(subject.temporary))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "throw new Error(\"tt match: unexpected case \" + JSON.stringify([{temps}]));"
                )
            }
            MissAction::ThrowUnexpected(UnexpectedKind::Literal) => {
                "throw new Error(\"tt match: unexpected literal \" + JSON.stringify($tt_m));"
                    .to_owned()
            }
            MissAction::ThrowUnexpected(UnexpectedKind::Case) => {
                "throw new Error(\"tt match: unexpected case \" + JSON.stringify($tt_m));"
                    .to_owned()
            }
            _ => panic!("internal compiler error: match has non-match miss action"),
        }
    }
}

/// Appends `value` to `out` in a position that can only regroup it across
/// a comma — an initializer, an assignment right-hand side, a `return`
/// operand, or a single call argument — wrapping it in parentheses only
/// when it has a top-level comma to protect.
///
/// The parentheses codegen writes around a lowered value are grouping, not
/// syntax: everything else in the expression binds tighter than the
/// position it lands in, so the pair is noise the reader has to see past.
/// A value whose text is not resolved yet (it carries layout breaks, so it
/// is a lowering rather than one expression) keeps its parentheses.
fn push_grouped<'a>(out: &mut Rope<'a>, value: Rope<'a>) {
    if needs_grouping(&value) {
        out.push_lit("(");
        out.append(value);
        out.push_lit(")");
    } else {
        out.append(value);
    }
}

/// Appends `value` as the receiver of a postfix step (`value.map(f)`).
/// Member access binds tighter than every operator, so the parentheses are
/// needed unless the receiver is already one primary expression.
fn push_receiver<'a>(out: &mut Rope<'a>, value: Rope<'a>) {
    let primary = value
        .resolved_text()
        .is_some_and(|text| crate::scanner::is_primary_expression(text.as_bytes(), 0, text.len()));
    if primary {
        out.append(value);
    } else {
        out.push_lit("(");
        out.append(value);
        out.push_lit(")");
    }
}

/// Whether a value delivered to one of those positions has to keep the
/// parentheses codegen wraps it in. See [`push_grouped`].
fn needs_grouping(value: &Rope<'_>) -> bool {
    match value.resolved_text() {
        Some(text) => grouping_required(&text),
        None => true,
    }
}

/// The same question about text codegen has not yet made a rope of.
fn grouping_required(text: &str) -> bool {
    crate::scanner::has_top_level_comma(text.as_bytes(), 0, text.len())
}

/// Ends the line when `rope` finishes inside a `//` comment, so whatever
/// codegen appends next is not swallowed by it. `depth` is where the
/// continued line starts inside the enclosing lowering.
fn guard_line_comment(mut rope: Rope<'_>, depth: u16) -> Rope<'_> {
    if rope.last_line_has_line_comment() {
        rope.push_break(depth);
    }
    rope
}

fn trimmed_end(source: &str, start: usize, end: usize) -> usize {
    start + source[start..end].trim_end().len()
}

fn binding_keyword(mode: BindingMode) -> &'static str {
    match mode {
        BindingMode::Const => "const",
        BindingMode::Let => "let",
        BindingMode::Var => "var",
    }
}

fn temp_name(temp: TempId) -> String {
    match temp {
        TempId::Statement(sequence) => format!("$tt_t{sequence}"),
        TempId::Result(sequence) => format!("$tt_r{sequence}"),
        TempId::Decision => "$tt_m".to_owned(),
        TempId::DecisionElement(sequence) => format!("$tt_m{sequence}"),
    }
}

fn constructor_node(constructor: &Constructor) -> NodeId {
    match constructor {
        Constructor::Resolved { node, .. } | Constructor::Recovery { node, .. } => *node,
    }
}

fn field_node(field: &FieldAccess) -> NodeId {
    match field {
        FieldAccess::Resolved { node, .. } | FieldAccess::Recovery { node, .. } => *node,
    }
}

fn pattern_has_test(plan: &PatternPlan) -> bool {
    match plan {
        PatternPlan::Any | PatternPlan::Bind(_) => false,
        PatternPlan::Test(_) => true,
        PatternPlan::AllOf(parts) | PatternPlan::AnyOf(parts) => parts.iter().any(pattern_has_test),
    }
}

fn pattern_has_literal_test(plan: &PatternPlan) -> bool {
    match plan {
        PatternPlan::Test(Test::Literal { .. }) => true,
        PatternPlan::AllOf(parts) | PatternPlan::AnyOf(parts) => {
            parts.iter().any(pattern_has_literal_test)
        }
        PatternPlan::Any | PatternPlan::Bind(_) | PatternPlan::Test(Test::Variant { .. }) => false,
    }
}

fn pattern_alternatives(plan: &PatternPlan) -> Vec<&PatternPlan> {
    match plan {
        PatternPlan::AnyOf(parts) => parts.iter().collect(),
        _ => vec![plan],
    }
}

type BindingGroup<'a> = (Place, Vec<(&'a Bind, bool)>);

fn collect_binding_groups<'a>(
    plan: &'a PatternPlan,
    mapped: bool,
    groups: &mut Vec<BindingGroup<'a>>,
) {
    match plan {
        PatternPlan::Bind(binding) => {
            let mut receiver = binding.source.clone();
            receiver.fields.pop();
            if let Some((_, bindings)) = groups
                .iter_mut()
                .find(|(existing, _)| same_place(existing, &receiver))
            {
                bindings.push((binding, mapped));
            } else {
                groups.push((receiver, vec![(binding, mapped)]));
            }
        }
        PatternPlan::AllOf(parts) => {
            for part in parts
                .iter()
                .filter(|part| matches!(part, PatternPlan::Bind(_)))
            {
                collect_binding_groups(part, mapped, groups);
            }
            for part in parts
                .iter()
                .filter(|part| !matches!(part, PatternPlan::Bind(_)))
            {
                collect_binding_groups(part, mapped, groups);
            }
        }
        PatternPlan::AnyOf(parts) => {
            if let Some(first) = parts.first() {
                collect_binding_groups(first, false, groups);
            }
        }
        PatternPlan::Any | PatternPlan::Test(_) => {}
    }
}

fn same_place(left: &Place, right: &Place) -> bool {
    left.subject == right.subject
        && left.fields.len() == right.fields.len()
        && left
            .fields
            .iter()
            .zip(&right.fields)
            .all(|(left, right)| field_node(left) == field_node(right))
}

struct BindingRecovery {
    available: HashSet<String>,
    emitted: HashSet<String>,
    discard_sequence: usize,
}

impl BindingRecovery {
    fn new(emitter: &Emitter<'_>, plan: &PatternPlan) -> BindingRecovery {
        let selected = if let PatternPlan::AnyOf(parts) = plan {
            parts.first().unwrap_or(plan)
        } else {
            plan
        };
        let mut groups = Vec::new();
        collect_binding_groups(
            selected,
            !matches!(plan, PatternPlan::AnyOf(_)),
            &mut groups,
        );
        let available = groups
            .into_iter()
            .flat_map(|(_, bindings)| bindings)
            .map(|(binding, _)| emitter.source_node(binding.binding).0.to_owned())
            .collect();
        BindingRecovery {
            available,
            emitted: HashSet::new(),
            discard_sequence: 0,
        }
    }

    fn replacement(&mut self, emitter: &Emitter<'_>, binding: &Bind) -> Option<String> {
        let name = emitter.source_node(binding.binding).0;
        if self.emitted.insert(name.to_owned()) {
            return None;
        }
        loop {
            let candidate = format!("$tt_discard{}", self.discard_sequence);
            self.discard_sequence += 1;
            if self.available.insert(candidate.clone()) {
                return Some(candidate);
            }
        }
    }
}

fn unexpected_switch(literal: bool) -> &'static str {
    if literal {
        "default: { throw new Error(\"tt match: unexpected literal \" + JSON.stringify($tt_m)); }"
    } else {
        "default: { throw new Error(\"tt match: unexpected case \" + JSON.stringify($tt_m)); }"
    }
}

/// The union type and constructor object one tt `enum` becomes, laid out
/// from the line the declaration sits on.
fn emit_adt<'a>(adt: &Adt) -> Rope<'a> {
    let export = if adt.exported { "export " } else { "" };
    let arms = adt
        .variants
        .iter()
        .map(|variant| match &variant.fields {
            Some(fields) if !fields.is_empty() => format!(
                "{{ kind: \"{}\"; {} }}",
                variant.name,
                fields
                    .iter()
                    .map(|field| format!(
                        "{}{}: {}",
                        field.name,
                        if field.optional { "?" } else { "" },
                        field.ty_text
                    ))
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            _ => format!("{{ kind: \"{}\" }}", variant.name),
        })
        .collect::<Vec<_>>();
    let type_args = if adt.generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", generic_param_names(&adt.generics).join(", "))
    };
    let constructors = adt
        .variants
        .iter()
        .filter_map(|variant| {
            if !variant.emit_constructor {
                return None;
            }
            Some(match &variant.fields {
                None => format!(
                    "{}: {{ kind: \"{}\" }} as const,",
                    variant.name, variant.name
                ),
                Some(fields) => {
                    let params = fields
                        .iter()
                        .map(|field| {
                            format!(
                                "{}{}: {}",
                                field.name,
                                if field.optional { "?" } else { "" },
                                field.ty_text
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    let object = std::iter::once(format!("kind: \"{}\"", variant.name))
                        .chain(fields.iter().map(|field| field.name.clone()))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!(
                        "{}: {}({params}): {}{type_args} => ({{ {object} }}),",
                        variant.name, adt.generics, adt.name
                    )
                }
            })
        })
        .collect::<Vec<_>>();
    let mut out = Rope::new();
    out.push_lit(format!("{export}type {}{} =", adt.name, adt.generics));
    for arm in arms {
        out.push_break(1);
        out.push_lit(format!("| {arm}"));
    }
    out.push_lit(";");
    out.push_break(0);
    out.push_lit(format!("{export}const {} = {{", adt.name));
    for constructor in constructors {
        out.push_break(1);
        out.push_lit(constructor);
    }
    out.push_break(0);
    out.push_lit("};");
    Rope::scoped(out)
}

fn generic_param_names(generics: &str) -> Vec<String> {
    let inner = &generics[1..generics.len() - 1];
    let source = inner.as_bytes();
    let mut names = Vec::new();
    let mut index = 0usize;
    while index < source.len() {
        index = skip_ws_comments(source, index, source.len());
        if index >= source.len() || !is_ident_start(source[index]) {
            break;
        }
        let end = ident_end(source, index, source.len());
        let word = &inner[index..end];
        if word == "const" || word == "in" || word == "out" {
            index = end;
            continue;
        }
        names.push(word.to_owned());
        index = scan_type_end(source, end, source.len());
        if at(source, index, source.len()) == Some(b',') {
            index += 1;
        }
    }
    names
}
