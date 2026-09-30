//! Parsed-module state and AST-parent collection data.

use super::*;

pub(super) struct ParsedModule {
    pub(super) module: Module,
    pub(super) start: HostOrigin,
}

pub(super) fn parse_module(
    code: &str,
    segments: &[ProjectionSourceSegment],
    source_kind: crate::SourceKind,
    tolerant: bool,
) -> Result<ParsedModule, ProgramSyntaxError> {
    if let Some((span, message)) = crate::lexer::host_syntax_error(code, source_kind) {
        return Err(parse_failure_at(
            code,
            segments,
            span.start,
            message.to_string(),
        ));
    }
    let input = HostInput::new(code);
    let start = input.origin();
    let mut parser = input.parser(source_kind);
    let result = parser.parse_program().map(|program| match program {
        swc_ecma_ast::Program::Module(module) => module,
        swc_ecma_ast::Program::Script(script) => Module {
            span: script.span,
            body: script.body.into_iter().map(ModuleItem::Stmt).collect(),
            shebang: script.shebang,
        },
    });
    let recovered = parser.take_errors();
    let left_to_typescript = |error: &swc_ecma_parser::error::Error| {
        if !crate::verify::checked_after_parsing(error.kind()) {
            return false;
        }
        let span = error.span();
        let (lo, hi) = (start.byte(span.lo()), start.byte(span.hi()));
        segments.iter().any(|segment| {
            segment.kind == ProjectionSegmentKind::Copied
                && segment.projected.start.0 <= lo
                && hi <= segment.projected.end.0
                && lo < segment.projected.end.0
        })
    };
    let reported = recovered.iter().find(|error| !left_to_typescript(error));
    let module = match result {
        Ok(module) => module,
        Err(error) => {
            return Err(parse_failure(
                code,
                segments,
                start,
                reported.unwrap_or(&error),
            ));
        }
    };
    if let Some(error) = reported
        && !tolerant
    {
        return Err(parse_failure(code, segments, start, error));
    }
    Ok(ParsedModule { module, start })
}

pub(super) fn directive_prologue_end(
    module: &Module,
    start: HostOrigin,
    segments: &[ProjectionSourceSegment],
) -> Result<Option<usize>, ProgramSyntaxError> {
    let Some((statement, literal)) = module
        .body
        .iter()
        .map_while(|item| match item {
            ModuleItem::Stmt(Stmt::Expr(statement)) => match &*statement.expr {
                swc_ecma_ast::Expr::Lit(swc_ecma_ast::Lit::Str(literal)) => {
                    Some((statement.span, literal.span))
                }
                _ => None,
            },
            _ => None,
        })
        .last()
    else {
        return Ok(None);
    };
    let copied_end = |span: swc_common::Span| {
        let hi = start.byte(span.hi);
        hi.checked_sub(1).and_then(|last| {
            segments.iter().find_map(|segment| {
                (segment.kind == ProjectionSegmentKind::Copied
                    && segment.projected.start.0 <= last
                    && last < segment.projected.end.0)
                    .then(|| segment.source.start + last - segment.projected.start.0 + 1)
            })
        })
    };
    copied_end(statement)
        .or_else(|| copied_end(literal))
        .map(Some)
        .ok_or(ProgramSyntaxError::UnmappedEvaluationSpan {
            start: start.byte(statement.lo),
            end: start.byte(statement.hi),
        })
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
    start: HostOrigin,
    error: &swc_ecma_parser::error::Error,
) -> ProgramSyntaxError {
    let message = error.kind().msg().to_string();
    // A parser can stop one byte past the end (`<eof>` expectations); that
    // byte belongs to the segment it ends.
    let at = start.byte(error.span().lo());
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
            source: segments
                .iter()
                .filter(|segment| {
                    segment.kind == ProjectionSegmentKind::Placeholder
                        && segment.projected.start <= at
                        && at < segment.projected.end
                })
                .min_by_key(|segment| segment.projected.end.0 - segment.projected.start.0)
                .map(|segment| segment.source),
        },
    }
}

pub(super) struct ParentCollector {
    pub(super) placeholders: HashSet<ProjectedSpan>,
    pub(super) arm_blocks: HashMap<ProjectedSpan, BodyId>,
    pub(super) tt_bindings: projection::TtBindings,
    pub(super) single_return_bodies: HashMap<ProjectedSpan, BodyId>,
    pub(super) source_start: HostOrigin,
    pub(super) expected_identifiers: HashMap<ProjectedSpan, TtNodeId>,
    pub(super) expected_calls: HashMap<ProjectedSpan, TtNodeId>,
    pub(super) expected_exit_calls: HashSet<TtNodeId>,
    pub(super) synthetic_returns: HashSet<ProjectedSpan>,
    pub(super) found: HashMap<TtNodeId, FoundOverlay>,
    pub(super) duplicates: Vec<TtNodeId>,
    pub(super) source_segments: ProjectionSegments,
    pub(super) projection_only_protocol_parents: HashSet<ProjectedSpan>,
    pub(super) host_owners: Vec<ProjectedHostOwner>,
    pub(super) protocol_frames: Vec<ProjectedProtocolFrame>,
    pub(super) occupied_names: HashSet<String>,
    pub(super) function_depth: usize,
    pub(super) function_targets: Vec<EvaluationOwner>,
    pub(super) contextual_types: Vec<Option<ProjectedSpan>>,
    pub(super) assertions: Vec<Option<ProjectedSpan>>,
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
    pub(super) global_statements: HashMap<ProjectedSpan, GlobalStatement>,
}

pub(super) struct CollectedProgramSyntax {
    pub(super) overlay: Vec<OverlayEntry>,
    pub(super) owners: Vec<HostOwnerSyntax>,
    pub(super) occupied_names: HashSet<String>,
    pub(super) globals: HashMap<SourceSpan, GlobalStatement>,
}

pub(super) struct FoundOverlay {
    pub(super) ambient: bool,
    pub(super) decorated_classes: Vec<usize>,
    pub(super) parents: Vec<AstParentKind>,
    pub(super) host_owners: Vec<ProjectedHostOwner>,
    pub(super) protocol_frames: Vec<ProjectedProtocolFrame>,
    pub(super) exits: Vec<ProjectedHostExit>,
    pub(super) function_target: Option<EvaluationOwner>,
    pub(super) contextual_type: Option<ProjectedSpan>,
    pub(super) assertion: Option<Option<ProjectedSpan>>,
    pub(super) loop_head_reads: bool,
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
    pub(super) value_argument: Option<ProjectedSpan>,
    pub(super) captured_break: bool,
    pub(super) requires_block: bool,
}

#[derive(Debug, Clone)]
pub(super) enum ProjectedProtocolFrame {
    Ordered {
        parent: ProjectedSpan,
        positions: Vec<(ProjectedSpan, Effects)>,
        kind: OrderedEvaluationKind,
        /// Whether the operation has no spread element. A spread copies its
        /// operand at the literal's own position, running whatever getters
        /// the operand has, and the positions below record only the
        /// operand's effects — so a lowering that moves the literal has to
        /// ask this separately. Always true where the kind cannot spread.
        spread_free: bool,
    },
    /// An assignment. Its target's reference is evaluated before the right
    /// operand (ECMA-262 §13.15.2): a member target's object and computed
    /// key, and — for any operator but `=` — the target's current value.
    Assignment {
        parent: ProjectedSpan,
        operator: AssignOp,
        target: ProjectedSpan,
        reference: Vec<(ProjectedSpan, Effects)>,
        right: ProjectedSpan,
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
        callee_reference: Option<ProjectedMemberReference>,
        arguments: Vec<(ProjectedSpan, bool, Effects)>,
        type_args: Option<ProjectedSpan>,
        /// For a call in an optional chain, the link that decides whether
        /// it is evaluated.
        optional: Option<OptionalCallTest>,
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
        tag_reference: Option<ProjectedMemberReference>,
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
            | ProjectedProtocolFrame::Assignment { parent, .. }
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

/// The parts of a member callee's reference: its object and its computed
/// key ([`HostReferencePart`]).
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct ProjectedMemberReference {
    pub(super) receiver: Option<ProjectedReferencePart>,
    pub(super) key: Option<ProjectedReferencePart>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ProjectedReferencePart {
    pub(super) span: ProjectedSpan,
    pub(super) effects: Effects,
    pub(super) read_at_call: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum OrderedEvaluationKind {
    Array,
    Object,
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
    pub(super) split: Option<ProjectedDeclaratorSplit>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ProjectedDeclaratorSplit {
    pub(super) previous: ProjectedSpan,
    pub(super) kind: DeclarationKind,
    pub(super) exported: bool,
    pub(super) declared: bool,
}

pub(super) fn object_evaluation_positions(
    node: &ObjectLit,
    source_start: HostOrigin,
    placeholders: &HashSet<ProjectedSpan>,
    segments: &ProjectionSegments,
) -> Vec<(ProjectedSpan, Effects)> {
    let mut positions = Vec::new();
    for property in &node.props {
        match property {
            PropOrSpread::Spread(spread) => {
                positions.push((
                    operand_span(&spread.expr, source_start, placeholders, segments),
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
                        operand_span(&property.value, source_start, placeholders, segments),
                        expression_effects(&property.value),
                    ));
                }
                Prop::Assign(property) => {
                    positions.push((
                        operand_span(&property.value, source_start, placeholders, segments),
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
    source_start: HostOrigin,
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
    source_start: HostOrigin,
    placeholders: &HashSet<ProjectedSpan>,
    segments: &ProjectionSegments,
) -> Vec<(ProjectedSpan, bool, Effects)> {
    arguments
        .iter()
        .map(|argument| {
            (
                operand_span(&argument.expr, source_start, placeholders, segments),
                argument.spread.is_some(),
                expression_effects(&argument.expr),
            )
        })
        .collect()
}

pub(super) fn jsx_expression_span(
    expression: &JSXExpr,
    source_start: HostOrigin,
) -> Option<ProjectedSpan> {
    match expression {
        JSXExpr::Expr(expression) => Some(projected_span(expression.span(), source_start)),
        JSXExpr::JSXEmptyExpr(_) => None,
    }
}

pub(super) fn jsx_evaluation_positions(
    node: &JSXElement,
    source_start: HostOrigin,
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
    source_start: HostOrigin,
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
