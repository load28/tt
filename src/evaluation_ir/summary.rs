use super::*;

#[derive(Debug, Clone, Default)]
pub(crate) struct StepsSummary {
    known: bool,
    conditionals: usize,
    loop_tests: usize,
    outermost: Option<crate::chain::Chain<PlannedEvaluationStep>>,
    conditional: Option<(usize, crate::chain::Chain<PlannedEvaluationStep>)>,
    reference_lost: bool,
    captures: CaptureSummary,
}

impl PartialEq for StepsSummary {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for StepsSummary {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CaptureSummary {
    envelope: Option<(usize, usize)>,
    tangled: bool,
    inside_end: usize,
    partial: Option<(usize, usize)>,
}

impl CaptureSummary {
    fn then(self, later: CaptureSummary) -> CaptureSummary {
        let tangled = self.tangled
            || later.tangled
            || matches!((self.envelope, later.envelope), (Some(a), Some(b)) if a.0 < b.1 && b.0 < a.1);
        CaptureSummary {
            envelope: match (self.envelope, later.envelope) {
                (Some(a), Some(b)) => Some((a.0.min(b.0), a.1.max(b.1))),
                (a, b) => a.or(b),
            },
            tangled,
            inside_end: self.inside_end.max(later.inside_end),
            partial: match (self.partial, later.partial) {
                (Some(a), Some(b)) => Some((a.0.max(b.0), a.1.min(b.1))),
                (a, b) => a.or(b),
            },
        }
    }

    fn capture(self, capture: SourceSpan, tt_spans: &TtSpans) -> CaptureSummary {
        let mut own = CaptureSummary {
            envelope: Some((capture.start, capture.end)),
            ..CaptureSummary::default()
        };
        if tt_spans.any_within(capture) {
            own.inside_end = capture.end;
        }
        for (_, span) in tt_spans.straddling(capture) {
            own.partial = Some(match own.partial {
                Some((start, end)) => (start.max(span.start), end.min(span.end)),
                None => (span.start, span.end),
            });
        }
        self.then(own)
    }

    fn admits(&self, value: SourceSpan) -> bool {
        !self.tangled
            && self.inside_end <= value.start
            && self
                .partial
                .is_none_or(|(start, end)| start <= value.start && value.end <= end)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct InputsSummary {
    reference_lost: bool,
    captures: CaptureSummary,
}

impl InputsSummary {
    pub(super) fn of(
        earlier: InputsSummary,
        inputs: &[PlannedEvaluationInput],
        tt_spans: &TtSpans,
    ) -> InputsSummary {
        let mut summary = earlier;
        for input in inputs {
            match input {
                PlannedEvaluationInput::Source {
                    mode: EvaluationInputMode::MemberReference,
                    receiver: None,
                    ..
                }
                | PlannedEvaluationInput::Slot {
                    mode: EvaluationInputMode::MemberReference,
                    ..
                } => summary.reference_lost = true,
                PlannedEvaluationInput::Source { .. }
                | PlannedEvaluationInput::Slot { .. }
                | PlannedEvaluationInput::Stable { .. } => {}
            }
            if let PlannedEvaluationInput::Source { source, .. } = input {
                summary.captures = summary.captures.capture(*source, tt_spans);
            }
        }
        summary
    }
}

impl StepsSummary {
    pub(super) fn link(
        step: &PlannedEvaluationStep,
        inputs: InputsSummary,
        outer: &crate::chain::Chain<PlannedEvaluationStep>,
    ) -> StepsSummary {
        let rest = outer.first().map(|step| &step.summary);
        let conditional = matches!(step.operation, HostEvaluationOperation::Conditional(_));
        let outer_conditional = outer
            .first()
            .is_some_and(|step| matches!(step.operation, HostEvaluationOperation::Conditional(_)));
        let loop_test = step.operation == HostEvaluationOperation::LoopTest;
        StepsSummary {
            known: rest.is_none_or(|rest| rest.known),
            conditionals: usize::from(conditional) + rest.map_or(0, |rest| rest.conditionals),
            loop_tests: usize::from(loop_test) + rest.map_or(0, |rest| rest.loop_tests),
            outermost: match rest {
                Some(rest) => rest.outermost.clone().or_else(|| Some(outer.clone())),
                None => None,
            },
            conditional: if conditional {
                None
            } else if outer_conditional {
                Some((1, outer.clone()))
            } else {
                rest.and_then(|rest| rest.conditional.clone())
                    .map(|(index, link)| (index + 1, link))
            },
            reference_lost: inputs.reference_lost || rest.is_some_and(|rest| rest.reference_lost),
            captures: match rest {
                Some(rest) => inputs.captures.then(rest.captures),
                None => inputs.captures,
            },
        }
    }
}

pub(super) fn whole_steps(
    steps: &crate::chain::ChainSlice<PlannedEvaluationStep>,
) -> Option<&StepsSummary> {
    steps
        .is_whole()
        .then(|| steps.iter().next().map(|step| &step.summary))
        .flatten()
        .filter(|summary| summary.known)
}

pub(super) fn outermost_step(
    steps: &crate::chain::ChainSlice<PlannedEvaluationStep>,
) -> Option<&PlannedEvaluationStep> {
    match whole_steps(steps) {
        Some(summary) => match &summary.outermost {
            Some(link) => link.first(),
            None => steps.iter().next(),
        },
        None => steps.last(),
    }
}

pub(super) fn loop_test_count(steps: &crate::chain::ChainSlice<PlannedEvaluationStep>) -> usize {
    match whole_steps(steps) {
        Some(summary) => summary.loop_tests,
        None => steps
            .iter()
            .filter(|step| step.operation == HostEvaluationOperation::LoopTest)
            .count(),
    }
}

pub(super) fn conditional_count(steps: &crate::chain::ChainSlice<PlannedEvaluationStep>) -> usize {
    match whole_steps(steps) {
        Some(summary) => summary.conditionals,
        None => steps
            .iter()
            .filter(|step| matches!(step.operation, HostEvaluationOperation::Conditional(_)))
            .count(),
    }
}

pub(super) fn sole_conditional(
    steps: &crate::chain::ChainSlice<PlannedEvaluationStep>,
) -> Option<(usize, &PlannedEvaluationStep)> {
    if let Some(summary) = whole_steps(steps) {
        if summary.conditionals != 1 {
            return None;
        }
        if let Some((index, link)) = &summary.conditional {
            return link.first().map(|step| (*index, step));
        }
        return steps.iter().next().map(|step| (0, step));
    }
    let mut conditional = steps
        .iter()
        .enumerate()
        .filter(|(_, step)| matches!(step.operation, HostEvaluationOperation::Conditional(_)));
    let found = conditional.next()?;
    conditional.next().is_none().then_some(found)
}

pub(super) fn reference_lost(
    steps: &crate::chain::ChainSlice<PlannedEvaluationStep>,
) -> Option<bool> {
    whole_steps(steps).map(|summary| summary.reference_lost)
}

pub(super) fn captures_admit(
    steps: &crate::chain::ChainSlice<PlannedEvaluationStep>,
    value: SourceSpan,
) -> bool {
    whole_steps(steps).is_some_and(|summary| summary.captures.admits(value))
}
