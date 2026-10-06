//! Host rewrite planning and source-preservation classification.

mod rewrites;

pub(super) use rewrites::{
    ArrowReturnRewrite, CallCompletionPlan, ComposeAction, ComposeRewrite, ComposeValue,
    DeclaratorSplitRewrite, ForInitializerPropagationRewrite, LocalSourceEdit, LoopTestRewrite,
    OwnerSlotRewrite, ResultReturnBoundary, SourceReplacement, compound_assignment_operator,
    declarator_separator, discarded_operand_comma,
};
use rewrites::{compound_assignment_frames, discarded_operand_commas};

use super::*;

/// Where a generated module-top `import` is written, and whether it needs a
/// line break before it: past a byte-order mark, a hashbang comment
/// (ECMA-262 §12.5), and the directive prologue the parsed program reports
/// (ECMA-262 §11.2.1).
///
/// An import written above a directive would demote it to a string
/// expression, so a bundler would stop seeing the boundary the author
/// declared. After the last directive, the rest of its line — whitespace and
/// comments — stays with it, and the import opens the next line. When code
/// follows the directive on its own line, the import is written right after
/// the directive statement on a line of its own.
///
/// Line terminators and white space are the scanner's (`crate::scanner`).
pub(super) fn module_import_position(source: &str, directive_end: Option<usize>) -> (usize, bool) {
    let bytes = source.as_bytes();
    let Some(end) = directive_end else {
        return (program_start(source), false);
    };
    let len = bytes.len();
    let mut at = end;
    loop {
        at = crate::scanner::skip_space(bytes, at, len, false);
        if let Some(next_line) = crate::scanner::line_break_end(bytes, at, len) {
            return (next_line, false);
        }
        match (bytes.get(at), bytes.get(at + 1)) {
            (None, _) => return (at, false),
            (Some(b'/'), Some(b'/')) => at = crate::scanner::line_end(bytes, at, len),
            (Some(b'/'), Some(b'*')) => at = crate::scanner::block_comment_end(bytes, at, len),
            _ => return (end, true),
        }
    }
}

fn program_start(source: &str) -> usize {
    let at = if source.starts_with('\u{feff}') {
        '\u{feff}'.len_utf8()
    } else {
        0
    };
    let bytes = source.as_bytes();
    if bytes[at..].starts_with(b"#!") {
        let line = crate::scanner::line_end(bytes, at, bytes.len());
        crate::scanner::line_break_end(bytes, line, bytes.len()).unwrap_or(bytes.len())
    } else {
        at
    }
}

/// Inline `$tt_ap(v, f)` as `f(v)` exactly when moving the input behind the
/// callee is proven unobservable. ProgramSyntax owns the effect proof; these
/// ExprIds also register the corresponding source relocation.
pub(super) fn direct_apply_inputs(
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

pub(super) fn reference_apply_steps(
    semantic: &SemanticFile,
    core: &CoreFile,
    source: &str,
    source_kind: SourceKind,
) -> HashSet<ExprId> {
    core.exprs
        .iter()
        .filter_map(|expr| match expr {
            Expr::Apply(apply) => Some(apply),
            _ => None,
        })
        .flat_map(|apply| apply.steps.iter().skip(usize::from(apply.head.is_none())))
        .filter(|step| matches!(step.mode, ApplyMode::Call))
        .filter_map(|step| {
            let Expr::Opaque(node) = &core.exprs[step.value.index()] else {
                return None;
            };
            let span = semantic.hir.source_map.node_span(*node)?;
            crate::program_syntax::source_reference_callee(source, span, source_kind)
                .then_some(step.value)
        })
        .collect()
}

pub(super) fn member_apply_steps(
    semantic: &SemanticFile,
    core: &CoreFile,
    source: &str,
    source_kind: SourceKind,
) -> HashMap<ExprId, crate::program_syntax::MemberCallee> {
    core.exprs
        .iter()
        .filter_map(|expr| match expr {
            Expr::Apply(apply) => Some(apply),
            _ => None,
        })
        .flat_map(|apply| &apply.steps)
        .filter(|step| matches!(step.mode, ApplyMode::Call))
        .filter_map(|step| {
            let Expr::Opaque(node) = &core.exprs[step.value.index()] else {
                return None;
            };
            let span = semantic.hir.source_map.node_span(*node)?;
            crate::program_syntax::source_member_callee(source, span, source_kind)
                .map(|member| (step.value, member))
        })
        .collect()
}

/// The pass-through ranges of a file, for the target's preservation check
/// ([`SourcePreservation::owned`]): the source the compiler does not
/// interpret — Core `Opaque` statements and expressions and template raw
/// parts, wherever they sit. Read off the Core IR — the same structure the
/// emitter walks — never off the output.
pub(super) fn pass_through_spans(semantic: &SemanticFile, core: &CoreFile) -> Vec<SourceSpan> {
    pub(super) fn span(semantic: &SemanticFile, node: NodeId, out: &mut Vec<SourceSpan>) {
        let span = semantic
            .hir
            .source_map
            .node_span(node)
            .unwrap_or_else(|| crate::ice::bug!("target node has no source span"));
        out.push(span.into());
    }

    pub(super) fn walk_body(
        semantic: &SemanticFile,
        core: &CoreFile,
        body: hir::BodyId,
        out: &mut Vec<SourceSpan>,
    ) {
        crate::stack::grow(|| walk_body_grown(semantic, core, body, out));
    }

    fn walk_body_grown(
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

    pub(super) fn walk_decision(
        semantic: &SemanticFile,
        core: &CoreFile,
        decision: &Decision,
        out: &mut Vec<SourceSpan>,
    ) {
        crate::stack::grow(|| walk_decision_grown(semantic, core, decision, out));
    }

    fn walk_decision_grown(
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

    pub(super) fn walk_expr(
        semantic: &SemanticFile,
        core: &CoreFile,
        expr: ExprId,
        out: &mut Vec<SourceSpan>,
    ) {
        crate::stack::grow(|| walk_expr_grown(semantic, core, expr, out));
    }

    fn walk_expr_grown(
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

pub(super) fn structured_expr_span(
    semantic: &SemanticFile,
    core: &CoreFile,
    expr: ExprId,
) -> Option<SourceSpan> {
    crate::stack::grow(|| structured_expr_span_grown(semantic, core, expr))
}

fn structured_expr_span_grown(
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

pub(super) fn structured_grouping_frames(
    semantic: &SemanticFile,
    core: &CoreFile,
    source: &str,
    expr: ExprId,
) -> Vec<SourceSpan> {
    pub(super) fn walk(
        semantic: &SemanticFile,
        core: &CoreFile,
        source: &str,
        expr: ExprId,
        out: &mut Vec<SourceSpan>,
    ) {
        crate::stack::grow(|| walk_grown(semantic, core, source, expr, out));
    }

    fn walk_grown(
        semantic: &SemanticFile,
        core: &CoreFile,
        source: &str,
        expr: ExprId,
        out: &mut Vec<SourceSpan>,
    ) {
        match &core.exprs[expr.index()] {
            Expr::Sequence(body) => {
                for statement in &core.bodies[body.index()].statements {
                    match statement {
                        Statement::Opaque(node) => {
                            let Some(span) = semantic.hir.source_map.node_span(*node) else {
                                continue;
                            };
                            let text = &source[span.start..span.end];
                            if text
                                .bytes()
                                .filter(|byte| !byte.is_ascii_whitespace())
                                .all(|byte| matches!(byte, b'(' | b')'))
                            {
                                out.push(SourceSpan::from(span));
                            }
                        }
                        Statement::Expr(inner) => walk(semantic, core, source, *inner, out),
                        Statement::Adt(_)
                        | Statement::Import(_)
                        | Statement::Propagate(_)
                        | Statement::Decision(_) => {}
                    }
                }
            }
            Expr::Apply(apply) => {
                if let Some(head) = apply.head {
                    walk(semantic, core, source, head, out);
                }
                for step in &apply.steps {
                    walk(semantic, core, source, step.value, out);
                }
            }
            Expr::Decision(_)
            | Expr::Propagate(_)
            | Expr::ResultRegion(_)
            | Expr::Template(_)
            | Expr::Opaque(_) => {}
        }
    }

    let mut out = Vec::new();
    walk(semantic, core, source, expr, &mut out);
    out
}

pub(super) struct TargetRewritePlan {
    pub(super) owner_slots: Vec<OwnerSlotRewrite>,
    pub(super) for_initializer_propagations: Vec<ForInitializerPropagationRewrite>,
    pub(super) composes: Vec<ComposeRewrite>,
    pub(super) declarator_splits: Vec<DeclaratorSplitRewrite>,
    pub(super) loop_tests: Vec<LoopTestRewrite>,
    pub(super) source_replacements: Vec<SourceReplacement>,
    /// The source spans of values whose lowering moves them into a prelude
    /// before their owner — a planned relocation the preservation check
    /// must know about ([`SourcePreservation::relocated`]).
    pub(super) relocated_values: Vec<SourceSpan>,
    /// The parent spans of lowered conditional operations: their operator
    /// tokens are claimed source ([`SourcePreservation::rewritten`]).
    pub(super) rewritten_operations: Vec<SourceSpan>,
    /// Expression propagations rejected by the host-capability check. The
    /// recovering projection emits `undefined` for them and claims their
    /// source so editor/type diagnostics can continue.
    pub(super) recovered_propagations: Vec<(ExprId, SourceSpan)>,
    pub(super) recovered_matches: Vec<(ExprId, SourceSpan)>,
    pub(super) owner_model: bool,
    /// tt values a conditional operation consumes; their inline Core
    /// position emits nothing (the operation's replacement covers it).
    pub(super) consumed_exprs: HashSet<ExprId>,
    pub(super) arrow_returns: Vec<ArrowReturnRewrite>,
    pub(super) slot_exprs: HashMap<ExprId, String>,
    pub(super) value_slots: HashMap<ExprId, String>,
    pub(super) piped_slots: HashMap<ExprId, Vec<String>>,
    pub(super) scheduled_slots: HashMap<crate::evaluation_ir::ValueSlotId, String>,
    pub(super) value_exits: HashMap<ExprId, Vec<HostExit>>,
    pub(super) nested_schedules: HashMap<ExprId, EvaluationSchedule>,
    pub(super) nested_operations: Vec<PlannedConditionalOperation>,
    pub(super) nested_values: HashSet<ExprId>,
    /// Every value whose Evaluation IR placement is structurally nested,
    /// before target-specific slot-substitution filtering.
    pub(super) structurally_nested_values: HashSet<ExprId>,
    pub(super) expression_boundary_name: String,
    pub(super) match_raise_name: String,
    pub(super) match_show_name: String,
    pub(super) spread_name: String,
    pub(super) guarded_if_tests: HashMap<SourceSpan, crate::program_syntax::IfTestFacts>,
    pub(super) host_error: String,
    pub(super) host_json: String,
    pub(super) host_string: String,
    pub(super) inline_subjects: HashMap<NodeId, Vec<String>>,
    pub(super) block_required_statements: HashSet<NodeId>,
    pub(super) block_required_owners: HashSet<SourceSpan>,
    pub(super) ambient_items: HashSet<NodeId>,
    pub(super) script: bool,
    pub(super) commonjs: bool,
    pub(super) global_temps: HashMap<crate::core_ir::TempId, String>,
}

/// Consume the host AST's single-return-body proof. Only opaque returned
/// values participate here; structured TT values retain their own schedules.
pub(super) fn single_return_arm_value(
    semantic: &SemanticFile,
    core: &CoreFile,
    body: hir::BodyId,
    exits: &[HostExit],
) -> Option<(SourceSpan, HostExit)> {
    let [Statement::Opaque(node)] = core.bodies[body.index()].statements.as_slice() else {
        return None;
    };
    let span = SourceSpan::from(semantic.hir.source_map.node_span(*node)?);
    exits
        .iter()
        .find(|exit| exit.single_return_body == Some(body))
        .map(|exit| (span, *exit))
}

/// Whether a match's arms may perform the consuming call themselves: every
/// arm is an opaque expression, or a never-completing block whose every
/// rewritten `return` can carry the call without landing inside a handler
/// or running before a finalizer or disposal ([`HostExit::call_safe`]).
pub(super) fn completable_decision_arms(core: &CoreFile, expr: ExprId, exits: &[HostExit]) -> bool {
    let Expr::Decision(decision) = &core.exprs[expr.index()] else {
        return false;
    };
    matches!(decision.kind, DecisionKind::Match { .. })
        && decision.arms.iter().all(|arm| match arm.action {
            ArmAction::Yield {
                body,
                kind: ArmBodyKind::Expression,
            } => core.bodies[body.index()]
                .statements
                .iter()
                .all(|stmt| matches!(stmt, Statement::Opaque(_))),
            ArmAction::Yield {
                body,
                kind: ArmBodyKind::Block { completes: false },
            } => {
                core.bodies[body.index()]
                    .statements
                    .iter()
                    .all(|stmt| matches!(stmt, Statement::Opaque(_)))
                    && exits
                        .iter()
                        .filter(|exit| exit.body == Some(body))
                        .all(|exit| exit.call_safe)
            }
            _ => false,
        })
}

/// Whether the authored text between the completed call's argument and the
/// value may be re-emitted inside every arm.
///
/// The steps below the call are the literal frames the value is nested in.
/// Only object and array literals qualify: their positions are exactly the
/// sub-expressions they evaluate, so "every earlier position is inert" is
/// the whole question — the rest of the frame is keys and punctuation. An
/// earlier position that is *not* inert would move from before the
/// scrutinee to after it, which the arms cannot undo.
fn framed_positions_are_inert(steps: &[PlannedEvaluationStep]) -> bool {
    steps.iter().all(|step| {
        matches!(
            step.operation,
            HostEvaluationOperation::Eager(
                crate::program_syntax::EagerPosition::ObjectEvaluation(_)
                    | crate::program_syntax::EagerPosition::ArrayElement(_)
            )
        ) && step
            .inputs
            .iter()
            .all(|input| matches!(input, PlannedEvaluationInput::Stable { .. }))
    })
}

/// Whether every arm delivers its value through the expression path.
///
/// A block arm rewrites its `return` through the string-building exit
/// prefix, which cannot carry authored bytes with their source mapping. A
/// framed completion has authored bytes to place, so it stays with the arms
/// that deliver through [`emit_value_delivery_control`], where a frame is
/// pushed as source.
fn all_arms_are_expressions(core: &CoreFile, expr: ExprId) -> bool {
    let Expr::Decision(decision) = &core.exprs[expr.index()] else {
        return false;
    };
    decision.arms.iter().all(|arm| {
        matches!(
            arm.action,
            ArmAction::Yield {
                kind: ArmBodyKind::Expression,
                ..
            }
        )
    })
}

fn scoped_call_completion(
    core: &CoreFile,
    value: &crate::evaluation_ir::PlannedValue,
    value_slot: &str,
    lowering: &LoweringPlan,
    source: &str,
) -> Option<CallCompletionPlan> {
    let (expr, exits, schedule, value_source) =
        (value.expr, &value.exits[..], &value.schedule, value.source);
    let completion = schedule.call_completion?;
    if !completable_decision_arms(core, expr, exits) {
        return None;
    }
    // One of the value's evaluation steps must be the proven call itself:
    // that is what ties the syntactic facts to this value. The step's inputs
    // are the callee plus every earlier argument, each already scheduled to
    // evaluate before the dispatch, so the arm's call re-reads them from
    // their capture slots (a sibling tt value answers with its join slot; a
    // proven-inert input re-evaluates unobservably in place).
    //
    // Steps before it are the literal frames between the argument and the
    // value; the arms re-emit their authored text around each arm value.
    let call_step = schedule.steps().iter().position(|step| {
        step.parent == completion.facts.call
            && matches!(
                step.operation,
                HostEvaluationOperation::Eager(crate::program_syntax::EagerPosition::CallArgument(
                    _
                ))
            )
    })?;
    let frame = if completion.facts.argument == value_source {
        None
    } else {
        if !completion.facts.literal_positions
            || call_step == 0
            || !framed_positions_are_inert(&schedule.steps()[..call_step])
            || !all_arms_are_expressions(core, expr)
        {
            return None;
        }
        Some((
            SourceSpan {
                start: completion.facts.argument.start,
                end: value_source.start,
            },
            SourceSpan {
                start: value_source.end,
                end: completion.facts.argument.end,
            },
        ))
    };
    let step = &schedule.steps()[call_step];
    let HostEvaluationOperation::Eager(crate::program_syntax::EagerPosition::CallArgument(index)) =
        step.operation
    else {
        return None;
    };
    if step.inputs.len() != usize::try_from(index).ok()?.checked_add(1)? {
        return None;
    }
    let PlannedEvaluationInput::Source {
        source: callee_span,
        target,
        mode,
        receiver,
        key,
    } = step.inputs.first()?
    else {
        return None;
    };
    let callee = lowering.slot_name(*target).to_owned();
    let (mut invoke, instantiation) = if *mode == EvaluationInputMode::MemberReference {
        receiver.as_ref()?;
        let mut invoke = member_callee(source, *callee_span, [*receiver, *key], |slot| {
            lowering.slot_name(slot)
        });
        if let Some(type_args) = completion.facts.type_args {
            invoke.push_str(&source[type_args.start..type_args.end]);
        }
        (invoke, None)
    } else {
        let instantiation = match (completion.facts.type_args, completion.instantiated) {
            (Some(type_args), Some(slot)) => Some((
                lowering.slot_name(slot).to_owned(),
                type_args,
                callee.clone(),
            )),
            (None, None) => None,
            _ => return None,
        };
        let function = instantiation
            .as_ref()
            .map_or(callee.clone(), |(name, ..)| name.clone());
        (function, instantiation)
    };
    invoke.push('(');
    let mut captures = Vec::new();
    for input in &step.inputs[1..] {
        match input {
            PlannedEvaluationInput::Source { target, .. } => {
                invoke.push_str(lowering.slot_name(*target));
            }
            PlannedEvaluationInput::Slot { slot, .. } => {
                invoke.push_str(lowering.slot_name(*slot));
            }
            // The arm reads generated names only, and this input's authored
            // position is inside the frame the completion claims. Bind it to
            // the name the schedule reserved; without one there is no way to
            // name it, and the completion does not apply.
            PlannedEvaluationInput::Stable { source, reserved } => {
                let name = lowering.slot_name((*reserved)?).to_owned();
                invoke.push_str(&name);
                captures.push((name, *source));
            }
        }
        invoke.push_str(", ");
    }
    Some(CallCompletionPlan {
        invoke,
        instantiation,
        captures,
        result: completion.facts.consumed.then(|| value_slot.to_owned()),
        label: callee,
        call: completion.facts.call,
        frame,
    })
}

fn can_defer_arm_values(
    semantic: &SemanticFile,
    core: &CoreFile,
    expr: ExprId,
    exits: &[HostExit],
) -> bool {
    let Expr::Decision(decision) = &core.exprs[expr.index()] else {
        return false;
    };
    fn has_bindings(pattern: &PatternPlan) -> bool {
        crate::stack::grow(|| has_bindings_grown(pattern))
    }

    fn has_bindings_grown(pattern: &PatternPlan) -> bool {
        match pattern {
            PatternPlan::Bind(_) => true,
            PatternPlan::AllOf(parts) | PatternPlan::AnyOf(parts) => parts.iter().any(has_bindings),
            PatternPlan::Any | PatternPlan::Test(_) => false,
        }
    }
    let supported_dispatch = match decision.kind {
        DecisionKind::Match {
            dispatch: MatchDispatch::Conditional,
            ..
        } => {
            // A total condition chain is an ordinary TS conditional expression.
            // Keep guards beside values so their narrowing remains in scope.
            decision
                .arms
                .last()
                .is_some_and(DecisionArm::always_matches)
        }
        DecisionKind::Match { .. } => true,
        _ => false,
    };
    supported_dispatch
        && !decision.arms.is_empty()
        && decision
            .subjects
            .iter()
            .all(|subject| !core.has_statement_form(subject.value))
        && decision.arms.iter().all(|arm| {
            !has_bindings(&arm.pattern)
                && arm
                    .guard
                    .is_none_or(|guard| !core.has_statement_form(guard))
                && match arm.action {
                    ArmAction::Yield {
                        body,
                        kind: ArmBodyKind::Expression,
                    } => {
                        core.bodies[body.index()]
                            .statements
                            .iter()
                            .all(|statement| match statement {
                                Statement::Opaque(_) => true,
                                Statement::Expr(expr) => !core.has_statement_form(*expr),
                                _ => false,
                            })
                    }
                    ArmAction::Yield {
                        body,
                        kind: ArmBodyKind::Block { completes: false },
                    } => single_return_arm_value(semantic, core, body, exits).is_some(),
                    _ => false,
                }
        })
}

/// Whether a step is the argument of an optional call, whose callee the
/// optional-call lowering binds or calls through its receiver itself.
pub(super) fn optional_call_step(step: &PlannedEvaluationStep) -> bool {
    matches!(
        step.operation,
        HostEvaluationOperation::Conditional(ConditionalBranch::OptionalCallArgument(_))
    )
}

/// Whether a call reads its member callee before the arguments: an
/// optional call tested at its callee (`o.m?.(x)`), whose test is on the
/// member's value, so that value is captured and called through its
/// receiver. Every other member callee is read where the call is made
/// ([`member_callee`]).
pub(super) fn callee_tested_step(step: &PlannedEvaluationStep) -> bool {
    optional_call_step(step)
        && step
            .conditional
            .as_ref()
            .and_then(|facts| facts.optional_test)
            == Some(OptionalCallTest::Callee)
}

/// How a call reads its member callee: the callee as written, with each
/// captured part of its reference (the receiver, then a computed key)
/// replaced by its slot. The member is read at the call, so the call is
/// the member call TypeScript types — `this`, a generic method's
/// inference, and a `this` parameter survive, as `bind` (which erases a
/// generic signature with a `this` parameter) and `call` (which
/// instantiates type parameters with `unknown`) do not. The parts are
/// evaluated before the arguments, as ECMA-262 `EvaluateCall` evaluates the
/// reference; the member's `GetValue` moves after the arguments, next to
/// the `IsCallable` check that already follows them.
pub(super) fn member_callee<'n>(
    source: &str,
    callee: SourceSpan,
    parts: [Option<PlannedReceiver>; 2],
    slot_name: impl Fn(crate::evaluation_ir::ValueSlotId) -> &'n str,
) -> String {
    let mut text = String::new();
    let mut cursor = callee.start;
    for part in parts.into_iter().flatten() {
        let PlannedReceiver::Captured { source: at, slot } = part else {
            continue;
        };
        text.push_str(&source[cursor..at.start]);
        text.push_str(slot_name(slot));
        cursor = at.end;
    }
    text.push_str(&source[cursor..callee.end]);
    text
}

impl TargetRewritePlan {
    pub(super) fn build(
        semantic: &SemanticFile,
        core: &CoreFile,
        source: &str,
        lowering: &LoweringPlan,
    ) -> Self {
        let source_code = source;
        let for_initializer_propagations: Vec<_> = lowering
            .for_initializer_propagations()
            .map(|propagation| ForInitializerPropagationRewrite {
                node: propagation.node,
                owner: propagation.owner.anchor(),
                source: propagation.source,
            })
            .collect();
        let recovered_propagations: Vec<_> = lowering
            .unsupported_expression_propagations()
            .into_iter()
            .map(|failure| (failure.expr, failure.source))
            .collect();
        let recovered_matches: Vec<_> = lowering
            .unsupported_matches()
            .into_iter()
            .map(|failure| (failure.expr, failure.source))
            .collect();
        let recovered: HashSet<_> = recovered_propagations
            .iter()
            .chain(&recovered_matches)
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
                        owner: rewrite.owner.anchor(),
                        source: structured_expr_span(semantic, core, value.expr)
                            .unwrap_or(value.source),
                        expr: value.expr,
                        slot: lowering.slot_name(slot).to_owned(),
                        continuation: value.context.continuation,
                        contextual_type: value.context.contextual_type,
                        contextual_type_awaited: value.context.contextual_type_awaited,
                        contextual_type_asserted: value.context.contextual_type_asserted,
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
                        source: value.source,
                        expr: value.expr,
                        slot: lowering.slot_name(slot).to_owned(),
                        contextual_type: value.context.contextual_type,
                        contextual_type_awaited: value.context.contextual_type_awaited,
                        contextual_type_asserted: value.context.contextual_type_asserted,
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
                                    call_completion: None,
                                    inline: false,
                                    expr: value.expr,
                                    source: value.source,
                                    slot: lowering.slot_name(slot).to_owned(),
                                    steps,
                                    defer_arm_values: false,
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
                        (value.context.continuation == HostContinuation::Compose
                            || (value.context.continuation == HostContinuation::ForInitialize
                                && !value.schedule.steps().is_empty()))
                            && !value
                                .schedule
                                .steps()
                                .iter()
                                .any(|step| step.operation == HostEvaluationOperation::LoopTest)
                            && !recovered.contains(&value.expr)
                            && value.capability == TargetCapability::StatementRegion
                    })
                    .collect();
                (!values.is_empty()).then(|| {
                    let inline = rewrite.values.len() > 1
                        && values.len() == rewrite.values.len()
                        && values.iter().all(|value| {
                            can_defer_arm_values(semantic, core, value.expr, &value.exits)
                        });
                    // A value consumed by a conditional operation is emitted
                    // by that operation's region, at the position of the
                    // operation's first value.
                    let operation_of: HashMap<ExprId, usize> = rewrite
                        .operations
                        .iter()
                        .filter(|_| !inline)
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
                                let slot_name = lowering.slot_name(slot).to_owned();
                                // A single-value owner prefers the deferred
                                // in-place arm evaluation; in a multi-value
                                // owner that plan does not apply, so the
                                // final-argument completion takes over.
                                let call_completion = if !inline
                                    && !(rewrite.values.len() == 1
                                        && can_defer_arm_values(
                                            semantic,
                                            core,
                                            value.expr,
                                            &value.exits,
                                        )) {
                                    scoped_call_completion(
                                        core, value, &slot_name, lowering, source,
                                    )
                                } else {
                                    None
                                };
                                Some(ComposeAction::Value(ComposeValue {
                                    call_completion,
                                    inline,
                                    expr: value.expr,
                                    source: value.source,
                                    slot: slot_name,
                                    steps: if inline {
                                        Vec::new()
                                    } else {
                                        value.schedule.steps().to_vec()
                                    },
                                    defer_arm_values: rewrite.values.len() == 1
                                        && can_defer_arm_values(
                                            semantic,
                                            core,
                                            value.expr,
                                            &value.exits,
                                        ),
                                }))
                            }
                        })
                        .collect();
                    ComposeRewrite {
                        owner: rewrite.owner.anchor(),
                        owner_kind: rewrite.owner.kind,
                        actions,
                    }
                })
            })
            .collect();
        let mut declarator_splits: Vec<_> = lowering
            .owners()
            .filter_map(|rewrite| {
                let split = rewrite.owner.split?;
                let anchor = rewrite.owner.anchor();
                let hoisted = owner_slots.iter().any(|slot| slot.owner == anchor)
                    || composes.iter().any(|compose| compose.owner == anchor);
                let statement = rewrite.owner.statement();
                hoisted.then(|| DeclaratorSplitRewrite {
                    separator: declarator_separator(source, split.previous_end),
                    at: anchor.start,
                    head: split.head(),
                    block: lowering
                        .block_required_owners()
                        .contains(&statement)
                        .then_some(statement),
                    statement,
                    last: false,
                })
            })
            .collect();
        let last_splits: HashMap<SourceSpan, usize> = declarator_splits.iter().fold(
            HashMap::new(),
            |mut last, split: &DeclaratorSplitRewrite| {
                let at = last.entry(split.statement).or_insert(split.at);
                *at = (*at).max(split.at);
                last
            },
        );
        for split in &mut declarator_splits {
            split.last = last_splits.get(&split.statement) == Some(&split.at);
        }
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
        let guarded_loop_tests: HashSet<SourceSpan> = loop_tests
            .iter()
            .flat_map(|rewrite| {
                rewrite
                    .actions
                    .iter()
                    .filter_map(move |action| match action {
                        ComposeAction::Operation(operation)
                            if operation.parent == rewrite.test
                                && matches!(
                                    operation.kind,
                                    PlannedConditionalKind::LogicalAnd
                                        | PlannedConditionalKind::LogicalOr
                                ) =>
                        {
                            Some(operation.parent)
                        }
                        _ => None,
                    })
            })
            .collect();
        let guarded_if_tests: HashMap<SourceSpan, crate::program_syntax::IfTestFacts> = composes
            .iter()
            .filter_map(|rewrite| match rewrite.actions.last() {
                Some(ComposeAction::Operation(operation)) => Some((rewrite, operation)),
                _ => None,
            })
            .filter_map(|(rewrite, operation)| {
                let facts = lowering
                    .if_tests()
                    .iter()
                    .find(|facts| facts.test == operation.parent)?;
                let end = facts.alternate.unwrap_or(facts.consequent).end;
                let guarded = operation.outer.is_empty()
                    && rewrite.owner.start < facts.test.start
                    && rewrite.owner.end == end
                    && match operation.kind {
                        PlannedConditionalKind::LogicalAnd => true,
                        PlannedConditionalKind::LogicalOr => facts.alternate.is_none(),
                        _ => false,
                    };
                guarded.then_some((operation.parent, *facts))
            })
            .collect();
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
        relocated_values.extend(lowering.nested_operations().iter().flat_map(|operation| {
            let condition = match operation.condition {
                PlannedEvaluationInput::Source { source, .. }
                | PlannedEvaluationInput::Stable { source, .. } => Some(source),
                PlannedEvaluationInput::Slot { .. } => None,
            };
            let branches = match &operation.kind {
                PlannedConditionalKind::Ternary {
                    consequent,
                    alternate,
                } => [consequent, alternate]
                    .into_iter()
                    .filter_map(|branch| match branch {
                        PlannedBranch::Source(span) => Some(*span),
                        PlannedBranch::Values(_) => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            std::iter::once(operation.parent)
                .chain(condition)
                .chain(branches)
                .chain(
                    operation
                        .active
                        .iter()
                        .flat_map(|active| &active.steps)
                        .chain(&operation.outer)
                        .flat_map(|step| {
                            std::iter::once(step.parent).chain(step.inputs.iter().filter_map(
                                |input| match input {
                                    PlannedEvaluationInput::Source { source, .. }
                                    | PlannedEvaluationInput::Stable { source, .. } => {
                                        Some(*source)
                                    }
                                    PlannedEvaluationInput::Slot { .. } => None,
                                },
                            ))
                        }),
                )
                .collect::<Vec<_>>()
        }));
        relocated_values.extend(loop_tests.iter().filter_map(|rewrite| rewrite.update));
        relocated_values.extend(lowering.nested_value_schedules().flat_map(|(_, schedule)| {
            schedule.steps().iter().flat_map(|step| {
                std::iter::once(step.parent).chain(step.inputs.iter().filter_map(
                    |input| match input {
                        PlannedEvaluationInput::Source { source, .. }
                        | PlannedEvaluationInput::Stable { source, .. } => Some(*source),
                        PlannedEvaluationInput::Slot { .. } => None,
                    },
                ))
            })
        }));
        // The operator frame of a lowered conditional operation (its tokens
        // between the fragments the region re-emits) is claimed source.
        let arrow_return_frames = arrow_returns.iter().flat_map(|rewrite| {
            let structured =
                structured_expr_span(semantic, core, rewrite.expr).unwrap_or(rewrite.source);
            [
                (rewrite.source.start < structured.start).then_some(SourceSpan {
                    start: rewrite.source.start,
                    end: structured.start,
                }),
                (structured.end < rewrite.source.end).then_some(SourceSpan {
                    start: structured.end,
                    end: rewrite.source.end,
                }),
            ]
            .into_iter()
            .flatten()
        });
        let compose_arrow_frames = composes
            .iter()
            .filter(|rewrite| rewrite.owner_kind == HostOwnerKind::ArrowExpression)
            .flat_map(|rewrite| {
                let mut spans = rewrite.actions.iter().filter_map(|action| {
                    let expr = match action {
                        ComposeAction::Value(value) => value.expr,
                        ComposeAction::Operation(operation) => *operation.values.first()?,
                    };
                    structured_expr_span(semantic, core, expr)
                });
                let Some(first) = spans.next() else {
                    return [None, None];
                };
                let (start, end) = spans.fold((first.start, first.end), |(start, end), span| {
                    (start.min(span.start), end.max(span.end))
                });
                [
                    (rewrite.owner.start < start).then_some(SourceSpan {
                        start: rewrite.owner.start,
                        end: start,
                    }),
                    (end < rewrite.owner.end).then_some(SourceSpan {
                        start: end,
                        end: rewrite.owner.end,
                    }),
                ]
            })
            .flatten();
        let structured_grouping_frames = arrow_returns
            .iter()
            .map(|rewrite| rewrite.expr)
            .chain(
                composes
                    .iter()
                    .filter(|rewrite| rewrite.actions.len() == 1)
                    .filter_map(|rewrite| {
                        let ComposeAction::Value(value) = &rewrite.actions[0] else {
                            return None;
                        };
                        (rewrite.owner.start == value.source.start).then_some(value.expr)
                    }),
            )
            .flat_map(|expr| structured_grouping_frames(semantic, core, source, expr));
        // A discarded call claims its whole statement — nothing of it
        // remains. A consumed call claims only the call expression's frame;
        // the value's join slot stands at the authored occurrence and the
        // rest of the statement keeps consuming it.
        let call_frames = || {
            composes.iter().flat_map(|rewrite| {
                rewrite
                    .actions
                    .iter()
                    .filter_map(|action| match action {
                        ComposeAction::Value(value) => value
                            .call_completion
                            .as_ref()
                            .map(|completion| (value, completion)),
                        _ => None,
                    })
                    .flat_map(move |(value, completion)| {
                        let (start, end) = if completion.result.is_some() {
                            (completion.call.start, completion.call.end)
                        } else {
                            (rewrite.owner.start, rewrite.owner.end)
                        };
                        [
                            (
                                SourceSpan {
                                    start,
                                    end: value.source.start,
                                },
                                value.expr,
                            ),
                            (
                                SourceSpan {
                                    start: value.source.end,
                                    end,
                                },
                                value.expr,
                            ),
                        ]
                    })
            })
        };
        let planned_steps = || {
            all_values()
                .flat_map(|value| &value.steps)
                .chain(all_operations().flat_map(|operation| {
                    operation
                        .active
                        .iter()
                        .flat_map(|active| &active.steps)
                        .chain(&operation.outer)
                }))
                .chain(lowering.nested_operations().iter().flat_map(|operation| {
                    operation
                        .active
                        .iter()
                        .flat_map(|active| &active.steps)
                        .chain(&operation.outer)
                }))
                .chain(
                    lowering
                        .nested_value_schedules()
                        .flat_map(|(_, schedule)| schedule.steps()),
                )
        };
        let compound_assignments = compound_assignment_frames(source, planned_steps());
        let discarded_commas = discarded_operand_commas(source, planned_steps());
        relocated_values.extend(compound_assignments.iter().copied());
        relocated_values.extend(discarded_commas.iter().copied());
        let rewritten_operations: Vec<SourceSpan> = all_operations()
            .map(|operation| operation.parent)
            .chain(compound_assignments)
            .chain(discarded_commas)
            .chain(
                lowering
                    .nested_operations()
                    .iter()
                    .map(|operation| operation.parent),
            )
            .chain(call_frames().map(|(span, _)| span))
            .chain(declarator_splits.iter().map(|split| split.separator))
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
            // Concise-arrow rewrites emit host grouping as block
            // delimiters. Claim only frames outside their Core values;
            // source between and inside values remains exactly preserved.
            .chain(arrow_return_frames)
            .chain(compose_arrow_frames)
            .chain(structured_grouping_frames)
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
                    claim: false,
                    rewrite: if matches!(
                        operation.kind,
                        PlannedConditionalKind::LogicalAssignment { .. }
                    ) {
                        Some(String::new())
                    } else if operation.kind == PlannedConditionalKind::LogicalOr
                        && guarded_if_tests.contains_key(&operation.parent)
                    {
                        Some("true".to_owned())
                    } else if guarded_loop_tests.contains(&operation.parent) {
                        Some(String::new())
                    } else {
                        None
                    },
                }
            })
            .collect();
        let mut source_replacements: Vec<_> = all_values()
            .flat_map(|value| &value.steps)
            .chain(all_operations().flat_map(|operation| {
                operation
                    .active
                    .iter()
                    .flat_map(|active| &active.steps)
                    .chain(&operation.outer)
            }))
            .flat_map(|step| step.inputs.iter().map(move |input| (step, input)))
            .flat_map(|(step, input)| match input {
                PlannedEvaluationInput::Source {
                    source: target_span,
                    target,
                    mode: EvaluationInputMode::CompoundAssignmentTarget { operator },
                    ..
                } => {
                    let slot = lowering.slot_name(*target);
                    vec![SourceReplacement {
                        source: compound_assignment_operator(source, *target_span, operator),
                        slot: slot.to_owned(),
                        jsx_child: false,
                        anchor: None,
                        claim: false,
                        rewrite: Some(format!("= {slot} {operator}")),
                    }]
                }
                PlannedEvaluationInput::Source {
                    source: operand,
                    target,
                    mode: EvaluationInputMode::Discarded,
                    ..
                } => [*operand, discarded_operand_comma(source, *operand)]
                    .into_iter()
                    .map(|span| SourceReplacement {
                        source: span,
                        slot: lowering.slot_name(*target).to_owned(),
                        jsx_child: false,
                        anchor: None,
                        claim: false,
                        rewrite: Some(String::new()),
                    })
                    .collect(),
                PlannedEvaluationInput::Source {
                    mode: EvaluationInputMode::MemberReference,
                    receiver,
                    key,
                    ..
                } if !callee_tested_step(step) => [*receiver, *key]
                    .into_iter()
                    .flatten()
                    .filter_map(|part| match part {
                        PlannedReceiver::Captured { source, slot } => Some(SourceReplacement {
                            source,
                            slot: lowering.slot_name(slot).to_owned(),
                            jsx_child: false,
                            anchor: None,
                            claim: false,
                            rewrite: None,
                        }),
                        PlannedReceiver::Stable { .. } => None,
                    })
                    .collect(),
                PlannedEvaluationInput::Source {
                    source,
                    target,
                    mode,
                    ..
                } => vec![SourceReplacement {
                    source: *source,
                    slot: lowering.slot_name(*target).to_owned(),
                    jsx_child: *mode == EvaluationInputMode::JsxChildValue,
                    anchor: None,
                    claim: false,
                    rewrite: (*mode == EvaluationInputMode::ShorthandProperty).then(|| {
                        format!(
                            "{}: {}",
                            &source_code[source.start..source.end],
                            lowering.slot_name(*target)
                        )
                    }),
                }],
                PlannedEvaluationInput::Slot { .. } | PlannedEvaluationInput::Stable { .. } => {
                    Vec::new()
                }
            })
            .chain(owner_slots.iter().map(|rewrite| SourceReplacement {
                source: rewrite.source,
                slot: rewrite.slot.clone(),
                jsx_child: false,
                anchor: Some(rewrite.expr),
                claim: false,
                rewrite: None,
            }))
            .chain(operation_replacements)
            .chain(all_operations().flat_map(|operation| {
                match &operation.condition {
                    PlannedEvaluationInput::Source {
                        mode: EvaluationInputMode::LogicalAssignmentTarget,
                        receiver,
                        key,
                        ..
                    } => [*receiver, *key]
                        .into_iter()
                        .flatten()
                        .filter_map(|part| match part {
                            PlannedReceiver::Captured { source, slot } => Some(SourceReplacement {
                                source,
                                slot: lowering.slot_name(slot).to_owned(),
                                jsx_child: false,
                                anchor: None,
                                claim: false,
                                rewrite: None,
                            }),
                            PlannedReceiver::Stable { .. } => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                }
            }))
            .collect();
        // A consumed call frame owns its original occurrence; captures
        // within that frame are still emitted while the value is active.
        source_replacements.splice(
            0..0,
            call_frames().map(|(source, expr)| SourceReplacement {
                source,
                slot: String::new(),
                jsx_child: false,
                anchor: Some(expr),
                claim: true,
                rewrite: None,
            }),
        );
        source_replacements.sort_by_key(|replacement| {
            (
                replacement.source.start,
                std::cmp::Reverse(replacement.source.end),
            )
        });
        let consumed_exprs: HashSet<ExprId> = compose_operations()
            .flat_map(|operation| operation.values.iter().copied())
            .chain(loop_operations().flat_map(|operation| operation.values.iter().copied()))
            // The replacement covers the entire operation, including values
            // evaluated before its conditional branch (for example its left
            // operand). Their actions still run, but their authored inline
            // occurrences must not be appended after the operation's join slot.
            .chain(composes.iter().flat_map(|rewrite| {
                let operations = || {
                    rewrite.actions.iter().filter_map(|action| match action {
                        ComposeAction::Operation(operation) => Some(operation),
                        ComposeAction::Value(_) => None,
                    })
                };
                rewrite
                    .actions
                    .iter()
                    .filter_map(move |action| match action {
                        ComposeAction::Value(value) => operations()
                            .any(|operation| {
                                operation.parent.start <= value.source.start
                                    && value.source.end <= operation.parent.end
                            })
                            .then_some(value.expr),
                        ComposeAction::Operation(_) => None,
                    })
            }))
            .chain(
                compose_values()
                    .filter(|value| {
                        value
                            .call_completion
                            .as_ref()
                            .is_some_and(|completion| completion.result.is_none())
                    })
                    .map(|value| value.expr),
            )
            // A sibling value inside a completed call's claimed frame has no
            // authored occurrence left; the arm's call reads its join slot
            // through the invoke prefix instead.
            .chain(composes.iter().flat_map(|rewrite| {
                rewrite
                    .actions
                    .iter()
                    .filter_map(|action| match action {
                        ComposeAction::Value(value) => value
                            .call_completion
                            .as_ref()
                            .map(|completion| (value, completion)),
                        _ => None,
                    })
                    .flat_map(move |(value, completion)| {
                        let start = if completion.result.is_some() {
                            completion.call.start
                        } else {
                            rewrite.owner.start
                        };
                        rewrite.actions.iter().filter_map(move |other| match other {
                            ComposeAction::Value(other_value)
                                if other_value.expr != value.expr
                                    && start <= other_value.source.start
                                    && other_value.source.end <= value.source.start =>
                            {
                                Some(other_value.expr)
                            }
                            _ => None,
                        })
                    })
            }))
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
        // A nested value uses its allocated slot only when a
        // statement-capable outer value structurally emits it. If the host
        // owner admits expressions only (for example a parameter default),
        // the nested construct must retain its ordinary expression-boundary
        // emission instead of referring to an unassigned join slot.
        let statement_value_spans: Vec<_> = lowering
            .owners()
            .flat_map(|owner| &owner.values)
            .filter(|value| value.capability == TargetCapability::StatementRegion)
            .map(|value| value.source)
            .chain(lowering.statement_decision_sources().iter().copied())
            .collect();
        let structurally_nested_values: HashSet<_> = lowering
            .nested_values()
            .chain(lowering.structurally_owned_children())
            .collect();
        let subject_values: HashSet<ExprId> = core
            .bodies
            .iter()
            .flat_map(|body| &body.statements)
            .filter_map(|statement| match statement {
                Statement::Decision(decision) => Some(decision),
                _ => None,
            })
            .flat_map(|decision| &decision.subjects)
            .flat_map(|subject| {
                std::iter::successors(Some(subject.value), |expr| {
                    match &core.exprs[expr.index()] {
                        Expr::Sequence(body) => match &core.bodies[body.index()].statements[..] {
                            [Statement::Expr(value)] => Some(*value),
                            _ => None,
                        },
                        _ => None,
                    }
                })
            })
            .collect();
        let nested_values = structurally_nested_values
            .iter()
            .copied()
            .filter(|expr| !subject_values.contains(expr))
            .filter(|expr| {
                // ResultRegion is an isolated expression boundary. Its own
                // emitter delivers the result through that boundary, so
                // replacing it with an enclosing statement slot would skip
                // the Result continuation and transfer ownership to the
                // outer value. Other structured values have no such private
                // boundary and may use the outer statement region's slot.
                if matches!(core.exprs[expr.index()], Expr::ResultRegion(_)) {
                    return false;
                }
                structured_expr_span(semantic, core, *expr).is_some_and(|nested| {
                    statement_value_spans.iter().any(|outer| {
                        outer.start <= nested.start
                            && nested.end <= outer.end
                            && (outer.start < nested.start || nested.end < outer.end)
                    })
                })
            })
            .collect();
        let expression_boundary_name = lowering.expression_boundary_name().to_owned();
        let inline_subjects = compose_values()
            .filter(|value| value.inline)
            .map(|value| {
                let Expr::Decision(decision) = &core.exprs[value.expr.index()] else {
                    crate::ice::bug!("inline match plan lost its decision")
                };
                (
                    decision.extent,
                    lowering.match_subject_names(value.expr).to_vec(),
                )
            })
            .collect();
        Self {
            inline_subjects,
            block_required_statements: lowering.block_required_statements().clone(),
            block_required_owners: lowering.block_required_owners().clone(),
            ambient_items: lowering.ambient_items().clone(),
            script: lowering.is_script(),
            commonjs: lowering.uses_commonjs_syntax(),
            global_temps: lowering.global_temps().clone(),
            match_raise_name: lowering.match_raise_name().to_owned(),
            match_show_name: lowering.match_show_name().to_owned(),
            spread_name: lowering.spread_name().to_owned(),
            guarded_if_tests,
            host_error: lowering.host_global("Error"),
            host_json: lowering.host_global("JSON"),
            host_string: lowering.host_global("String"),
            owner_slots,
            for_initializer_propagations,
            composes,
            declarator_splits,
            loop_tests,
            source_replacements,
            relocated_values,
            rewritten_operations,
            recovered_propagations,
            recovered_matches,
            owner_model: lowering.has_owner_model(),
            consumed_exprs,
            arrow_returns,
            slot_exprs,
            value_slots,
            piped_slots: lowering.piped_slot_names().collect(),
            scheduled_slots,
            value_exits,
            nested_schedules,
            nested_operations: lowering.nested_operations().to_vec(),
            nested_values,
            structurally_nested_values,
            expression_boundary_name,
        }
    }
}
