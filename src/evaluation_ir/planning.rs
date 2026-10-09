//! Target schedule and generated-slot planning.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_schedule(
    protocol: HostEvaluationProtocol,
    tt_spans: &TtSpans,
    slots: &HashMap<SourceSpan, ValueSlotId>,
    source_slots: &mut HashMap<SourceSpan, PlannedSourceSlot>,
    links: &mut PlannedLinks,
    next_slot: &mut u32,
    slot_names: &mut Vec<String>,
    occupied_names: &mut HashSet<String>,
) -> Result<EvaluationSchedule, EvaluationError> {
    // A completed call is re-emitted inside the match's dispatch, where the
    // authored position of an elided input no longer exists. Reserve a name
    // for each one so that lowering can capture it once instead of copying
    // its source into every arm; the elision itself still stands for every
    // other lowering.
    let reserved_steps = protocol.call_completion.as_ref().map_or(0, |facts| {
        protocol
            .steps()
            .iter()
            .position(|step| {
                step.parent == facts.call
                    && matches!(
                        step.operation,
                        HostEvaluationOperation::Eager(
                            crate::program_syntax::EagerPosition::CallArgument(_)
                        )
                    )
            })
            .map_or(0, |call_step| call_step + 1)
    });
    let mut schedule = resolve_schedule_steps(
        protocol.steps(),
        protocol.steps().len(),
        reserved_steps,
        Elision {
            tt_spans,
            reserve_names: false,
        },
        slots,
        source_slots,
        links,
        next_slot,
        slot_names,
        occupied_names,
    )?;
    schedule.call_completion = protocol
        .call_completion
        .map(|facts| {
            Ok::<_, EvaluationError>(PlannedCallCompletion {
                instantiated: facts
                    .type_args
                    .map(|_| allocate_value_slot(next_slot, slot_names, occupied_names))
                    .transpose()?,
                facts,
            })
        })
        .transpose()?;
    Ok(schedule)
}

#[derive(Clone, Copy)]
pub(super) struct Elision<'a> {
    pub(super) tt_spans: &'a TtSpans,
    pub(super) reserve_names: bool,
}

#[derive(Default)]
pub(super) struct PlannedLinks {
    steps: HashMap<
        usize,
        (
            crate::chain::Chain<crate::program_syntax::HostEvaluationStep>,
            crate::chain::Chain<PlannedEvaluationStep>,
        ),
    >,
    inputs: InputLinks,
    summaries: HashMap<
        usize,
        (
            crate::chain::Segments<PlannedEvaluationInput>,
            InputsSummary,
        ),
    >,
}

impl PlannedLinks {
    pub(super) fn summarize(
        &mut self,
        inputs: &crate::chain::Segments<PlannedEvaluationInput>,
        tt_spans: &TtSpans,
    ) -> InputsSummary {
        let mut pending = Vec::new();
        let mut at = inputs;
        let mut summary = InputsSummary::default();
        while !at.is_empty() {
            if let Some((_, found)) = self.summaries.get(&at.identity()) {
                summary = *found;
                break;
            }
            pending.push(at.clone());
            let Some(earlier) = at.earlier() else {
                break;
            };
            at = earlier;
        }
        for segment in pending.into_iter().rev() {
            summary = InputsSummary::of(summary, segment.own(), tt_spans);
            self.summaries
                .insert(segment.identity(), (segment, summary));
        }
        summary
    }
}

type InputLinks = HashMap<
    usize,
    (
        crate::chain::Segments<crate::program_syntax::HostEvaluationInput>,
        crate::chain::Segments<PlannedEvaluationInput>,
    ),
>;

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_schedule_steps(
    protocol_steps: &crate::chain::Chain<crate::program_syntax::HostEvaluationStep>,
    count: usize,
    reserved_steps: usize,
    elision: Elision<'_>,
    slots: &HashMap<SourceSpan, ValueSlotId>,
    source_slots: &mut HashMap<SourceSpan, PlannedSourceSlot>,
    links: &mut PlannedLinks,
    next_slot: &mut u32,
    slot_names: &mut Vec<String>,
    occupied_names: &mut HashSet<String>,
) -> Result<EvaluationSchedule, EvaluationError> {
    let shared = count == protocol_steps.len();
    let mut resolved = Vec::new();
    let mut rest = protocol_steps;
    let mut tail = crate::chain::Chain::new();
    for index in 0..count {
        let reserve_names = index < reserved_steps;
        let elision = Elision {
            reserve_names,
            ..elision
        };
        if shared
            && !reserve_names
            && let Some((_, planned)) = links.steps.get(&rest.identity())
        {
            tail = planned.clone();
            break;
        }
        let (Some(step), Some(next)) = (rest.first(), rest.rest()) else {
            break;
        };
        resolved.push((
            rest.clone(),
            reserve_names,
            resolve_step(
                step,
                elision,
                slots,
                source_slots,
                (!elision.reserve_names).then_some(&mut links.inputs),
                next_slot,
                slot_names,
                occupied_names,
            )?,
        ));
        rest = next;
    }
    for (host, reserved, mut step) in resolved.into_iter().rev() {
        let inputs = links.summarize(&step.inputs, elision.tt_spans);
        step.summary = StepsSummary::link(&step, inputs, &tail);
        tail = crate::chain::Chain::cons(step, tail);
        if shared && !reserved {
            links.steps.insert(host.identity(), (host, tail.clone()));
        }
    }
    Ok(EvaluationSchedule {
        steps: crate::chain::ChainSlice::whole(tail),
        call_completion: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn resolve_step(
    step: &crate::program_syntax::HostEvaluationStep,
    elision: Elision<'_>,
    slots: &HashMap<SourceSpan, ValueSlotId>,
    source_slots: &mut HashMap<SourceSpan, PlannedSourceSlot>,
    inputs_links: Option<&mut InputLinks>,
    next_slot: &mut u32,
    slot_names: &mut Vec<String>,
    occupied_names: &mut HashSet<String>,
) -> Result<PlannedEvaluationStep, EvaluationError> {
    crate::work::tick("planned evaluation steps");
    Ok(PlannedEvaluationStep {
        summary: StepsSummary::default(),
        parent: step.parent,
        operation: step.operation,
        conditional: step.conditional.clone(),
        loop_test: step.loop_test,
        inputs: resolve_inputs(
            &step.inputs,
            elision,
            slots,
            source_slots,
            inputs_links,
            next_slot,
            slot_names,
            occupied_names,
        )?,
    })
}

#[allow(clippy::too_many_arguments)]
fn resolve_inputs(
    inputs: &crate::chain::Segments<crate::program_syntax::HostEvaluationInput>,
    elision: Elision<'_>,
    slots: &HashMap<SourceSpan, ValueSlotId>,
    source_slots: &mut HashMap<SourceSpan, PlannedSourceSlot>,
    links: Option<&mut InputLinks>,
    next_slot: &mut u32,
    slot_names: &mut Vec<String>,
    occupied_names: &mut HashSet<String>,
) -> Result<crate::chain::Segments<PlannedEvaluationInput>, EvaluationError> {
    let mut resolve = |input: &crate::program_syntax::HostEvaluationInput| {
        resolve_input(
            input,
            elision,
            slots,
            source_slots,
            next_slot,
            slot_names,
            occupied_names,
        )
    };
    let Some(links) = links else {
        return Ok(crate::chain::Segments::from_vec(
            inputs
                .iter()
                .map(&mut resolve)
                .collect::<Result<Vec<_>, _>>()?,
        ));
    };
    let mut pending = Vec::new();
    let mut at = inputs;
    let mut base = crate::chain::Segments::new();
    while !at.is_empty() {
        if let Some((_, planned)) = links.get(&at.identity()) {
            base = planned.clone();
            break;
        }
        pending.push(at.clone());
        let Some(earlier) = at.earlier() else {
            break;
        };
        at = earlier;
    }
    for host in pending.into_iter().rev() {
        let own = host
            .own()
            .iter()
            .map(&mut resolve)
            .collect::<Result<Vec<_>, _>>()?;
        base = crate::chain::Segments::extend(base, own);
        links.insert(host.identity(), (host, base.clone()));
    }
    Ok(base)
}

fn resolve_input(
    input: &crate::program_syntax::HostEvaluationInput,
    elision: Elision<'_>,
    slots: &HashMap<SourceSpan, ValueSlotId>,
    source_slots: &mut HashMap<SourceSpan, PlannedSourceSlot>,
    next_slot: &mut u32,
    slot_names: &mut Vec<String>,
    occupied_names: &mut HashSet<String>,
) -> Result<PlannedEvaluationInput, EvaluationError> {
    slots.get(&input.source).map_or_else(
        || {
            // §9 capture elision: an inert value input
            // is left in place — its only role here was
            // order preservation, and evaluating it is
            // unobservable.
            if matches!(
                input.mode,
                EvaluationInputMode::Value
                    | EvaluationInputMode::JsxChildValue
                    | EvaluationInputMode::Discarded
                    | EvaluationInputMode::SpreadElement
                    | EvaluationInputMode::ObjectSpread
                    | EvaluationInputMode::TemplateSubstitution
            ) && input.effects.is_inert()
                && !elision.tt_spans.any_within(input.source)
            {
                return Ok(PlannedEvaluationInput::Stable {
                    source: input.source,
                    reserved: elision
                        .reserve_names
                        .then(|| allocate_value_slot(next_slot, slot_names, occupied_names))
                        .transpose()?,
                });
            }
            if let Some(slot) = source_slots.get(&input.source) {
                return Ok(PlannedEvaluationInput::Source {
                    source: input.source,
                    mode: input.mode,
                    target: slot.target,
                    receiver: slot.receiver,
                    key: slot.key,
                });
            }
            let target = allocate_value_slot(next_slot, slot_names, occupied_names)?;
            let mut part = |part: Option<crate::program_syntax::HostReferencePart>| {
                part.map(|part| {
                    if part.this_of_super {
                        Ok(PlannedReceiver::ThisOfSuper {
                            source: part.source,
                        })
                    } else if part.read_at_call || part.effects.is_inert() {
                        Ok(PlannedReceiver::Stable {
                            source: part.source,
                        })
                    } else {
                        Ok(PlannedReceiver::Captured {
                            source: part.source,
                            slot: allocate_value_slot(next_slot, slot_names, occupied_names)?,
                        })
                    }
                })
                .transpose()
            };
            let receiver = part(input.receiver)?;
            let key = part(input.key)?;
            source_slots.insert(
                input.source,
                PlannedSourceSlot {
                    target,
                    receiver,
                    key,
                },
            );
            Ok(PlannedEvaluationInput::Source {
                source: input.source,
                mode: input.mode,
                target,
                receiver,
                key,
            })
        },
        |slot| {
            Ok(PlannedEvaluationInput::Slot {
                slot: *slot,
                mode: input.mode,
            })
        },
    )
}

pub(super) fn planned_chain(
    steps: Vec<PlannedEvaluationStep>,
    tt_spans: &TtSpans,
) -> crate::chain::Chain<PlannedEvaluationStep> {
    let mut links = PlannedLinks::default();
    let mut tail = crate::chain::Chain::new();
    for mut step in steps.into_iter().rev() {
        let inputs = links.summarize(&step.inputs, tt_spans);
        step.summary = StepsSummary::link(&step, inputs, &tail);
        tail = crate::chain::Chain::cons(step, tail);
    }
    tail
}

pub(super) fn overlaps(left: SourceSpan, right: SourceSpan) -> bool {
    left.start < right.end && right.start < left.end
}

/// The sole conditional step of a value's schedule. Eager steps before it
/// belong to the active branch; steps after it belong to the operation's
/// outer host context.
pub(super) fn whole_operation_step(
    schedule: &EvaluationSchedule,
) -> Option<(usize, &PlannedEvaluationStep)> {
    sole_conditional(schedule.steps())
}

/// Groups the owner's conditional-candidate values into whole conditional
/// operations (결정 17). A group that cannot form a complete operation is
/// downgraded to the expression boundary — never half-lowered.
pub(super) fn plan_conditional_operations(
    values: &mut [PlannedValue],
    tt_spans: &TtSpans,
    next_slot: &mut u32,
    slot_names: &mut Vec<String>,
    occupied_names: &mut HashSet<String>,
) -> Result<Vec<PlannedConditionalOperation>, EvaluationError> {
    let mut order: Vec<SourceSpan> = Vec::new();
    let mut groups: HashMap<SourceSpan, Vec<usize>> = HashMap::new();
    for (index, value) in values.iter().enumerate() {
        if value.capability != TargetCapability::StatementRegion {
            continue;
        }
        let Some((_, step)) = whole_operation_step(&value.schedule) else {
            continue;
        };
        if !groups.contains_key(&step.parent) {
            order.push(step.parent);
        }
        groups.entry(step.parent).or_default().push(index);
    }
    let mut operations = Vec::with_capacity(order.len());
    for parent in order {
        let members = &groups[&parent];
        match plan_one_operation(
            values,
            members,
            parent,
            tt_spans,
            next_slot,
            slot_names,
            occupied_names,
        )? {
            Some(operation) => operations.push(operation),
            None => {
                for member in members {
                    values[*member].capability = TargetCapability::ExpressionBoundary(
                        ExpressionBoundaryReason::ConditionalOperationNotStructurable,
                    );
                }
            }
        }
    }
    Ok(operations)
}

pub(super) fn plan_one_operation(
    values: &[PlannedValue],
    members: &[usize],
    parent: SourceSpan,
    tt_spans: &TtSpans,
    next_slot: &mut u32,
    slot_names: &mut Vec<String>,
    occupied_names: &mut HashSet<String>,
) -> Result<Option<PlannedConditionalOperation>, EvaluationError> {
    let first = &values[members[0]];
    let first_steps = first.schedule.steps();
    let Some((conditional_index, step)) = whole_operation_step(&first.schedule) else {
        return Ok(None);
    };
    // Every member shares the operation, so it must share the operation's
    // host context; a mismatch means the projection joined two different
    // operations to one span, and the group cannot be owned whole.
    if members.iter().any(|member| {
        let Some((member_index, _)) = whole_operation_step(&values[*member].schedule) else {
            return true;
        };
        values[*member].schedule.steps().skip(member_index + 1)
            != first_steps.skip(conditional_index + 1)
    }) {
        return Ok(None);
    }
    let Some(condition) = step.inputs.first().copied() else {
        return Ok(None);
    };
    let Some(facts) = step.conditional.clone() else {
        return Ok(None);
    };
    // An enclosing tt region owns this operation; it is not syntax inside
    // a relocated operand. Only descendants can make that operand opaque.
    let overlaps_tt = |span: SourceSpan| {
        let encloses = |tt: &SourceSpan| tt.start <= parent.start && parent.end <= tt.end;
        tt_spans
            .straddling(span)
            .iter()
            .any(|(_, tt)| !encloses(tt))
            || (tt_spans.any_within(span)
                && tt_spans.within(span).iter().any(|(at, tt)| {
                    let owner = tt_spans.owner(*at);
                    overlaps(span, *tt)
                        && !encloses(tt)
                        && !(span.start <= owner.start && owner.end <= span.end)
                }))
    };
    let mut active = Vec::new();
    let kind = match step.operation {
        HostEvaluationOperation::Conditional(ConditionalBranch::LogicalAndRight)
        | HostEvaluationOperation::Conditional(ConditionalBranch::LogicalOrRight)
        | HostEvaluationOperation::Conditional(ConditionalBranch::NullishRight) => {
            match step.operation {
                HostEvaluationOperation::Conditional(ConditionalBranch::LogicalAndRight) => {
                    PlannedConditionalKind::LogicalAnd
                }
                HostEvaluationOperation::Conditional(ConditionalBranch::LogicalOrRight) => {
                    PlannedConditionalKind::LogicalOr
                }
                _ => PlannedConditionalKind::Nullish,
            }
        }
        HostEvaluationOperation::Conditional(
            ConditionalBranch::Consequent | ConditionalBranch::Alternate,
        ) => {
            let mut consequent: Option<(Vec<ExprId>, Option<SourceSpan>)> = None;
            let mut alternate: Option<(Vec<ExprId>, Option<SourceSpan>)> = None;
            for member in members {
                let value = &values[*member];
                let Some((member_index, member_step)) = whole_operation_step(&value.schedule)
                else {
                    return Ok(None);
                };
                let Some(member_facts) = &member_step.conditional else {
                    return Ok(None);
                };
                active.push(PlannedActiveBranch {
                    value: value.expr,
                    branch: member_facts.branch,
                    steps: value.schedule.steps().take(member_index),
                });
                let side = match member_step.operation {
                    HostEvaluationOperation::Conditional(ConditionalBranch::Consequent) => {
                        &mut consequent
                    }
                    HostEvaluationOperation::Conditional(ConditionalBranch::Alternate) => {
                        &mut alternate
                    }
                    _ => return Ok(None),
                };
                match side {
                    Some((values, _)) => values.push(value.expr),
                    None => *side = Some((vec![value.expr], member_facts.skipped)),
                }
            }
            // A branch that is exactly one value delivers it straight into
            // the result slot; any other branch is rebuilt around its values.
            active.retain(|entry| {
                !entry.steps.is_empty()
                    || members
                        .iter()
                        .map(|member| &values[*member])
                        .any(|value| value.expr == entry.value && value.source != entry.branch)
                    || [&consequent, &alternate]
                        .into_iter()
                        .flatten()
                        .any(|(values, _)| values.len() > 1 && values.contains(&entry.value))
            });
            let fill = |taken: Option<(Vec<ExprId>, Option<SourceSpan>)>,
                        other: Option<Option<SourceSpan>>|
             -> Option<PlannedBranch> {
                match taken {
                    Some((values, _)) => Some(PlannedBranch::Values(values)),
                    // The side with no tt value is the other member's
                    // skipped span — original source relocated into the
                    // branch, which must not contain tt of its own.
                    None => {
                        let span = other.flatten()?;
                        (!overlaps_tt(span)).then_some(PlannedBranch::Source(span))
                    }
                }
            };
            let consequent_skipped = consequent.as_ref().map(|(_, skipped)| *skipped);
            let alternate_skipped = alternate.as_ref().map(|(_, skipped)| *skipped);
            let Some(consequent_branch) = fill(consequent, alternate_skipped) else {
                return Ok(None);
            };
            let Some(alternate_branch) = fill(alternate, consequent_skipped) else {
                return Ok(None);
            };
            PlannedConditionalKind::Ternary {
                consequent: consequent_branch,
                alternate: alternate_branch,
            }
        }
        HostEvaluationOperation::Conditional(ConditionalBranch::LogicalAssignmentRight {
            operator,
            ..
        }) => PlannedConditionalKind::LogicalAssignment { operator },
        HostEvaluationOperation::Conditional(ConditionalBranch::OptionalCallArgument(_)) => {
            // A member callee the chain tests is called through its
            // captured receiver (`callee.call(receiver, ...)`), which cannot
            // carry explicit type arguments.
            let member_callee = matches!(
                condition,
                PlannedEvaluationInput::Source {
                    mode: EvaluationInputMode::MemberReference,
                    ..
                }
            );
            if member_callee
                && facts.type_args.is_some()
                && facts.optional_test == Some(OptionalCallTest::Callee)
            {
                return Ok(None);
            }
            // The receiver test needs the receiver as its own input.
            let test = match facts.optional_test {
                Some(OptionalCallTest::Callee) => OptionalCallTest::Callee,
                Some(OptionalCallTest::Receiver) if member_callee => OptionalCallTest::Receiver,
                Some(OptionalCallTest::Receiver | OptionalCallTest::Inner) | None => {
                    return Ok(None);
                }
            };
            let mut value_indices: HashMap<u32, Vec<(ExprId, usize)>> = HashMap::new();
            for member in members {
                let value = &values[*member];
                let Some((member_index, member_step)) = whole_operation_step(&value.schedule)
                else {
                    return Ok(None);
                };
                let HostEvaluationOperation::Conditional(ConditionalBranch::OptionalCallArgument(
                    index,
                )) = member_step.operation
                else {
                    return Ok(None);
                };
                value_indices
                    .entry(index)
                    .or_default()
                    .push((value.expr, member_index));
            }
            let last_value = *value_indices.keys().max().unwrap_or(&0);
            // Argument capture slots come from the members' planned inputs:
            // the schedule already assigned one to every argument that
            // evaluates before a value.
            // A captured argument answers with its slot; an inert one with
            // `None` — the rebuilt call inlines it, which is unobservable.
            let capture_of = |span: SourceSpan| {
                members.iter().find_map(|member| {
                    let (member_index, _) = whole_operation_step(&values[*member].schedule)?;
                    values[*member]
                        .schedule
                        .steps()
                        .get(member_index)?
                        .inputs
                        .iter()
                        .find_map(|input| match input {
                            PlannedEvaluationInput::Source { source, target, .. }
                                if *source == span =>
                            {
                                Some(Some(*target))
                            }
                            PlannedEvaluationInput::Stable { source, .. } if *source == span => {
                                Some(None)
                            }
                            _ => None,
                        })
                })
            };
            let mut arguments = Vec::with_capacity(facts.operands.len());
            for (index, operand) in facts.operands.iter().enumerate() {
                let index = u32::try_from(index).map_err(|_| EvaluationError::IdOverflow)?;
                match value_indices.get(&index).map(Vec::as_slice) {
                    Some(&[(expr, 0)])
                        if members
                            .iter()
                            .map(|member| &values[*member])
                            .any(|value| value.expr == expr && value.source == operand.span) =>
                    {
                        arguments.push(PlannedOperand::Value(expr))
                    }
                    Some(argument_values) => {
                        for &(expr, member_index) in argument_values {
                            let Some(value) = members
                                .iter()
                                .map(|member| &values[*member])
                                .find(|value| value.expr == expr)
                            else {
                                return Ok(None);
                            };
                            active.push(PlannedActiveBranch {
                                value: expr,
                                branch: operand.span,
                                steps: value.schedule.steps().take(member_index),
                            });
                        }
                        let capture = (index < last_value)
                            .then(|| allocate_value_slot(next_slot, slot_names, occupied_names))
                            .transpose()?;
                        arguments.push(PlannedOperand::Composed {
                            span: operand.span,
                            spread: operand.spread,
                            values: argument_values.iter().map(|(expr, _)| *expr).collect(),
                            capture,
                        });
                    }
                    None => {
                        if overlaps_tt(operand.span) {
                            return Ok(None);
                        }
                        let capture = if index < last_value {
                            match capture_of(operand.span) {
                                Some(capture) => capture,
                                None => return Ok(None),
                            }
                        } else {
                            None
                        };
                        arguments.push(PlannedOperand::Source {
                            span: operand.span,
                            spread: operand.spread,
                            capture,
                        });
                    }
                }
            }
            PlannedConditionalKind::OptionalCall {
                arguments,
                type_args: facts.type_args,
                test,
            }
        }
        _ => return Ok(None),
    };
    let result = allocate_value_slot(next_slot, slot_names, occupied_names)?;
    let logical = matches!(
        &kind,
        PlannedConditionalKind::LogicalAnd
            | PlannedConditionalKind::LogicalOr
            | PlannedConditionalKind::Nullish
    );
    let assignment = matches!(&kind, PlannedConditionalKind::LogicalAssignment { .. });
    let condition = match condition {
        PlannedEvaluationInput::Stable { source, reserved } if logical => {
            PlannedEvaluationInput::Source {
                source,
                mode: EvaluationInputMode::Value,
                target: match reserved {
                    Some(slot) => slot,
                    None => allocate_value_slot(next_slot, slot_names, occupied_names)?,
                },
                receiver: None,
                key: None,
            }
        }
        condition => condition,
    };
    if logical || assignment {
        for member in members {
            let value = &values[*member];
            let Some((member_index, _)) = whole_operation_step(&value.schedule) else {
                return Ok(None);
            };
            active.push(PlannedActiveBranch {
                value: value.expr,
                branch: facts.branch,
                steps: value.schedule.steps().take(member_index),
            });
        }
    }
    Ok(Some(PlannedConditionalOperation {
        parent,
        result,
        kind,
        condition,
        values: members.iter().map(|member| values[*member].expr).collect(),
        active,
        outer: first_steps.skip(conditional_index + 1),
        gaps: facts.gaps.clone(),
    }))
}

/// Decides whether a host value's Core control flow may become statements
/// in its host owner, from typed facts alone: what kind of owner it has,
/// how often the owner is reached, whether the Core value has a statement
/// form, and what the schedule would have to capture and preserve.
///
/// Every refusal names its reason. Target lowering consumes the decision;
/// [`EvaluationFile::validate_order`] and
/// [`EvaluationFile::validate_reference`] re-check the resulting plan
/// independently.
pub(super) fn target_capability(
    core: &CoreFile,
    tt_spans: &TtSpans,
    expr: ExprId,
    source: SourceSpan,
    context: &EvaluationContext,
    schedule: &EvaluationSchedule,
) -> TargetCapability {
    use ExpressionBoundaryReason as Reason;
    if matches!(
        context.owner,
        EvaluationOwner::ParameterInitializer
            | EvaluationOwner::ClassInitializer
            | EvaluationOwner::ClassDefinition
            | EvaluationOwner::EnumInitializer
    ) {
        return TargetCapability::ExpressionBoundary(Reason::OwnerTakesNoStatements);
    }
    if !core.has_statement_form(expr) {
        return TargetCapability::ExpressionBoundary(Reason::ValueHasNoStatementForm);
    }
    if context.loop_head_declarator {
        return TargetCapability::ExpressionBoundary(Reason::LoopHeadDeclarator);
    }
    if context.loop_head_binding {
        return TargetCapability::ExpressionBoundary(Reason::LoopHeadBinding);
    }
    match context.owner_reach {
        OwnerReach::Same => {}
        OwnerReach::Repeated => {
            if !matches!(core.exprs[expr.index()], Expr::Decision(_)) {
                return TargetCapability::ExpressionBoundary(Reason::RepeatedInOwner);
            }
            if schedule.loop_test_count() != 1
                || schedule.outermost_step().is_none_or(|step| {
                    step.operation != HostEvaluationOperation::LoopTest || step.loop_test.is_none()
                })
            {
                return TargetCapability::ExpressionBoundary(Reason::RepeatedInOwner);
            }
        }
        OwnerReach::UnmodeledConditional => {
            return TargetCapability::ExpressionBoundary(Reason::ConditionalInOwner);
        }
    }
    let steps = schedule.steps();
    match reference_lost(steps) {
        Some(true) => {
            return TargetCapability::ExpressionBoundary(Reason::ReferenceNotPreservable);
        }
        Some(false) => {}
        None => {
            for step in steps {
                for input in &step.inputs {
                    match input {
                        PlannedEvaluationInput::Source {
                            mode: EvaluationInputMode::MemberReference,
                            receiver: None,
                            ..
                        }
                        | PlannedEvaluationInput::Slot {
                            mode: EvaluationInputMode::MemberReference,
                            ..
                        } => {
                            return TargetCapability::ExpressionBoundary(
                                Reason::ReferenceNotPreservable,
                            );
                        }
                        PlannedEvaluationInput::Source { .. }
                        | PlannedEvaluationInput::Slot { .. }
                        | PlannedEvaluationInput::Stable { .. } => {}
                    }
                }
            }
        }
    }
    // A conditional step is lowerable only when the whole operation can be
    // owned as one region. Exactly one conditional boundary is allowed;
    // eager steps inside its active branch are kept there by the operation
    // plan. Anything else takes the boundary — never a promoted value under
    // the original syntax.
    let conditional_steps = conditional_count(steps);
    if conditional_steps > 0 {
        let Some((index, step)) = whole_operation_step(schedule) else {
            return TargetCapability::ExpressionBoundary(
                Reason::ConditionalOperationNotStructurable,
            );
        };
        if let HostEvaluationOperation::Conditional(ConditionalBranch::LogicalAssignmentRight {
            consumed,
            ..
        }) = step.operation
            && (consumed || index + 1 < steps.len())
        {
            return TargetCapability::ExpressionBoundary(Reason::LogicalAssignmentValue);
        }
        // An optional call skipped at a link inside its callee's receiver
        // cannot be tested before that link is evaluated.
        let structurable = step.conditional.as_ref().is_some_and(|facts| {
            facts.branch.start <= source.start
                && source.end <= facts.branch.end
                && facts.optional_test != Some(OptionalCallTest::Inner)
        });
        if !structurable {
            return TargetCapability::ExpressionBoundary(
                Reason::ConditionalOperationNotStructurable,
            );
        }
    }
    if captures_admit(steps, source) {
        return TargetCapability::StatementRegion;
    }
    let mut captured: Vec<SourceSpan> = Vec::new();
    for step in steps {
        for input in &step.inputs {
            let PlannedEvaluationInput::Source {
                source: capture, ..
            } = input
            else {
                continue;
            };
            if captured.contains(capture) {
                continue;
            }
            // The capture copies raw source bytes; a sibling tt node inside
            // them is lowered or relocated elsewhere.
            // An enclosing tt root is different: its structured lowering
            // owns this schedule and composes the captured source into it.
            if tt_spans.overlapping(*capture).iter().any(|span| {
                !(span.start <= source.start && source.end <= span.end)
                    && !(span.end <= source.start
                        && capture.start <= span.start
                        && span.end <= capture.end)
            }) || captured.iter().any(|span| {
                // An earlier capture inside this one is its dependency: the
                // capture reads that slot instead of the source again.
                overlaps(*capture, *span)
                    && !(capture.start <= span.start && span.end <= capture.end)
            }) {
                return TargetCapability::ExpressionBoundary(Reason::CaptureOverlapsValue);
            }
            captured.push(*capture);
        }
    }
    TargetCapability::StatementRegion
}

pub(super) fn allocate_value_slot(
    next_slot: &mut u32,
    slot_names: &mut Vec<String>,
    occupied: &mut HashSet<String>,
) -> Result<ValueSlotId, EvaluationError> {
    let slot = ValueSlotId(*next_slot);
    *next_slot = next_slot
        .checked_add(1)
        .ok_or(EvaluationError::IdOverflow)?;
    slot_names.push(allocate_slot_name(slot, occupied)?);
    Ok(slot)
}

pub(super) fn allocate_slot_name(
    slot: ValueSlotId,
    occupied: &mut HashSet<String>,
) -> Result<String, EvaluationError> {
    allocate_generated_name(&format!("$tt_v{}", slot.0), occupied)
}

pub(super) fn allocate_generated_name(
    base: &str,
    occupied: &mut HashSet<String>,
) -> Result<String, EvaluationError> {
    crate::generated_names::allocate(base, occupied).ok_or(EvaluationError::GeneratedNameOverflow)
}
