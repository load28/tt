//! TypeScript target lowering from validated Core IR.
//!
//! This module is intentionally independent of `ast`: source text enters
//! only through HIR nodes and the source map. Every tt surface reaches this
//! module through a shared Core primitive.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

use super::rope::{Flat, Rope, SourcePreservation};
use crate::analysis::SemanticFile;
use crate::core_ir::*;
use crate::evaluation_ir::{
    EvaluationSchedule, LoweringPlan, PlannedBranch, PlannedConditionalKind,
    PlannedConditionalOperation, PlannedEvaluationInput, PlannedEvaluationStep, PlannedOperand,
    PlannedReceiver, TargetCapability, ValueTarget,
};
use crate::hir::ids::Idx;
use crate::hir::{self, ArmBodyKind, BindingMode, ExprId, NodeId};
use crate::program_syntax::{
    ConditionalBranch, EvaluationInputMode, HostContinuation, HostEvaluationOperation, HostExit,
    HostOwnerKind, LoopTestKind, SourceSpan,
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
pub(crate) enum LoweringFailure {
    /// The syntax substrate's message and source byte where parsing stopped.
    SourceNotTypeScript { message: String, source: usize },
    /// A host projection failure before Evaluation IR planning begins.
    HostProjection {
        error: crate::program_syntax::ProgramSyntaxError,
        source: SourceSpan,
    },
    /// A fallible Evaluation IR construction or planning failure, located at
    /// the first tt construct participating in the failed lowering.
    Evaluation {
        error: crate::evaluation_ir::EvaluationError,
        source: SourceSpan,
    },
}

/// Builds the host-lowering plan for a file, ahead of emission.
///
/// Emission is infallible by contract (`docs/design/compiler-architecture.md`),
/// so the fallible half — projecting the file to TypeScript, joining every tt
/// value to its owner, and planning the rewrites — is this separate step, run
/// by the phase that can report. Every failure but
/// [`LoweringFailure`] is reported by the phase that owns diagnostics.
pub(crate) fn lowering_plan(
    semantic: &SemanticFile,
    core: &CoreFile,
    source: &str,
    source_kind: SourceKind,
) -> Result<LoweringPlan, LoweringFailure> {
    if !core.requires_host_lowering() {
        return Ok(LoweringPlan::default());
    }
    let primary_source = || {
        semantic
            .hir
            .source_map
            .first_node_span()
            .map(SourceSpan::from)
            .unwrap_or(SourceSpan {
                start: 0,
                end: source.len(),
            })
    };
    let syntax =
        match crate::program_syntax::ProgramSyntax::build(semantic, core, source, source_kind) {
            Ok(syntax) => syntax,
            Err(crate::program_syntax::ProgramSyntaxError::SourceNotTypeScript {
                message,
                source,
            }) => return Err(LoweringFailure::SourceNotTypeScript { message, source }),
            Err(error) => {
                return Err(LoweringFailure::HostProjection {
                    error,
                    source: primary_source(),
                });
            }
        };
    let evaluation =
        crate::evaluation_ir::EvaluationFile::build(&syntax, core).map_err(|error| {
            LoweringFailure::Evaluation {
                error,
                source: primary_source(),
            }
        })?;
    let plan = evaluation
        .lowering_plan(core)
        .map_err(|error| LoweringFailure::Evaluation {
            error,
            source: evaluation.primary_source(),
        })?;
    // The plan validators are pipeline stages, not tests: a violated
    // evaluation contract fails the build here, before emission starts
    // (`docs/design/program-lowering.md` §11).
    if let Err(error) = evaluation.validate_order(&plan) {
        error.raise();
    }
    if let Err(error) = evaluation.validate_reference(&plan) {
        error.raise();
    }
    Ok(plan)
}

pub(crate) fn emit_with_map<'a>(
    semantic: &'a SemanticFile,
    core: &'a CoreFile,
    source: &'a str,
    source_kind: SourceKind,
    lowering_plan: &LoweringPlan,
    rewrite_imports: ImportRewrite,
    std_imports: StdImports<'a>,
) -> Flat {
    let target = TargetRewritePlan::build(semantic, core, lowering_plan);
    let direct_apply_inputs = direct_apply_inputs(semantic, core, source, source_kind);
    let mut relocated: Vec<SourceSpan> = target
        .source_replacements
        .iter()
        .map(|replacement| replacement.source)
        .collect();
    relocated.extend(target.relocated_values.iter().copied());
    relocated.extend(direct_apply_inputs.iter().filter_map(|expr| {
        let Expr::Opaque(node) = &core.exprs[expr.index()] else {
            return None;
        };
        semantic
            .hir
            .source_map
            .node_span(*node)
            .map(SourceSpan::from)
    }));
    let rewritten_operations = target.rewritten_operations.clone();
    let result_return_args: Vec<_> = target
        .value_exits
        .iter()
        .filter(|(expr, _)| matches!(core.exprs[expr.index()], Expr::ResultRegion(_)))
        .flat_map(|(_, exits)| exits.iter().filter_map(|exit| exit.argument))
        .collect();
    relocated.extend(result_return_args.iter().copied());
    let target_recovered_propagations: Vec<_> = target
        .recovered_propagations
        .iter()
        .map(|(_, span)| *span)
        .collect();
    let emitter = Emitter {
        semantic,
        core,
        source,
        direct_apply_inputs,
        rewrite_imports,
        std_imports,
        owner_slot_rewrites: target.owner_slots,
        for_initializer_propagations: target.for_initializer_propagations,
        compose_rewrites: target.composes,
        loop_test_rewrites: target.loop_tests,
        source_replacements: target.source_replacements,
        consumed_exprs: target.consumed_exprs,
        arrow_return_rewrites: target.arrow_returns,
        slot_exprs: target.slot_exprs,
        value_slots: target.value_slots,
        scheduled_slots: target.scheduled_slots,
        value_exits: target.value_exits,
        nested_schedules: target.nested_schedules,
        nested_values: target.nested_values,
        recovered_propagations: target
            .recovered_propagations
            .into_iter()
            .map(|(expr, _)| expr)
            .collect(),
        expression_boundary_name: target.expression_boundary_name,
        result_return_name: target.result_return_name,
        conditional_region_depth: Cell::new(0),
        active_structured_exprs: RefCell::new(Vec::new()),
        active_scheduled_exprs: RefCell::new(Vec::new()),
        emitted_owner_rewrites: RefCell::new(HashSet::new()),
        loop_region_depth: Cell::new(0),
        used_expression_boundary: Cell::new(false),
        used_pipe: Cell::new(false),
        used_flow: Cell::new(false),
    };
    let mut output = emitter.emit_body(core.root);
    let used_pipe = emitter.used_pipe.get();
    let used_flow = emitter.used_flow.get();
    if used_pipe || used_flow {
        let names = match (used_pipe, used_flow) {
            (true, true) => "$tt_ap, $tt_fl",
            (true, false) => "$tt_ap",
            (false, true) => "$tt_fl",
            // The enclosing `if` is `used_pipe || used_flow`.
            (false, false) => unreachable!("no helper is needed, so no import is written"),
        };
        let runtime = std_imports
            .get(crate::StdModule::Runtime)
            .unwrap_or_else(|| crate::StdModule::Runtime.specifier());
        // Which helpers the file needs is only known once the whole file
        // is emitted, but where an import belongs is the top — after
        // anything that has to come before one (TASK-219).
        let at = directive_prologue_end(source);
        // A prologue that runs to the end of the file leaves nothing to
        // insert before, so the import lands at the end and needs the
        // line break the source did not write.
        if at >= source.len() && !output.ends_with_newline() {
            output.push_lit("\n");
        }
        output.insert_lit_at_source(at, format!("import {{ {names} }} from \"{runtime}\";\n"));
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
    // A block arm's `return` frame (the keyword, and anything after the
    // argument) is claimed by the exit rewrite, as is the operator frame of
    // a lowered conditional operation; the arguments themselves stay
    // pass-through.
    let result_return_rewrites = emitter.result_return_rewrite_spans();
    let mut rewritten: Vec<SourceSpan> = emitter
        .value_exits
        .values()
        .flatten()
        .flat_map(|exit| match exit.argument {
            Some(argument) => vec![
                SourceSpan {
                    start: exit.statement.start,
                    end: argument.start,
                },
                SourceSpan {
                    start: argument.end,
                    end: exit.statement.end,
                },
            ],
            None => vec![exit.statement],
        })
        .collect();
    rewritten.extend(result_return_rewrites);
    // Result-targeted propagation is emitted as a completion sequence rather
    // than a copied source fragment. Register its complete tt span even when
    // it sits inside a rewritten Result-owned return.
    rewritten.extend(core.exprs.iter().filter_map(|expr| {
        match expr {
            Expr::ResultRegion(region) => semantic
                .hir
                .source_map
                .node_span(region.node)
                .map(SourceSpan::from),
            Expr::Propagate(propagate) if matches!(propagate.exit, ExitTarget::ResultRegion(_)) => {
                semantic
                    .hir
                    .source_map
                    .node_span(propagate.node)
                    .map(SourceSpan::from)
            }
            _ => None,
        }
    }));
    rewritten.extend(rewritten_operations);
    rewritten.extend(target_recovered_propagations);
    let preservation = SourcePreservation {
        owned: pass_through_spans(semantic, core),
        relocated,
        rewritten,
    };
    let mut flat = output.flatten(source, &preservation);
    for result_return in &mut flat.result_return_temps {
        result_return.src_end = result_return_args
            .iter()
            .find(|argument| argument.start == result_return.src)
            .map_or(result_return.src, |argument| argument.end);
    }
    flat
}

/// Inline `$tt_ap(v, f)` as `f(v)` exactly when moving the input behind the
/// callee is proven unobservable. ProgramSyntax owns the effect proof; these
/// ExprIds also register the corresponding source relocation.
/// The byte offset a generated `import` may be written at: past a shebang
/// and past the file's directive prologue.
///
/// A directive (`"use client"`, `"use strict"`) is only a directive while
/// it is the first thing in the file, so an import written above one would
/// silently turn it into a string expression — a bundler would stop seeing
/// the boundary the author declared. Everything else about the top of a
/// file (a license comment, a blank line) is ordinary text an import may
/// precede, so the scan stops at the first statement that is not one of
/// these two.
///
/// ASCII bytes decide, and multi-byte UTF-8 is opaque: a string's contents
/// are skipped by its quotes, not read.
fn directive_prologue_end(source: &str) -> usize {
    let bytes = source.as_bytes();
    let mut at = 0;
    // A shebang is not a statement, but nothing may precede it either.
    if bytes.starts_with(b"#!") {
        at = source.find('\n').map_or(bytes.len(), |nl| nl + 1);
    }
    let mut end = at;
    loop {
        let open = skip_trivia(bytes, at);
        let Some(&quote) = bytes.get(open) else { break };
        if quote != b'"' && quote != b'\'' {
            break;
        }
        let Some(close) = string_literal_end(bytes, open) else {
            break;
        };
        // What follows decides whether that string was a directive or the
        // start of an expression (`"a" + b`).
        let mut after = close;
        while matches!(bytes.get(after), Some(b' ' | b'\t' | b'\r')) {
            after += 1;
        }
        let directive_end = match bytes.get(after) {
            Some(b';') => after + 1,
            None | Some(b'\n') => close,
            _ => break,
        };
        // Past the rest of that line, so what is written next opens a line
        // of its own rather than trailing the directive.
        end = match bytes[directive_end..].iter().position(|&b| b == b'\n') {
            Some(nl) => directive_end + nl + 1,
            None => bytes.len(),
        };
        at = end;
    }
    end
}

/// The offset past whitespace and comments starting at `at`.
fn skip_trivia(bytes: &[u8], mut at: usize) -> usize {
    loop {
        while matches!(bytes.get(at), Some(b) if b.is_ascii_whitespace()) {
            at += 1;
        }
        match (bytes.get(at), bytes.get(at + 1)) {
            (Some(b'/'), Some(b'/')) => {
                at = match bytes[at..].iter().position(|&b| b == b'\n') {
                    Some(nl) => at + nl + 1,
                    None => bytes.len(),
                };
            }
            (Some(b'/'), Some(b'*')) => {
                at = match bytes[at + 2..].windows(2).position(|w| w == b"*/") {
                    Some(close) => at + 2 + close + 2,
                    None => bytes.len(),
                };
            }
            _ => return at,
        }
    }
}

/// The offset just past the string literal opening at `at`, or `None` when
/// it is unterminated.
fn string_literal_end(bytes: &[u8], at: usize) -> Option<usize> {
    let quote = bytes[at];
    let mut i = at + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'\n' => return None,
            b if b == quote => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

fn direct_apply_inputs(
    semantic: &SemanticFile,
    core: &CoreFile,
    source: &str,
    source_kind: SourceKind,
) -> HashSet<ExprId> {
    core.exprs
        .iter()
        .filter_map(|expr| {
            let Expr::Apply(apply) = expr else {
                return None;
            };
            if !matches!(
                apply.steps.first().map(|step| step.mode),
                Some(ApplyMode::Call)
            ) {
                return None;
            }
            let head = apply.head?;
            let Expr::Opaque(node) = &core.exprs[head.index()] else {
                return None;
            };
            let span = semantic.hir.source_map.node_span(*node)?;
            crate::program_syntax::source_expression_effects(source, span, source_kind)
                .is_inert()
                .then_some(head)
        })
        .collect()
}

/// The pass-through ranges of a file, for the target's preservation check
/// ([`SourcePreservation::owned`]): the source the compiler does not
/// interpret — Core `Opaque` statements and expressions and template raw
/// parts, wherever they sit. Read off the Core IR — the same structure the
/// emitter walks — never off the output.
fn pass_through_spans(semantic: &SemanticFile, core: &CoreFile) -> Vec<SourceSpan> {
    fn span(semantic: &SemanticFile, node: NodeId, out: &mut Vec<SourceSpan>) {
        let span = semantic
            .hir
            .source_map
            .node_span(node)
            .unwrap_or_else(|| crate::ice::bug!("target node has no source span"));
        out.push(span.into());
    }

    fn walk_body(
        semantic: &SemanticFile,
        core: &CoreFile,
        body: hir::BodyId,
        out: &mut Vec<SourceSpan>,
    ) {
        for statement in &core.bodies[body.index()].statements {
            match statement {
                Statement::Opaque(node) => span(semantic, *node, out),
                Statement::Adt(_) | Statement::Import(_) => {}
                Statement::Propagate(propagate) => {
                    walk_expr(semantic, core, propagate.value, out);
                }
                Statement::Decision(decision) => walk_decision(semantic, core, decision, out),
                Statement::Expr(expr) => walk_expr(semantic, core, *expr, out),
            }
        }
    }

    fn walk_decision(
        semantic: &SemanticFile,
        core: &CoreFile,
        decision: &Decision,
        out: &mut Vec<SourceSpan>,
    ) {
        for subject in &decision.subjects {
            walk_expr(semantic, core, subject.value, out);
        }
        for arm in &decision.arms {
            if let Some(guard) = arm.guard {
                walk_expr(semantic, core, guard, out);
            }
            match arm.action {
                ArmAction::Yield { body, .. } | ArmAction::Execute(body) => {
                    walk_body(semantic, core, body, out);
                }
                ArmAction::BindThrough(_) => {}
            }
        }
        match &decision.miss {
            MissAction::Execute(body) => walk_body(semantic, core, *body, out),
            MissAction::Decision(inner) => walk_decision(semantic, core, inner, out),
            MissAction::ThrowUnexpected(_) | MissAction::Nothing => {}
        }
    }

    fn walk_expr(
        semantic: &SemanticFile,
        core: &CoreFile,
        expr: ExprId,
        out: &mut Vec<SourceSpan>,
    ) {
        match &core.exprs[expr.index()] {
            Expr::Opaque(node) => span(semantic, *node, out),
            Expr::Sequence(body) => walk_body(semantic, core, *body, out),
            Expr::Decision(decision) => walk_decision(semantic, core, decision, out),
            Expr::Propagate(propagate) => {
                walk_expr(semantic, core, propagate.value, out);
            }
            Expr::Apply(apply) => {
                if let Some(head) = apply.head {
                    walk_expr(semantic, core, head, out);
                }
                for step in &apply.steps {
                    walk_expr(semantic, core, step.value, out);
                }
            }
            Expr::ResultRegion(region) => {
                for item in &region.items {
                    let ResultRegionItem::Statements(body) = item;
                    walk_body(semantic, core, *body, out);
                }
                if let Some(value) = region.value {
                    walk_expr(semantic, core, value, out);
                }
            }
            Expr::Template(template) => {
                for part in &template.parts {
                    match part {
                        TemplatePart::Raw(node) => span(semantic, *node, out),
                        TemplatePart::Interpolation(expr) => walk_expr(semantic, core, *expr, out),
                    }
                }
            }
        }
    }

    let mut out = Vec::new();
    walk_body(semantic, core, core.root, &mut out);
    out
}

fn structured_expr_span(
    semantic: &SemanticFile,
    core: &CoreFile,
    expr: ExprId,
) -> Option<SourceSpan> {
    let node_span = |node| {
        semantic
            .hir
            .source_map
            .node_span(node)
            .map(SourceSpan::from)
    };
    match &core.exprs[expr.index()] {
        Expr::Opaque(node) => node_span(*node),
        Expr::Decision(decision) => node_span(decision.extent),
        Expr::Propagate(propagate) => node_span(propagate.node),
        Expr::ResultRegion(region) => node_span(region.node),
        Expr::Template(template) => node_span(template.node),
        Expr::Apply(apply) => {
            let mut span = node_span(apply.node)?;
            for step in &apply.steps {
                let step = node_span(step.node)?;
                span.start = span.start.min(step.start);
                span.end = span.end.max(step.end);
            }
            Some(span)
        }
        Expr::Sequence(body) => core.bodies[body.index()]
            .statements
            .iter()
            .find_map(|statement| match statement {
                Statement::Expr(value) if core.has_statement_form(*value) => {
                    structured_expr_span(semantic, core, *value)
                }
                _ => None,
            })
            .or_else(|| {
                core.body_value_expr(*body)
                    .and_then(|value| structured_expr_span(semantic, core, value))
            }),
    }
}

struct TargetRewritePlan {
    owner_slots: Vec<OwnerSlotRewrite>,
    for_initializer_propagations: Vec<ForInitializerPropagationRewrite>,
    composes: Vec<ComposeRewrite>,
    loop_tests: Vec<LoopTestRewrite>,
    source_replacements: Vec<SourceReplacement>,
    /// The source spans of values whose lowering moves them into a prelude
    /// before their owner — a planned relocation the preservation check
    /// must know about ([`SourcePreservation::relocated`]).
    relocated_values: Vec<SourceSpan>,
    /// The parent spans of lowered conditional operations: their operator
    /// tokens are claimed source ([`SourcePreservation::rewritten`]).
    rewritten_operations: Vec<SourceSpan>,
    /// Expression propagations rejected by the host-capability check. The
    /// recovering projection emits `undefined` for them and claims their
    /// source so editor/type diagnostics can continue.
    recovered_propagations: Vec<(ExprId, SourceSpan)>,
    /// tt values a conditional operation consumes; their inline Core
    /// position emits nothing (the operation's replacement covers it).
    consumed_exprs: HashSet<ExprId>,
    arrow_returns: Vec<ArrowReturnRewrite>,
    slot_exprs: HashMap<ExprId, String>,
    value_slots: HashMap<ExprId, String>,
    scheduled_slots: HashMap<crate::evaluation_ir::ValueSlotId, String>,
    value_exits: HashMap<ExprId, Vec<HostExit>>,
    nested_schedules: HashMap<ExprId, EvaluationSchedule>,
    nested_values: HashSet<ExprId>,
    expression_boundary_name: String,
    result_return_name: String,
}

#[derive(Debug, Clone)]
struct OwnerSlotRewrite {
    owner: SourceSpan,
    source: SourceSpan,
    expr: ExprId,
    slot: String,
    continuation: HostContinuation,
    contextual_type: Option<SourceSpan>,
    contextual_type_awaited: bool,
}

#[derive(Debug, Clone, Copy)]
struct ForInitializerPropagationRewrite {
    node: NodeId,
    owner: SourceSpan,
    source: SourceSpan,
}

#[derive(Debug, Clone)]
struct ArrowReturnRewrite {
    expr: ExprId,
    slot: String,
    parenthesized: bool,
    contextual_type: Option<SourceSpan>,
    contextual_type_awaited: bool,
}

#[derive(Debug, Clone)]
struct ComposeRewrite {
    owner: SourceSpan,
    owner_kind: HostOwnerKind,
    actions: Vec<ComposeAction>,
}

#[derive(Debug, Clone)]
struct LoopTestRewrite {
    owner: SourceSpan,
    kind: LoopTestKind,
    test: SourceSpan,
    body: SourceSpan,
    update: Option<SourceSpan>,
    first_expr: ExprId,
    first_source: SourceSpan,
    actions: Vec<ComposeAction>,
}

/// One unit of a compose prelude, in source order: a plain host value, or a
/// whole conditional operation (결정 17).
#[derive(Debug, Clone)]
enum ComposeAction {
    Value(ComposeValue),
    Operation(PlannedConditionalOperation),
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
    jsx_child: bool,
    /// The tt value whose construct anchor the replacement's generated
    /// name carries — a conditional operation's result stands for the whole
    /// operation, so diagnostics on it belong to its primary tt value.
    anchor: Option<ExprId>,
}

type NestedSourceReplacement = (SourceSpan, String, Option<(AnchorKind, usize)>);

#[derive(Debug, Clone)]
struct LocalSourceEdit {
    span: SourceSpan,
    text: String,
    result_return_mark: Option<SourceSpan>,
}

impl TargetRewritePlan {
    fn build(semantic: &SemanticFile, core: &CoreFile, lowering: &LoweringPlan) -> Self {
        let for_initializer_propagations: Vec<_> = lowering
            .for_initializer_propagations()
            .map(|propagation| ForInitializerPropagationRewrite {
                node: propagation.node,
                owner: propagation.owner.span,
                source: propagation.source,
            })
            .collect();
        let recovered_propagations: Vec<_> = lowering
            .unsupported_expression_propagations()
            .into_iter()
            .map(|failure| (failure.expr, failure.source))
            .collect();
        let recovered: HashSet<_> = recovered_propagations
            .iter()
            .map(|(expr, _)| *expr)
            .collect();
        // Whether a value's control flow may become statements in its host
        // owner was decided by the Evaluation IR and recorded on the value
        // ([`TargetCapability`]); this plan only picks the statement *shape*
        // that fits each host continuation.
        let owner_slots: Vec<_> = lowering
            .owners()
            .flat_map(|rewrite| rewrite.values.iter().map(move |value| (rewrite, value)))
            .filter_map(|(rewrite, value)| {
                let ValueTarget::Slot(slot) = value.target;
                (!recovered.contains(&value.expr)
                    && matches!(
                        value.context.continuation,
                        HostContinuation::Initialize
                            | HostContinuation::ForInitialize
                            | HostContinuation::Return
                            | HostContinuation::Discard
                    )
                    && value.schedule.steps().is_empty()
                    && value.capability == TargetCapability::StatementRegion)
                    .then(|| OwnerSlotRewrite {
                        owner: rewrite.owner.span,
                        source: structured_expr_span(semantic, core, value.expr)
                            .unwrap_or(value.source),
                        expr: value.expr,
                        slot: lowering.slot_name(slot).to_owned(),
                        continuation: value.context.continuation,
                        contextual_type: value.context.contextual_type,
                        contextual_type_awaited: value.context.contextual_type_awaited,
                    })
            })
            .collect();
        let arrow_returns: Vec<_> = lowering
            .owners()
            .filter(|rewrite| rewrite.values.len() == 1)
            .filter_map(|rewrite| {
                let value = &rewrite.values[0];
                let ValueTarget::Slot(slot) = value.target;
                (!recovered.contains(&value.expr)
                    && value.context.continuation == HostContinuation::ArrowReturn
                    && value.schedule.steps().is_empty()
                    && value.capability == TargetCapability::StatementRegion)
                    .then(|| ArrowReturnRewrite {
                        expr: value.expr,
                        slot: lowering.slot_name(slot).to_owned(),
                        parenthesized: rewrite.owner.span != value.source,
                        contextual_type: value.context.contextual_type,
                        contextual_type_awaited: value.context.contextual_type_awaited,
                    })
            })
            .collect();
        let loop_tests: Vec<_> = lowering
            .owners()
            .filter_map(|rewrite| {
                let first = rewrite.values.iter().find(|value| {
                    value
                        .schedule
                        .steps()
                        .last()
                        .is_some_and(|step| step.loop_test.is_some())
                })?;
                let facts = first.schedule.steps().last()?.loop_test?;
                let values: Vec<_> = rewrite
                    .values
                    .iter()
                    .filter(|value| {
                        value.schedule.steps().last().is_some_and(|step| {
                            step.operation == HostEvaluationOperation::LoopTest
                                && step.loop_test == Some(facts)
                        })
                    })
                    .collect();
                let can_rewrite = !values.is_empty()
                    && values.iter().all(|value| {
                        !recovered.contains(&value.expr)
                            && value.context.continuation == HostContinuation::Compose
                            && value.capability == TargetCapability::StatementRegion
                    });
                can_rewrite.then(|| {
                    let operation_of: HashMap<ExprId, usize> = rewrite
                        .operations
                        .iter()
                        .enumerate()
                        .flat_map(|(index, operation)| {
                            operation.values.iter().map(move |expr| (*expr, index))
                        })
                        .collect();
                    let mut emitted_operations = HashSet::new();
                    let actions = values
                        .into_iter()
                        .filter_map(|value| match operation_of.get(&value.expr) {
                            Some(index) => emitted_operations.insert(*index).then(|| {
                                let mut operation = rewrite.operations[*index].clone();
                                let loop_step = operation.outer.pop().unwrap_or_else(|| {
                                    crate::ice::bug!(
                                        "loop conditional operation lost its loop step"
                                    )
                                });
                                if loop_step.operation != HostEvaluationOperation::LoopTest {
                                    crate::ice::bug!(
                                        "loop conditional operation has a non-loop outer step"
                                    )
                                }
                                ComposeAction::Operation(operation)
                            }),
                            None => {
                                let ValueTarget::Slot(slot) = value.target;
                                let mut steps = value.schedule.steps().to_vec();
                                let loop_step = steps.pop().unwrap_or_else(|| {
                                    crate::ice::bug!("loop value lost its loop step")
                                });
                                if loop_step.operation != HostEvaluationOperation::LoopTest {
                                    crate::ice::bug!("loop value has a non-loop outer step")
                                }
                                Some(ComposeAction::Value(ComposeValue {
                                    expr: value.expr,
                                    slot: lowering.slot_name(slot).to_owned(),
                                    steps,
                                }))
                            }
                        })
                        .collect();
                    LoopTestRewrite {
                        owner: rewrite.owner.span,
                        kind: facts.kind,
                        test: facts.test,
                        body: facts.body,
                        update: facts.update,
                        first_expr: first.expr,
                        first_source: first.source,
                        actions,
                    }
                })
            })
            .collect();
        let composes: Vec<_> = lowering
            .owners()
            .filter_map(|rewrite| {
                let values: Vec<_> = rewrite
                    .values
                    .iter()
                    .filter(|value| {
                        value.context.continuation == HostContinuation::Compose
                            && !value
                                .schedule
                                .steps()
                                .iter()
                                .any(|step| step.operation == HostEvaluationOperation::LoopTest)
                    })
                    .collect();
                let can_compose = !values.is_empty()
                    && values.iter().all(|value| {
                        !recovered.contains(&value.expr)
                            && value.capability == TargetCapability::StatementRegion
                    });
                can_compose.then(|| {
                    // A value consumed by a conditional operation is emitted
                    // by that operation's region, at the position of the
                    // operation's first value.
                    let operation_of: HashMap<ExprId, usize> = rewrite
                        .operations
                        .iter()
                        .enumerate()
                        .flat_map(|(index, operation)| {
                            operation.values.iter().map(move |expr| (*expr, index))
                        })
                        .collect();
                    let mut emitted_operations = HashSet::new();
                    let actions = values
                        .into_iter()
                        .filter_map(|value| match operation_of.get(&value.expr) {
                            Some(index) => emitted_operations.insert(*index).then(|| {
                                ComposeAction::Operation(rewrite.operations[*index].clone())
                            }),
                            None => {
                                let ValueTarget::Slot(slot) = value.target;
                                Some(ComposeAction::Value(ComposeValue {
                                    expr: value.expr,
                                    slot: lowering.slot_name(slot).to_owned(),
                                    steps: value.schedule.steps().to_vec(),
                                }))
                            }
                        })
                        .collect();
                    ComposeRewrite {
                        owner: rewrite.owner.span,
                        owner_kind: rewrite.owner.kind,
                        actions,
                    }
                })
            })
            .collect();
        let compose_values = || {
            composes.iter().flat_map(|rewrite| {
                rewrite.actions.iter().filter_map(|action| match action {
                    ComposeAction::Value(value) => Some(value),
                    ComposeAction::Operation(_) => None,
                })
            })
        };
        let compose_operations = || {
            composes.iter().flat_map(|rewrite| {
                rewrite.actions.iter().filter_map(|action| match action {
                    ComposeAction::Operation(operation) => Some(operation),
                    ComposeAction::Value(_) => None,
                })
            })
        };
        let loop_values = || {
            loop_tests.iter().flat_map(|rewrite| {
                rewrite.actions.iter().filter_map(|action| match action {
                    ComposeAction::Value(value) => Some(value),
                    ComposeAction::Operation(_) => None,
                })
            })
        };
        let loop_operations = || {
            loop_tests.iter().flat_map(|rewrite| {
                rewrite.actions.iter().filter_map(|action| match action {
                    ComposeAction::Operation(operation) => Some(operation),
                    ComposeAction::Value(_) => None,
                })
            })
        };
        let all_values = || compose_values().chain(loop_values());
        let all_operations = || compose_operations().chain(loop_operations());
        // owner-slot and compose rewrites hoist the value's control flow
        // to a prelude before the owner; arrow-return rewrites restructure
        // the value in place, so they relocate nothing. A conditional
        // operation relocates its whole parent expression.
        let hoisted: HashSet<ExprId> = owner_slots
            .iter()
            .map(|rewrite| rewrite.expr)
            .chain(compose_values().map(|value| value.expr))
            .chain(compose_operations().flat_map(|operation| operation.values.iter().copied()))
            .chain(loop_values().map(|value| value.expr))
            .chain(loop_operations().flat_map(|operation| operation.values.iter().copied()))
            .collect();
        let mut relocated_values: Vec<SourceSpan> = lowering
            .owners()
            .flat_map(|rewrite| &rewrite.values)
            .filter(|value| hoisted.contains(&value.expr))
            .map(|value| value.source)
            .collect();
        relocated_values.extend(
            for_initializer_propagations
                .iter()
                .map(|rewrite| rewrite.source),
        );
        relocated_values.extend(lowering.nested_relocations());
        relocated_values.extend(
            lowering
                .nested_values()
                .filter_map(|expr| structured_expr_span(semantic, core, expr)),
        );
        relocated_values.extend(
            lowering
                .nested_value_exits()
                .flat_map(|(_, exits)| exits.iter().filter_map(|exit| exit.argument)),
        );
        relocated_values.extend(all_operations().map(|operation| operation.parent));
        relocated_values.extend(loop_tests.iter().filter_map(|rewrite| rewrite.update));
        relocated_values.extend(lowering.nested_value_schedules().flat_map(|(_, schedule)| {
            schedule.steps().iter().flat_map(|step| {
                std::iter::once(step.parent).chain(step.inputs.iter().filter_map(
                    |input| match input {
                        PlannedEvaluationInput::Source { source, .. }
                        | PlannedEvaluationInput::Stable { source } => Some(*source),
                        PlannedEvaluationInput::Slot { .. } => None,
                    },
                ))
            })
        }));
        // The operator frame of a lowered conditional operation (its tokens
        // between the fragments the region re-emits) is claimed source.
        let rewritten_operations: Vec<SourceSpan> = all_operations()
            .map(|operation| operation.parent)
            .chain(loop_tests.iter().flat_map(|rewrite| {
                let prefix = (rewrite.kind == LoopTestKind::While).then_some(SourceSpan {
                    start: rewrite.owner.start,
                    end: rewrite.test.start,
                });
                prefix.into_iter().chain(std::iter::once(SourceSpan {
                    start: rewrite.test.end,
                    end: rewrite.body.start,
                }))
            }))
            .collect();
        let operation_replacements: Vec<SourceReplacement> = all_operations()
            .map(|operation| {
                let primary = operation
                    .values
                    .first()
                    .copied()
                    .unwrap_or_else(|| crate::ice::bug!("conditional operation has no value"));
                SourceReplacement {
                    source: operation.parent,
                    slot: lowering.slot_name(operation.result).to_owned(),
                    jsx_child: false,
                    anchor: Some(primary),
                }
            })
            .collect();
        let source_replacements = all_values()
            .flat_map(|value| &value.steps)
            .chain(
                all_operations()
                    .flat_map(|operation| operation.active_steps.iter().chain(&operation.outer)),
            )
            .flat_map(|step| &step.inputs)
            .filter_map(|input| match input {
                PlannedEvaluationInput::Source {
                    source,
                    target,
                    mode,
                    ..
                } => Some(SourceReplacement {
                    source: *source,
                    slot: lowering.slot_name(*target).to_owned(),
                    jsx_child: *mode == EvaluationInputMode::JsxChildValue,
                    anchor: None,
                }),
                PlannedEvaluationInput::Slot { .. } | PlannedEvaluationInput::Stable { .. } => None,
            })
            .chain(owner_slots.iter().map(|rewrite| SourceReplacement {
                source: rewrite.source,
                slot: rewrite.slot.clone(),
                jsx_child: false,
                anchor: Some(rewrite.expr),
            }))
            .chain(operation_replacements)
            .collect();
        let consumed_exprs: HashSet<ExprId> = compose_operations()
            .flat_map(|operation| operation.values.iter().copied())
            .chain(loop_operations().flat_map(|operation| operation.values.iter().copied()))
            .collect();
        let slot_exprs = owner_slots
            .iter()
            .map(|rewrite| (rewrite.expr, rewrite.slot.clone()))
            .chain(compose_values().map(|value| (value.expr, value.slot.clone())))
            .chain(loop_values().map(|value| (value.expr, value.slot.clone())))
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
            .chain(
                lowering
                    .nested_value_exits()
                    .map(|(expr, exits)| (expr, exits.to_vec())),
            )
            .collect();
        let nested_schedules = lowering
            .nested_value_schedules()
            .map(|(expr, schedule)| (expr, schedule.clone()))
            .collect();
        let nested_values = lowering.nested_values().collect();
        let expression_boundary_name = lowering.expression_boundary_name().to_owned();
        let result_return_name = lowering.result_return_name().to_owned();
        Self {
            owner_slots,
            for_initializer_propagations,
            composes,
            loop_tests,
            source_replacements,
            relocated_values,
            rewritten_operations,
            recovered_propagations,
            consumed_exprs,
            arrow_returns,
            slot_exprs,
            value_slots,
            scheduled_slots,
            value_exits,
            nested_schedules,
            nested_values,
            expression_boundary_name,
            result_return_name,
        }
    }
}

struct Emitter<'a> {
    semantic: &'a SemanticFile,
    core: &'a CoreFile,
    source: &'a str,
    direct_apply_inputs: HashSet<ExprId>,
    rewrite_imports: ImportRewrite,
    std_imports: StdImports<'a>,
    owner_slot_rewrites: Vec<OwnerSlotRewrite>,
    for_initializer_propagations: Vec<ForInitializerPropagationRewrite>,
    compose_rewrites: Vec<ComposeRewrite>,
    loop_test_rewrites: Vec<LoopTestRewrite>,
    source_replacements: Vec<SourceReplacement>,
    consumed_exprs: HashSet<ExprId>,
    arrow_return_rewrites: Vec<ArrowReturnRewrite>,
    slot_exprs: HashMap<ExprId, String>,
    value_slots: HashMap<ExprId, String>,
    scheduled_slots: HashMap<crate::evaluation_ir::ValueSlotId, String>,
    value_exits: HashMap<ExprId, Vec<HostExit>>,
    nested_schedules: HashMap<ExprId, EvaluationSchedule>,
    nested_values: HashSet<ExprId>,
    recovered_propagations: HashSet<ExprId>,
    expression_boundary_name: String,
    result_return_name: String,
    /// How many conditional-operation regions are being emitted right now.
    /// Inside one, the operation's own host replacement does not apply —
    /// the region re-emits the operation's fragments itself.
    conditional_region_depth: Cell<u32>,
    /// Structured owner-slot emission reads the value's authored children.
    /// Suppress only that value's own replacement; nested structured values
    /// must still replace their host occurrences compositionally.
    active_structured_exprs: RefCell<Vec<ExprId>>,
    active_scheduled_exprs: RefCell<Vec<ExprId>>,
    /// Owner preludes can be reached either through an opaque source prefix
    /// or through the Core expression entry. Record which path emitted the
    /// prelude so the other path contributes only the join-slot occurrence.
    emitted_owner_rewrites: RefCell<HashSet<ExprId>>,
    /// Loop-test actions emit their tt values before the rebuilt source test.
    /// Host replacements apply only to that source test, not while the
    /// actions recursively emit their own source fragments.
    loop_region_depth: Cell<u32>,
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
    Return,
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
    /// The single target that leaves a conditional dispatch after it has
    /// delivered a value. A fall-through block arm already requires
    /// `$tt_b`; in that case expression arms use the same label instead of
    /// adding a nested `do { ... } while (false)` target.
    chain_exit_label: Option<&'context str>,
}

struct ResultEmissionContext<'context, 'name> {
    failure: &'context ValueContinuation<'name>,
    success: &'context ValueContinuation<'name>,
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

    fn returning() -> Self {
        Self {
            destination: ValueDestination::Return,
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
            ValueDestination::Expression | ValueDestination::Return => None,
            ValueDestination::Assign(target) => Some(target),
        }
    }

    /// The text an early exit's `return ` becomes. `grouped` says whether
    /// the value it returns has to keep its parentheses ([`push_grouped`]).
    fn assignment_prefix(&self, grouped: bool) -> String {
        let mut prefix = match self.destination {
            ValueDestination::Return => "return ".to_owned(),
            ValueDestination::Assign(target) => format!("{target} = "),
            ValueDestination::Expression => {
                crate::ice::bug!("inline expression continuation cannot rewrite an exit")
            }
        };
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
                kind: ArmBodyKind::Block { .. },
                ..
            }
        )
    })
}

fn exit_label(target: &str) -> String {
    format!("$tt_y_{}", target.strip_prefix("$tt_").unwrap_or(target))
}

fn push_region_break(out: &mut Rope<'_>, label: Option<&str>) {
    match label {
        Some(label) => out.push_lit(format!(" break {label};")),
        None => out.push_lit(" break;"),
    }
}

fn push_control_break(out: &mut Rope<'_>, depth: u16, label: Option<&str>) {
    out.push_break(depth);
    match label {
        Some(label) => out.push_lit(format!("break {label};")),
        None => out.push_lit("break;"),
    }
}

fn result_failure_test(temp: &str, layout: ResultLayout) -> String {
    match layout.discriminator {
        ResultDiscriminator::SuccessFieldPresent(field) => {
            format!("!(\"{field}\" in {temp})")
        }
    }
}

impl<'a> Emitter<'a> {
    fn exits_for_expr(&self, expr: ExprId) -> Vec<HostExit> {
        self.value_exits.get(&expr).cloned().unwrap_or_default()
    }

    fn result_return_rewrite_spans(&self) -> Vec<SourceSpan> {
        self.core
            .exprs
            .iter()
            .enumerate()
            .flat_map(|(index, expr)| {
                let Expr::ResultRegion(region) = expr else {
                    return Vec::new();
                };
                let expr = ExprId::new(index);
                let exits = self
                    .value_exits
                    .get(&expr)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                region
                    .items
                    .iter()
                    .map(|item| {
                        let ResultRegionItem::Statements(body) = item;
                        *body
                    })
                    .flat_map(|body| {
                        exits.iter().filter_map(move |exit| {
                            exit.argument
                                .and_then(|argument| self.result_return_propagate(body, argument))
                                .map(|_| exit.statement)
                        })
                    })
                    .collect()
            })
            .collect()
    }

    fn span(&self, node: NodeId) -> hir::Span {
        self.semantic
            .hir
            .source_map
            .node_span(node)
            .unwrap_or_else(|| crate::ice::bug!("target node has no source span"))
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
        let mut propagation_insertions = self
            .for_initializer_propagations
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
        let mut loop_endings: Vec<_> = self
            .loop_test_rewrites
            .iter()
            .filter(|rewrite| span.start < rewrite.body.end && rewrite.body.end <= span.end)
            .collect();
        loop_endings.sort_unstable_by_key(|rewrite| rewrite.body.end);
        let mut loop_endings = loop_endings.into_iter().peekable();
        let mut cursor = span.start;
        while cursor < span.end {
            while let Some(_rewrite) = loop_endings.next_if(|rewrite| rewrite.body.end == cursor) {
                rope.push_lit("}");
            }
            while let Some(rewrite) = compose_endings.next_if(|rewrite| rewrite.owner.end == cursor)
            {
                rope.append(self.emit_compose_suffix(rewrite));
            }
            while let Some(rewrite) = insertions.next_if(|rewrite| rewrite.owner.start == cursor) {
                self.emitted_owner_rewrites
                    .borrow_mut()
                    .insert(rewrite.expr);
                rope.append(self.emit_owner_slot_rewrite(rewrite));
            }
            while let Some(rewrite) =
                propagation_insertions.next_if(|rewrite| rewrite.owner.start == cursor)
            {
                rope.append(self.emit_for_initializer_propagation_prelude(rewrite));
            }
            while let Some(rewrite) =
                compose_insertions.next_if(|rewrite| rewrite.owner.start == cursor)
            {
                rope.append(self.emit_compose_rewrite(rewrite));
            }
            if let Some(rewrite) = self.loop_test_rewrites.iter().find(|rewrite| {
                rewrite.kind == LoopTestKind::While
                    && rewrite.owner.start <= cursor
                    && cursor < rewrite.test.start
            }) {
                if cursor == rewrite.owner.start {
                    rope.append(self.emit_loop_test_prefix(rewrite));
                }
                cursor = rewrite.test.start.min(span.end);
                continue;
            }
            if let Some(rewrite) = self
                .loop_test_rewrites
                .iter()
                .find(|rewrite| rewrite.kind == LoopTestKind::For && cursor == rewrite.test.start)
            {
                rope.append(self.emit_loop_test_prefix(rewrite));
            }
            if self.loop_region_depth.get() == 0
                && let Some(operation) = self
                    .loop_test_rewrites
                    .iter()
                    .flat_map(|rewrite| &rewrite.actions)
                    .filter_map(|action| match action {
                        ComposeAction::Operation(operation) => Some(operation),
                        ComposeAction::Value(_) => None,
                    })
                    .find(|operation| {
                        operation.parent.start < cursor && cursor < operation.parent.end
                    })
            {
                cursor = operation.parent.end.min(span.end);
                continue;
            }
            if let Some(rewrite) = self
                .loop_test_rewrites
                .iter()
                .find(|rewrite| rewrite.test.end <= cursor && cursor < rewrite.body.start)
            {
                if cursor == rewrite.test.end {
                    rope.push_lit(")) break; ");
                }
                cursor = rewrite.body.start.min(span.end);
                continue;
            }
            if let Some(replacement) = self.source_replacements.iter().find(|replacement| {
                if replacement.anchor.is_some() {
                    self.conditional_region_depth.get() == 0
                        && self.loop_region_depth.get() == 0
                        && !replacement.anchor.is_some_and(|expr| {
                            self.active_structured_exprs.borrow().contains(&expr)
                        })
                        && replacement.source.start <= cursor
                        && cursor < replacement.source.end
                } else {
                    replacement.source.start <= cursor && cursor < replacement.source.end
                }
            }) {
                if cursor == replacement.source.start {
                    if replacement.jsx_child {
                        rope.push_lit("{");
                    }
                    match replacement.anchor {
                        Some(expr) => {
                            let (kind, start, end, extent) = self.value_anchor(expr);
                            let mut name = Rope::new();
                            name.push_lit(replacement.slot.clone());
                            rope.anchored(kind, start, end, extent, name);
                        }
                        None => rope.push_lit(replacement.slot.clone()),
                    }
                    if replacement.jsx_child {
                        rope.push_lit("}");
                    }
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
            let next_propagation = propagation_insertions
                .peek()
                .map_or(span.end, |rewrite| rewrite.owner.start);
            let next_compose_end = compose_endings
                .peek()
                .map_or(span.end, |rewrite| rewrite.owner.end);
            let next_loop_boundary = self
                .loop_test_rewrites
                .iter()
                .flat_map(|rewrite| {
                    [
                        rewrite.owner.start,
                        rewrite.test.start,
                        rewrite.test.end,
                        rewrite.body.start,
                        rewrite.body.end,
                    ]
                })
                .filter(|boundary| cursor < *boundary && *boundary < span.end)
                .min()
                .unwrap_or(span.end);
            let next_replacement = self
                .source_replacements
                .iter()
                .filter(|replacement| {
                    (replacement.anchor.is_none()
                        || (self.conditional_region_depth.get() == 0
                            && self.loop_region_depth.get() == 0
                            && !replacement.anchor.is_some_and(|expr| {
                                self.active_structured_exprs.borrow().contains(&expr)
                            })))
                        && cursor < replacement.source.start
                        && replacement.source.start < span.end
                })
                .map(|replacement| replacement.source.start)
                .min()
                .unwrap_or(span.end);
            let next = next_insertion
                .min(next_compose)
                .min(next_propagation)
                .min(next_compose_end)
                .min(next_loop_boundary)
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
        while let Some(_rewrite) = loop_endings.next_if(|rewrite| rewrite.body.end == span.end) {
            rope.push_lit("}");
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
            if let Some(mark) = edit.result_return_mark {
                let marker_offset = edit
                    .text
                    .find(&self.result_return_name)
                    .unwrap_or_else(|| crate::ice::bug!("Result return edit lost its temporary"));
                let name_end = marker_offset + self.result_return_name.len();
                out.push_lit(edit.text[..marker_offset].to_owned());
                out.push_result_return_mark(mark.start);
                out.push_lit(edit.text[marker_offset..name_end].to_owned());
                out.push_lit(edit.text[name_end..].to_owned());
            } else {
                out.push_lit(edit.text.clone());
            }
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
        label: Option<&str>,
        generated_indent: &str,
    ) -> Rope<'a> {
        // Without a label the region's own dispatch is the nearest `break`
        // target already ([`HostExit::captured_break`]).
        let leave = label.map_or_else(|| "break;".to_owned(), |label| format!("break {label};"));
        let mut edits = Vec::new();
        for exit in exits {
            let line_start = self.source[..exit.statement.start]
                .rfind('\n')
                .map_or(0, |index| index + 1);
            let line_indent = &self.source[line_start..exit.statement.start];
            let starts_own_line = line_indent.bytes().all(|byte| matches!(byte, b' ' | b'\t'));
            match exit.argument {
                Some(argument) => {
                    let grouped =
                        grouping_required(self.source[argument.start..argument.end].trim());
                    edits.push(LocalSourceEdit {
                        span: SourceSpan {
                            start: exit.statement.start,
                            end: argument.start,
                        },
                        text: format!(
                            "{}{}{}",
                            if starts_own_line {
                                generated_indent
                            } else {
                                ""
                            },
                            if exit.requires_block { "{ " } else { "" },
                            continuation.assignment_prefix(grouped)
                        ),
                        result_return_mark: None,
                    });
                    edits.push(LocalSourceEdit {
                        span: SourceSpan {
                            start: argument.end,
                            end: exit.statement.end,
                        },
                        text: if starts_own_line {
                            format!(
                                "{};\n{line_indent}{generated_indent}{leave}",
                                continuation.assignment_suffix(grouped),
                            )
                        } else {
                            format!(
                                "{}; {leave}{}",
                                continuation.assignment_suffix(grouped),
                                if exit.requires_block { " }" } else { "" }
                            )
                        },
                        result_return_mark: None,
                    });
                }
                None => edits.push(LocalSourceEdit {
                    span: exit.statement,
                    text: if starts_own_line {
                        format!(
                            "{generated_indent}{}undefined{};\n{line_indent}{generated_indent}{leave}",
                            continuation.assignment_prefix(false),
                            continuation.assignment_suffix(false)
                        )
                    } else {
                        format!(
                            "{}{}undefined{}; {leave}{}",
                            if exit.requires_block { "{ " } else { "" },
                            continuation.assignment_prefix(false),
                            continuation.assignment_suffix(false),
                            if exit.requires_block { " }" } else { "" }
                        )
                    },
                    result_return_mark: None,
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
                Statement::Adt(adt) => {
                    // The union type and constructor object are this
                    // declaration's glue: a frame inside a generated
                    // constructor belongs to the `variant` that wrote it.
                    let span = self.span(adt.node);
                    out.anchored(
                        AnchorKind::Variant,
                        span.start,
                        span.end,
                        span.end,
                        emit_adt(adt),
                    );
                }
                Statement::Import(import) => self.emit_import(import, &mut out),
                Statement::Propagate(propagate) => {
                    let owner = self.span(propagate.owner);
                    if let Some(rewrite) = self
                        .compose_rewrites
                        .iter()
                        .find(|rewrite| rewrite.owner == SourceSpan::from(owner))
                    {
                        out.append(self.emit_compose_rewrite(rewrite));
                    }
                    let span = self.span(propagate.node);
                    let emitted = if self.is_for_initializer_propagation(propagate.node) {
                        self.emit_for_initializer_payload(propagate)
                    } else {
                        self.emit_propagate(propagate)
                    };
                    out.anchored(AnchorKind::Try, span.start, span.end, span.end, emitted);
                }
                Statement::Decision(decision) => self.emit_statement_decision(decision, &mut out),
                Statement::Expr(expr)
                    if matches!(self.core.exprs[expr.index()], Expr::Decision(_))
                        && (self.owner_slot_rewrites.iter().any(|rewrite| {
                            rewrite.expr == *expr
                                && rewrite.continuation == HostContinuation::Discard
                        }) || (!self
                            .owner_slot_rewrites
                            .iter()
                            .any(|rewrite| rewrite.expr == *expr)
                            && !self.value_slots.contains_key(expr))) =>
                {
                    self.emit_statement_expr(*expr, &mut out);
                }
                Statement::Expr(expr) => out.append(self.emit_expr(*expr)),
            }
        }
        out
    }

    fn emit_statement_expr(&self, expr: ExprId, out: &mut Rope<'a>) {
        // A tt value that is itself an expression statement has no opaque
        // source owner around it where `source_range_rope` could insert the
        // planned statement region. Consume that plan here before the inline
        // occurrence is replaced by its join slot.
        if matches!(self.core.exprs[expr.index()], Expr::Decision(_)) {
            if let Some(rewrite) = self
                .owner_slot_rewrites
                .iter()
                .find(|rewrite| rewrite.expr == expr)
            {
                if self.emitted_owner_rewrites.borrow().contains(&expr) {
                    out.append(self.emit_expr(expr));
                } else {
                    self.emitted_owner_rewrites.borrow_mut().insert(expr);
                    out.append(self.emit_owner_slot_rewrite(rewrite));
                }
                return;
            }
            if let Some(slot) = self.value_slots.get(&expr) {
                out.push_lit(format!("let {slot};"));
                out.push_break(0);
                out.append(
                    self.emit_continued_expr(expr, &ValueContinuation::assign(slot))
                        .unwrap_or_else(|| {
                            crate::ice::bug!("statement match has no structured emission")
                        }),
                );
                return;
            }
            // An incomplete editor buffer can leave the surrounding
            // TypeScript owner unparsable even though the tt match itself is
            // complete. There is then no safe source owner to rewrite and no
            // planned slot. Keep the structurally parsed match available to
            // the language service through the existing expression boundary.
            out.push_lit("(() => { let $tt_recovery; ");
            out.append(
                self.emit_continued_expr(expr, &ValueContinuation::assign("$tt_recovery"))
                    .unwrap_or_else(|| {
                        crate::ice::bug!("statement match has no expression-boundary emission")
                    }),
            );
            out.push_lit(" return $tt_recovery; })()");
            return;
        }
        out.append(self.emit_expr(expr));
    }

    fn emit_sequence_continued(
        &self,
        body: hir::BodyId,
        continuation: &ValueContinuation<'_>,
    ) -> Option<Rope<'a>> {
        let statements = &self.core.bodies[body.index()].statements;
        if let Some((value_index, value)) = crate::core_ir::sequence_value(statements)
            && statements
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
        {
            let mut out = self.emit_statements(&statements[..value_index]);
            out.append(self.emit_continued_expr(value, continuation)?);
            out.append(self.emit_statements(&statements[value_index + 1..]));
            return Some(out);
        }

        let nested: Vec<_> = statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Expr(inner)
                    if self.core.has_statement_form(*inner)
                        && !self.slot_exprs.contains_key(inner) =>
                {
                    self.structured_value_slot(*inner)
                        .map(|slot| (*inner, slot.clone()))
                }
                _ => None,
            })
            .collect();
        if nested.is_empty() {
            return None;
        }
        let nested_exprs: Vec<_> = nested.iter().map(|(inner, _)| *inner).collect();
        let mut out = Rope::new();
        for (inner, slot) in nested {
            out.push_lit(format!("let {slot};"));
            out.push_break(0);
            out.append(self.emit_continued_expr(inner, &ValueContinuation::assign(&slot))?);
            out.push_break(0);
        }
        let sequence_node = self
            .core
            .sequence_node(body)
            .unwrap_or_else(|| crate::ice::bug!("embedded sequence has no source extent"));
        let span = SourceSpan::from(self.span(sequence_node));
        out.append(self.emit_value_delivery(
            self.source_range_with_value_slots(span, &nested_exprs),
            None,
            continuation,
        ));
        Some(Rope::scoped(out))
    }

    fn emit_expr(&self, expr: ExprId) -> Rope<'a> {
        // A value a conditional operation consumed is emitted by the
        // operation's region; its inline position sits inside the replaced
        // operation span and prints nothing.
        if self.consumed_exprs.contains(&expr) {
            return Rope::new();
        }
        if let Some(rewrite) = self
            .arrow_return_rewrites
            .iter()
            .find(|rewrite| rewrite.expr == expr)
        {
            return self.emit_arrow_return_rewrite(rewrite);
        }
        if let Some(slot) = self.slot_exprs.get(&expr) {
            let (kind, start, end, extent) = self.value_anchor(expr);
            let mut out = Rope::new();
            let mut rendered_slot = slot.as_str();
            if let Some(rewrite) = self.loop_test_rewrites.iter().find(|rewrite| {
                rewrite.first_expr == expr && rewrite.first_source.start == rewrite.test.start
            }) {
                if rewrite.kind == LoopTestKind::For {
                    out.append(self.emit_loop_test_prefix(rewrite));
                }
                if let Some(operation) = rewrite.actions.iter().find_map(|action| match action {
                    ComposeAction::Operation(operation)
                        if operation.parent.start == rewrite.first_source.start =>
                    {
                        Some(operation)
                    }
                    ComposeAction::Operation(_) | ComposeAction::Value(_) => None,
                }) {
                    rendered_slot = self.value_slot_name(operation.result);
                }
            }
            let mut generated = Rope::new();
            generated.push_lit(rendered_slot.to_owned());
            out.anchored(kind, start, end, extent, generated);
            return out;
        }
        if self.nested_values.contains(&expr)
            && matches!(
                self.core.exprs[expr.index()],
                Expr::Decision(_) | Expr::ResultRegion(_) | Expr::Propagate(_)
            )
            && let Some(slot) = self.value_slots.get(&expr)
        {
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
            Expr::Propagate(propagate) => {
                if matches!(propagate.exit, ExitTarget::ResultRegion(_)) {
                    let span = self.span(propagate.node);
                    let mut out = Rope::new();
                    out.anchored(
                        AnchorKind::Try,
                        span.start,
                        span.end,
                        span.end,
                        self.emit_propagate(propagate),
                    );
                    return out;
                }
                if !self.recovered_propagations.contains(&expr) {
                    crate::ice::bug!("unscheduled expression try reached inline emission");
                }
                let span = self.span(propagate.node);
                let mut generated = Rope::new();
                generated.push_lit("undefined");
                let mut out = Rope::new();
                out.anchored(AnchorKind::Try, span.start, span.end, span.end, generated);
                out
            }
            Expr::Apply(apply) => self.emit_apply(apply),
            Expr::ResultRegion(region) => self.emit_result_region(expr, region),
            Expr::Template(template) => self.emit_template(template),
        }
    }

    fn emit_owner_slot_rewrite(&self, rewrite: &OwnerSlotRewrite) -> Rope<'a> {
        self.active_structured_exprs.borrow_mut().push(rewrite.expr);
        let anchored = self
            .emit_continued_expr(rewrite.expr, &ValueContinuation::assign(&rewrite.slot))
            .unwrap_or_else(|| {
                crate::ice::bug!("initializer rewrite is not structurally emit-able")
            });
        let popped = self.active_structured_exprs.borrow_mut().pop();
        debug_assert_eq!(popped, Some(rewrite.expr));
        let mut out = Rope::new();
        out.push_lit(format!("let {}", rewrite.slot));
        self.push_contextual_type(
            &mut out,
            rewrite.contextual_type,
            rewrite.contextual_type_awaited,
        );
        out.push_lit(";");
        out.push_break(0);
        out.append(anchored);
        out.push_break(0);
        Rope::scoped(out)
    }

    fn push_contextual_type(
        &self,
        out: &mut Rope<'a>,
        annotation: Option<SourceSpan>,
        awaited: bool,
    ) {
        let Some(annotation) = annotation else {
            return;
        };
        let authored = &self.source[annotation.start..annotation.end];
        if awaited {
            let ty = authored.strip_prefix(':').unwrap_or(authored);
            out.push_lit(format!(": Awaited<{ty}>"));
        } else {
            out.push_lit(authored.to_owned());
        }
    }

    fn is_for_initializer_propagation(&self, node: NodeId) -> bool {
        self.for_initializer_propagations
            .iter()
            .any(|rewrite| rewrite.node == node)
    }

    fn emit_for_initializer_propagation_prelude(
        &self,
        rewrite: &ForInitializerPropagationRewrite,
    ) -> Rope<'a> {
        let propagate = self
            .core
            .bodies
            .iter()
            .flat_map(|body| &body.statements)
            .find_map(|statement| match statement {
                Statement::Propagate(propagate) if propagate.node == rewrite.node => {
                    Some(propagate)
                }
                _ => None,
            })
            .unwrap_or_else(|| {
                crate::ice::bug!("for initializer propagation is missing from Core IR")
            });
        let temp = temp_name(propagate.temporary);
        let mut out = self.emit_propagate_input(propagate.value, &temp);
        out.push_break(0);
        out.push_lit(format!(
            "if ({}) {{",
            result_failure_test(&temp, propagate.layout)
        ));
        out.push_break(1);
        out.push_lit(format!("return {temp};"));
        out.push_break(0);
        out.push_lit("}");
        Rope::scoped(out)
    }

    fn emit_for_initializer_payload(&self, propagate: &Propagate) -> Rope<'a> {
        let temp = temp_name(propagate.temporary);
        let mut out = Rope::new();
        if let Some(binding) = propagate.binding {
            out.push_lit(format!("{} ", binding_keyword(binding.mode)));
            out.append(self.source_rope(binding.node));
            out.push_lit(format!(" = {temp}.{};", propagate.layout.payload_field));
        }
        Rope::scoped(out)
    }

    fn emit_compose_rewrite(&self, rewrite: &ComposeRewrite) -> Rope<'a> {
        let mut out = Rope::new();
        if rewrite.owner_kind == HostOwnerKind::ArrowExpression {
            out.push_lit("{");
            out.push_break(0);
        }
        for action in &rewrite.actions {
            let slot = match action {
                ComposeAction::Value(value) => &value.slot,
                ComposeAction::Operation(operation) => self.value_slot_name(operation.result),
            };
            out.push_lit(format!("let {slot};"));
            out.push_break(0);
        }
        let mut captured = HashSet::new();
        for action in &rewrite.actions {
            match action {
                ComposeAction::Value(value) => {
                    let mut lowered = self
                        .emit_continued_expr(value.expr, &ValueContinuation::assign(&value.slot))
                        .unwrap_or_else(|| {
                            crate::ice::bug!("compose value is not structurally emit-able")
                        });
                    for step in &value.steps {
                        lowered = self.emit_scheduled_step(step, lowered, &mut captured);
                    }
                    out.append(lowered);
                }
                ComposeAction::Operation(operation) => {
                    let mut lowered = self.emit_conditional_operation(operation, &mut captured);
                    for step in &operation.outer {
                        lowered = self.emit_scheduled_step(step, lowered, &mut captured);
                    }
                    out.append(lowered);
                }
            }
        }
        out.push_break(0);
        if rewrite.owner_kind == HostOwnerKind::ArrowExpression {
            out.push_lit("return ");
        }
        Rope::scoped(out)
    }

    fn emit_loop_test_prefix(&self, rewrite: &LoopTestRewrite) -> Rope<'a> {
        let mut out = Rope::new();
        match rewrite.kind {
            LoopTestKind::While => out.push_lit("while (true) {"),
            LoopTestKind::For => {
                out.push_lit("; ");
                if let Some(update) = rewrite.update {
                    out.push_src(&self.source[update.start..update.end], update.start);
                }
                out.push_lit(") {");
            }
        }
        out.push_break(1);
        for action in &rewrite.actions {
            let slot = match action {
                ComposeAction::Value(value) => &value.slot,
                ComposeAction::Operation(operation) => self.value_slot_name(operation.result),
            };
            out.push_lit(format!("let {slot};"));
            out.push_break(1);
        }
        self.loop_region_depth.set(self.loop_region_depth.get() + 1);
        let mut captured = HashSet::new();
        for action in &rewrite.actions {
            let lowered = match action {
                ComposeAction::Value(value) => {
                    let mut lowered = self
                        .emit_continued_expr(value.expr, &ValueContinuation::assign(&value.slot))
                        .unwrap_or_else(|| {
                            crate::ice::bug!("loop-test value is not structurally emit-able")
                        });
                    for step in &value.steps {
                        lowered = self.emit_scheduled_step(step, lowered, &mut captured);
                    }
                    lowered
                }
                ComposeAction::Operation(operation) => {
                    let mut lowered = self.emit_conditional_operation(operation, &mut captured);
                    for step in &operation.outer {
                        lowered = self.emit_scheduled_step(step, lowered, &mut captured);
                    }
                    lowered
                }
            };
            out.append(Rope::indented(1, lowered));
        }
        self.loop_region_depth.set(self.loop_region_depth.get() - 1);
        out.push_break(1);
        out.push_lit("if (!(");
        Rope::scoped(out)
    }

    /// Lowers one whole conditional operation (결정 17): evaluate the
    /// condition or callee once, branch, run the active branch's
    /// evaluations — tt regions included — in source order, and write every
    /// path's result into the operation's slot. All paths assign, so
    /// TypeScript sees the same definite-assignment correlation the original
    /// operation had, and an optional call's arguments evaluate only past
    /// its nullish check.
    fn emit_conditional_operation(
        &self,
        operation: &PlannedConditionalOperation,
        captured: &mut HashSet<crate::evaluation_ir::ValueSlotId>,
    ) -> Rope<'a> {
        self.conditional_region_depth
            .set(self.conditional_region_depth.get() + 1);
        let result = self.value_slot_name(operation.result);
        let mut out = Rope::new();
        let condition = self.emit_condition_capture(&operation.condition, captured, &mut out);
        let deliver_value = |expr: ExprId, target: &str| {
            self.emit_continued_expr(expr, &ValueContinuation::assign(target))
                .unwrap_or_else(|| {
                    crate::ice::bug!("conditional operation value is not structurally emit-able")
                })
        };
        let assign_condition = |out: &mut Rope<'a>| {
            out.push_lit(format!("{result} = {condition};"));
        };
        match &operation.kind {
            PlannedConditionalKind::LogicalAnd => {
                let value = operation.values[0];
                out.push_lit(format!("if ({condition}) {{"));
                out.push_break(1);
                out.append(Rope::indented(
                    1,
                    self.emit_conditional_active_branch(operation, value, result, captured),
                ));
                out.push_break(0);
                out.push_lit("} else {");
                out.push_break(1);
                assign_condition(&mut out);
                out.push_break(0);
                out.push_lit("}");
            }
            PlannedConditionalKind::LogicalOr => {
                let value = operation.values[0];
                out.push_lit(format!("if ({condition}) {{"));
                out.push_break(1);
                assign_condition(&mut out);
                out.push_break(0);
                out.push_lit("} else {");
                out.push_break(1);
                out.append(Rope::indented(
                    1,
                    self.emit_conditional_active_branch(operation, value, result, captured),
                ));
                out.push_break(0);
                out.push_lit("}");
            }
            PlannedConditionalKind::Nullish => {
                let value = operation.values[0];
                out.push_lit(format!("if ({condition} == null) {{"));
                out.push_break(1);
                out.append(Rope::indented(
                    1,
                    self.emit_conditional_active_branch(operation, value, result, captured),
                ));
                out.push_break(0);
                out.push_lit("} else {");
                out.push_break(1);
                assign_condition(&mut out);
                out.push_break(0);
                out.push_lit("}");
            }
            PlannedConditionalKind::Ternary {
                consequent,
                alternate,
            } => {
                let branch = |out: &mut Rope<'a>, content: &PlannedBranch| match content {
                    PlannedBranch::Value(expr) => {
                        out.append(Rope::indented(1, deliver_value(*expr, result)));
                    }
                    PlannedBranch::Source(span) => {
                        out.push_lit(format!("{result} = "));
                        let mut source = Rope::new();
                        source.push_src(&self.source[span.start..span.end], span.start);
                        push_grouped(out, source);
                        out.push_lit(";");
                    }
                };
                out.push_lit(format!("if ({condition}) {{"));
                out.push_break(1);
                branch(&mut out, consequent);
                out.push_break(0);
                out.push_lit("} else {");
                out.push_break(1);
                branch(&mut out, alternate);
                out.push_break(0);
                out.push_lit("}");
            }
            PlannedConditionalKind::OptionalCall {
                arguments,
                type_args,
            } => {
                out.push_lit(format!("if ({condition} != null) {{"));
                out.push_break(1);
                // The active branch: arguments in source order — captures
                // for those before a tt value, regions for the values —
                // then the call, through the receiver when the callee is a
                // member reference.
                let mut body = Rope::new();
                for argument in arguments {
                    match argument {
                        PlannedOperand::Value(expr) => {
                            let name = self.value_name_of(*expr);
                            body.push_lit(format!("let {name};"));
                            body.push_break(0);
                            body.append(deliver_value(*expr, name));
                            body.push_break(0);
                        }
                        PlannedOperand::Source {
                            span,
                            capture: Some(slot),
                            ..
                        } => {
                            if captured.insert(*slot) {
                                body.push_lit(format!("const {} = (", self.value_slot_name(*slot)));
                                body.push_src(&self.source[span.start..span.end], span.start);
                                body.push_lit(");");
                                body.push_break(0);
                            }
                        }
                        PlannedOperand::Source { capture: None, .. } => {}
                    }
                }
                body.push_lit(format!("{result} = {condition}"));
                if let Some(span) = type_args {
                    body.push_src(&self.source[span.start..span.end], span.start);
                }
                let receiver = match &operation.condition {
                    PlannedEvaluationInput::Source {
                        mode: EvaluationInputMode::MemberReference,
                        receiver,
                        ..
                    } => Some(
                        receiver
                            .unwrap_or_else(|| crate::ice::bug!("member callee has no receiver")),
                    ),
                    _ => None,
                };
                match receiver {
                    Some(receiver) => {
                        body.push_lit(".call(");
                        self.push_planned_receiver(&receiver, false, &mut body);
                        for argument in arguments {
                            body.push_lit(", ");
                            self.push_operand(argument, &mut body);
                        }
                        body.push_lit(");");
                    }
                    None => {
                        body.push_lit("(");
                        for (index, argument) in arguments.iter().enumerate() {
                            if index > 0 {
                                body.push_lit(", ");
                            }
                            self.push_operand(argument, &mut body);
                        }
                        body.push_lit(");");
                    }
                }
                out.append(Rope::indented(1, body));
                out.push_break(0);
                out.push_lit("} else {");
                out.push_break(1);
                out.push_lit(format!("{result} = undefined;"));
                out.push_break(0);
                out.push_lit("}");
            }
        }
        out.push_break(0);
        self.conditional_region_depth
            .set(self.conditional_region_depth.get() - 1);
        let primary = operation.values[0];
        let (kind, start, end, extent) = self.value_anchor(primary);
        let mut anchored = Rope::new();
        anchored.anchored(kind, start, end, extent, Rope::scoped(out));
        anchored
    }

    fn emit_conditional_active_branch(
        &self,
        operation: &PlannedConditionalOperation,
        value: ExprId,
        result: &str,
        captured: &mut HashSet<crate::evaluation_ir::ValueSlotId>,
    ) -> Rope<'a> {
        let Some(branch) = operation.active_branch else {
            return self
                .emit_continued_expr(value, &ValueContinuation::assign(result))
                .unwrap_or_else(|| {
                    crate::ice::bug!("conditional operation value is not structurally emit-able")
                });
        };
        let value_slot = self.value_name_of(value);
        let mut out = Rope::new();
        out.push_lit(format!("let {value_slot};"));
        out.push_break(0);
        let mut lowered = self
            .emit_continued_expr(value, &ValueContinuation::assign(value_slot))
            .unwrap_or_else(|| {
                crate::ice::bug!("conditional branch value is not structurally emit-able")
            });
        for step in &operation.active_steps {
            lowered = self.emit_scheduled_step(step, lowered, captured);
        }
        out.append(lowered);
        out.push_break(0);
        out.push_lit(format!("{result} = "));
        push_grouped(
            &mut out,
            self.source_range_with_value_slots(branch, &operation.values),
        );
        out.push_lit(";");
        out
    }

    fn source_range_with_value_slots(&self, span: SourceSpan, values: &[ExprId]) -> Rope<'a> {
        let mut replacements: Vec<_> = values
            .iter()
            .map(|expr| {
                let (kind, start, head_end, extent) = self.value_anchor(*expr);
                (*expr, kind, SourceSpan { start, end: extent }, head_end)
            })
            .filter(|(_, _, value, _)| span.start <= value.start && value.end <= span.end)
            .collect();
        replacements.sort_unstable_by_key(|(_, _, value, _)| value.start);
        let mut out = Rope::new();
        let mut cursor = span.start;
        for (expr, kind, value, head_end) in replacements {
            if cursor < value.start {
                out.append(self.source_range_rope(hir::Span {
                    start: cursor,
                    end: value.start,
                }));
            }
            let mut slot = Rope::new();
            slot.push_lit(self.value_name_of(expr).to_owned());
            out.anchored(kind, value.start, head_end, value.end, slot);
            cursor = value.end;
        }
        if cursor < span.end {
            out.append(self.source_range_rope(hir::Span {
                start: cursor,
                end: span.end,
            }));
        }
        out
    }

    fn source_range_with_nested_schedule(
        &self,
        span: SourceSpan,
        expr: ExprId,
        schedule: &EvaluationSchedule,
    ) -> Rope<'a> {
        let (kind, start, head_end, extent) = self.value_anchor(expr);
        let mut replacements: Vec<NestedSourceReplacement> = vec![(
            SourceSpan { start, end: extent },
            self.value_name_of(expr).to_owned(),
            Some((kind, head_end)),
        )];
        for step in schedule.steps() {
            for input in &step.inputs {
                if let PlannedEvaluationInput::Source { source, target, .. } = input {
                    replacements.push((*source, self.value_slot_name(*target).to_owned(), None));
                }
            }
        }
        replacements.retain(|(source, _, _)| span.start <= source.start && source.end <= span.end);
        replacements.sort_unstable_by_key(|(source, _, _)| source.start);
        let mut out = Rope::new();
        let mut cursor = span.start;
        for (source, slot, anchor) in replacements {
            if source.start < cursor {
                continue;
            }
            if cursor < source.start {
                out.append(self.source_range_rope(hir::Span::new(cursor, source.start)));
            }
            if let Some((kind, head_end)) = anchor {
                let mut rendered = Rope::new();
                rendered.push_lit(slot);
                out.anchored(kind, source.start, head_end, source.end, rendered);
            } else {
                out.push_lit(slot);
            }
            cursor = source.end;
        }
        if cursor < span.end {
            out.append(self.source_range_rope(hir::Span::new(cursor, span.end)));
        }
        out
    }

    /// One rebuilt argument of an optional call.
    fn push_operand(&self, operand: &PlannedOperand, out: &mut Rope<'a>) {
        match operand {
            PlannedOperand::Value(expr) => {
                out.push_lit(self.value_name_of(*expr).to_owned());
            }
            PlannedOperand::Source {
                spread,
                capture: Some(slot),
                ..
            } => {
                if *spread {
                    out.push_lit("...");
                }
                out.push_lit(self.value_slot_name(*slot).to_owned());
            }
            PlannedOperand::Source {
                span,
                spread,
                capture: None,
            } => {
                if *spread {
                    out.push_lit("...");
                }
                out.push_src(&self.source[span.start..span.end], span.start);
            }
        }
    }

    /// Captures the condition (or callee) of a conditional operation and
    /// returns the name the region tests and calls. A member callee keeps
    /// its receiver in a slot of its own — the rebuilt call goes through
    /// `.call(receiver, ...)`, so no `.bind` is written.
    fn emit_condition_capture(
        &self,
        condition: &PlannedEvaluationInput,
        captured: &mut HashSet<crate::evaluation_ir::ValueSlotId>,
        out: &mut Rope<'a>,
    ) -> String {
        match condition {
            PlannedEvaluationInput::Slot { slot, .. } => self.value_slot_name(*slot).to_owned(),
            // An inert condition needs no capture; re-reading it in each
            // branch is unobservable and yields the same value.
            PlannedEvaluationInput::Stable { source } => {
                format!("({})", &self.source[source.start..source.end])
            }
            PlannedEvaluationInput::Source {
                source,
                target,
                receiver: Some(receiver),
                mode: EvaluationInputMode::MemberReference,
            } => {
                if captured.insert(*target) {
                    let receiver_source = self.capture_planned_receiver(receiver, captured, out);
                    out.push_lit(format!("const {} = (", self.value_slot_name(*target)));
                    if source.start < receiver_source.start {
                        out.push_src(
                            &self.source[source.start..receiver_source.start],
                            source.start,
                        );
                    }
                    if !self.push_pipeline_member_reference(*source, receiver_source, receiver, out)
                    {
                        self.push_planned_receiver(receiver, true, out);
                        if receiver_source.end < source.end {
                            out.push_src(
                                &self.source[receiver_source.end..source.end],
                                receiver_source.end,
                            );
                        }
                    }
                    out.push_lit(");");
                    out.push_break(0);
                }
                self.value_slot_name(*target).to_owned()
            }
            PlannedEvaluationInput::Source { source, target, .. } => {
                if captured.insert(*target) {
                    out.push_lit(format!("const {} = (", self.value_slot_name(*target)));
                    out.push_src(&self.source[source.start..source.end], source.start);
                    out.push_lit(");");
                    out.push_break(0);
                }
                self.value_slot_name(*target).to_owned()
            }
        }
    }

    fn capture_planned_receiver(
        &self,
        receiver: &PlannedReceiver,
        captured: &mut HashSet<crate::evaluation_ir::ValueSlotId>,
        out: &mut Rope<'a>,
    ) -> SourceSpan {
        match *receiver {
            PlannedReceiver::Captured { source, slot } => {
                if captured.insert(slot) {
                    out.push_lit(format!("const {} = (", self.value_slot_name(slot)));
                    out.push_src(&self.source[source.start..source.end], source.start);
                    out.push_lit(");");
                    out.push_break(0);
                }
                source
            }
            PlannedReceiver::Stable { source } => source,
        }
    }

    fn push_planned_receiver(&self, receiver: &PlannedReceiver, mapped: bool, out: &mut Rope<'a>) {
        match *receiver {
            PlannedReceiver::Captured { slot, .. } => {
                out.push_lit(self.value_slot_name(slot).to_owned());
            }
            PlannedReceiver::Stable { source } => {
                let text = &self.source[source.start..source.end];
                if mapped {
                    out.push_src(text, source.start);
                } else {
                    out.push_lit(text.to_owned());
                }
            }
        }
    }

    fn push_pipeline_member_reference(
        &self,
        source: SourceSpan,
        receiver_source: SourceSpan,
        receiver: &PlannedReceiver,
        out: &mut Rope<'a>,
    ) -> bool {
        let suffix = self.core.exprs.iter().find_map(|expr| {
            let Expr::Apply(apply) = expr else {
                return None;
            };
            let head = apply.head?;
            let Expr::Opaque(head_node) = &self.core.exprs[head.index()] else {
                return None;
            };
            if SourceSpan::from(self.span(*head_node)) != receiver_source {
                return None;
            }
            apply.steps.iter().find_map(|step| {
                let ApplyMode::Postfix { optional: true } = step.mode else {
                    return None;
                };
                let step_span = SourceSpan::from(self.span(step.node));
                (source.start == receiver_source.start
                    && step_span.start < source.end
                    && source.end <= step_span.end)
                    .then_some(SourceSpan {
                        // Keep optional property access so a null receiver
                        // never reaches the member lookup. The explicit
                        // branch below controls argument evaluation.
                        start: step_span.start,
                        end: source.end,
                    })
            })
        });
        let Some(suffix) = suffix else {
            return false;
        };
        self.push_planned_receiver(receiver, true, out);
        out.push_src(&self.source[suffix.start..suffix.end], suffix.start);
        true
    }

    /// The join slot name of a tt value, from the plan.
    fn value_name_of(&self, expr: ExprId) -> &str {
        self.value_slots
            .get(&expr)
            .map(String::as_str)
            .unwrap_or_else(|| crate::ice::bug!("conditional operation value has no slot"))
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
                let receiver = receiver
                    .unwrap_or_else(|| crate::ice::bug!("member reference has no receiver"));
                let receiver_source =
                    self.capture_planned_receiver(&receiver, captured, &mut prefix);
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
                self.push_planned_receiver(&receiver, true, &mut prefix);
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
                    prefix.push_lit(format!("{target_name} = {target_name}.bind("));
                    self.push_planned_receiver(&receiver, false, &mut prefix);
                    prefix.push_lit(");");
                    prefix.push_break(0);
                    prefix.push_lit("}");
                    prefix.push_break(0);
                } else {
                    prefix.push_lit(").bind(");
                    self.push_planned_receiver(&receiver, false, &mut prefix);
                    prefix.push_lit(");");
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
                let input = step
                    .inputs
                    .first()
                    .unwrap_or_else(|| crate::ice::bug!("conditional schedule has no condition"));
                let condition = match input {
                    PlannedEvaluationInput::Source { target, .. }
                    | PlannedEvaluationInput::Slot { slot: target, .. } => {
                        self.value_slot_name(*target).to_owned()
                    }
                    PlannedEvaluationInput::Stable { source } => {
                        format!("({})", &self.source[source.start..source.end])
                    }
                };
                let condition = condition.as_str();
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
            HostEvaluationOperation::LoopTest => {
                crate::ice::bug!("loop-test schedule reached ordinary step emission")
            }
        }
    }

    fn value_slot_name(&self, slot: crate::evaluation_ir::ValueSlotId) -> &str {
        self.scheduled_slots
            .get(&slot)
            .map(String::as_str)
            .unwrap_or_else(|| crate::ice::bug!("scheduled value slot has no generated name"))
    }

    fn structured_value_slot(&self, expr: ExprId) -> Option<&String> {
        self.value_slots.get(&expr).or_else(|| {
            let Expr::Sequence(body) = &self.core.exprs[expr.index()] else {
                return None;
            };
            self.core
                .body_value_expr(*body)
                .and_then(|value| self.structured_value_slot(value))
        })
    }

    fn value_anchor(&self, expr: ExprId) -> (AnchorKind, usize, usize, usize) {
        match &self.core.exprs[expr.index()] {
            Expr::Decision(decision) => {
                let head = self.span(decision.head);
                let extent = self.span(decision.extent);
                (AnchorKind::Match, head.start, head.end, extent.end)
            }
            Expr::Propagate(propagate) => {
                let span = self.span(propagate.node);
                (AnchorKind::Try, span.start, span.end, span.end)
            }
            Expr::ResultRegion(region) => {
                let (start, end) = self.result_bind_anchor(region);
                (AnchorKind::Result, start, end, end)
            }
            Expr::Sequence(body) => {
                let value = self
                    .core
                    .body_value_expr(*body)
                    .unwrap_or_else(|| crate::ice::bug!("slotted sequence has no value"));
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
            Expr::Template(template) => template
                .parts
                .iter()
                .find_map(|part| match part {
                    TemplatePart::Interpolation(inner) if self.core.has_statement_form(*inner) => {
                        Some(self.value_anchor(*inner))
                    }
                    TemplatePart::Raw(_) | TemplatePart::Interpolation(_) => None,
                })
                .unwrap_or_else(|| crate::ice::bug!("unstructured template owns a join slot")),
            Expr::Opaque(_) => crate::ice::bug!("unstructured expression owns a join slot"),
        }
    }

    fn result_bind_anchor(&self, region: &ResultRegion) -> (usize, usize) {
        let span = self.span(region.node);
        (span.start, span.end)
    }

    fn emit_arrow_return_rewrite(&self, rewrite: &ArrowReturnRewrite) -> Rope<'a> {
        let anchored = self
            .emit_continued_expr(rewrite.expr, &ValueContinuation::assign(&rewrite.slot))
            .unwrap_or_else(|| {
                crate::ice::bug!("arrow return rewrite is not structurally emit-able")
            });
        let mut out = Rope::new();
        if rewrite.parenthesized {
            out.push_lit("(() => {");
        } else {
            out.push_lit("{");
        }
        out.push_break(1);
        out.push_lit(format!("let {}", rewrite.slot));
        self.push_contextual_type(
            &mut out,
            rewrite.contextual_type,
            rewrite.contextual_type_awaited,
        );
        out.push_lit(";");
        out.push_break(1);
        out.append(Rope::indented(1, anchored));
        out.push_break(1);
        out.push_lit(format!("return {};", rewrite.slot));
        out.push_break(0);
        out.push_lit(if rewrite.parenthesized { "})()" } else { "}" });
        Rope::scoped(out)
    }

    /// The lowering of one `try`. It opens its own layout scope: a
    /// structured propagation value writes block structure into it.
    fn emit_propagate(&self, propagate: &Propagate) -> Rope<'a> {
        let temp = temp_name(propagate.temporary);
        let mut out = self.emit_propagate_input(propagate.value, &temp);
        out.push_break(0);
        out.push_lit(format!(
            "if ({}) {{",
            result_failure_test(&temp, propagate.layout)
        ));
        out.push_break(1);
        out.push_lit(format!("return {temp};"));
        out.push_break(0);
        out.push_lit("}");
        if let Some(binding) = propagate.binding {
            out.push_break(0);
            out.push_lit(format!("{} ", binding_keyword(binding.mode)));
            out.append(self.source_rope(binding.node));
            out.push_lit(format!(" = {temp}.{};", propagate.layout.payload_field));
        }
        Rope::scoped(out)
    }

    fn emit_propagate_input(&self, value: ExprId, temp: &str) -> Rope<'a> {
        let mut out = Rope::new();
        if self.core.has_statement_form(value) && self.can_inline_continued_expr(value) {
            let slot = self
                .structured_value_slot(value)
                .unwrap_or_else(|| crate::ice::bug!("structured propagation value has no slot"));
            out.push_lit(format!("let {slot};"));
            out.push_break(0);
            out.append(
                self.emit_continued_expr(value, &ValueContinuation::assign(slot))
                    .unwrap_or_else(|| crate::ice::bug!("propagation value was not emitted")),
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
        let Some((value_index, _)) = crate::core_ir::sequence_value(statements) else {
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

    fn emit_result_region(&self, expr: ExprId, region: &ResultRegion) -> Rope<'a> {
        let mut out = Rope::new();
        self.used_expression_boundary.set(true);
        out.push_lit(if region.is_async {
            format!("(await {}(async () => {{", self.expression_boundary_name)
        } else {
            format!("{}(() => {{", self.expression_boundary_name)
        });
        out.push_break(1);
        for item in &region.items {
            let ResultRegionItem::Statements(body) = item;
            let exits = self.exits_for_expr(expr);
            let failure = ValueContinuation::returning();
            let success = failure.wrap_result_ok();
            out.append(Rope::indented(
                1,
                self.emit_result_body_with_exits(*body, &exits, &failure, &success, None)
                    .trim(),
            ));
            out.push_break(1);
        }
        if region.value.is_some() || !region.completes {
            out.push_lit("return { kind: \"Ok\" as const, value: ");
            if let Some(value) = region.value {
                push_grouped(
                    &mut out,
                    guard_line_comment(self.emit_expr(value).trim(), 0),
                );
            } else {
                out.push_lit("undefined");
            }
            out.push_lit(" };");
            out.push_break(0);
        }
        out.push_lit(if region.is_async { "}))" } else { "})" });
        Rope::scoped(out)
    }

    /// Result-owned returns inside the expression-boundary printer complete
    /// its lexical arrow. They must therefore wrap `Ok`, unlike ordinary
    /// function returns and unlike the statement-host continuation below.
    fn emit_result_body_with_exits(
        &self,
        body: hir::BodyId,
        exits: &[HostExit],
        failure: &ValueContinuation<'_>,
        success: &ValueContinuation<'_>,
        exit_label: Option<&str>,
    ) -> Rope<'a> {
        let leave =
            exit_label.map_or_else(|| "break;".to_owned(), |label| format!("break {label};"));
        let mut edits = Vec::new();
        let mut propagating_returns = Vec::new();
        for exit in exits {
            let line_start = self.source[..exit.statement.start]
                .rfind('\n')
                .map_or(0, |index| index + 1);
            let line_indent = &self.source[line_start..exit.statement.start];
            let starts_own_line = line_indent.bytes().all(|byte| matches!(byte, b' ' | b'\t'));
            let inner_indent = format!("{line_indent}  ");
            if let Some((expr, propagate)) = exit
                .argument
                .and_then(|argument| self.result_return_propagate(body, argument))
            {
                propagating_returns.push((
                    exit.statement,
                    exit.argument.expect("checked above"),
                    expr,
                    propagate,
                ));
                continue;
            }
            match exit.argument {
                Some(argument) => {
                    let contextual_value = success
                        .assignment_target()
                        .filter(|target| {
                            self.owner_slot_rewrites.iter().any(|rewrite| {
                                rewrite.slot.as_str() == *target
                                    && rewrite.contextual_type.is_some()
                            }) || self.arrow_return_rewrites.iter().any(|rewrite| {
                                rewrite.slot.as_str() == *target
                                    && rewrite.contextual_type.is_some()
                            })
                        })
                        .map_or_else(String::new, |target| {
                            format!(": Extract<typeof {target}, {{ kind: \"Ok\" }}>[\"value\"]")
                        });
                    edits.push(LocalSourceEdit {
                        span: SourceSpan {
                            start: exit.statement.start,
                            end: argument.start,
                        },
                        text: if starts_own_line {
                            format!(
                                "{{\n{inner_indent}const {}{} = ",
                                self.result_return_name, contextual_value
                            )
                        } else {
                            format!(
                                "{{ const {}{} = ",
                                self.result_return_name, contextual_value
                            )
                        },
                        result_return_mark: Some(argument),
                    });
                    edits.push(LocalSourceEdit {
                        span: SourceSpan {
                            start: argument.end,
                            end: exit.statement.end,
                        },
                        text: if starts_own_line {
                            format!(
                                ";\n{inner_indent}{}{}{};{}\n{line_indent}}}",
                                success.assignment_prefix(false),
                                self.result_return_name,
                                success.assignment_suffix(false),
                                if success.assigns() {
                                    format!("\n{inner_indent}{leave}")
                                } else {
                                    String::new()
                                }
                            )
                        } else {
                            format!(
                                "; {}{}{};{} }}",
                                success.assignment_prefix(false),
                                self.result_return_name,
                                success.assignment_suffix(false),
                                if success.assigns() {
                                    format!(" {leave}")
                                } else {
                                    String::new()
                                }
                            )
                        },
                        result_return_mark: None,
                    });
                }
                None => {
                    let mut text = if starts_own_line {
                        format!(
                            "{{\n{inner_indent}{}undefined{};",
                            success.assignment_prefix(false),
                            success.assignment_suffix(false),
                        )
                    } else {
                        format!(
                            "{{{}undefined{};",
                            success.assignment_prefix(false),
                            success.assignment_suffix(false),
                        )
                    };
                    if success.assigns() {
                        if starts_own_line {
                            text.push('\n');
                            text.push_str(&inner_indent);
                        } else {
                            text.push(' ');
                        }
                        text.push_str(&leave);
                    }
                    if starts_own_line {
                        text.push('\n');
                        text.push_str(line_indent);
                        text.push('}');
                    } else {
                        text.push_str(" }");
                    }
                    edits.push(LocalSourceEdit {
                        span: exit.statement,
                        text,
                        result_return_mark: None,
                    });
                }
            }
        }
        edits.sort_unstable_by_key(|edit| edit.span.start);
        let context = ResultEmissionContext {
            failure,
            success,
            exit_label,
        };
        self.emit_result_statements_with_exits(
            &self.core.bodies[body.index()].statements,
            exits,
            &edits,
            &propagating_returns,
            &context,
        )
    }

    /// Finds a `return try value;` whose `try` belongs directly to this
    /// Result region. The host parser gives the return argument span, while
    /// Core IR gives the propagation target; both facts are required before
    /// replacing an entire TypeScript statement.
    fn result_return_propagate(
        &self,
        body: hir::BodyId,
        argument: SourceSpan,
    ) -> Option<(ExprId, &Propagate)> {
        fn find<'a>(
            emitter: &'a Emitter<'a>,
            expr: ExprId,
            argument: SourceSpan,
        ) -> Option<(ExprId, &'a Propagate)> {
            match &emitter.core.exprs[expr.index()] {
                Expr::Propagate(propagate)
                    if matches!(propagate.exit, ExitTarget::ResultRegion(_)) && {
                        let span = SourceSpan::from(emitter.span(propagate.node));
                        argument.start <= span.start && span.end <= argument.end
                    } =>
                {
                    Some((expr, propagate))
                }
                Expr::Sequence(body) => emitter.core.bodies[body.index()]
                    .statements
                    .iter()
                    .find_map(|statement| match statement {
                        Statement::Expr(inner) => find(emitter, *inner, argument),
                        _ => None,
                    }),
                _ => None,
            }
        }
        self.core.bodies[body.index()]
            .statements
            .iter()
            .find_map(|statement| {
                let Statement::Expr(expr) = statement else {
                    return None;
                };
                find(self, *expr, argument)
            })
    }

    fn emit_result_statements_with_exits(
        &self,
        statements: &[Statement],
        exits: &[HostExit],
        edits: &[LocalSourceEdit],
        propagating_returns: &[(SourceSpan, SourceSpan, ExprId, &Propagate)],
        context: &ResultEmissionContext<'_, '_>,
    ) -> Rope<'a> {
        let mut out = Rope::new();
        for statement in statements {
            match statement {
                Statement::Opaque(node) => {
                    // An opaque segment may contain both a nested function
                    // and the Result-owned return that follows it.  Dropping
                    // the whole segment would also drop the nested function's
                    // punctuation and source-backed propagation.  Erase only
                    // the return being rebuilt below; every surrounding byte
                    // remains pass-through.
                    let mut opaque_edits = edits.to_vec();
                    opaque_edits.extend(propagating_returns.iter().map(
                        |(return_span, _, _, _)| LocalSourceEdit {
                            span: SourceSpan {
                                start: return_span.start.max(self.span(*node).start),
                                end: return_span.end.min(self.span(*node).end),
                            },
                            text: String::new(),
                            result_return_mark: None,
                        },
                    ));
                    opaque_edits.retain(|edit| edit.span.start < edit.span.end);
                    opaque_edits.sort_unstable_by_key(|edit| edit.span.start);
                    out.append(self.source_rope_with_edits(*node, &opaque_edits));
                }
                Statement::Expr(expr) => {
                    if let Some((return_span, argument, _, propagate)) = propagating_returns
                        .iter()
                        .find(|(_, _, candidate, _)| candidate == expr)
                    {
                        let mut replacement = self.emit_region_propagate(
                            propagate,
                            context.failure,
                            context.exit_label,
                        );
                        let temp = temp_name(propagate.temporary);
                        let mut payload = Rope::new();
                        let try_span = self.span(propagate.node);
                        if argument.start < try_span.start {
                            payload.push_src(
                                &self.source[argument.start..try_span.start],
                                argument.start,
                            );
                        }
                        payload.push_lit(format!("{temp}.{}", propagate.layout.payload_field));
                        if try_span.end < argument.end {
                            payload
                                .push_src(&self.source[try_span.end..argument.end], try_span.end);
                        }
                        replacement.push_break(0);
                        replacement.append(self.emit_value_delivery_with_exit(
                            payload,
                            None,
                            context.success,
                            context.exit_label,
                            Some(0),
                        ));
                        out.anchored(
                            AnchorKind::Try,
                            return_span.start,
                            return_span.end,
                            return_span.end,
                            replacement,
                        );
                    } else {
                        self.emit_statement_expr(*expr, &mut out);
                    }
                }
                Statement::Adt(adt) => {
                    let span = self.span(adt.node);
                    out.anchored(
                        AnchorKind::Variant,
                        span.start,
                        span.end,
                        span.end,
                        emit_adt(adt),
                    );
                }
                Statement::Import(import) => self.emit_import(import, &mut out),
                Statement::Propagate(propagate) => {
                    let span = self.span(propagate.node);
                    let emitted = if matches!(propagate.exit, ExitTarget::ResultRegion(_)) {
                        self.emit_region_propagate(propagate, context.failure, context.exit_label)
                    } else {
                        self.emit_propagate(propagate)
                    };
                    out.anchored(AnchorKind::Try, span.start, span.end, span.end, emitted);
                }
                Statement::Decision(decision) => self.emit_result_statement_decision(
                    decision,
                    exits,
                    context.failure,
                    context.success,
                    context.exit_label,
                    &mut out,
                ),
            }
        }
        out
    }

    fn emit_result_statement_decision(
        &self,
        decision: &Decision,
        exits: &[HostExit],
        failure: &ValueContinuation<'_>,
        success: &ValueContinuation<'_>,
        exit_label: Option<&str>,
        out: &mut Rope<'a>,
    ) {
        let span = self.span(decision.head);
        let (kind, inner) = match &decision.kind {
            DecisionKind::LetElse { binding_mode, .. } => (
                AnchorKind::LetElse,
                self.emit_result_let_else(
                    decision,
                    *binding_mode,
                    exits,
                    failure,
                    success,
                    exit_label,
                ),
            ),
            DecisionKind::IfLet => (
                AnchorKind::IfLet,
                self.emit_result_if_let(decision, exits, failure, success, exit_label),
            ),
            DecisionKind::Match { .. } => {
                crate::ice::bug!("expression decision in a result statement body")
            }
        };
        out.anchored(kind, span.start, span.end, span.end, inner);
    }

    fn emit_result_let_else(
        &self,
        decision: &Decision,
        mode: BindingMode,
        exits: &[HostExit],
        failure: &ValueContinuation<'_>,
        success: &ValueContinuation<'_>,
        exit_label: Option<&str>,
    ) -> Rope<'a> {
        let subject = &decision.subjects[0];
        let temp = temp_name(subject.temporary);
        let arm = &decision.arms[0];
        let mut out = self.emit_subject_initialization(subject, &temp, decision.head);
        out.push_break(0);
        out.push_lit("if (");
        let DecisionKind::LetElse {
            direct_variants, ..
        } = &decision.kind
        else {
            crate::ice::bug!("let-else has wrong Core decision kind")
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
        out.push_lit(") {");
        let MissAction::Execute(body) = decision.miss else {
            crate::ice::bug!("let-else has no else body")
        };
        let body = self
            .emit_result_body_with_exits(body, exits, failure, success, exit_label)
            .trim();
        out.push_break(1);
        out.append(Rope::indented(1, body));
        out.push_break(0);
        out.push_lit("}");
        let mut recovery = BindingRecovery::new(self, &arm.pattern);
        out.push_break(0);
        out.append(
            self.emit_bindings(&arm.pattern, decision, Some(mode), &mut recovery, Some(0))
                .trim(),
        );
        Rope::scoped(out)
    }

    fn emit_result_if_let(
        &self,
        decision: &Decision,
        exits: &[HostExit],
        failure: &ValueContinuation<'_>,
        success: &ValueContinuation<'_>,
        exit_label: Option<&str>,
    ) -> Rope<'a> {
        let subject = &decision.subjects[0];
        let temp = temp_name(subject.temporary);
        let arm = &decision.arms[0];
        let mut out = Rope::new();
        out.push_lit("{");
        out.push_break(1);
        out.append(self.emit_subject_initialization(subject, &temp, decision.head));
        out.push_break(1);
        out.push_lit("if (");
        out.append(self.emit_condition(&arm.pattern, decision));
        out.push_lit(") {");
        let mut recovery = BindingRecovery::new(self, &arm.pattern);
        let bindings = self.emit_bindings(&arm.pattern, decision, None, &mut recovery, Some(2));
        if !bindings.is_empty() {
            out.push_break(2);
            out.append(bindings.trim());
        }
        let ArmAction::Execute(body) = arm.action else {
            crate::ice::bug!("if-let has no then body")
        };
        out.push_break(2);
        out.append(Rope::indented(
            2,
            self.emit_result_body_with_exits(body, exits, failure, success, exit_label)
                .trim(),
        ));
        out.push_break(1);
        out.push_lit("}");
        match &decision.miss {
            MissAction::Execute(body) => {
                out.push_lit(" else {");
                out.push_break(2);
                out.append(Rope::indented(
                    2,
                    self.emit_result_body_with_exits(*body, exits, failure, success, exit_label)
                        .trim(),
                ));
                out.push_break(1);
                out.push_lit("}");
            }
            MissAction::Decision(inner) => {
                out.push_lit(" else ");
                out.append(self.emit_result_if_let(inner, exits, failure, success, exit_label));
            }
            MissAction::Nothing => {}
            MissAction::ThrowUnexpected(_) => {
                crate::ice::bug!("if-let has match miss action")
            }
        }
        out.push_break(0);
        out.push_lit("}");
        Rope::scoped(out)
    }

    fn emit_result_region_continued(
        &self,
        expr: ExprId,
        region: &ResultRegion,
        continuation: &ValueContinuation<'_>,
    ) -> Rope<'a> {
        let mut out = Rope::new();
        // The block's own source follows, leading whitespace included, so
        // the opener does not break the line the region body starts on.
        let exit_label = continuation.assignment_target();
        if let Some(label) = exit_label {
            out.push_lit(format!("{label}: {{"));
        } else {
            out.push_lit("{");
        }
        out.push_break(1);
        let success = continuation.wrap_result_ok();
        let exits = self.exits_for_expr(expr);
        for item in &region.items {
            let ResultRegionItem::Statements(body) = item;
            out.append(Rope::indented(
                1,
                self.emit_result_body_with_exits(*body, &exits, continuation, &success, exit_label)
                    .trim(),
            ));
        }
        if let Some(value) = region.value {
            out.push_break(1);
            if let Some(structured) = self.emit_continued_expr(value, &success) {
                out.append(Rope::indented(1, structured));
                if continuation.assigns() {
                    push_control_break(&mut out, 1, exit_label);
                }
            } else {
                out.append(Rope::indented(
                    1,
                    self.emit_value_delivery_with_exit(
                        guard_line_comment(self.emit_expr(value).trim(), 0),
                        None,
                        &success,
                        exit_label,
                        Some(0),
                    ),
                ));
            }
        }
        out.push_break(0);
        out.push_lit("}");
        let (binding_start, binding_end) = self.result_bind_anchor(region);
        let mut anchored = Rope::new();
        anchored.anchored(
            AnchorKind::Result,
            binding_start,
            binding_end,
            binding_end,
            Rope::scoped(out),
        );
        anchored
    }

    fn emit_region_propagate(
        &self,
        propagate: &Propagate,
        continuation: &ValueContinuation<'_>,
        exit_label: Option<&str>,
    ) -> Rope<'a> {
        let temp = temp_name(propagate.temporary);
        let mut out = self.emit_propagate_input(propagate.value, &temp);
        out.push_break(0);
        out.push_lit(format!(
            "if ({}) {{",
            result_failure_test(&temp, propagate.layout)
        ));
        out.push_break(1);
        let mut value = Rope::new();
        value.push_lit(temp.clone());
        out.append(Rope::indented(
            1,
            self.emit_value_delivery_with_exit(value, None, continuation, exit_label, Some(0)),
        ));
        out.push_break(0);
        out.push_lit("}");
        if let Some(binding) = propagate.binding {
            out.push_break(0);
            out.push_lit(format!("{} ", binding_keyword(binding.mode)));
            out.append(self.source_rope(binding.node));
            out.push_lit(format!(" = {temp}.{};", propagate.layout.payload_field));
        }
        Rope::scoped(out)
    }

    fn emit_apply(&self, apply: &Apply) -> Rope<'a> {
        let start = self.span(apply.node).start;
        let end = apply.steps.last().map_or_else(
            || self.span(apply.node).end,
            |step| self.span(step.node).end,
        );
        let inner = match apply.head {
            Some(head) => {
                let mut acc = guard_line_comment(self.emit_expr(head).trim(), 0);
                let mut accumulator_is_inert = self.expression_is_inert(head);
                // Where the value flowing into the current step was
                // produced: the head, then each step in turn — the place a
                // label on a rejected value points back at.
                let mut produced = self.span(apply.node);
                for step in &apply.steps {
                    let step_span = self.span(step.node);
                    let context = Some((produced.start, produced.end));
                    let body = guard_line_comment(self.emit_expr(step.value).trim(), 0);
                    let mut next = Rope::new();
                    // The value flowing into a step occupies a position the
                    // checker types against that step, so a diagnostic that
                    // lands on it belongs to the step that rejected the
                    // value — each piped-value position is anchored to the
                    // step consuming it. Verbatim spans still resolve
                    // exactly; only glue-crossing spans re-home here.
                    let mut input = Rope::new();
                    match step.mode {
                        ApplyMode::Postfix { .. } => {
                            push_receiver(&mut input, acc);
                            next.anchored_with_context(
                                AnchorKind::Pipe,
                                step_span.start,
                                step_span.end,
                                end,
                                context,
                                input,
                            );
                            next.append(body);
                        }
                        ApplyMode::Call => {
                            if accumulator_is_inert {
                                push_grouped(&mut next, body);
                                next.push_lit("(");
                                push_grouped(&mut input, acc);
                                next.anchored_with_context(
                                    AnchorKind::Pipe,
                                    step_span.start,
                                    step_span.end,
                                    end,
                                    context,
                                    input,
                                );
                                next.push_lit(")");
                            } else {
                                self.used_pipe.set(true);
                                next.push_lit("$tt_ap(");
                                push_grouped(&mut input, acc);
                                next.anchored_with_context(
                                    AnchorKind::Pipe,
                                    step_span.start,
                                    step_span.end,
                                    end,
                                    context,
                                    input,
                                );
                                next.push_lit(", ");
                                push_grouped(&mut next, body);
                                next.push_lit(")");
                            }
                        }
                    }
                    acc = next;
                    // A call or member operation can return any value and
                    // can have arbitrary effects. Only the original head's
                    // syntax proof can authorize inline reordering.
                    accumulator_is_inert = false;
                    produced = step_span;
                }
                acc
            }
            None => self.emit_flow(apply, end),
        };
        let mut out = Rope::new();
        out.anchored(AnchorKind::Pipe, start, end, end, inner);
        out
    }

    fn emit_flow(&self, apply: &Apply, owner_end: usize) -> Rope<'a> {
        let mut steps = apply.steps.iter();
        let first = steps
            .next()
            .unwrap_or_else(|| crate::ice::bug!("flow has no step"));
        let mut acc = Rope::new();
        push_grouped(
            &mut acc,
            guard_line_comment(self.emit_expr(first.value).trim(), 0),
        );
        let mut produced = self.span(first.node);
        for step in steps {
            self.used_flow.set(true);
            let step_span = self.span(step.node);
            let body = guard_line_comment(self.emit_expr(step.value).trim(), 0);
            let mut next = Rope::new();
            next.push_lit("$tt_fl(");
            // The composition built so far is what this step composes onto;
            // a mismatch on it means this step rejected it (see
            // `emit_apply`).
            next.anchored_with_context(
                AnchorKind::Pipe,
                step_span.start,
                step_span.end,
                owner_end,
                Some((produced.start, produced.end)),
                acc,
            );
            match step.mode {
                ApplyMode::Postfix { .. } => {
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
            produced = step_span;
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
                crate::ice::bug!("expression decision in a statement body")
            }
        };
        out.anchored(kind, span.start, span.end, span.end, inner);
    }

    fn emit_subject_initialization(
        &self,
        subject: &crate::core_ir::Subject,
        temp: &str,
        mark: NodeId,
    ) -> Rope<'a> {
        let mut out = Rope::new();
        if self.core.has_statement_form(subject.value)
            && self.can_inline_continued_expr(subject.value)
        {
            out.push_lit("let ");
            out.push_mark(self.span(mark).start);
            out.push_lit(format!("{temp};"));
            out.push_lit(" ");
            out.append(
                self.emit_continued_expr(subject.value, &ValueContinuation::assign(temp))
                    .unwrap_or_else(|| crate::ice::bug!("decision subject was not emitted")),
            );
        } else {
            out.push_lit("const ");
            out.push_mark(self.span(mark).start);
            out.push_lit(format!("{temp} = "));
            push_grouped(&mut out, self.emit_expr(subject.value).trim());
            out.push_lit(";");
        }
        out
    }

    fn emit_let_else(&self, decision: &Decision, mode: BindingMode) -> Rope<'a> {
        let subject = &decision.subjects[0];
        let temp = temp_name(subject.temporary);
        let arm = &decision.arms[0];
        let mut out = self.emit_subject_initialization(subject, &temp, decision.head);
        out.push_break(0);
        out.push_lit("if (");
        let DecisionKind::LetElse {
            direct_variants, ..
        } = &decision.kind
        else {
            crate::ice::bug!("let-else has wrong Core decision kind")
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
        out.push_lit(") {");
        let MissAction::Execute(body) = decision.miss else {
            crate::ice::bug!("let-else has no else body")
        };
        out.push_break(1);
        out.append(Rope::indented(1, self.emit_body(body).trim()));
        out.push_break(0);
        out.push_lit("}");
        let mut recovery = BindingRecovery::new(self, &arm.pattern);
        out.push_break(0);
        out.append(
            self.emit_bindings(&arm.pattern, decision, Some(mode), &mut recovery, Some(0))
                .trim(),
        );
        Rope::scoped(out)
    }

    fn emit_if_let(&self, decision: &Decision) -> Rope<'a> {
        let subject = &decision.subjects[0];
        let temp = temp_name(subject.temporary);
        let arm = &decision.arms[0];
        let mut out = Rope::new();
        out.push_lit("{");
        out.push_break(1);
        out.append(self.emit_subject_initialization(subject, &temp, decision.head));
        out.push_break(1);
        out.push_lit("if (");
        out.append(self.emit_condition(&arm.pattern, decision));
        out.push_lit(") {");
        let mut recovery = BindingRecovery::new(self, &arm.pattern);
        let bindings = self.emit_bindings(&arm.pattern, decision, None, &mut recovery, Some(2));
        if !bindings.is_empty() {
            out.push_break(2);
            out.append(bindings.trim());
        }
        let ArmAction::Execute(body) = arm.action else {
            crate::ice::bug!("if-let has no then body")
        };
        out.push_break(2);
        out.append(Rope::indented(2, self.emit_body(body).trim()));
        out.push_break(1);
        out.push_lit("}");
        match &decision.miss {
            MissAction::Execute(body) => {
                out.push_lit(" else {");
                out.push_break(2);
                out.append(Rope::indented(2, self.emit_body(*body).trim()));
                out.push_break(1);
                out.push_lit("}");
            }
            MissAction::Decision(inner) => {
                out.push_lit(" else ");
                out.append(self.emit_if_let(inner));
            }
            MissAction::Nothing => {}
            MissAction::ThrowUnexpected(_) => {
                crate::ice::bug!("if-let has match miss action")
            }
        }
        out.push_break(0);
        out.push_lit("}");
        Rope::scoped(out)
    }

    fn emit_value_decision(
        &self,
        decision: &Decision,
        continuation: &ValueContinuation<'_>,
        exits: &[HostExit],
    ) -> Rope<'a> {
        let DecisionKind::Match { dispatch, .. } = decision.kind else {
            crate::ice::bug!("value decision is not a match")
        };
        let mut out = Rope::new();
        // The region needs a label exactly when a rewritten exit sits
        // inside a loop or `switch` the arm body wrote, which would swallow
        // the `break` the rewrite emits. Otherwise the dispatch the region
        // already generates — an if-chain's `do { … } while (false)`, or
        // the `switch` itself — is the nearest `break` target, so the
        // labeled block around it would be a second exit target for the
        // same region (TASK-199, TASK-160 §6).
        let label = continuation
            .assignment_target()
            .filter(|_| decision_has_block_arm(decision))
            .filter(|_| exits.iter().any(|exit| exit.captured_break))
            .map(exit_label);
        if let Some(label) = &label {
            out.push_lit(format!("{label}: "));
        }
        if continuation.is_expression() {
            crate::ice::bug!("match reached expression emission without a host rewrite")
        }
        out.push_lit("{");
        for subject in &decision.subjects {
            out.push_break(1);
            out.append(self.emit_subject_initialization(
                subject,
                &temp_name(subject.temporary),
                decision.head,
            ));
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
        out.push_lit("}");
        Rope::scoped(out)
    }

    fn emit_continued_expr(
        &self,
        expr: ExprId,
        continuation: &ValueContinuation<'_>,
    ) -> Option<Rope<'a>> {
        if !self.core.has_statement_form(expr) {
            return None;
        }
        if let Some(schedule) = self.nested_schedules.get(&expr)
            && !schedule.steps().is_empty()
            && !self.active_scheduled_exprs.borrow().contains(&expr)
        {
            let slot = self
                .value_slots
                .get(&expr)
                .unwrap_or_else(|| crate::ice::bug!("scheduled nested value has no slot"));
            self.active_scheduled_exprs.borrow_mut().push(expr);
            let mut action = self.emit_continued_expr(expr, &ValueContinuation::assign(slot))?;
            let popped = self.active_scheduled_exprs.borrow_mut().pop();
            debug_assert_eq!(popped, Some(expr));
            let mut captured = HashSet::new();
            for step in schedule.steps() {
                action = self.emit_scheduled_step(step, action, &mut captured);
            }
            let parent = schedule
                .steps()
                .last()
                .map(|step| step.parent)
                .unwrap_or_else(|| crate::ice::bug!("nested schedule lost its parent"));
            let mut out = Rope::new();
            out.push_lit(format!("let {slot};"));
            out.push_break(0);
            out.append(action);
            out.push_break(0);
            out.append(self.emit_value_delivery(
                self.source_range_with_nested_schedule(parent, expr, schedule),
                None,
                continuation,
            ));
            return Some(Rope::scoped(out));
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
                Some(self.emit_result_region_continued(expr, region, continuation))
            }
            Expr::Propagate(propagate) => {
                Some(self.emit_expression_propagate(propagate, continuation))
            }
            Expr::Sequence(body) => self.emit_sequence_continued(*body, continuation),
            Expr::Apply(apply) => self.emit_apply_continued(expr, apply, continuation),
            Expr::Template(template) => {
                let mut out = Rope::new();
                for part in &template.parts {
                    let TemplatePart::Interpolation(inner) = part else {
                        continue;
                    };
                    if !self.core.has_statement_form(*inner) {
                        continue;
                    }
                    let slot = self
                        .structured_value_slot(*inner)
                        .unwrap_or_else(|| crate::ice::bug!("nested template value has no slot"));
                    out.push_lit(format!("let {slot};"));
                    out.push_break(0);
                    out.append(self.emit_continued_expr(*inner, &ValueContinuation::assign(slot))?);
                    out.push_break(0);
                }
                out.append(self.emit_value_delivery(
                    self.emit_template(template),
                    None,
                    continuation,
                ));
                Some(Rope::scoped(out))
            }
            Expr::Opaque(_) => None,
        }
    }

    fn emit_expression_propagate(
        &self,
        propagate: &Propagate,
        continuation: &ValueContinuation<'_>,
    ) -> Rope<'a> {
        let temp = temp_name(propagate.temporary);
        let mut out = self.emit_propagate_input(propagate.value, &temp);
        out.push_break(0);
        out.push_lit(format!(
            "if ({}) {{",
            result_failure_test(&temp, propagate.layout)
        ));
        out.push_break(1);
        out.push_lit(format!("return {temp};"));
        out.push_break(0);
        out.push_lit("}");
        out.push_break(0);
        let grouped = false;
        out.push_lit(continuation.assignment_prefix(grouped));
        out.push_lit(format!("{temp}.{}", propagate.layout.payload_field));
        out.push_lit(continuation.assignment_suffix(grouped));
        out.push_lit(";");
        let span = self.span(propagate.node);
        let mut anchored = Rope::new();
        anchored.anchored(
            AnchorKind::Try,
            span.start,
            span.end,
            span.end,
            Rope::scoped(out),
        );
        anchored
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
        let start = self.span(apply.node).start;
        let end = apply.steps.last().map_or_else(
            || self.span(apply.node).end,
            |step| self.span(step.node).end,
        );
        let accumulator = self
            .value_slots
            .get(&expr)
            .unwrap_or_else(|| crate::ice::bug!("structured apply has no value slot"));
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
        if self.core.has_statement_form(head) {
            inner.append(Rope::indented(
                1,
                self.emit_continued_expr(head, &ValueContinuation::assign(accumulator))
                    .unwrap_or_else(|| crate::ice::bug!("structured apply head was not emitted")),
            ));
        } else {
            inner.push_lit(format!("{accumulator} = "));
            push_grouped(
                &mut inner,
                guard_line_comment(self.emit_expr(head).trim(), 1),
            );
            inner.push_lit(";");
        }
        let mut produced = self.span(apply.node);
        for step in &apply.steps {
            let conditionally_reached = matches!(step.mode, ApplyMode::Postfix { optional: true });
            let step_value = if self.core.has_statement_form(step.value) && !conditionally_reached {
                let slot = self
                    .structured_value_slot(step.value)
                    .unwrap_or_else(|| crate::ice::bug!("structured apply step has no value slot"));
                inner.push_break(1);
                inner.push_lit(format!("let {slot};"));
                inner.push_break(1);
                inner.append(Rope::indented(
                    1,
                    self.emit_continued_expr(step.value, &ValueContinuation::assign(slot))
                        .unwrap_or_else(|| {
                            crate::ice::bug!("structured apply step was not emitted")
                        }),
                ));
                let mut value = Rope::new();
                value.push_lit(slot.clone());
                value
            } else {
                guard_line_comment(self.emit_expr(step.value).trim(), 1)
            };
            inner.push_break(1);
            let step_span = self.span(step.node);
            let context = Some((produced.start, produced.end));
            // The re-piped accumulator is the value this step consumes — a
            // mismatch on it belongs to this step (see `emit_apply`).
            let mut input = Rope::new();
            input.push_lit(accumulator.clone());
            match step.mode {
                ApplyMode::Postfix { .. } => {
                    inner.push_lit(format!("{accumulator} = "));
                    inner.anchored_with_context(
                        AnchorKind::Pipe,
                        step_span.start,
                        step_span.end,
                        end,
                        context,
                        input,
                    );
                    inner.append(step_value);
                    inner.push_lit(";");
                }
                ApplyMode::Call => {
                    // The accumulator has already been evaluated into a
                    // collision-free compiler slot. Reading that slot is
                    // unobservable, so the callee can occupy its natural
                    // call position without changing source evaluation.
                    inner.push_lit(format!("{accumulator} = "));
                    push_grouped(&mut inner, step_value);
                    inner.push_lit("(");
                    inner.anchored_with_context(
                        AnchorKind::Pipe,
                        step_span.start,
                        step_span.end,
                        end,
                        context,
                        input,
                    );
                    inner.push_lit(");");
                }
            }
            produced = step_span;
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
        let mut out = Rope::new();
        out.anchored(AnchorKind::Pipe, start, end, end, Rope::scoped(inner));
        Some(out)
    }

    fn expression_is_inert(&self, expr: ExprId) -> bool {
        self.direct_apply_inputs.contains(&expr)
    }

    fn emit_switch(
        &self,
        decision: &Decision,
        continuation: &ValueContinuation<'_>,
        exits: &[HostExit],
        exit_label: Option<&str>,
    ) -> Rope<'a> {
        let DecisionKind::Match { dispatch, .. } = decision.kind else {
            crate::ice::bug!("switch decision is not a match")
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
            out.push_lit(": {");
            let bindings = self.emit_bindings(&arm.pattern, decision, None, &mut recovery, Some(2));
            if !bindings.is_empty() {
                out.push_break(2);
                out.append(bindings.trim());
            }
            out.push_break(2);
            self.emit_arm_action(
                arm,
                &ArmEmissionContext {
                    depth: 2,
                    chain: false,
                    continuation,
                    exits,
                    exit_label,
                    chain_exit_label: None,
                },
                &mut out,
            );
            out.push_break(1);
            out.push_lit("}");
        }
        if !wildcard {
            out.push_break(1);
            out.push_lit("default: {");
            out.push_break(2);
            out.push_lit(self.unexpected_throw(decision));
            out.push_break(1);
            out.push_lit("}");
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
            crate::ice::bug!("conditional decision is not a match")
        };
        let mut out = Rope::new();
        let mut depth = 0;
        let chain_exit_label = needs_label.then_some("$tt_b");
        if needs_label {
            out.push_break(depth);
            out.push_lit("$tt_b: {");
            depth += 1;
        } else if continuation.assigns() {
            out.push_break(depth);
            out.push_lit("do {");
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
                out.push_lit(") {");
                let mut recovery = BindingRecovery::new(self, &arm.pattern);
                let bindings = self.emit_bindings(
                    &arm.pattern,
                    decision,
                    None,
                    &mut recovery,
                    Some(depth + 1),
                );
                if !bindings.is_empty() {
                    out.push_break(depth + 1);
                    out.append(bindings.trim());
                }
                out.push_break(depth + 1);
            }
            self.emit_arm_action(
                arm,
                &ArmEmissionContext {
                    depth: if is_any { depth } else { depth + 1 },
                    chain: true,
                    continuation,
                    exits,
                    exit_label,
                    chain_exit_label,
                },
                &mut out,
            );
            if !is_any || arm.guard.is_some() {
                out.push_break(depth);
                out.push_lit("}");
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
        } else if continuation.assigns() {
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
        let action_depth = depth + u16::from(arm.guard.is_some());
        let chain = context.chain;
        let continuation = context.continuation;
        let exits = context.exits;
        let exit_label = context.exit_label;
        let chain_exit_label = context.chain_exit_label;
        let ArmAction::Yield { body, kind } = arm.action else {
            crate::ice::bug!("match arm does not yield")
        };
        let body_expr = self.core.body_value_expr(body);
        let structured_body = matches!(kind, ArmBodyKind::Expression)
            .then(|| {
                body_expr.and_then(|expr| {
                    (!continuation.is_expression())
                        .then(|| self.emit_continued_expr(expr, continuation))
                        .flatten()
                })
            })
            .flatten();
        // A block arm's body sits between braces this lowering writes, and
        // the author's own line break and indentation after their `{` is
        // the layout the rest of their block is written against — so it
        // stays (TASK-219). Every other body is spliced into a line.
        let block_layout = matches!(kind, ArmBodyKind::Block { .. }) && chain;
        let body = if structured_body.is_some() {
            Rope::new()
        } else if matches!(kind, ArmBodyKind::Block { .. }) && continuation.assigns() {
            // Switch arms are indented as a generated case body after their
            // source is spliced in. Conditional chains retain the authored
            // source column, so their rewritten exits must not add that unit.
            let generated_indent = if chain { "" } else { "  " };
            let body =
                self.emit_body_with_exits(body, exits, continuation, exit_label, generated_indent);
            if block_layout {
                body.trim_end()
            } else {
                body.trim()
            }
        } else {
            let body = self.emit_body(body);
            if block_layout {
                body.trim_end()
            } else {
                body.trim()
            }
        };
        let mut action = Rope::new();
        match kind {
            ArmBodyKind::Expression => {
                if let Some(structured) = structured_body {
                    action.append(structured);
                    if continuation.assigns() {
                        push_control_break(&mut action, action_depth, chain_exit_label);
                    }
                } else {
                    let close = body.last_line_has_line_comment().then_some(action_depth);
                    action.append(self.emit_value_delivery_with_exit(
                        body,
                        close,
                        continuation,
                        chain_exit_label,
                        Some(action_depth),
                    ));
                }
            }
            // A block that always leaves has written the arm's value on
            // every path it takes, so neither the fall-through to
            // `undefined` nor the exit after it can be reached.
            ArmBodyKind::Block { completes } if chain => {
                action.push_lit("{");
                action.append(body);
                if completes {
                    if continuation.assigns() {
                        action.push_break(action_depth + 1);
                        action.push_lit(format!(
                            "{} = undefined;",
                            // `assigns()` is exactly "this continuation has
                            // an assignment target", tested one line above.
                            continuation
                                .assignment_target()
                                .expect("an assigning continuation names its target")
                        ));
                    }
                    push_control_break(&mut action, action_depth + 1, chain_exit_label);
                }
                action.push_break(action_depth);
                action.push_lit("}");
            }
            ArmBodyKind::Block { completes } => {
                action.append(body);
                if completes {
                    if continuation.assigns() {
                        action.push_break(action_depth + 1);
                        action.push_lit(format!(
                            "{} = undefined;",
                            // `assigns()` is exactly "this continuation has
                            // an assignment target", tested one line above.
                            continuation
                                .assignment_target()
                                .expect("an assigning continuation names its target")
                        ));
                    }
                    action.push_break(action_depth + 1);
                    action.push_lit("break;");
                }
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
            out.push_lit(") {");
            out.push_break(action_depth);
        }
        out.append(action);
        if arm.guard.is_some() {
            out.push_break(depth);
            out.push_lit("}");
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
        self.emit_value_delivery_with_exit(body, close, continuation, None, None)
    }

    fn emit_value_delivery_with_exit(
        &self,
        body: Rope<'a>,
        close: Option<u16>,
        continuation: &ValueContinuation<'_>,
        break_label: Option<&str>,
        exit_depth: Option<u16>,
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
            ValueDestination::Expression | ValueDestination::Return => out.push_lit("return "),
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
            if let Some(depth) = exit_depth {
                push_control_break(&mut out, depth, break_label);
            } else {
                push_region_break(&mut out, break_label);
            }
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
                    .unwrap_or_else(|| crate::ice::bug!("literal has no span"));
                let (literal, at) = self.source_span(span);
                out.push_src(literal, at);
                out
            }
            Test::InstanceOf { place, constructor } => {
                let mut out = self.emit_place(place, decision, None);
                out.push_lit(" instanceof ");
                let span = self.span(*constructor);
                let (source, at) = self.source_span(span);
                out.push_src(source, at);
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
        separator_depth: Option<u16>,
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
            if group_index > 0
                && let Some(depth) = separator_depth
            {
                out.push_break(depth);
            }
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
            out.push_lit(if declaration.is_some() || separator_depth.is_some() {
                ";"
            } else {
                "; "
            });
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
            .unwrap_or_else(|| crate::ice::bug!("binding has no source field"));
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
            crate::ice::bug!("switch literal alternative is not literal")
        };
        // Every pattern the lowering kept came from source text the HIR
        // recorded a span for; one without a span is a broken lowering, not
        // an input the user can write.
        let Some(span) = self.semantic.hir.source_map.pattern_span(*pattern) else {
            crate::ice::bug!("switch literal pattern has no source span")
        };
        let (text, at) = self.source_span(span);
        let mut out = Rope::new();
        out.push_src(text, at);
        out
    }

    fn variant_label(&self, plan: &PatternPlan) -> String {
        let PatternPlan::AllOf(parts) = plan else {
            crate::ice::bug!("switch variant alternative is not constructor")
        };
        let constructor = parts.iter().find_map(|part| match part {
            PatternPlan::Test(Test::Variant { constructor, .. }) => Some(constructor),
            _ => None,
        });
        // A switch is only built over variant tests, so an alternative with
        // no variant part is a plan this emitter should never have been
        // handed.
        let Some(constructor) = constructor else {
            crate::ice::bug!("switch variant alternative tests no constructor")
        };
        self.constructor_name(constructor)
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
            _ => crate::ice::bug!("match has non-match miss action"),
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
        PatternPlan::Any
        | PatternPlan::Bind(_)
        | PatternPlan::Test(Test::Variant { .. } | Test::InstanceOf { .. }) => false,
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

/// The union type and constructor object one tt `variant` becomes, laid out
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
