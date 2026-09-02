//! Evaluation protocol and source-coordinate mapping.

use super::*;

pub(super) fn evaluation_protocol(
    segments: &[ProjectionSourceSegment],
    value: ProjectedSpan,
    source_value: SourceSpan,
    frames: &[ProjectedProtocolFrame],
) -> Result<HostEvaluationProtocol, ProgramSyntaxError> {
    let steps = frames
        .iter()
        .rev()
        .map(|frame| protocol_step(segments, value, source_value, frame))
        .collect::<Result<Vec<_>, ProgramSyntaxError>>()?
        .into_iter()
        .flatten()
        .collect();
    Ok(HostEvaluationProtocol { steps })
}

/// The projected shape of one conditional operation, before span mapping.
pub(super) struct ProjectedConditionalFacts {
    branch: ProjectedSpan,
    skipped: Option<ProjectedSpan>,
    operands: Vec<(ProjectedSpan, bool)>,
    type_args: Option<ProjectedSpan>,
}

pub(super) fn protocol_step(
    segments: &[ProjectionSourceSegment],
    value: ProjectedSpan,
    source_value: SourceSpan,
    frame: &ProjectedProtocolFrame,
) -> Result<Option<HostEvaluationStep>, ProgramSyntaxError> {
    let mut conditional: Option<ProjectedConditionalFacts> = None;
    let mut loop_test: Option<(
        LoopTestKind,
        ProjectedSpan,
        ProjectedSpan,
        Option<ProjectedSpan>,
    )> = None;
    let (parent, operation, inputs) = match frame {
        ProjectedProtocolFrame::Ordered {
            parent,
            positions,
            kind,
        } => {
            let Some(position) = positions
                .iter()
                .position(|(span, _)| projected_contains(*span, value))
            else {
                return Ok(None);
            };
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let operation = HostEvaluationOperation::Eager(match kind {
                OrderedEvaluationKind::Array => EagerPosition::ArrayElement(index),
                OrderedEvaluationKind::Object => EagerPosition::ObjectEvaluation(index),
                OrderedEvaluationKind::Assignment => EagerPosition::AssignmentRight,
                OrderedEvaluationKind::Sequence => EagerPosition::SequenceElement(index),
                OrderedEvaluationKind::Unary => EagerPosition::UnaryOperand,
            });
            let inputs = positions[..position]
                .iter()
                .copied()
                .map(|(span, effects)| (span, EvaluationInputMode::Value, None, effects))
                .collect();
            (*parent, operation, inputs)
        }
        ProjectedProtocolFrame::Binary {
            parent,
            left: (left, _),
            ..
        } if projected_contains(*left, value) => (
            *parent,
            HostEvaluationOperation::Eager(EagerPosition::BinaryLeft),
            Vec::new(),
        ),
        ProjectedProtocolFrame::Binary {
            parent,
            operator,
            left,
            right,
            ..
        } if projected_contains(*right, value) => {
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
                });
            }
            let (left, effects) = *left;
            (
                *parent,
                operation,
                vec![(left, EvaluationInputMode::Value, None, effects)],
            )
        }
        ProjectedProtocolFrame::Conditional {
            parent,
            test,
            consequent,
            alternate,
        } if projected_contains(*consequent, value) => {
            conditional = Some(ProjectedConditionalFacts {
                branch: *consequent,
                skipped: Some(*alternate),
                operands: Vec::new(),
                type_args: None,
            });
            let (test, effects) = *test;
            (
                *parent,
                HostEvaluationOperation::Conditional(ConditionalBranch::Consequent),
                vec![(test, EvaluationInputMode::Value, None, effects)],
            )
        }
        ProjectedProtocolFrame::Conditional {
            parent,
            test,
            consequent,
            alternate,
        } if projected_contains(*alternate, value) => {
            conditional = Some(ProjectedConditionalFacts {
                branch: *alternate,
                skipped: Some(*consequent),
                operands: Vec::new(),
                type_args: None,
            });
            let (test, effects) = *test;
            (
                *parent,
                HostEvaluationOperation::Conditional(ConditionalBranch::Alternate),
                vec![(test, EvaluationInputMode::Value, None, effects)],
            )
        }
        ProjectedProtocolFrame::Call {
            parent,
            callee: Some(callee),
            optional,
            ..
        } if projected_contains(*callee, value) => (
            *parent,
            HostEvaluationOperation::Reference(if *optional {
                ReferencePosition::OptionalCallCallee
            } else {
                ReferencePosition::CallCallee
            }),
            Vec::new(),
        ),
        ProjectedProtocolFrame::Call {
            parent,
            callee,
            callee_mode,
            callee_receiver,
            arguments,
            type_args,
            optional,
        } => {
            let Some(position) = arguments
                .iter()
                .position(|(argument, _, _)| projected_contains(*argument, value))
            else {
                return Ok(None);
            };
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let operation = if *optional {
                conditional = Some(ProjectedConditionalFacts {
                    branch: arguments[position].0,
                    skipped: None,
                    operands: arguments
                        .iter()
                        .map(|(span, spread, _)| (*span, *spread))
                        .collect(),
                    type_args: *type_args,
                });
                HostEvaluationOperation::Conditional(ConditionalBranch::OptionalCallArgument(index))
            } else {
                HostEvaluationOperation::Eager(EagerPosition::CallArgument(index))
            };
            let inputs = callee
                .iter()
                .copied()
                .map(|callee| (callee, *callee_mode, *callee_receiver, Effects::ANY))
                .chain(arguments[..position].iter().map(|(argument, _, effects)| {
                    (*argument, EvaluationInputMode::Value, None, *effects)
                }))
                .collect();
            (*parent, operation, inputs)
        }
        ProjectedProtocolFrame::Member {
            parent,
            object: (object, _),
            ..
        } if projected_contains(*object, value) => (
            *parent,
            HostEvaluationOperation::Reference(ReferencePosition::MemberObject),
            Vec::new(),
        ),
        ProjectedProtocolFrame::Member {
            parent,
            object: (object, effects),
            property: Some(property),
        } if projected_contains(*property, value) => (
            *parent,
            HostEvaluationOperation::Reference(ReferencePosition::MemberProperty),
            vec![(*object, EvaluationInputMode::Value, None, *effects)],
        ),
        ProjectedProtocolFrame::Construct { parent, callee, .. }
            if projected_contains(*callee, value) =>
        {
            (
                *parent,
                HostEvaluationOperation::Reference(ReferencePosition::ConstructorCallee),
                Vec::new(),
            )
        }
        ProjectedProtocolFrame::Construct {
            parent,
            callee,
            arguments,
        } => {
            let Some(position) = arguments
                .iter()
                .position(|(argument, _, _)| projected_contains(*argument, value))
            else {
                return Ok(None);
            };
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let inputs = std::iter::once((*callee, EvaluationInputMode::Value, None, Effects::ANY))
                .chain(arguments[..position].iter().map(|(argument, _, effects)| {
                    (*argument, EvaluationInputMode::Value, None, *effects)
                }))
                .collect();
            (
                *parent,
                HostEvaluationOperation::Eager(EagerPosition::ConstructArgument(index)),
                inputs,
            )
        }
        ProjectedProtocolFrame::TaggedTemplate { parent, tag, .. }
            if projected_contains(*tag, value) =>
        {
            (
                *parent,
                HostEvaluationOperation::Reference(ReferencePosition::TaggedTemplateTag),
                Vec::new(),
            )
        }
        ProjectedProtocolFrame::TaggedTemplate {
            parent,
            tag,
            tag_mode,
            tag_receiver,
            expressions,
        } => {
            let Some(position) = expressions
                .iter()
                .position(|(span, _)| projected_contains(*span, value))
            else {
                return Ok(None);
            };
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let inputs = std::iter::once((*tag, *tag_mode, *tag_receiver, Effects::ANY))
                .chain(
                    expressions[..position]
                        .iter()
                        .copied()
                        .map(|(expression, effects)| {
                            (expression, EvaluationInputMode::Value, None, effects)
                        }),
                )
                .collect();
            (
                *parent,
                HostEvaluationOperation::Eager(EagerPosition::TemplateInterpolation(index)),
                inputs,
            )
        }
        ProjectedProtocolFrame::Template {
            parent,
            expressions,
        } => {
            let Some(position) = expressions
                .iter()
                .position(|(span, _)| projected_contains(*span, value))
            else {
                return Ok(None);
            };
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let inputs = expressions[..position]
                .iter()
                .copied()
                .map(|(expression, effects)| {
                    (expression, EvaluationInputMode::Value, None, effects)
                })
                .collect();
            (
                *parent,
                HostEvaluationOperation::Eager(EagerPosition::TemplateInterpolation(index)),
                inputs,
            )
        }
        ProjectedProtocolFrame::Jsx {
            parent,
            expressions,
        } => {
            let Some(position) = expressions
                .iter()
                .position(|(span, _, _)| projected_contains(*span, value))
            else {
                return Ok(None);
            };
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let inputs = expressions[..position]
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
        ProjectedProtocolFrame::Suspend {
            parent,
            kind,
            value: Some(argument),
        } if projected_contains(*argument, value) => {
            (*parent, HostEvaluationOperation::Suspend(*kind), Vec::new())
        }
        ProjectedProtocolFrame::LoopTest {
            parent,
            kind,
            test,
            body,
            update,
        } if projected_contains(*test, value) => {
            loop_test = Some((*kind, *test, *body, *update));
            (*parent, HostEvaluationOperation::LoopTest, Vec::new())
        }
        _ => return Ok(None),
    };
    let parent = if operation == HostEvaluationOperation::LoopTest {
        source_value
    } else {
        map_evaluation_span(segments, parent)?
    };
    let inputs = inputs
        .into_iter()
        .map(|(source, mode, receiver, effects)| {
            Ok(HostEvaluationInput {
                source: map_evaluation_span(segments, source)?,
                mode,
                receiver: receiver
                    .map(|(receiver, effects)| {
                        Ok((map_evaluation_span(segments, receiver)?, effects))
                    })
                    .transpose()?,
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
    let conditional = conditional
        .map(|facts| {
            Ok(ConditionalFacts {
                // The branch holds the tt value, so it has no contiguous
                // source mapping; its bounds map endpoint-by-endpoint.
                branch: map_evaluation_span(segments, facts.branch)?,
                skipped: facts
                    .skipped
                    .map(|span| map_evaluation_span(segments, span))
                    .transpose()?,
                operands: facts
                    .operands
                    .into_iter()
                    .map(|(span, spread)| {
                        Ok(ConditionalOperand {
                            span: map_evaluation_span(segments, span)?,
                            spread,
                        })
                    })
                    .collect::<Result<Vec<_>, ProgramSyntaxError>>()?,
                type_args: facts
                    .type_args
                    .map(|span| map_evaluation_span(segments, span))
                    .transpose()?,
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
    Ok(Some(HostEvaluationStep {
        parent,
        operation,
        inputs,
        conditional,
        loop_test,
    }))
}

pub(super) fn map_evaluation_span(
    segments: &[ProjectionSourceSegment],
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
    segments: &[ProjectionSourceSegment],
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

pub(super) fn call_callee_mode(
    expression: &swc_ecma_ast::Expr,
) -> (EvaluationInputMode, Option<&swc_ecma_ast::Expr>) {
    use swc_ecma_ast::{Expr as SwcExpr, OptChainBase};

    match expression {
        SwcExpr::Member(member) => (EvaluationInputMode::MemberReference, Some(&member.obj)),
        SwcExpr::SuperProp(_) => (EvaluationInputMode::MemberReference, None),
        SwcExpr::OptChain(chain) => match &*chain.base {
            OptChainBase::Member(member) => {
                (EvaluationInputMode::MemberReference, Some(&member.obj))
            }
            OptChainBase::Call(_) => (EvaluationInputMode::DirectReference, None),
        },
        SwcExpr::Paren(paren) => call_callee_mode(&paren.expr),
        SwcExpr::TsAs(expression) => call_callee_mode(&expression.expr),
        SwcExpr::TsTypeAssertion(expression) => call_callee_mode(&expression.expr),
        SwcExpr::TsNonNull(expression) => call_callee_mode(&expression.expr),
        SwcExpr::TsInstantiation(expression) => call_callee_mode(&expression.expr),
        SwcExpr::TsSatisfies(expression) => call_callee_mode(&expression.expr),
        _ => (EvaluationInputMode::DirectReference, None),
    }
}

pub(super) fn reference_value_span(expression: &swc_ecma_ast::Expr) -> swc_common::Span {
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

pub(super) fn projected_span(span: swc_common::Span, source_start: u32) -> ProjectedSpan {
    ProjectedSpan {
        start: ProjectedByte(span.lo.0.saturating_sub(source_start) as usize),
        end: ProjectedByte(span.hi.0.saturating_sub(source_start) as usize),
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
            ProjectionSegmentKind::SourceBoundary => Some(segment.source.start),
            ProjectionSegmentKind::Placeholder => None,
        }
    })
}

pub(super) fn source_span_for_projection(
    segments: &[ProjectionSourceSegment],
    projected: ProjectedSpan,
) -> Option<SourceSpan> {
    let start = segments.iter().find_map(|segment| {
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
    let end = segments.iter().find_map(|segment| {
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
