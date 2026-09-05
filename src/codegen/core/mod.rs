//! TypeScript target lowering from validated Core IR.
//!
//! This module is intentionally independent of `ast`: source text enters
//! only through HIR nodes and the source map. Every tt surface reaches this
//! module through a shared Core primitive.

mod emitter;
mod planning;

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
    let target = TargetRewritePlan::build(semantic, core, source, lowering_plan);
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
        source_kind,
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
        structurally_nested_values: target.structurally_nested_values,
        recovered_propagations: target
            .recovered_propagations
            .into_iter()
            .map(|(expr, _)| expr)
            .collect(),
        expression_boundary_name: target.expression_boundary_name,
        match_raise_name: target.match_raise_name,
        inline_subjects: target.inline_subjects,
        used_match_raise: Cell::new(false),
        conditional_region_depth: Cell::new(0),
        active_structured_exprs: ActiveExprStack::default(),
        active_scheduled_exprs: ActiveExprStack::default(),
        emitted_owner_rewrites: EmittedOwnerRewrites::default(),
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
    if emitter.used_match_raise.get() {
        if !output.ends_with_newline() {
            output.push_lit("\n");
        }
        output.push_lit(format!(
            "function {}(error: unknown): never {{ throw error; }}\n",
            emitter.match_raise_name
        ));
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
