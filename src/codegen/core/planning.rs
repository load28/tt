//! Host rewrite planning and source-preservation classification.

use super::*;

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
pub(super) fn directive_prologue_end(source: &str) -> usize {
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
pub(super) fn skip_trivia(bytes: &[u8], mut at: usize) -> usize {
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
pub(super) fn string_literal_end(bytes: &[u8], at: usize) -> Option<usize> {
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
    /// tt values a conditional operation consumes; their inline Core
    /// position emits nothing (the operation's replacement covers it).
    pub(super) consumed_exprs: HashSet<ExprId>,
    pub(super) arrow_returns: Vec<ArrowReturnRewrite>,
    pub(super) slot_exprs: HashMap<ExprId, String>,
    pub(super) value_slots: HashMap<ExprId, String>,
    pub(super) scheduled_slots: HashMap<crate::evaluation_ir::ValueSlotId, String>,
    pub(super) value_exits: HashMap<ExprId, Vec<HostExit>>,
    pub(super) nested_schedules: HashMap<ExprId, EvaluationSchedule>,
    pub(super) nested_values: HashSet<ExprId>,
    /// Every value whose Evaluation IR placement is structurally nested,
    /// before target-specific slot-substitution filtering.
    pub(super) structurally_nested_values: HashSet<ExprId>,
    pub(super) expression_boundary_name: String,
    pub(super) match_raise_name: String,
    pub(super) inline_subjects: HashMap<NodeId, Vec<String>>,
}

#[derive(Debug, Clone)]
pub(super) struct OwnerSlotRewrite {
    pub(super) owner: SourceSpan,
    pub(super) source: SourceSpan,
    pub(super) expr: ExprId,
    pub(super) slot: String,
    pub(super) continuation: HostContinuation,
    pub(super) contextual_type: Option<SourceSpan>,
    pub(super) contextual_type_awaited: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ForInitializerPropagationRewrite {
    pub(super) node: NodeId,
    pub(super) owner: SourceSpan,
    pub(super) source: SourceSpan,
}

#[derive(Debug, Clone)]
pub(super) struct ArrowReturnRewrite {
    pub(super) source: SourceSpan,
    pub(super) expr: ExprId,
    pub(super) slot: String,
    pub(super) parenthesized: bool,
    pub(super) contextual_type: Option<SourceSpan>,
    pub(super) contextual_type_awaited: bool,
}

#[derive(Debug, Clone)]
pub(super) struct ComposeRewrite {
    pub(super) owner: SourceSpan,
    pub(super) owner_kind: HostOwnerKind,
    pub(super) actions: Vec<ComposeAction>,
}

#[derive(Debug, Clone)]
pub(super) struct LoopTestRewrite {
    pub(super) owner: SourceSpan,
    pub(super) kind: LoopTestKind,
    pub(super) test: SourceSpan,
    pub(super) body: SourceSpan,
    pub(super) update: Option<SourceSpan>,
    pub(super) first_expr: ExprId,
    pub(super) first_source: SourceSpan,
    pub(super) actions: Vec<ComposeAction>,
}

/// One unit of a compose prelude, in source order: a plain host value, or a
/// whole conditional operation (결정 17).
#[derive(Debug, Clone)]
pub(super) enum ComposeAction {
    Value(ComposeValue),
    Operation(PlannedConditionalOperation),
}

/// A syntax-proven call the dispatch arms perform themselves, so the
/// argument keeps the consumer's contextual type (TASK-324, TASK-327).
#[derive(Debug, Clone)]
pub(super) struct CallCompletionPlan {
    /// The text each arm calls through, up to and excluding the argument:
    /// the captured (possibly instantiated) callee plus `(`.
    pub(super) invoke: String,
    /// A capture emitted once before the dispatch, binding the instantiated
    /// callee: generated name, authored type-argument span, callee slot.
    pub(super) instantiation: Option<(String, SourceSpan, String)>,
    /// Elided captures the dispatch has to name after all: generated name
    /// and the authored source it binds. The completion re-emits the call
    /// inside the arms, where the input's authored position is gone, so it
    /// is captured once here rather than copied into every arm.
    pub(super) captures: Vec<(String, SourceSpan)>,
    /// The value slot receiving the call's result when the authored call is
    /// consumed; `None` for a discarded expression-statement call.
    pub(super) result: Option<String>,
    /// The callee slot's generated name — a valid identifier that seeds the
    /// region's exit label when the discarded form needs one.
    pub(super) label: String,
    /// The whole authored call expression.
    pub(super) call: SourceSpan,
    /// The authored literal the value sits inside, split at the value: the
    /// argument's text before it and after it. Empty when the value is the
    /// whole argument. Each arm re-emits both around its own value, which is
    /// what puts the arm value back in the consumer's contextual position.
    pub(super) frame: Option<(SourceSpan, SourceSpan)>,
}

#[derive(Debug, Clone)]
pub(super) struct ComposeValue {
    pub(super) call_completion: Option<CallCompletionPlan>,
    /// Multi-value owners keep each match at its native evaluation position.
    pub(super) inline: bool,
    pub(super) expr: ExprId,
    pub(super) source: SourceSpan,
    pub(super) slot: String,
    pub(super) steps: Vec<PlannedEvaluationStep>,
    /// Select an expression arm in the prelude, but evaluate its value in
    /// the authored host so TypeScript can apply contextual typing.
    pub(super) defer_arm_values: bool,
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
    expr: ExprId,
    exits: &[HostExit],
    schedule: &EvaluationSchedule,
    value_slot: &str,
    value_source: SourceSpan,
    lowering: &LoweringPlan,
) -> Option<CallCompletionPlan> {
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
    let PlannedEvaluationInput::Source { target, .. } = step.inputs.first()? else {
        return None;
    };
    let callee = lowering.slot_name(*target).to_owned();
    let instantiation = match (completion.facts.type_args, completion.instantiated) {
        (Some(type_args), Some(slot)) => Some((
            lowering.slot_name(slot).to_owned(),
            type_args,
            callee.clone(),
        )),
        (None, None) => None,
        _ => return None,
    };
    let mut invoke = format!(
        "{}(",
        instantiation
            .as_ref()
            .map_or(callee.as_str(), |(name, ..)| name.as_str())
    );
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
                .is_some_and(|arm| matches!(arm.pattern, PatternPlan::Any) && arm.guard.is_none())
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

#[derive(Debug, Clone)]
pub(super) struct SourceReplacement {
    pub(super) source: SourceSpan,
    pub(super) slot: String,
    pub(super) jsx_child: bool,
    /// The tt value whose construct anchor the replacement's generated
    /// name carries — a conditional operation's result stands for the whole
    /// operation, so diagnostics on it belong to its primary tt value.
    pub(super) anchor: Option<ExprId>,
    /// A completed call's claimed frame. It erases the frame only from the
    /// remaining statement walk; while any value emits structurally (a
    /// sibling's dispatch reading its subject or arm source inside the
    /// frame), the authored text still passes through.
    pub(super) claim: bool,
}

pub(super) type NestedSourceReplacement = (SourceSpan, String, Option<(AnchorKind, usize)>);

#[derive(Debug, Clone)]
pub(super) struct LocalSourceEdit {
    pub(super) span: SourceSpan,
    pub(super) text: String,
    pub(super) result_return_mark: Option<(SourceSpan, ResultReturnBoundary)>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ResultReturnBoundary {
    Start,
    End,
}

impl TargetRewritePlan {
    pub(super) fn build(
        semantic: &SemanticFile,
        core: &CoreFile,
        source: &str,
        lowering: &LoweringPlan,
    ) -> Self {
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
                        source: value.source,
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
                        value.context.continuation == HostContinuation::Compose
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
                                        core,
                                        value.expr,
                                        &value.exits,
                                        &value.schedule,
                                        &slot_name,
                                        value.source,
                                        lowering,
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
        let rewritten_operations: Vec<SourceSpan> = all_operations()
            .map(|operation| operation.parent)
            .chain(call_frames().map(|(span, _)| span))
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
            // Concise-arrow rewrites emit host grouping as block/IIFE
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
                }
            })
            .collect();
        let mut source_replacements: Vec<_> = all_values()
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
                    claim: false,
                }),
                PlannedEvaluationInput::Slot { .. } | PlannedEvaluationInput::Stable { .. } => None,
            })
            .chain(owner_slots.iter().map(|rewrite| SourceReplacement {
                source: rewrite.source,
                slot: rewrite.slot.clone(),
                jsx_child: false,
                anchor: Some(rewrite.expr),
                claim: false,
            }))
            .chain(operation_replacements)
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
            }),
        );
        let consumed_exprs: HashSet<ExprId> = compose_operations()
            .flat_map(|operation| operation.values.iter().copied())
            .chain(loop_operations().flat_map(|operation| operation.values.iter().copied()))
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
            .collect();
        let structurally_nested_values: HashSet<_> = lowering
            .nested_values()
            .chain(lowering.structurally_owned_children())
            .collect();
        let nested_values = structurally_nested_values
            .iter()
            .copied()
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
            match_raise_name: lowering.match_raise_name().to_owned(),
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
            structurally_nested_values,
            expression_boundary_name,
        }
    }
}
