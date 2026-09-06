//! Target schedule and generated-slot planning.

use super::*;

pub(super) fn resolve_schedule(
    protocol: HostEvaluationProtocol,
    slots: &HashMap<SourceSpan, ValueSlotId>,
    source_slots: &mut HashMap<SourceSpan, PlannedSourceSlot>,
    next_slot: &mut u32,
    slot_names: &mut Vec<String>,
    occupied_names: &mut HashSet<String>,
) -> Result<EvaluationSchedule, EvaluationError> {
    // A completed call is re-emitted inside the match's dispatch, where the
    // authored position of an elided input no longer exists. Reserve a name
    // for each one so that lowering can capture it once instead of copying
    // its source into every arm; the elision itself still stands for every
    // other lowering.
    let mut schedule = resolve_schedule_steps(
        protocol.steps(),
        slots,
        source_slots,
        next_slot,
        slot_names,
        occupied_names,
        protocol.call_completion.is_some(),
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

pub(super) fn resolve_schedule_steps(
    protocol_steps: &[crate::program_syntax::HostEvaluationStep],
    slots: &HashMap<SourceSpan, ValueSlotId>,
    source_slots: &mut HashMap<SourceSpan, PlannedSourceSlot>,
    next_slot: &mut u32,
    slot_names: &mut Vec<String>,
    occupied_names: &mut HashSet<String>,
    reserve_elided_names: bool,
) -> Result<EvaluationSchedule, EvaluationError> {
    let steps = protocol_steps
        .iter()
        .map(|step| {
            Ok(PlannedEvaluationStep {
                parent: step.parent,
                operation: step.operation,
                conditional: step.conditional.clone(),
                loop_test: step.loop_test,
                inputs: step
                    .inputs
                    .iter()
                    .map(|input| {
                        slots.get(&input.source).map_or_else(
                            || {
                                // §9 capture elision: an inert value input
                                // is left in place — its only role here was
                                // order preservation, and evaluating it is
                                // unobservable.
                                if matches!(
                                    input.mode,
                                    EvaluationInputMode::Value | EvaluationInputMode::JsxChildValue
                                ) && input.effects.is_inert()
                                {
                                    return Ok(PlannedEvaluationInput::Stable {
                                        source: input.source,
                                        reserved: reserve_elided_names
                                            .then(|| {
                                                allocate_value_slot(
                                                    next_slot,
                                                    slot_names,
                                                    occupied_names,
                                                )
                                            })
                                            .transpose()?,
                                    });
                                }
                                if let Some(slot) = source_slots.get(&input.source) {
                                    return Ok(PlannedEvaluationInput::Source {
                                        source: input.source,
                                        mode: input.mode,
                                        target: slot.target,
                                        receiver: slot.receiver,
                                    });
                                }
                                let target =
                                    allocate_value_slot(next_slot, slot_names, occupied_names)?;
                                let receiver = input
                                    .receiver
                                    .map(|(source, effects)| {
                                        if effects.is_inert() {
                                            Ok(PlannedReceiver::Stable { source })
                                        } else {
                                            Ok(PlannedReceiver::Captured {
                                                source,
                                                slot: allocate_value_slot(
                                                    next_slot,
                                                    slot_names,
                                                    occupied_names,
                                                )?,
                                            })
                                        }
                                    })
                                    .transpose()?;
                                source_slots
                                    .insert(input.source, PlannedSourceSlot { target, receiver });
                                Ok(PlannedEvaluationInput::Source {
                                    source: input.source,
                                    mode: input.mode,
                                    target,
                                    receiver,
                                })
                            },
                            |slot| {
                                Ok(PlannedEvaluationInput::Slot {
                                    slot: *slot,
                                    mode: input.mode,
                                })
                            },
                        )
                    })
                    .collect::<Result<Vec<_>, EvaluationError>>()?,
            })
        })
        .collect::<Result<Vec<_>, EvaluationError>>()?;
    Ok(EvaluationSchedule {
        steps,
        call_completion: None,
    })
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
    let steps = schedule.steps();
    let mut conditional = steps
        .iter()
        .enumerate()
        .filter(|(_, step)| matches!(step.operation, HostEvaluationOperation::Conditional(_)));
    let found = conditional.next()?;
    conditional.next().is_none().then_some(found)
}

/// Groups the owner's conditional-candidate values into whole conditional
/// operations (결정 17). A group that cannot form a complete operation is
/// downgraded to the expression boundary — never half-lowered.
pub(super) fn plan_conditional_operations(
    values: &mut [PlannedValue],
    tt_spans: &[SourceSpan],
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
    tt_spans: &[SourceSpan],
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
        member_index != conditional_index
            || values[*member].schedule.steps().len() != first_steps.len()
            || values[*member].schedule.steps()[conditional_index + 1..]
                != first_steps[conditional_index + 1..]
    }) {
        return Ok(None);
    }
    let Some(condition) = step.inputs.first().copied() else {
        return Ok(None);
    };
    let Some(facts) = step.conditional.clone() else {
        return Ok(None);
    };
    let overlaps_tt = |span: SourceSpan| tt_spans.iter().any(|tt| overlaps(span, *tt));
    let kind = match step.operation {
        HostEvaluationOperation::Conditional(ConditionalBranch::LogicalAndRight)
        | HostEvaluationOperation::Conditional(ConditionalBranch::LogicalOrRight)
        | HostEvaluationOperation::Conditional(ConditionalBranch::NullishRight) => {
            if members.len() != 1 {
                return Ok(None);
            }
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
            if conditional_index != 0 {
                return Ok(None);
            }
            let mut consequent = None;
            let mut alternate = None;
            for member in members {
                let value = &values[*member];
                let Some(member_facts) = &value.schedule.steps()[0].conditional else {
                    return Ok(None);
                };
                let side = match value.schedule.steps()[0].operation {
                    HostEvaluationOperation::Conditional(ConditionalBranch::Consequent) => {
                        &mut consequent
                    }
                    HostEvaluationOperation::Conditional(ConditionalBranch::Alternate) => {
                        &mut alternate
                    }
                    _ => return Ok(None),
                };
                if side.is_some() {
                    return Ok(None);
                }
                *side = Some((PlannedBranch::Value(value.expr), member_facts.skipped));
            }
            let fill = |taken: Option<(PlannedBranch, Option<SourceSpan>)>,
                        other: &Option<(PlannedBranch, Option<SourceSpan>)>|
             -> Option<PlannedBranch> {
                match taken {
                    Some((branch, _)) => Some(branch),
                    // The side with no tt value is the other member's
                    // skipped span — original source relocated into the
                    // branch, which must not contain tt of its own.
                    None => {
                        let (_, skipped) = other.as_ref()?;
                        let span = (*skipped)?;
                        (!overlaps_tt(span)).then_some(PlannedBranch::Source(span))
                    }
                }
            };
            let Some(consequent_branch) = fill(consequent, &alternate) else {
                return Ok(None);
            };
            let Some(alternate_branch) = fill(alternate, &consequent) else {
                return Ok(None);
            };
            PlannedConditionalKind::Ternary {
                consequent: consequent_branch,
                alternate: alternate_branch,
            }
        }
        HostEvaluationOperation::Conditional(ConditionalBranch::OptionalCallArgument(_)) => {
            if conditional_index != 0 {
                return Ok(None);
            }
            // A member callee calls through its captured receiver
            // (`callee.call(receiver, ...)`), which cannot carry explicit
            // type arguments.
            let member_callee = matches!(
                condition,
                PlannedEvaluationInput::Source {
                    mode: EvaluationInputMode::MemberReference,
                    ..
                }
            );
            if member_callee && facts.type_args.is_some() {
                return Ok(None);
            }
            let mut value_indices: HashMap<u32, ExprId> = HashMap::new();
            for member in members {
                let value = &values[*member];
                let HostEvaluationOperation::Conditional(ConditionalBranch::OptionalCallArgument(
                    index,
                )) = value.schedule.steps()[0].operation
                else {
                    return Ok(None);
                };
                if value_indices.insert(index, value.expr).is_some() {
                    return Ok(None);
                }
            }
            let last_value = *value_indices.keys().max().unwrap_or(&0);
            // Argument capture slots come from the members' planned inputs:
            // the schedule already assigned one to every argument that
            // evaluates before a value.
            // A captured argument answers with its slot; an inert one with
            // `None` — the rebuilt call inlines it, which is unobservable.
            let capture_of = |span: SourceSpan| {
                members.iter().find_map(|member| {
                    values[*member].schedule.steps()[0].inputs.iter().find_map(
                        |input| match input {
                            PlannedEvaluationInput::Source { source, target, .. }
                                if *source == span =>
                            {
                                Some(Some(*target))
                            }
                            PlannedEvaluationInput::Stable { source, .. } if *source == span => {
                                Some(None)
                            }
                            _ => None,
                        },
                    )
                })
            };
            let mut arguments = Vec::with_capacity(facts.operands.len());
            for (index, operand) in facts.operands.iter().enumerate() {
                let index = u32::try_from(index).map_err(|_| EvaluationError::IdOverflow)?;
                match value_indices.get(&index) {
                    Some(expr) => arguments.push(PlannedOperand::Value(*expr)),
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
            }
        }
        _ => return Ok(None),
    };
    let result = allocate_value_slot(next_slot, slot_names, occupied_names)?;
    let active_branch = matches!(
        &kind,
        PlannedConditionalKind::LogicalAnd
            | PlannedConditionalKind::LogicalOr
            | PlannedConditionalKind::Nullish
    )
    .then_some(facts.branch);
    Ok(Some(PlannedConditionalOperation {
        parent,
        result,
        kind,
        condition,
        values: members.iter().map(|member| values[*member].expr).collect(),
        active_branch,
        active_steps: first_steps[..conditional_index].to_vec(),
        outer: first_steps[conditional_index + 1..].to_vec(),
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
    tt_spans: &[SourceSpan],
    expr: ExprId,
    source: SourceSpan,
    context: &EvaluationContext,
    schedule: &EvaluationSchedule,
) -> TargetCapability {
    use ExpressionBoundaryReason as Reason;
    if matches!(
        context.owner,
        EvaluationOwner::ParameterInitializer | EvaluationOwner::ClassInitializer
    ) {
        return TargetCapability::ExpressionBoundary(Reason::OwnerTakesNoStatements);
    }
    if !core.has_statement_form(expr) {
        return TargetCapability::ExpressionBoundary(Reason::ValueHasNoStatementForm);
    }
    match context.owner_reach {
        OwnerReach::Same => {}
        OwnerReach::Repeated => {
            if !matches!(core.exprs[expr.index()], Expr::Decision(_)) {
                return TargetCapability::ExpressionBoundary(Reason::RepeatedInOwner);
            }
            let loop_steps = schedule
                .steps()
                .iter()
                .filter(|step| step.operation == HostEvaluationOperation::LoopTest)
                .count();
            if loop_steps != 1
                || schedule.steps().last().is_none_or(|step| {
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
                    return TargetCapability::ExpressionBoundary(Reason::ReferenceNotPreservable);
                }
                PlannedEvaluationInput::Source { .. }
                | PlannedEvaluationInput::Slot { .. }
                | PlannedEvaluationInput::Stable { .. } => {}
            }
        }
    }
    // A conditional step is lowerable only when the whole operation can be
    // owned as one region. Exactly one conditional boundary is allowed;
    // eager steps inside its active branch are kept there by the operation
    // plan. Anything else takes the boundary — never a promoted value under
    // the original syntax.
    let conditional_steps = steps
        .iter()
        .filter(|step| matches!(step.operation, HostEvaluationOperation::Conditional(_)))
        .count();
    if conditional_steps > 0 {
        let Some((_, step)) = whole_operation_step(schedule) else {
            return TargetCapability::ExpressionBoundary(
                Reason::ConditionalOperationNotStructurable,
            );
        };
        let structurable = step.conditional.as_ref().is_some_and(|facts| {
            facts.branch.start <= source.start && source.end <= facts.branch.end
        });
        if !structurable {
            return TargetCapability::ExpressionBoundary(
                Reason::ConditionalOperationNotStructurable,
            );
        }
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
            // The capture copies raw source bytes; a sibling tt node or
            // another capture inside them is lowered or relocated elsewhere.
            // An enclosing tt root is different: its structured lowering
            // owns this schedule and composes the captured source into it.
            if tt_spans.iter().any(|span| {
                overlaps(*capture, *span) && !(span.start <= source.start && source.end <= span.end)
            }) || captured.iter().any(|span| overlaps(*capture, *span))
            {
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
    let base = format!("$tt_v{}", slot.0);
    if occupied.insert(base.clone()) {
        return Ok(base);
    }
    let mut suffix = 1u32;
    loop {
        let candidate = format!("{base}_{suffix}");
        if occupied.insert(candidate.clone()) {
            return Ok(candidate);
        }
        suffix = suffix
            .checked_add(1)
            .ok_or(EvaluationError::GeneratedNameOverflow)?;
    }
}

pub(super) fn allocate_generated_name(
    base: &str,
    occupied: &mut HashSet<String>,
) -> Result<String, EvaluationError> {
    if occupied.insert(base.to_owned()) {
        return Ok(base.to_owned());
    }
    let mut suffix = 1u32;
    loop {
        let candidate = format!("{base}_{suffix}");
        if occupied.insert(candidate.clone()) {
            return Ok(candidate);
        }
        suffix = suffix
            .checked_add(1)
            .ok_or(EvaluationError::GeneratedNameOverflow)?;
    }
}
