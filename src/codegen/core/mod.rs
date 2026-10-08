//! TypeScript target lowering from validated Core IR.
//!
//! This module is intentionally independent of `ast`: source text enters
//! only through HIR nodes and the source map. Every tt surface reaches this
//! module through a shared Core primitive.

mod authored;
mod emitter;
mod planning;

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

use super::rope::{Flat, GovernedStatement, Rope, SourcePreservation};
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
    HostOwnerKind, LoopTestKind, OptionalCallTest, SourceSpan,
};
use crate::{AnchorKind, RewrittenExtensions, SourceKind, StdImports};

use authored::*;
use emitter::*;
use planning::*;

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
    tokens: &[crate::lexer::Token],
) -> Result<LoweringPlan, LoweringFailure> {
    lowering_plan_with(
        semantic,
        core,
        source,
        source_kind,
        tokens,
        crate::program_syntax::SyntaxMode::Strict,
    )
}

pub(crate) fn lowering_plan_with(
    semantic: &SemanticFile,
    core: &CoreFile,
    source: &str,
    source_kind: SourceKind,
    tokens: &[crate::lexer::Token],
    mode: crate::program_syntax::SyntaxMode,
) -> Result<LoweringPlan, LoweringFailure> {
    if !core.requires_host_lowering() && !core.imports_std() {
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
    let mut syntax = match crate::program_syntax::ProgramSyntax::build_with(
        semantic,
        core,
        source,
        source_kind,
        tokens,
        mode,
    ) {
        Ok(syntax) => syntax,
        Err(crate::program_syntax::ProgramSyntaxError::SourceNotTypeScript { message, source }) => {
            return Err(LoweringFailure::SourceNotTypeScript { message, source });
        }
        Err(error) => {
            let source = match &error {
                crate::program_syntax::ProgramSyntaxError::Parse {
                    source: Some(source),
                    ..
                } => *source,
                _ => primary_source(),
            };
            return Err(LoweringFailure::HostProjection { error, source });
        }
    };
    let evaluation =
        crate::evaluation_ir::EvaluationFile::build(&syntax, core).map_err(|error| {
            LoweringFailure::Evaluation {
                source: error.source().unwrap_or_else(primary_source),
                error,
            }
        })?;
    let mut plan = evaluation
        .lowering_plan(core)
        .map_err(|error| LoweringFailure::Evaluation {
            source: error
                .source()
                .unwrap_or_else(|| evaluation.primary_source()),
            error,
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
    plan.completion_scopes = syntax.take_completion_scopes();
    Ok(plan)
}

fn governed_statements(
    semantic: &SemanticFile,
    core: &CoreFile,
    target: &TargetRewritePlan,
    governed: &[crate::ast::Span],
) -> Vec<GovernedStatement> {
    if governed.is_empty() {
        return Vec::new();
    }
    let node_span = |node| {
        semantic
            .hir
            .source_map
            .node_span(node)
            .map(SourceSpan::from)
    };
    let lowered = core
        .bodies
        .iter()
        .flat_map(|body| &body.statements)
        .filter_map(|statement| match statement {
            Statement::Propagate(propagate) => node_span(propagate.owner),
            Statement::Decision(decision) => node_span(decision.extent),
            Statement::Expr(expr) => structured_expr_span(semantic, core, *expr),
            Statement::Adt(adt) => node_span(adt.node),
            Statement::Opaque(_) | Statement::Import(_) => None,
        });
    let owners = target
        .owner_slots
        .iter()
        .map(|rewrite| rewrite.owner)
        .chain(target.composes.iter().map(|rewrite| rewrite.owner))
        .chain(
            target
                .for_initializer_propagations
                .iter()
                .map(|rewrite| rewrite.owner),
        )
        .chain(target.loop_tests.iter().map(|rewrite| rewrite.owner))
        .chain(target.declarator_splits.iter().map(|split| split.statement));
    let mut statements: Vec<GovernedStatement> = lowered
        .chain(owners)
        .filter_map(|span| {
            let line = governed
                .iter()
                .find(|line| line.start <= span.start && span.start <= line.end)?;
            Some(GovernedStatement {
                start: span.start,
                end: span.end.max(line.end),
                line_end: line.end,
            })
        })
        .collect();
    statements.extend(decisions(core).flat_map(|decision| {
        let ends = decision
            .arms
            .iter()
            .skip(1)
            .map(|arm| arm.gap)
            .chain([decision.trailing]);
        decision
            .arms
            .iter()
            .zip(ends)
            .filter_map(|(arm, end)| {
                let start = arm.gap?.end;
                let end = end?.start;
                let line = governed
                    .iter()
                    .find(|line| line.start <= start && start <= line.end)?;
                Some(GovernedStatement {
                    start,
                    end: end.max(line.end),
                    line_end: line.end,
                })
            })
            .collect::<Vec<_>>()
    }));
    statements.sort_unstable_by_key(|statement| (statement.start, statement.end));
    statements.dedup();
    statements
}

/// Every pattern decision of the file: those written as statements and
/// those inside expressions.
fn decisions(core: &CoreFile) -> impl Iterator<Item = &Decision> {
    core.bodies
        .iter()
        .flat_map(|body| &body.statements)
        .filter_map(|statement| match statement {
            Statement::Decision(decision) => Some(decision),
            _ => None,
        })
        .chain(core.exprs.iter().filter_map(|expr| match expr {
            Expr::Decision(decision) => Some(decision),
            _ => None,
        }))
}

fn span_index(spans: impl Iterator<Item = SourceSpan>) -> crate::span_index::SpanIndex {
    crate::span_index::SpanIndex::new(spans.map(|span| (span.start, span.end)))
}

fn match_show_body(json: &str, string: &str) -> String {
    format!(
        "{{\n  if (typeof value === \"string\") {{\n    return {json}.stringify(value);\n  }}\n  if (typeof value === \"bigint\") {{\n    return {string}(value) + \"n\";\n  }}\n  if (typeof value === \"object\" || typeof value === \"function\") {{\n    try {{\n      const text = {json}.stringify(value);\n      if (typeof text === \"string\") {{\n        return text;\n      }}\n    }} catch {{}}\n    return typeof value;\n  }}\n  return {string}(value);\n}}"
    )
}

fn script_runtime_helper(export: &str, local: &str) -> String {
    match export {
        "$tt_ap" => format!(
            "var {local}: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {{\n  return f(v);\n}};\n"
        ),
        "$tt_fl" => format!(
            "var {local}: <A extends unknown[], B, C>(\n  f: (...a: A) => B,\n  g: (b: B) => C,\n) => (...a: A) => C = function (f, g) {{\n  return (...a) => g(f(...a));\n}};\n"
        ),
        _ => crate::ice::bug!("{export} is not a runtime helper"),
    }
}

/// The file target lowering emits: its text, its TypeScript surface, and
/// the statement boundaries its automatic semicolons make, which the target
/// keeps ([`crate::lexer::automatic_semicolons`]).
#[derive(Clone, Copy)]
pub(crate) struct EmitSource<'a> {
    pub(crate) text: &'a str,
    pub(crate) kind: SourceKind,
    pub(crate) automatic_semicolons: &'a [crate::lexer::AutomaticSemicolon],
    pub(crate) comments: &'a [crate::ast::Span],
}

pub(crate) fn emit_with_map<'a>(
    semantic: &'a SemanticFile,
    core: &'a CoreFile,
    emit_source: EmitSource<'a>,
    lowering_plan: &LoweringPlan,
    rewrite_imports: Option<RewrittenExtensions>,
    std_imports: StdImports<'a>,
) -> Flat {
    let EmitSource {
        text: source,
        kind: source_kind,
        automatic_semicolons,
        comments,
    } = emit_source;
    let governed = crate::lexer::directive_governed_lines(source, comments);
    let target = TargetRewritePlan::build(semantic, core, source, lowering_plan);
    let script = target.script;
    let std_imports = match std_imports.commonjs {
        Some(commonjs) if target.commonjs && !script => *commonjs,
        _ => std_imports,
    };
    let local_runtime = script || target.commonjs;
    let direct_apply_inputs = direct_apply_inputs(semantic, core, source, source_kind);
    let member_apply_steps = member_apply_steps(semantic, core, source, source_kind);
    let reference_apply_steps = reference_apply_steps(semantic, core, source, source_kind);
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
    relocated.extend(member_apply_steps.keys().filter_map(|expr| {
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
        .chain(&target.recovered_matches)
        .map(|(_, span)| *span)
        .collect();
    let governed_statements = governed_statements(semantic, core, &target, &governed);
    let for_initializer_nodes: HashSet<NodeId> = target
        .for_initializer_propagations
        .iter()
        .map(|rewrite| rewrite.node)
        .collect();
    let rebuilt_statements = core
        .bodies
        .iter()
        .flat_map(|body| &body.statements)
        .filter_map(|statement| match statement {
            Statement::Propagate(propagate)
                if propagate.binding.is_some()
                    && !for_initializer_nodes.contains(&propagate.node) =>
            {
                Some(propagate.owner)
            }
            Statement::Decision(decision)
                if matches!(decision.kind, DecisionKind::LetElse { .. }) =>
            {
                Some(decision.extent)
            }
            _ => None,
        })
        .filter_map(|node| semantic.hir.source_map.node_span(node))
        .map(|span| span.start);
    let relocated_documentation: HashMap<usize, SourceSpan> = target
        .owner_slots
        .iter()
        .map(|rewrite| rewrite.owner.start)
        .chain(
            target
                .for_initializer_propagations
                .iter()
                .map(|rewrite| rewrite.owner.start),
        )
        .chain(target.composes.iter().map(|rewrite| rewrite.owner.start))
        .chain(rebuilt_statements)
        .filter_map(|statement| {
            crate::lexer::leading_documentation(source, comments, &governed, statement).map(
                |span| {
                    (
                        statement,
                        SourceSpan {
                            start: span.start,
                            end: span.end,
                        },
                    )
                },
            )
        })
        .collect();
    relocated.extend(relocated_documentation.values().copied());
    let emitter = Emitter {
        semantic,
        core,
        source,
        source_kind,
        direct_apply_inputs,
        member_apply_steps,
        reference_apply_steps,
        rewrite_imports,
        std_imports,
        owner_slot_index: span_index(target.owner_slots.iter().map(|rewrite| rewrite.owner)),
        owner_slots_by_expr: target.owner_slots.iter().enumerate().fold(
            HashMap::new(),
            |mut by_expr: HashMap<ExprId, Vec<usize>>, (index, rewrite)| {
                by_expr.entry(rewrite.expr).or_default().push(index);
                by_expr
            },
        ),
        owner_slot_rewrites: target.owner_slots,
        propagation_index: span_index(
            target
                .for_initializer_propagations
                .iter()
                .map(|rewrite| rewrite.owner),
        ),
        for_initializer_propagations: target.for_initializer_propagations,
        compose_index: span_index(target.composes.iter().map(|rewrite| rewrite.owner)),
        compose_rewrites: target.composes,
        declarator_splits: target.declarator_splits,
        emitted_declarator_separators: ClosedComposeBlocks::default(),
        emitted_declarator_heads: ClosedComposeBlocks::default(),
        opened_declaration_scopes: ClosedComposeBlocks::default(),
        closed_declaration_scopes: ClosedComposeBlocks::default(),
        loop_body_index: span_index(target.loop_tests.iter().map(|rewrite| rewrite.body)),
        loop_test_rewrites: target.loop_tests,
        replacement_index: span_index(
            target
                .source_replacements
                .iter()
                .map(|replacement| replacement.source),
        ),
        replacement_order: crate::span_index::NestedOrder::new(
            target
                .source_replacements
                .iter()
                .map(|replacement| (replacement.source.start, replacement.source.end)),
        ),
        claim_order: {
            let claims: Vec<usize> = target
                .source_replacements
                .iter()
                .enumerate()
                .filter(|(_, replacement)| replacement.claim)
                .map(|(index, _)| index)
                .collect();
            (
                crate::span_index::NestedOrder::new(claims.iter().map(|&index| {
                    let source = target.source_replacements[index].source;
                    (source.start, source.end)
                })),
                claims,
            )
        },
        value_order: std::cell::OnceCell::new(),
        slot_value_order: std::cell::OnceCell::new(),
        nested_input_order: std::cell::OnceCell::new(),
        piped_steps: std::cell::OnceCell::new(),
        slots_named: std::cell::OnceCell::new(),
        deferred_arm_values: std::cell::OnceCell::new(),
        statement_order: std::cell::OnceCell::new(),
        claimed_frames: target
            .source_replacements
            .iter()
            .enumerate()
            .filter(|(_, replacement)| replacement.claim)
            .fold(
                std::collections::BTreeMap::<usize, Vec<(usize, usize)>>::new(),
                |mut frames, (index, replacement)| {
                    frames
                        .entry(replacement.source.start)
                        .or_default()
                        .push((replacement.source.end, index));
                    frames
                },
            )
            .into_iter()
            .map(|(start, mut ends)| {
                ends.sort_unstable();
                (start, ends)
            })
            .collect(),
        source_replacements: target.source_replacements,
        active_capture_sources: RefCell::new(Vec::new()),
        rebuilt_sources: RefCell::new(Vec::new()),
        delivered_conditional_values: RefCell::new(HashSet::new()),
        consumed_exprs: target.consumed_exprs,
        arrow_returns_by_expr: target.arrow_returns.iter().enumerate().rev().fold(
            HashMap::new(),
            |mut by_expr, (index, rewrite)| {
                by_expr.insert(rewrite.expr, index);
                by_expr
            },
        ),
        arrow_return_rewrites: target.arrow_returns,
        slot_exprs: target.slot_exprs,
        value_slots: target.value_slots,
        piped_slots: target.piped_slots,
        scheduled_slots: target.scheduled_slots,
        result_failures: RefCell::new(HashMap::new()),
        value_exits: target.value_exits,
        nested_schedules: target.nested_schedules,
        nested_operations: target.nested_operations,
        nested_values: target.nested_values,
        structurally_nested_values: target.structurally_nested_values,
        recovered_propagations: target
            .recovered_propagations
            .into_iter()
            .map(|(expr, _)| expr)
            .collect(),
        recovered_matches: target
            .recovered_matches
            .into_iter()
            .map(|(expr, _)| expr)
            .collect(),
        owner_model: target.owner_model,
        recovered_sources: RefCell::new(Vec::new()),
        expression_boundary_name: target.expression_boundary_name,
        match_raise_name: target.match_raise_name,
        match_show_name: target.match_show_name,
        spread_name: target.spread_name,
        guarded_if_tests: target.guarded_if_tests,
        anonymous_functions: target.anonymous_functions,
        function_names: target.function_names,
        discarded_values: target.discarded_values,
        if_test_closings: RefCell::new(Vec::new()),
        host_error: target.host_error,
        host_json: target.host_json,
        host_string: target.host_string,
        inline_subjects: target.inline_subjects,
        block_required_statements: target.block_required_statements,
        documentation_starts: relocated_documentation
            .values()
            .map(|span| (span.start, *span))
            .collect(),
        relocated_documentation,
        emitted_documentation: ClosedComposeBlocks::default(),
        block_required_by_end: target.block_required_owners.iter().fold(
            std::collections::BTreeMap::new(),
            |mut by_end: std::collections::BTreeMap<usize, Vec<SourceSpan>>, owner| {
                by_end.entry(owner.end).or_default().push(*owner);
                by_end
            },
        ),
        block_required_owners: target.block_required_owners,
        opened_owner_blocks: ClosedComposeBlocks::default(),
        closed_owner_blocks: ClosedComposeBlocks::default(),
        emitting_owner_preludes: RefCell::new(Vec::new()),
        ambient_items: target.ambient_items,
        used_match_raise: Cell::new(false),
        used_match_show: Cell::new(false),
        used_spread: Cell::new(false),
        used_host_error: Cell::new(false),
        conditional_region_depth: Cell::new(0),
        active_structured_exprs: ActiveExprStack::default(),
        emitted_owner_rewrites: EmittedOwnerRewrites::default(),
        closed_compose_blocks: ClosedComposeBlocks::default(),
        emitted_compose_rewrites: ClosedComposeBlocks::default(),
        emitted_loop_tests: ClosedComposeBlocks::default(),
        loop_region_depth: Cell::new(0),
        used_expression_boundary: Cell::new(false),
        used_pipe: Cell::new(false),
        used_flow: Cell::new(false),
        imported_std: RefCell::new(Vec::new()),
        generated_names: RefCell::new(lowering_plan.generated_names().cloned().unwrap_or_else(
            || crate::generated_names::GeneratedNames::for_source(source, source_kind),
        )),
        global_temps: target.global_temps,
        comments,
    };
    let mut output = emitter.emit_file(core.root);
    let used_pipe = emitter.used_pipe.get();
    let used_flow = emitter.used_flow.get();
    let used_show = emitter.used_match_show.get();
    let aliases: Vec<_> = [
        ("Error", emitter.used_host_error.get()),
        ("JSON", used_show),
        ("String", used_show),
    ]
    .into_iter()
    .filter(|(_, used)| *used)
    .filter_map(|(global, _)| lowering_plan.host_global_alias(global))
    .collect();
    let runtime_helpers: Vec<(&str, String)> = [("$tt_ap", used_pipe), ("$tt_fl", used_flow)]
        .into_iter()
        .filter(|(_, used)| *used)
        .map(|(export, _)| (export, emitter.generated_name(export)))
        .collect();
    let show = used_show.then(|| match_show_body(&emitter.host_json, &emitter.host_string));
    let mut prelude: Vec<String> = Vec::new();
    if target.commonjs && !script {
        for (export, local) in &runtime_helpers {
            prelude.push(script_runtime_helper(export, local));
        }
    }
    if script {
        for alias in &aliases {
            prelude.push(format!("var {} = {};\n", alias.name, alias.capture));
        }
        for (export, local) in &runtime_helpers {
            prelude.push(script_runtime_helper(export, local));
        }
        if emitter.used_match_raise.get() {
            prelude.push(format!(
                "var {}: (error: unknown) => never = function (error) {{ throw error; }};\n",
                emitter.match_raise_name
            ));
        }
        if let Some(body) = &show {
            prelude.push(format!(
                "var {}: (value: unknown) => string = function (value) {body};\n",
                emitter.match_show_name
            ));
        }
        if emitter.used_spread.get() {
            prelude.push(format!(
                "var {}: {{\n  <T extends readonly unknown[]>(values: T): [...T];\n  <T>(values: Iterable<T>): T[];\n}} = function (values: Iterable<unknown>) {{\n  return [...values];\n}};\n",
                emitter.spread_name
            ));
        }
        if emitter.used_expression_boundary.get() {
            prelude.push(format!(
                "var {}: <T>(run: () => T) => T = function (run) {{ return run(); }};\n",
                emitter.expression_boundary_name
            ));
        }
    } else {
        if !local_runtime && !runtime_helpers.is_empty() {
            let names = runtime_helpers
                .iter()
                .map(|(export, local)| {
                    if local == export {
                        local.clone()
                    } else {
                        format!("{export} as {local}")
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            let runtime = std_imports
                .get(crate::StdModule::Runtime)
                .unwrap_or_else(|| crate::StdModule::Runtime.specifier());
            prelude.push(format!("import {{ {names} }} from \"{runtime}\";\n"));
        }
        for alias in &aliases {
            prelude.push(format!("const {} = {};\n", alias.name, alias.capture));
        }
        // The helpers are declarations no source text owns, so they go
        // where no source text comes before them: a bracket the user left
        // open cannot take them into its own syntax. A function
        // declaration is hoisted, so where it stands in the module does
        // not change what it means.
        if emitter.used_match_raise.get() {
            prelude.push(format!(
                "function {}(error: unknown): never {{ throw error; }}\n",
                emitter.match_raise_name
            ));
        }
        if let Some(body) = &show {
            prelude.push(format!(
                "function {}(value: unknown): string {body}\n",
                emitter.match_show_name
            ));
        }
        if emitter.used_spread.get() {
            let name = &emitter.spread_name;
            prelude.push(format!(
                "function {name}<T extends readonly unknown[]>(values: T): [...T];\nfunction {name}<T>(values: Iterable<T>): T[];\nfunction {name}(values: Iterable<unknown>) {{\n  return [...values];\n}}\n"
            ));
        }
        if emitter.used_expression_boundary.get() {
            prelude.push(format!(
                "function {}<T>(run: () => T): T {{ return run(); }}\n",
                emitter.expression_boundary_name
            ));
        }
    }
    if !prelude.is_empty() {
        // Which helpers the file needs is only known once the whole file
        // is emitted, but where an import belongs is the top — after
        // anything that has to come before one (TASK-219).
        // TypeScript reads a file's pragmas only before its first token, in
        // a module as in a script.
        let (mut at, after_code) =
            module_import_position(source, lowering_plan.directive_prologue_end());
        if !after_code {
            at = crate::lexer::pragmas::after_file_pragmas(source, at);
        }
        // A prologue that runs to the end of the file leaves nothing to
        // insert before, so the import lands at the end and needs the
        // line break the source did not write.
        let separator = if after_code || (at >= source.len() && !output.ends_with_newline()) {
            "\n"
        } else {
            ""
        };
        if let Some(first) = prelude.first_mut() {
            first.insert_str(0, separator);
        }
        output.insert_declarations_at_source(at, prelude);
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
    rewritten.extend(emitter.recovered_sources.take());
    let arm_comments: Vec<SourceSpan> = decisions(core)
        .flat_map(|decision| {
            decision
                .arms
                .iter()
                .map(|arm| arm.gap)
                .chain([decision.trailing])
                .flat_map(|gap| emitter::gap_comments(comments, gap))
                .chain(
                    decision
                        .arms
                        .iter()
                        .flat_map(|arm| emitter::head_comments(comments, &arm.head)),
                )
        })
        .map(|comment| SourceSpan {
            start: comment.start,
            end: comment.end,
        })
        .collect();
    let mut owned = pass_through_spans(semantic, core);
    owned.extend(arm_comments.iter().copied());
    relocated.extend(arm_comments);
    let preservation = SourcePreservation {
        owned,
        relocated,
        rewritten,
    };
    let boundaries: Vec<usize> = automatic_semicolons
        .iter()
        .map(|boundary| boundary.next)
        .collect();
    let mut flat = output.flatten(
        source,
        source_kind,
        &boundaries,
        &preservation,
        &governed_statements,
    );
    for result_return in &mut flat.result_return_temps {
        result_return.src_end = result_return_args
            .iter()
            .find(|argument| argument.start == result_return.src)
            .map_or(result_return.src, |argument| argument.end);
    }
    flat.generated_names = emitter.generated_names.into_inner().into_allocated();
    flat.commonjs = target.commonjs && !script;
    let imports_runtime = !local_runtime && !runtime_helpers.is_empty();
    let imported_std = emitter.imported_std.into_inner();
    flat.support_imports = crate::StdModule::ALL
        .into_iter()
        .filter(|module| {
            imported_std.contains(module)
                || (*module == crate::StdModule::Runtime && imports_runtime)
        })
        .collect();
    flat
}
