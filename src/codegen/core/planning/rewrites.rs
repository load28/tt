//! Source rewrite records and local source-edit boundaries.

use super::*;

#[derive(Debug, Clone)]
pub(in super::super) struct OwnerSlotRewrite {
    pub(in super::super) owner: SourceSpan,
    pub(in super::super) source: SourceSpan,
    pub(in super::super) expr: ExprId,
    pub(in super::super) slot: String,
    pub(in super::super) continuation: HostContinuation,
    pub(in super::super) contextual_type: Option<SourceSpan>,
    pub(in super::super) contextual_type_awaited: bool,
    pub(in super::super) contextual_type_asserted: bool,
}

#[derive(Debug, Clone, Copy)]
pub(in super::super) struct ForInitializerPropagationRewrite {
    pub(in super::super) node: NodeId,
    pub(in super::super) owner: SourceSpan,
    pub(in super::super) source: SourceSpan,
}

#[derive(Debug, Clone)]
pub(in super::super) struct ArrowReturnRewrite {
    pub(in super::super) source: SourceSpan,
    pub(in super::super) expr: ExprId,
    pub(in super::super) slot: String,
    pub(in super::super) contextual_type: Option<SourceSpan>,
    pub(in super::super) contextual_type_awaited: bool,
    pub(in super::super) contextual_type_asserted: bool,
}

#[derive(Debug, Clone)]
pub(in super::super) struct DeclaratorSplitRewrite {
    pub(in super::super) separator: SourceSpan,
    pub(in super::super) at: usize,
    pub(in super::super) head: String,
    pub(in super::super) block: Option<SourceSpan>,
    pub(in super::super) statement: SourceSpan,
    pub(in super::super) last: bool,
}

pub(in super::super) fn declarator_separator(source: &str, previous_end: usize) -> SourceSpan {
    let bytes = source.as_bytes();
    let (comma, _) = crate::scanner::skip_trivia(bytes, previous_end, bytes.len());
    if bytes.get(comma) != Some(&b',') {
        crate::ice::bug!("a declarator is not preceded by its comma");
    }
    SourceSpan {
        start: comma,
        end: comma + 1,
    }
}

#[derive(Debug, Clone)]
pub(in super::super) struct ComposeRewrite {
    pub(in super::super) owner: SourceSpan,
    pub(in super::super) owner_kind: HostOwnerKind,
    pub(in super::super) actions: Vec<ComposeAction>,
}

#[derive(Debug, Clone)]
pub(in super::super) struct LoopTestRewrite {
    pub(in super::super) owner: SourceSpan,
    pub(in super::super) kind: LoopTestKind,
    pub(in super::super) test: SourceSpan,
    pub(in super::super) body: SourceSpan,
    pub(in super::super) update: Option<SourceSpan>,
    pub(in super::super) first_expr: ExprId,
    pub(in super::super) first_source: SourceSpan,
    pub(in super::super) actions: Vec<ComposeAction>,
}

/// One unit of a compose prelude, in source order: a plain host value, or a
/// whole conditional operation (결정 17).
#[derive(Debug, Clone)]
pub(in super::super) enum ComposeAction {
    Value(ComposeValue),
    Operation(PlannedConditionalOperation),
}

/// A syntax-proven call the dispatch arms perform themselves, so the
/// argument keeps the consumer's contextual type (TASK-324, TASK-327).
#[derive(Debug, Clone)]
pub(in super::super) struct CallCompletionPlan {
    /// The text each arm calls through, up to and excluding the argument:
    /// the captured (possibly instantiated) callee plus `(`.
    pub(in super::super) invoke: AuthoredText,
    pub(in super::super) close: AuthoredText,
    /// A capture emitted once before the dispatch, binding the instantiated
    /// callee: generated name, authored type-argument span, callee slot.
    pub(in super::super) instantiation: Option<(String, SourceSpan, String)>,
    /// Elided captures the dispatch has to name after all: generated name
    /// and the authored source it binds. The completion re-emits the call
    /// inside the arms, where the input's authored position is gone, so it
    /// is captured once here rather than copied into every arm.
    pub(in super::super) captures: Vec<(String, SourceSpan)>,
    /// The value slot receiving the call's result when the authored call is
    /// consumed; `None` for a discarded expression-statement call.
    pub(in super::super) result: Option<String>,
    /// The callee slot's generated name — a valid identifier that seeds the
    /// region's exit label when the discarded form needs one.
    pub(in super::super) label: String,
    /// The whole authored call expression.
    pub(in super::super) call: SourceSpan,
    /// The authored literal the value sits inside, split at the value: the
    /// argument's text before it and after it. Empty when the value is the
    /// whole argument. Each arm re-emits both around its own value, which is
    /// what puts the arm value back in the consumer's contextual position.
    pub(in super::super) frame: Option<(SourceSpan, SourceSpan)>,
}

#[derive(Debug, Clone)]
pub(in super::super) struct ComposeValue {
    pub(in super::super) call_completion: Option<CallCompletionPlan>,
    /// Multi-value owners keep each match at its native evaluation position.
    pub(in super::super) inline: bool,
    pub(in super::super) expr: ExprId,
    pub(in super::super) source: SourceSpan,
    pub(in super::super) slot: String,
    pub(in super::super) steps: crate::chain::ChainSlice<PlannedEvaluationStep>,
    /// Select an expression arm in the prelude, but evaluate its value in
    /// the authored host so TypeScript can apply contextual typing.
    pub(in super::super) defer_arm_values: bool,
}

#[derive(Debug, Clone)]
pub(in super::super) struct SourceReplacement {
    pub(in super::super) source: SourceSpan,
    pub(in super::super) slot: String,
    /// What the source walk writes in place of `source` when it is not the
    /// slot's name — a compound assignment's operator, rewritten to apply
    /// to the accumulator that read the target ([`compound_assignment_operator`]).
    pub(in super::super) rewrite: Option<String>,
    pub(in super::super) jsx_child: bool,
    /// The tt value whose construct anchor the replacement's generated
    /// name carries — a conditional operation's result stands for the whole
    /// operation, so diagnostics on it belong to its primary tt value.
    pub(in super::super) anchor: Option<ExprId>,
    /// A completed call's claimed frame. Its own active value retains the
    /// authored source; unrelated enclosing values do not inhibit the claim.
    pub(in super::super) claim: bool,
}

impl SourceReplacement {
    /// The text the source walk writes in place of the replaced source.
    pub(in super::super) fn written(&self) -> &str {
        self.rewrite.as_deref().unwrap_or(&self.slot)
    }
}

/// The operator token of the compound assignment whose target is `target`.
///
/// Only trivia separates an assignment's target from its operator, and the
/// target's span includes any parentheses around it.
pub(in super::super) fn compound_assignment_operator(
    source: &str,
    target: SourceSpan,
    operator: &str,
) -> SourceSpan {
    let bytes = source.as_bytes();
    let (start, _) = crate::scanner::skip_trivia(bytes, target.end, bytes.len());
    let end = start + operator.len();
    if source.get(start..end) != Some(operator) {
        crate::ice::bug!("a compound assignment's operator does not follow its target");
    }
    SourceSpan { start, end }
}

/// The comma after a discarded comma operand. Once the operand ran as a
/// statement, the lowering removes it and this comma where they were
/// written, keeping the trivia between them. Only trivia separates an
/// operand from its comma.
pub(in super::super) fn discarded_operand_comma(source: &str, operand: SourceSpan) -> SourceSpan {
    let bytes = source.as_bytes();
    let (comma, _) = crate::scanner::skip_trivia(bytes, operand.end, bytes.len());
    if bytes.get(comma) != Some(&b',') {
        crate::ice::bug!("a discarded comma operand is not followed by its comma");
    }
    SourceSpan {
        start: comma,
        end: comma + 1,
    }
}

/// The comma of every discarded comma operand a schedule evaluates as a
/// statement. The operand is relocated and the comma is removed with it,
/// so the plan claims the comma.
pub(super) fn discarded_operand_commas<'s>(
    source: &str,
    steps: impl Iterator<Item = &'s PlannedEvaluationStep>,
) -> Vec<SourceSpan> {
    steps
        .flat_map(|step| &step.inputs)
        .filter_map(|input| match input {
            PlannedEvaluationInput::Source {
                source: operand,
                mode: EvaluationInputMode::Discarded,
                ..
            } => Some(discarded_operand_comma(source, *operand)),
            _ => None,
        })
        .collect()
}

/// The target and operator span of every compound assignment whose target a
/// schedule reads before the right operand. The target is printed twice —
/// read into its accumulator, then assigned — and the operator is rewritten,
/// so the plan claims both.
pub(super) fn compound_assignment_frames<'s>(
    source: &str,
    steps: impl Iterator<Item = &'s PlannedEvaluationStep>,
) -> Vec<SourceSpan> {
    steps
        .flat_map(|step| &step.inputs)
        .filter_map(|input| match input {
            PlannedEvaluationInput::Source {
                source: target,
                mode: EvaluationInputMode::CompoundAssignmentTarget { operator },
                ..
            } => Some([
                *target,
                compound_assignment_operator(source, *target, operator),
            ]),
            _ => None,
        })
        .flatten()
        .collect()
}

#[derive(Debug, Clone)]
pub(in super::super) struct LocalSourceEdit {
    pub(in super::super) span: SourceSpan,
    pub(in super::super) text: String,
    pub(in super::super) result_return_mark: Option<(SourceSpan, ResultReturnBoundary)>,
}

#[derive(Debug, Clone, Copy)]
pub(in super::super) enum ResultReturnBoundary {
    Start,
    End,
}

pub(in super::super) fn jsx_closing_name(source: &str, element: SourceSpan) -> Option<SourceSpan> {
    let text = &source[element.start..element.end];
    if text.trim_end().ends_with("/>") {
        return None;
    }
    let open = text.rfind("</")? + 2;
    let bytes = text.as_bytes();
    let start = open
        + bytes[open..]
            .iter()
            .take_while(|byte| byte.is_ascii_whitespace())
            .count();
    let end = start
        + bytes[start..]
            .iter()
            .take_while(|byte| !byte.is_ascii_whitespace() && **byte != b'>')
            .count();
    (start < end).then_some(SourceSpan {
        start: element.start + start,
        end: element.start + end,
    })
}
