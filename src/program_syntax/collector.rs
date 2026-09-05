//! Parsed-module state and AST-parent collection data.

use super::*;

pub(super) struct ParsedModule {
    pub(super) module: Module,
    pub(super) start: u32,
}

pub(super) fn parse_module(
    code: &str,
    segments: &[ProjectionSourceSegment],
    source_kind: crate::SourceKind,
) -> Result<ParsedModule, ProgramSyntaxError> {
    if let Some((span, message)) = crate::lexer::host_syntax_error(code, source_kind) {
        return Err(parse_failure_at(
            code,
            segments,
            span.start,
            message.to_string(),
        ));
    }
    let source_map: Lrc<SourceMap> = Default::default();
    let file = source_map.new_source_file(Lrc::new(FileName::Anon), code.to_string());
    let start = file.start_pos.0;
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: source_kind.is_tsx(),
            decorators: true,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*file),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let result = parser.parse_module();
    let mut errors = parser.take_errors();
    let module = match result {
        Ok(module) => module,
        Err(error) => {
            errors.push(error);
            return Err(parse_failure(code, segments, start, &errors[0]));
        }
    };
    if let Some(error) = errors.into_iter().next() {
        return Err(parse_failure(code, segments, start, &error));
    }
    Ok(ParsedModule { module, start })
}

/// Classifies a projection parse failure by the byte it stopped at.
///
/// The projection is a sequence of two kinds of bytes: text copied from the
/// source, and placeholders this compiler wrote. Which kind the parser
/// stopped on *is* the cause, so the classification is a lookup in the
/// projection's own segment table rather than a guess about the message.
pub(super) fn parse_failure(
    code: &str,
    segments: &[ProjectionSourceSegment],
    start: u32,
    error: &swc_ecma_parser::error::Error,
) -> ProgramSyntaxError {
    let message = error.kind().msg().to_string();
    // A parser can stop one byte past the end (`<eof>` expectations); that
    // byte belongs to the segment it ends.
    let at = usize::try_from(error.span().lo().0.saturating_sub(start)).unwrap_or(0);
    parse_failure_at(code, segments, at, message)
}

fn parse_failure_at(
    code: &str,
    segments: &[ProjectionSourceSegment],
    at: usize,
    message: String,
) -> ProgramSyntaxError {
    let at = ProjectedByte(at.min(code.len().saturating_sub(1)));
    match source_byte_for_projection(segments, at) {
        Some(source) => ProgramSyntaxError::SourceNotTypeScript { message, source },
        None => ProgramSyntaxError::Parse {
            message,
            projection: code.to_owned(),
        },
    }
}

pub(super) struct ParentCollector {
    pub(super) arm_blocks: HashMap<ProjectedSpan, BodyId>,
    pub(super) single_return_bodies: HashMap<ProjectedSpan, BodyId>,
    pub(super) source_start: u32,
    pub(super) expected_identifiers: HashMap<ProjectedSpan, TtNodeId>,
    pub(super) expected_calls: HashMap<ProjectedSpan, TtNodeId>,
    pub(super) expected_exit_calls: HashSet<TtNodeId>,
    pub(super) synthetic_returns: HashSet<ProjectedSpan>,
    pub(super) found: HashMap<TtNodeId, FoundOverlay>,
    pub(super) duplicates: Vec<TtNodeId>,
    pub(super) source_segments: Vec<ProjectionSourceSegment>,
    pub(super) projection_only_protocol_parents: HashSet<ProjectedSpan>,
    pub(super) host_owners: Vec<ProjectedHostOwner>,
    pub(super) protocol_frames: Vec<ProjectedProtocolFrame>,
    pub(super) occupied_names: HashSet<String>,
    pub(super) function_depth: usize,
    pub(super) function_targets: Vec<EvaluationOwner>,
    pub(super) contextual_types: Vec<Option<ProjectedSpan>>,
    pub(super) function_return_types: Vec<Option<ProjectedSpan>>,
    pub(super) function_return_async: Vec<bool>,
    /// How many enclosing statements consume an unlabeled `break`
    /// (loops and `switch`).
    pub(super) break_capture_depth: usize,
    /// The exit-collecting regions in scope, with the function depth an
    /// exit must sit at and the break-capture depth the region opened at.
    pub(super) exit_regions: Vec<(TtNodeId, usize, usize)>,
    /// The projected arm blocks currently being visited: the arm's Core
    /// body, whether the block is free of cleanup boundaries, and the
    /// function depth the block sits at.
    pub(super) arm_block_scopes: Vec<(BodyId, bool, usize)>,
}

pub(super) struct CollectedProgramSyntax {
    pub(super) overlay: Vec<OverlayEntry>,
    pub(super) owners: Vec<HostOwnerSyntax>,
    pub(super) occupied_names: HashSet<String>,
}

pub(super) struct FoundOverlay {
    pub(super) parents: Vec<AstParentKind>,
    pub(super) host_owners: Vec<ProjectedHostOwner>,
    pub(super) protocol_frames: Vec<ProjectedProtocolFrame>,
    pub(super) exits: Vec<ProjectedHostExit>,
    pub(super) function_target: Option<EvaluationOwner>,
    pub(super) contextual_type: Option<ProjectedSpan>,
    pub(super) function_return_type: Option<ProjectedSpan>,
    pub(super) function_return_awaited: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ProjectedHostExit {
    pub(super) body: Option<BodyId>,
    pub(super) call_safe: bool,
    pub(super) single_return_body: Option<BodyId>,
    pub(super) statement: ProjectedSpan,
    pub(super) argument: Option<ProjectedSpan>,
    pub(super) captured_break: bool,
    pub(super) requires_block: bool,
}

#[derive(Debug, Clone)]
pub(super) enum ProjectedProtocolFrame {
    Ordered {
        parent: ProjectedSpan,
        positions: Vec<(ProjectedSpan, Effects)>,
        kind: OrderedEvaluationKind,
    },
    Binary {
        parent: ProjectedSpan,
        operator: BinaryOp,
        left: (ProjectedSpan, Effects),
        right: ProjectedSpan,
    },
    Conditional {
        parent: ProjectedSpan,
        test: (ProjectedSpan, Effects),
        consequent: ProjectedSpan,
        alternate: ProjectedSpan,
    },
    Call {
        /// Whether the call sits in expression-statement position, so its
        /// result is discarded.
        discarded: bool,
        parent: ProjectedSpan,
        callee: Option<ProjectedSpan>,
        callee_mode: EvaluationInputMode,
        callee_receiver: Option<(ProjectedSpan, Effects)>,
        arguments: Vec<(ProjectedSpan, bool, Effects)>,
        type_args: Option<ProjectedSpan>,
        optional: bool,
    },
    Member {
        parent: ProjectedSpan,
        object: (ProjectedSpan, Effects),
        property: Option<ProjectedSpan>,
    },
    Construct {
        parent: ProjectedSpan,
        callee: ProjectedSpan,
        arguments: Vec<(ProjectedSpan, bool, Effects)>,
    },
    TaggedTemplate {
        parent: ProjectedSpan,
        tag: ProjectedSpan,
        tag_mode: EvaluationInputMode,
        tag_receiver: Option<(ProjectedSpan, Effects)>,
        expressions: Vec<(ProjectedSpan, Effects)>,
    },
    Template {
        parent: ProjectedSpan,
        expressions: Vec<(ProjectedSpan, Effects)>,
    },
    Jsx {
        parent: ProjectedSpan,
        expressions: Vec<(ProjectedSpan, Effects, bool)>,
    },
    Suspend {
        parent: ProjectedSpan,
        kind: SuspensionKind,
        value: Option<ProjectedSpan>,
    },
    LoopTest {
        parent: ProjectedSpan,
        kind: LoopTestKind,
        test: ProjectedSpan,
        body: ProjectedSpan,
        update: Option<ProjectedSpan>,
    },
}

impl ProjectedProtocolFrame {
    /// The projected span of the node that owns this evaluation frame.
    pub(super) fn parent(&self) -> ProjectedSpan {
        match self {
            ProjectedProtocolFrame::Ordered { parent, .. }
            | ProjectedProtocolFrame::Binary { parent, .. }
            | ProjectedProtocolFrame::Conditional { parent, .. }
            | ProjectedProtocolFrame::Call { parent, .. }
            | ProjectedProtocolFrame::Member { parent, .. }
            | ProjectedProtocolFrame::Construct { parent, .. }
            | ProjectedProtocolFrame::TaggedTemplate { parent, .. }
            | ProjectedProtocolFrame::Template { parent, .. }
            | ProjectedProtocolFrame::Jsx { parent, .. }
            | ProjectedProtocolFrame::Suspend { parent, .. }
            | ProjectedProtocolFrame::LoopTest { parent, .. } => *parent,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum OrderedEvaluationKind {
    Array,
    Object,
    Assignment,
    Sequence,
    Unary,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ProjectedHostOwner {
    pub(super) kind: HostOwnerKind,
    pub(super) span: ProjectedSpan,
    /// How many parent edges precede this owner's own child edges. Slicing
    /// a value's parent path here leaves exactly the edges between the owner
    /// and the value ([`owner_reach`]).
    pub(super) edge: usize,
}

pub(super) fn object_evaluation_positions(
    node: &ObjectLit,
    source_start: u32,
) -> Vec<(ProjectedSpan, Effects)> {
    let mut positions = Vec::new();
    for property in &node.props {
        match property {
            PropOrSpread::Spread(spread) => {
                positions.push((
                    projected_span(spread.expr.span(), source_start),
                    expression_effects(&spread.expr),
                ));
            }
            PropOrSpread::Prop(property) => match &**property {
                Prop::Shorthand(identifier) => {
                    positions.push((projected_span(identifier.span, source_start), Effects::ANY));
                }
                Prop::KeyValue(property) => {
                    push_computed_property(&mut positions, &property.key, source_start);
                    positions.push((
                        projected_span(property.value.span(), source_start),
                        expression_effects(&property.value),
                    ));
                }
                Prop::Assign(property) => {
                    positions.push((
                        projected_span(property.value.span(), source_start),
                        expression_effects(&property.value),
                    ));
                }
                Prop::Getter(property) => {
                    push_computed_property(&mut positions, &property.key, source_start);
                }
                Prop::Setter(property) => {
                    push_computed_property(&mut positions, &property.key, source_start);
                }
                Prop::Method(property) => {
                    push_computed_property(&mut positions, &property.key, source_start);
                }
            },
        }
    }
    positions
}

pub(super) fn push_computed_property(
    positions: &mut Vec<(ProjectedSpan, Effects)>,
    name: &PropName,
    source_start: u32,
) {
    if let PropName::Computed(computed) = name {
        positions.push((
            projected_span(computed.expr.span(), source_start),
            expression_effects(&computed.expr),
        ));
    }
}

/// The evaluation positions of a call's arguments: the argument
/// *expression* spans (a spread's `...` stays with the call, so a capture
/// of the position captures a value, not spread syntax), plus whether the
/// call spreads each.
pub(super) fn argument_positions(
    arguments: &[swc_ecma_ast::ExprOrSpread],
    source_start: u32,
) -> Vec<(ProjectedSpan, bool, Effects)> {
    arguments
        .iter()
        .map(|argument| {
            (
                projected_span(argument.expr.span(), source_start),
                argument.spread.is_some(),
                expression_effects(&argument.expr),
            )
        })
        .collect()
}

pub(super) fn jsx_expression_span(
    expression: &JSXExpr,
    source_start: u32,
) -> Option<ProjectedSpan> {
    match expression {
        JSXExpr::Expr(expression) => Some(projected_span(expression.span(), source_start)),
        JSXExpr::JSXEmptyExpr(_) => None,
    }
}

pub(super) fn jsx_evaluation_positions(
    node: &JSXElement,
    source_start: u32,
) -> Vec<(ProjectedSpan, bool)> {
    let attributes = node
        .opening
        .attrs
        .iter()
        .filter_map(|attribute| match attribute {
            JSXAttrOrSpread::SpreadElement(spread) => {
                Some((projected_span(spread.expr.span(), source_start), false))
            }
            JSXAttrOrSpread::JSXAttr(attribute) => match attribute.value.as_ref()? {
                JSXAttrValue::JSXExprContainer(container) => {
                    jsx_expression_span(&container.expr, source_start).map(|span| (span, false))
                }
                JSXAttrValue::JSXElement(element) => {
                    Some((projected_span(element.span, source_start), false))
                }
                JSXAttrValue::JSXFragment(fragment) => {
                    Some((projected_span(fragment.span, source_start), false))
                }
                JSXAttrValue::Str(_) => None,
            },
        });
    let children = node.children.iter().filter_map(|child| match child {
        JSXElementChild::JSXExprContainer(container) => {
            jsx_expression_span(&container.expr, source_start).map(|span| (span, false))
        }
        JSXElementChild::JSXSpreadChild(spread) => {
            Some((projected_span(spread.expr.span(), source_start), false))
        }
        JSXElementChild::JSXElement(element) => {
            Some((projected_span(element.span, source_start), true))
        }
        JSXElementChild::JSXFragment(fragment) => {
            Some((projected_span(fragment.span, source_start), true))
        }
        JSXElementChild::JSXText(_) => None,
    });
    attributes.chain(children).collect()
}

pub(super) fn jsx_fragment_positions(
    node: &JSXFragment,
    source_start: u32,
) -> Vec<(ProjectedSpan, bool)> {
    node.children
        .iter()
        .filter_map(|child| match child {
            JSXElementChild::JSXExprContainer(container) => {
                jsx_expression_span(&container.expr, source_start).map(|span| (span, false))
            }
            JSXElementChild::JSXSpreadChild(spread) => {
                Some((projected_span(spread.expr.span(), source_start), false))
            }
            JSXElementChild::JSXElement(element) => {
                Some((projected_span(element.span, source_start), true))
            }
            JSXElementChild::JSXFragment(fragment) => {
                Some((projected_span(fragment.span, source_start), true))
            }
            JSXElementChild::JSXText(_) => None,
        })
        .collect()
}
