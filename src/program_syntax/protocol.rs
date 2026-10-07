//! Evaluation protocol and source-coordinate mapping.

use super::*;

type ScopeKey = (ProjectedSpan, Option<ProjectedSpan>, bool);

pub(super) struct ProtocolScope {
    pub(super) owner: ProjectedSpan,
    pub(super) overlay: Option<ProjectedSpan>,
    pub(super) decision: bool,
}

#[derive(Clone, Copy)]
struct CompletionTrack {
    parent: ProjectedSpan,
    argument: ProjectedSpan,
    discarded: bool,
    type_args: Option<ProjectedSpan>,
    literal: bool,
    target: ProjectedSpan,
}

#[derive(Clone, Default)]
pub(super) struct OuterState {
    steps: crate::chain::Chain<HostEvaluationStep>,
    completion: Option<CompletionTrack>,
}

pub(super) fn evaluation_protocol(
    segments: &ProjectionSegments,
    value: ProjectedSpan,
    source_value: SourceSpan,
    frames: &crate::chain::Chain<std::rc::Rc<ProjectedProtocolFrame>>,
    scope: ProtocolScope,
    projection_only: &HashSet<ProjectedSpan>,
    links: &mut StepLinks,
) -> Result<HostEvaluationProtocol, ProgramSyntaxError> {
    let scope_key = (scope.owner, scope.overlay, scope.decision);
    let mut walked = Vec::new();
    let mut state = OuterState::default();
    let mut at = frames;
    while let (Some(frame), Some(rest)) = (at.first(), at.rest()) {
        let parent = frame.parent();
        if !projected_contains(scope.owner, parent)
            || scope
                .overlay
                .is_some_and(|ancestor| projected_contains(parent, ancestor))
        {
            break;
        }
        walked.push(at);
        if let Some(found) = links.outer.get(&(at.identity(), scope_key)) {
            state = found.clone();
            break;
        }
        at = rest;
    }
    let mut shared = true;
    for link in walked.into_iter().rev() {
        if shared {
            links
                .outer
                .entry((link.identity(), scope_key))
                .or_insert_with(|| state.clone());
        }
        let Some(frame) = link.first() else {
            continue;
        };
        if projection_only.contains(&frame.parent())
            || (matches!(&**frame, ProjectedProtocolFrame::LoopTest { .. }) && !scope.decision)
        {
            continue;
        }
        let selection = step_selection(frame, value);
        state.completion = advance_completion(state.completion, frame, value);
        let Some(selection) = selection else {
            continue;
        };
        state.steps = link_step(
            segments,
            value,
            source_value,
            frame,
            selection,
            std::mem::take(&mut state.steps),
            links,
        )?;
        if selection == StepSelection::LoopTest {
            shared = false;
        }
    }
    // The innermost call whose final non-spread argument contains the value.
    // Earlier arguments evaluate before the value and are captured by the
    // schedule; an argument after the value would have to run inside the
    // dispatch, so only the final position completes.
    //
    // The argument may be wider than the value — `f({ k: match … })` — and
    // then the authored text around the value has to be re-emitted inside
    // each arm. Whether that is legal depends on what that text evaluates,
    // which is the schedule's answer, not syntax's: this records the
    // argument's extent and leaves the decision to target planning.
    let call_completion = state
        .completion
        .map(|track| {
            Ok::<_, ProgramSyntaxError>(CallCompletionFacts {
                call: map_structural_span(segments, track.parent)?,
                argument: map_structural_span(segments, track.argument)?,
                literal_positions: track.literal && track.target == value,
                consumed: !track.discarded,
                type_args: track
                    .type_args
                    .map(|span| map_evaluation_span(segments, span))
                    .transpose()?,
            })
        })
        .transpose()?;
    Ok(HostEvaluationProtocol {
        steps: state.steps,
        call_completion,
    })
}

fn link_step(
    segments: &ProjectionSegments,
    value: ProjectedSpan,
    source_value: SourceSpan,
    frame: &std::rc::Rc<ProjectedProtocolFrame>,
    selection: StepSelection,
    steps: crate::chain::Chain<HostEvaluationStep>,
    links: &mut StepLinks,
) -> Result<crate::chain::Chain<HostEvaluationStep>, ProgramSyntaxError> {
    let key = (
        steps.identity(),
        std::rc::Rc::as_ptr(frame) as usize,
        selection,
        (selection == StepSelection::LoopTest).then_some(value),
    );
    if let Some(link) = links.links.get(&key) {
        return Ok(link.clone());
    }
    crate::work::tick("protocol step links");
    let siblings = (key.0, key.1);
    let earlier = match selection {
        StepSelection::Position(position) => links
            .siblings
            .get(&siblings)
            .filter(|(earlier, _)| *earlier <= position),
        _ => None,
    };
    let step = protocol_step(
        segments,
        value,
        source_value,
        frame,
        selection,
        earlier.map(|(position, inputs)| (*position, inputs)),
    )?;
    if let StepSelection::Position(position) = selection {
        links
            .siblings
            .insert(siblings, (position, step.inputs.clone()));
    }
    let link = crate::chain::Chain::cons(step, steps);
    links.links.insert(key, link.clone());
    Ok(link)
}

/// Whether the path from the completed call's final argument down to the
/// value runs only through whole object- and array-literal positions.
///
/// That is what makes the text around the value re-emittable: a literal
/// position holds one complete expression, so replacing it with an arm's
/// value leaves the rest of the literal meaning what it meant. Anything else
/// between them — a cast, an operator, a call — binds to the value, and
/// re-emitting its text around a different expression would rebind it.
fn advance_completion(
    track: Option<CompletionTrack>,
    frame: &ProjectedProtocolFrame,
    value: ProjectedSpan,
) -> Option<CompletionTrack> {
    if let ProjectedProtocolFrame::Call {
        discarded,
        parent,
        callee: Some(_),
        arguments,
        type_args,
        optional: None,
        spread_free: true,
        ..
    } = frame
        && let Some((argument, false, _)) = arguments.last()
        && projected_contains(*argument, value)
    {
        return Some(CompletionTrack {
            parent: *parent,
            argument: *argument,
            discarded: *discarded,
            type_args: *type_args,
            literal: true,
            target: *argument,
        });
    }
    let mut track = track?;
    if track.literal {
        track.literal = false;
        if let ProjectedProtocolFrame::Ordered {
            parent,
            positions,
            kind: OrderedEvaluationKind::Object | OrderedEvaluationKind::Array,
            spread_free: true,
        } = frame
            && *parent == track.target
            && let Some(position) = position_holding(positions, |(span, ..)| *span, value)
        {
            track.literal = true;
            track.target = positions[position].0;
        }
    }
    Some(track)
}

/// The projected shape of one conditional operation, before span mapping.
pub(super) struct ProjectedConditionalFacts {
    branch: ProjectedSpan,
    skipped: Option<ProjectedSpan>,
    operands: Vec<(ProjectedSpan, bool)>,
    type_args: Option<ProjectedSpan>,
    optional_test: Option<OptionalCallTest>,
}

#[derive(Default)]
pub(super) struct StepLinks {
    outer: HashMap<(usize, ScopeKey), OuterState>,
    links: HashMap<
        (usize, usize, StepSelection, Option<ProjectedSpan>),
        crate::chain::Chain<HostEvaluationStep>,
    >,
    siblings: HashMap<(usize, usize), (usize, crate::chain::Segments<HostEvaluationInput>)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum StepSelection {
    Position(usize),
    LogicalAssignmentRight,
    AssignmentRight,
    BinaryLeft,
    BinaryRight,
    Consequent,
    Alternate,
    Callee,
    Object,
    Property,
    SuspendedValue,
    LoopTest,
}

pub(super) fn step_selection(
    frame: &ProjectedProtocolFrame,
    value: ProjectedSpan,
) -> Option<StepSelection> {
    match frame {
        ProjectedProtocolFrame::Ordered { positions, .. } => {
            position_holding(positions, |(span, ..)| *span, value).map(StepSelection::Position)
        }
        ProjectedProtocolFrame::Assignment {
            operator, right, ..
        } if projected_contains(*right, value) && operator.may_short_circuit() => {
            Some(StepSelection::LogicalAssignmentRight)
        }
        ProjectedProtocolFrame::Assignment { right, .. } if projected_contains(*right, value) => {
            Some(StepSelection::AssignmentRight)
        }
        ProjectedProtocolFrame::Binary {
            left: (left, _), ..
        } if projected_contains(*left, value) => Some(StepSelection::BinaryLeft),
        ProjectedProtocolFrame::Binary { right, .. } if projected_contains(*right, value) => {
            Some(StepSelection::BinaryRight)
        }
        ProjectedProtocolFrame::Conditional { consequent, .. }
            if projected_contains(*consequent, value) =>
        {
            Some(StepSelection::Consequent)
        }
        ProjectedProtocolFrame::Conditional { alternate, .. }
            if projected_contains(*alternate, value) =>
        {
            Some(StepSelection::Alternate)
        }
        ProjectedProtocolFrame::Call {
            callee: Some(callee),
            ..
        } if projected_contains(*callee, value) => Some(StepSelection::Callee),
        ProjectedProtocolFrame::Call { arguments, .. } => {
            position_holding(arguments, |(argument, ..)| *argument, value)
                .map(StepSelection::Position)
        }
        ProjectedProtocolFrame::Member {
            object: (object, _),
            ..
        } if projected_contains(*object, value) => Some(StepSelection::Object),
        ProjectedProtocolFrame::Member {
            property: Some(property),
            ..
        } if projected_contains(*property, value) => Some(StepSelection::Property),
        ProjectedProtocolFrame::Construct { callee, .. } if projected_contains(*callee, value) => {
            Some(StepSelection::Callee)
        }
        ProjectedProtocolFrame::Construct { arguments, .. } => {
            position_holding(arguments, |(argument, ..)| *argument, value)
                .map(StepSelection::Position)
        }
        ProjectedProtocolFrame::TaggedTemplate { tag, .. } if projected_contains(*tag, value) => {
            Some(StepSelection::Callee)
        }
        ProjectedProtocolFrame::TaggedTemplate { expressions, .. }
        | ProjectedProtocolFrame::Template { expressions, .. } => {
            position_holding(expressions, |(span, _)| *span, value).map(StepSelection::Position)
        }
        ProjectedProtocolFrame::Jsx { expressions, .. } => {
            position_holding(expressions, |(span, ..)| *span, value).map(StepSelection::Position)
        }
        ProjectedProtocolFrame::Suspend {
            value: Some(argument),
            ..
        } if projected_contains(*argument, value) => Some(StepSelection::SuspendedValue),
        ProjectedProtocolFrame::LoopTest { test, .. } if projected_contains(*test, value) => {
            Some(StepSelection::LoopTest)
        }
        _ => None,
    }
}

pub(super) fn protocol_step(
    segments: &ProjectionSegments,
    value: ProjectedSpan,
    source_value: SourceSpan,
    frame: &ProjectedProtocolFrame,
    selection: StepSelection,
    earlier: Option<(usize, &crate::chain::Segments<HostEvaluationInput>)>,
) -> Result<HostEvaluationStep, ProgramSyntaxError> {
    let (first, base) = match earlier {
        Some((position, inputs)) => (position, inputs.clone()),
        None => (0, crate::chain::Segments::new()),
    };
    let lead = earlier.is_none();
    let mut conditional: Option<ProjectedConditionalFacts> = None;
    let mut loop_test: Option<(
        LoopTestKind,
        ProjectedSpan,
        ProjectedSpan,
        Option<ProjectedSpan>,
    )> = None;
    let (parent, operation, inputs) = match (frame, selection) {
        (
            ProjectedProtocolFrame::Ordered {
                parent,
                positions,
                kind,
                ..
            },
            StepSelection::Position(position),
        ) => {
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let operation = HostEvaluationOperation::Eager(match kind {
                OrderedEvaluationKind::Array => EagerPosition::ArrayElement(index),
                OrderedEvaluationKind::Object => EagerPosition::ObjectEvaluation(index),
                OrderedEvaluationKind::Sequence => EagerPosition::SequenceElement(index),
                OrderedEvaluationKind::Unary => EagerPosition::UnaryOperand,
            });
            let inputs = positions[first..position]
                .iter()
                .copied()
                .map(|(span, effects, mode)| (span, mode, None, effects))
                .collect();
            (*parent, operation, inputs)
        }
        (
            ProjectedProtocolFrame::Assignment {
                parent,
                operator,
                target,
                parts,
                discarded,
                right,
                ..
            },
            StepSelection::LogicalAssignmentRight,
        ) => {
            let branch = match operator {
                AssignOp::AndAssign => LogicalAssignment::And,
                AssignOp::OrAssign => LogicalAssignment::Or,
                _ => LogicalAssignment::Nullish,
            };
            conditional = Some(ProjectedConditionalFacts {
                branch: *right,
                skipped: None,
                operands: Vec::new(),
                type_args: None,
                optional_test: None,
            });
            (
                *parent,
                HostEvaluationOperation::Conditional(ConditionalBranch::LogicalAssignmentRight {
                    operator: branch,
                    consumed: !discarded,
                }),
                vec![(
                    *target,
                    EvaluationInputMode::LogicalAssignmentTarget,
                    Some(*parts),
                    Effects::ANY,
                )],
            )
        }
        (
            ProjectedProtocolFrame::Assignment {
                parent,
                operator,
                target,
                reference,
                ..
            },
            StepSelection::AssignmentRight,
        ) => (
            *parent,
            HostEvaluationOperation::Eager(EagerPosition::AssignmentRight),
            reference
                .iter()
                .map(|(span, effects)| (*span, EvaluationInputMode::Value, None, *effects))
                .chain(
                    (operator.to_update().is_some() && !operator.may_short_circuit()).then(|| {
                        (
                            *target,
                            EvaluationInputMode::CompoundAssignmentTarget {
                                operator: operator.as_str(),
                            },
                            None,
                            Effects::ANY,
                        )
                    }),
                )
                .collect(),
        ),
        (ProjectedProtocolFrame::Binary { parent, .. }, StepSelection::BinaryLeft) => (
            *parent,
            HostEvaluationOperation::Eager(EagerPosition::BinaryLeft),
            Vec::new(),
        ),
        (
            ProjectedProtocolFrame::Binary {
                parent,
                operator,
                left,
                right,
            },
            StepSelection::BinaryRight,
        ) => {
            let operation = match operator {
                BinaryOp::LogicalAnd => {
                    HostEvaluationOperation::Conditional(ConditionalBranch::LogicalAndRight)
                }
                BinaryOp::LogicalOr => {
                    HostEvaluationOperation::Conditional(ConditionalBranch::LogicalOrRight)
                }
                BinaryOp::NullishCoalescing => {
                    HostEvaluationOperation::Conditional(ConditionalBranch::NullishRight)
                }
                _ => HostEvaluationOperation::Eager(EagerPosition::BinaryRight),
            };
            if matches!(operation, HostEvaluationOperation::Conditional(_)) {
                conditional = Some(ProjectedConditionalFacts {
                    branch: *right,
                    skipped: None,
                    operands: Vec::new(),
                    type_args: None,
                    optional_test: None,
                });
            }
            let (left, effects) = *left;
            (
                *parent,
                operation,
                vec![(left, EvaluationInputMode::Value, None, effects)],
            )
        }
        (
            ProjectedProtocolFrame::Conditional {
                parent,
                test,
                consequent,
                alternate,
            },
            StepSelection::Consequent,
        ) => {
            conditional = Some(ProjectedConditionalFacts {
                branch: *consequent,
                skipped: Some(*alternate),
                operands: Vec::new(),
                type_args: None,
                optional_test: None,
            });
            let (test, effects) = *test;
            (
                *parent,
                HostEvaluationOperation::Conditional(ConditionalBranch::Consequent),
                vec![(test, EvaluationInputMode::Value, None, effects)],
            )
        }
        (
            ProjectedProtocolFrame::Conditional {
                parent,
                test,
                consequent,
                alternate,
            },
            StepSelection::Alternate,
        ) => {
            conditional = Some(ProjectedConditionalFacts {
                branch: *alternate,
                skipped: Some(*consequent),
                operands: Vec::new(),
                type_args: None,
                optional_test: None,
            });
            let (test, effects) = *test;
            (
                *parent,
                HostEvaluationOperation::Conditional(ConditionalBranch::Alternate),
                vec![(test, EvaluationInputMode::Value, None, effects)],
            )
        }
        (
            ProjectedProtocolFrame::Call {
                parent, optional, ..
            },
            StepSelection::Callee,
        ) => (
            *parent,
            HostEvaluationOperation::Reference(if optional.is_some() {
                ReferencePosition::OptionalCallCallee
            } else {
                ReferencePosition::CallCallee
            }),
            Vec::new(),
        ),
        (
            ProjectedProtocolFrame::Call {
                parent,
                callee,
                callee_mode,
                callee_reference,
                arguments,
                type_args,
                optional,
                ..
            },
            StepSelection::Position(position),
        ) => {
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let operation = if let Some(test) = optional {
                conditional = Some(ProjectedConditionalFacts {
                    branch: arguments[position].0,
                    skipped: None,
                    operands: arguments
                        .iter()
                        .map(|(span, spread, _)| (*span, *spread))
                        .collect(),
                    type_args: *type_args,
                    optional_test: Some(*test),
                });
                HostEvaluationOperation::Conditional(ConditionalBranch::OptionalCallArgument(index))
            } else {
                HostEvaluationOperation::Eager(EagerPosition::CallArgument(index))
            };
            let inputs = callee
                .iter()
                .filter(|_| lead)
                .copied()
                .map(|callee| (callee, *callee_mode, *callee_reference, Effects::ANY))
                .chain(
                    arguments[first..position]
                        .iter()
                        .map(|(argument, spread, effects)| {
                            let mode = if *spread {
                                EvaluationInputMode::SpreadElement
                            } else {
                                EvaluationInputMode::Value
                            };
                            (*argument, mode, None, *effects)
                        }),
                )
                .collect();
            (*parent, operation, inputs)
        }
        (ProjectedProtocolFrame::Member { parent, .. }, StepSelection::Object) => (
            *parent,
            HostEvaluationOperation::Reference(ReferencePosition::MemberObject),
            Vec::new(),
        ),
        (
            ProjectedProtocolFrame::Member {
                parent,
                object: (object, effects),
                ..
            },
            StepSelection::Property,
        ) => (
            *parent,
            HostEvaluationOperation::Reference(ReferencePosition::MemberProperty),
            vec![(*object, EvaluationInputMode::Value, None, *effects)],
        ),
        (ProjectedProtocolFrame::Construct { parent, .. }, StepSelection::Callee) => (
            *parent,
            HostEvaluationOperation::Reference(ReferencePosition::ConstructorCallee),
            Vec::new(),
        ),
        (
            ProjectedProtocolFrame::Construct {
                parent,
                callee,
                arguments,
            },
            StepSelection::Position(position),
        ) => {
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let inputs = std::iter::once((*callee, EvaluationInputMode::Value, None, Effects::ANY))
                .filter(|_| lead)
                .chain(
                    arguments[first..position]
                        .iter()
                        .map(|(argument, spread, effects)| {
                            let mode = if *spread {
                                EvaluationInputMode::SpreadElement
                            } else {
                                EvaluationInputMode::Value
                            };
                            (*argument, mode, None, *effects)
                        }),
                )
                .collect();
            (
                *parent,
                HostEvaluationOperation::Eager(EagerPosition::ConstructArgument(index)),
                inputs,
            )
        }
        (ProjectedProtocolFrame::TaggedTemplate { parent, .. }, StepSelection::Callee) => (
            *parent,
            HostEvaluationOperation::Reference(ReferencePosition::TaggedTemplateTag),
            Vec::new(),
        ),
        (
            ProjectedProtocolFrame::TaggedTemplate {
                parent,
                tag,
                tag_mode,
                tag_reference,
                expressions,
            },
            StepSelection::Position(position),
        ) => {
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let inputs =
                std::iter::once((*tag, *tag_mode, *tag_reference, Effects::ANY))
                    .filter(|_| lead)
                    .chain(expressions[first..position].iter().copied().map(
                        |(expression, effects)| {
                            (expression, EvaluationInputMode::Value, None, effects)
                        },
                    ))
                    .collect();
            (
                *parent,
                HostEvaluationOperation::Eager(EagerPosition::TemplateInterpolation(index)),
                inputs,
            )
        }
        (
            ProjectedProtocolFrame::Template {
                parent,
                expressions,
            },
            StepSelection::Position(position),
        ) => {
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let inputs = expressions[first..position]
                .iter()
                .copied()
                .map(|(expression, effects)| {
                    (
                        expression,
                        EvaluationInputMode::TemplateSubstitution,
                        None,
                        effects,
                    )
                })
                .collect();
            (
                *parent,
                HostEvaluationOperation::Eager(EagerPosition::TemplateInterpolation(index)),
                inputs,
            )
        }
        (
            ProjectedProtocolFrame::Jsx {
                parent,
                expressions,
            },
            StepSelection::Position(position),
        ) => {
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let inputs = expressions[first..position]
                .iter()
                .copied()
                .map(|(expression, effects, child)| {
                    (
                        expression,
                        if child {
                            EvaluationInputMode::JsxChildValue
                        } else {
                            EvaluationInputMode::Value
                        },
                        None,
                        effects,
                    )
                })
                .collect();
            (
                *parent,
                HostEvaluationOperation::Eager(EagerPosition::JsxExpression(index)),
                inputs,
            )
        }
        (ProjectedProtocolFrame::Suspend { parent, kind, .. }, StepSelection::SuspendedValue) => {
            (*parent, HostEvaluationOperation::Suspend(*kind), Vec::new())
        }
        (
            ProjectedProtocolFrame::LoopTest {
                parent,
                kind,
                test,
                body,
                update,
            },
            StepSelection::LoopTest,
        ) => {
            loop_test = Some((*kind, *test, *body, *update));
            (*parent, HostEvaluationOperation::LoopTest, Vec::new())
        }
        _ => crate::ice::bug!("a step selection does not fit its frame"),
    };
    let parent = if operation == HostEvaluationOperation::LoopTest {
        source_value
    } else {
        map_evaluation_span(segments, parent)?
    };
    let inputs = inputs
        .into_iter()
        .map(|(source, mode, reference, effects)| {
            let reference: ProjectedMemberReference = reference.unwrap_or_default();
            let part = |part: Option<ProjectedReferencePart>| {
                part.map(|part| {
                    Ok(HostReferencePart {
                        source: map_evaluation_span(segments, part.span)?,
                        effects: part.effects,
                        read_at_call: part.read_at_call,
                    })
                })
                .transpose()
            };
            Ok(HostEvaluationInput {
                source: map_evaluation_span(segments, source)?,
                mode,
                receiver: part(reference.receiver)?,
                key: part(reference.key)?,
                effects: Effects {
                    requires_reference: matches!(
                        mode,
                        EvaluationInputMode::DirectReference | EvaluationInputMode::MemberReference
                    ),
                    ..effects
                },
            })
        })
        .collect::<Result<Vec<_>, ProgramSyntaxError>>()?;
    let inputs = crate::chain::Segments::extend(base, inputs);
    let map_operand = |span| map_evaluation_span(segments, span);
    let conditional = conditional
        .map(|facts| {
            Ok(ConditionalFacts {
                // The branch holds the tt value, so it has no contiguous
                // source mapping; its bounds map endpoint-by-endpoint.
                branch: map_operand(facts.branch)?,
                skipped: facts.skipped.map(map_operand).transpose()?,
                operands: facts
                    .operands
                    .into_iter()
                    .map(|(span, spread)| {
                        Ok(ConditionalOperand {
                            span: map_operand(span)?,
                            spread,
                        })
                    })
                    .collect::<Result<Vec<_>, ProgramSyntaxError>>()?,
                type_args: facts
                    .type_args
                    .map(|span| map_evaluation_span(segments, span))
                    .transpose()?,
                optional_test: facts.optional_test,
            })
        })
        .transpose()?;
    let loop_test = loop_test
        .map(|(kind, test, body, update)| {
            Ok(LoopTestFacts {
                kind,
                test: if test == value {
                    source_value
                } else {
                    map_structural_span(segments, test)?
                },
                body: map_structural_span(segments, body)?,
                update: update
                    .map(|span| map_structural_span(segments, span))
                    .transpose()?,
            })
        })
        .transpose()?;
    Ok(HostEvaluationStep {
        parent,
        operation,
        inputs,
        conditional,
        loop_test,
    })
}

pub(super) fn map_evaluation_span(
    segments: &ProjectionSegments,
    projected: ProjectedSpan,
) -> Result<SourceSpan, ProgramSyntaxError> {
    source_span_for_projection(segments, projected).ok_or(
        ProgramSyntaxError::UnmappedEvaluationSpan {
            start: projected.start.0,
            end: projected.end.0,
        },
    )
}

pub(super) fn map_structural_span(
    segments: &ProjectionSegments,
    projected: ProjectedSpan,
) -> Result<SourceSpan, ProgramSyntaxError> {
    if let Some(span) = source_span_for_projection(segments, projected) {
        return Ok(span);
    }
    let start = segments
        .iter()
        .find(|segment| {
            projected.start <= segment.projected.start && segment.projected.start < projected.end
        })
        .map(|segment| segment.source.start);
    let end = segments
        .iter()
        .rev()
        .find(|segment| {
            projected.start < segment.projected.end && segment.projected.end <= projected.end
        })
        .map(|segment| segment.source.end);
    match start.zip(end) {
        Some((start, end)) if start <= end => Ok(SourceSpan { start, end }),
        _ => Err(ProgramSyntaxError::UnmappedEvaluationSpan {
            start: projected.start.0,
            end: projected.end.0,
        }),
    }
}

pub(super) fn projected_contains(container: ProjectedSpan, value: ProjectedSpan) -> bool {
    container.start <= value.start && value.end <= container.end
}

/// Which link of its optional chain a call inside the chain is skipped at
/// ([`OptionalCallTest`]): `own_link` is whether the call itself is written
/// `?.(`.
pub(super) fn optional_call_test(own_link: bool, callee: &swc_ecma_ast::Expr) -> OptionalCallTest {
    use swc_ecma_ast::{Expr as SwcExpr, OptChainBase};

    let member_link = match callee {
        SwcExpr::OptChain(chain) => match &*chain.base {
            OptChainBase::Member(_) => Some(chain.optional),
            OptChainBase::Call(_) => None,
        },
        _ => None,
    };
    match (own_link, member_link) {
        (_, Some(false)) => OptionalCallTest::Inner,
        (true, _) => OptionalCallTest::Callee,
        (false, Some(true)) => OptionalCallTest::Receiver,
        (false, None) => OptionalCallTest::Inner,
    }
}

/// How a call reads its callee, and for a member callee the parts of its
/// reference evaluated before the arguments: the object, then the computed
/// key ([`member_reference`], [`super_reference`]).
pub(super) fn call_callee_mode(
    expression: &swc_ecma_ast::Expr,
) -> (EvaluationInputMode, [Option<&swc_ecma_ast::Expr>; 2]) {
    crate::stack::grow(|| call_callee_mode_grown(expression))
}

fn call_callee_mode_grown(
    expression: &swc_ecma_ast::Expr,
) -> (EvaluationInputMode, [Option<&swc_ecma_ast::Expr>; 2]) {
    use swc_ecma_ast::{Expr as SwcExpr, OptChainBase};

    match expression {
        SwcExpr::Member(member) => (
            EvaluationInputMode::MemberReference,
            member_reference(member),
        ),
        SwcExpr::SuperProp(member) => (
            EvaluationInputMode::MemberReference,
            super_reference(member),
        ),
        SwcExpr::OptChain(chain) => match &*chain.base {
            OptChainBase::Member(member) => (
                EvaluationInputMode::MemberReference,
                member_reference(member),
            ),
            OptChainBase::Call(_) => (EvaluationInputMode::DirectReference, [None, None]),
        },
        SwcExpr::Paren(paren) => call_callee_mode(&paren.expr),
        SwcExpr::TsAs(expression) => call_callee_mode(&expression.expr),
        SwcExpr::TsTypeAssertion(expression) => call_callee_mode(&expression.expr),
        SwcExpr::TsNonNull(expression) => call_callee_mode(&expression.expr),
        SwcExpr::TsInstantiation(expression) => call_callee_mode(&expression.expr),
        SwcExpr::TsSatisfies(expression) => call_callee_mode(&expression.expr),
        _ => (EvaluationInputMode::DirectReference, [None, None]),
    }
}

/// The parts of an assignment target's reference that evaluate before the
/// right operand and are captured there: a member target's object, then its
/// computed key. An identifier target resolves a binding, which evaluates
/// nothing, and a destructuring pattern is evaluated after the right
/// operand.
///
/// A part that is an identifier or `this` is read again where the
/// assignment is performed, as TypeScript's own down-level transforms read
/// a simple-copiable operand (`isSimpleCopiableExpression`): capturing it
/// would write through a generated name, and TypeScript narrows an assigned
/// reference such as `state.value`, and infers a class property from a
/// constructor's `this.value = ...`, only when the assignment names it.
pub(super) fn assignment_reference(
    target: &swc_ecma_ast::AssignTarget,
) -> impl Iterator<Item = &swc_ecma_ast::Expr> {
    target_reference(target)
        .into_iter()
        .flatten()
        .filter(|part| !simple_copiable(part))
}

/// An identifier or `this`, possibly parenthesized: what TypeScript's
/// down-level transforms read again instead of copying
/// (`isSimpleCopiableExpression`).
pub(super) fn simple_copiable(expression: &swc_ecma_ast::Expr) -> bool {
    matches!(
        peel_parens(expression),
        swc_ecma_ast::Expr::Ident(_) | swc_ecma_ast::Expr::This(_)
    )
}

pub(super) fn read_at_call(callee: &swc_ecma_ast::Callee) -> bool {
    match callee {
        swc_ecma_ast::Callee::Super(_) | swc_ecma_ast::Callee::Import(_) => true,
        swc_ecma_ast::Callee::Expr(expression) => match peel_parens(expression) {
            swc_ecma_ast::Expr::Ident(ident) => &*ident.sym == "eval",
            swc_ecma_ast::Expr::SuperProp(member) => match &member.prop {
                swc_ecma_ast::SuperProp::Ident(_) => true,
                swc_ecma_ast::SuperProp::Computed(computed) => {
                    super::simple_copiable(&computed.expr)
                }
            },
            _ => false,
        },
    }
}

fn peel_parens(expression: &swc_ecma_ast::Expr) -> &swc_ecma_ast::Expr {
    crate::stack::grow(|| peel_parens_grown(expression))
}

fn peel_parens_grown(expression: &swc_ecma_ast::Expr) -> &swc_ecma_ast::Expr {
    match expression {
        swc_ecma_ast::Expr::Paren(inner) => peel_parens(&inner.expr),
        _ => expression,
    }
}

pub(super) fn target_reference(
    target: &swc_ecma_ast::AssignTarget,
) -> [Option<&swc_ecma_ast::Expr>; 2] {
    use swc_ecma_ast::{AssignTarget, SimpleAssignTarget};

    let AssignTarget::Simple(target) = target else {
        return [None, None];
    };
    match target {
        SimpleAssignTarget::Member(member) => member_reference(member),
        SimpleAssignTarget::SuperProp(member) => super_reference(member),
        SimpleAssignTarget::Paren(expression) => expression_reference(&expression.expr),
        SimpleAssignTarget::TsAs(expression) => expression_reference(&expression.expr),
        SimpleAssignTarget::TsSatisfies(expression) => expression_reference(&expression.expr),
        SimpleAssignTarget::TsNonNull(expression) => expression_reference(&expression.expr),
        SimpleAssignTarget::TsTypeAssertion(expression) => expression_reference(&expression.expr),
        SimpleAssignTarget::TsInstantiation(expression) => expression_reference(&expression.expr),
        SimpleAssignTarget::Ident(_)
        | SimpleAssignTarget::OptChain(_)
        | SimpleAssignTarget::Invalid(_) => [None, None],
    }
}

fn expression_reference(expression: &swc_ecma_ast::Expr) -> [Option<&swc_ecma_ast::Expr>; 2] {
    crate::stack::grow(|| expression_reference_grown(expression))
}

fn expression_reference_grown(expression: &swc_ecma_ast::Expr) -> [Option<&swc_ecma_ast::Expr>; 2] {
    use swc_ecma_ast::Expr as SwcExpr;

    match expression {
        SwcExpr::Member(member) => member_reference(member),
        SwcExpr::SuperProp(member) => super_reference(member),
        SwcExpr::Paren(expression) => expression_reference(&expression.expr),
        SwcExpr::TsAs(expression) => expression_reference(&expression.expr),
        SwcExpr::TsSatisfies(expression) => expression_reference(&expression.expr),
        SwcExpr::TsNonNull(expression) => expression_reference(&expression.expr),
        SwcExpr::TsTypeAssertion(expression) => expression_reference(&expression.expr),
        SwcExpr::TsInstantiation(expression) => expression_reference(&expression.expr),
        _ => [None, None],
    }
}

fn member_reference(member: &swc_ecma_ast::MemberExpr) -> [Option<&swc_ecma_ast::Expr>; 2] {
    [
        Some(&member.obj),
        match &member.prop {
            MemberProp::Computed(computed) => Some(&computed.expr),
            MemberProp::Ident(_) | MemberProp::PrivateName(_) => None,
        },
    ]
}

fn super_reference(member: &swc_ecma_ast::SuperPropExpr) -> [Option<&swc_ecma_ast::Expr>; 2] {
    [
        None,
        match &member.prop {
            swc_ecma_ast::SuperProp::Computed(computed) => Some(&computed.expr),
            swc_ecma_ast::SuperProp::Ident(_) => None,
        },
    ]
}

pub(super) fn operand_span(
    expression: &swc_ecma_ast::Expr,
    source_start: HostOrigin,
    placeholders: &HashSet<ProjectedSpan>,
    segments: &ProjectionSegments,
) -> ProjectedSpan {
    let mut inner = expression;
    while let swc_ecma_ast::Expr::Paren(paren) = inner {
        let within = projected_span(paren.expr.span(), source_start);
        if placeholders.contains(&projected_span(paren.span, source_start))
            || source_span_for_projection(segments, within).is_none()
        {
            break;
        }
        inner = &paren.expr;
        if placeholders.contains(&within) {
            break;
        }
    }
    projected_span(inner.span(), source_start)
}

pub(super) fn reference_value_span(expression: &swc_ecma_ast::Expr) -> swc_common::Span {
    crate::stack::grow(|| reference_value_span_grown(expression))
}

fn reference_value_span_grown(expression: &swc_ecma_ast::Expr) -> swc_common::Span {
    use swc_ecma_ast::Expr as SwcExpr;

    match expression {
        SwcExpr::Paren(expression) => reference_value_span(&expression.expr),
        SwcExpr::TsAs(expression) => reference_value_span(&expression.expr),
        SwcExpr::TsTypeAssertion(expression) => reference_value_span(&expression.expr),
        SwcExpr::TsNonNull(expression) => reference_value_span(&expression.expr),
        SwcExpr::TsInstantiation(expression) => reference_value_span(&expression.expr),
        SwcExpr::TsSatisfies(expression) => reference_value_span(&expression.expr),
        _ => expression.span(),
    }
}

pub(super) fn projected_span(span: swc_common::Span, source_start: HostOrigin) -> ProjectedSpan {
    ProjectedSpan {
        start: ProjectedByte(source_start.byte(span.lo)),
        end: ProjectedByte(source_start.byte(span.hi)),
    }
}

/// The source byte a projected byte was copied from, or `None` when no
/// copied segment owns it — a byte of a placeholder this compiler wrote.
pub(super) fn source_byte_for_projection(
    segments: &[ProjectionSourceSegment],
    projected: ProjectedByte,
) -> Option<usize> {
    segments.iter().find_map(|segment| {
        if !(segment.projected.start <= projected && projected < segment.projected.end) {
            return None;
        }
        match segment.kind {
            ProjectionSegmentKind::Copied => {
                Some(segment.source.start + projected.0 - segment.projected.start.0)
            }
            ProjectionSegmentKind::SourceBoundary | ProjectionSegmentKind::AutomaticSemicolon => {
                Some(segment.source.start)
            }
            ProjectionSegmentKind::Placeholder => None,
        }
    })
}

/// Whether the projected span is text the author wrote, inside one copied
/// segment, rather than a placeholder the projection wrote for a tt value
/// or a piped value.
pub(super) fn authored_span(segments: &ProjectionSegments, projected: ProjectedSpan) -> bool {
    segments
        .starting_at(projected.start)
        .into_iter()
        .chain(segments.containing(projected.start))
        .map(|index| &segments[index])
        .any(|segment| {
            segment.kind == ProjectionSegmentKind::Copied
                && segment.projected.start <= projected.start
                && projected.end <= segment.projected.end
        })
}

/// The parts of a member callee's reference as the protocol records them.
/// A part that is an authored identifier or `this` is read again at the
/// call ([`HostReferencePart::read_at_call`]).
pub(super) fn projected_member_reference(
    parts: [Option<&swc_ecma_ast::Expr>; 2],
    source_start: HostOrigin,
    segments: &ProjectionSegments,
) -> ProjectedMemberReference {
    let part = |part: Option<&swc_ecma_ast::Expr>| {
        part.map(|expression| {
            let span = projected_span(expression.span(), source_start);
            ProjectedReferencePart {
                span,
                effects: expression_effects(expression),
                read_at_call: simple_copiable(expression) && authored_span(segments, span),
            }
        })
    };
    let [receiver, key] = parts;
    ProjectedMemberReference {
        receiver: part(receiver),
        key: part(key),
    }
}

pub(super) fn source_span_for_projection(
    segments: &ProjectionSegments,
    projected: ProjectedSpan,
) -> Option<SourceSpan> {
    segments.mapped(projected, || look_up_source_span(segments, projected))
}

fn look_up_source_span(
    segments: &ProjectionSegments,
    projected: ProjectedSpan,
) -> Option<SourceSpan> {
    crate::work::tick("projection span lookups");
    if let Some(segment) = segments
        .starting_at(projected.start)
        .into_iter()
        .map(|index| &segments[index])
        .find(|segment| {
            segment.kind != ProjectionSegmentKind::SourceBoundary && segment.projected == projected
        })
    {
        return Some(segment.source);
    }
    let start = in_segment_order(
        segments.starting_at(projected.start),
        segments.containing(projected.start),
    )
    .map(|index| &segments[index])
    .find_map(|segment| {
        if segment.kind != ProjectionSegmentKind::SourceBoundary
            && projected.start == segment.projected.start
        {
            Some(segment.source.start)
        } else if segment.projected.start < projected.start
            && projected.start < segment.projected.end
            && segment.kind == ProjectionSegmentKind::Copied
        {
            Some(segment.source.start + projected.start.0 - segment.projected.start.0)
        } else {
            None
        }
    })?;
    let end = in_segment_order(
        segments.ending_at(projected.end),
        segments.containing(projected.end),
    )
    .map(|index| &segments[index])
    .find_map(|segment| {
        if segment.kind != ProjectionSegmentKind::SourceBoundary
            && projected.end == segment.projected.end
        {
            Some(segment.source.end)
        } else if segment.projected.start < projected.end
            && projected.end < segment.projected.end
            && segment.kind == ProjectionSegmentKind::Copied
        {
            Some(segment.source.start + projected.end.0 - segment.projected.start.0)
        } else {
            None
        }
    })?;
    Some(SourceSpan { start, end })
}

fn in_segment_order(mut first: Vec<usize>, second: Vec<usize>) -> impl Iterator<Item = usize> {
    first.extend(second);
    first.sort_unstable();
    first.dedup();
    first.into_iter()
}

fn position_holding<T>(
    positions: &[T],
    span: impl Fn(&T) -> ProjectedSpan,
    value: ProjectedSpan,
) -> Option<usize> {
    let index = positions.partition_point(|position| span(position).end <= value.start);
    positions
        .get(index)
        .filter(|position| projected_contains(span(position), value))
        .map(|_| index)
}
