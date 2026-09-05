//! TypeScript projection construction and the public syntax view.

use super::*;

/// A complete SWC view of one tt-containing TypeScript module.
#[derive(Debug)]
pub(crate) struct ProgramSyntax {
    pub(super) source_len: usize,
    pub(super) projection: String,
    pub(super) module: Module,
    pub(super) overlay: Vec<OverlayEntry>,
    pub(super) owners: Vec<HostOwnerSyntax>,
    pub(super) occupied_names: HashSet<String>,
}

#[derive(Debug)]
pub(crate) struct HostOwnerSyntax {
    pub(crate) owner: HostOwner,
    pub(crate) roots: Vec<TtNodeId>,
}

/// Why a shadow program model could not be built.
///
/// Every variant but [`ProgramSyntaxError::SourceNotTypeScript`] is a broken
/// compiler invariant — a validator failure in the sense of
/// `docs/design/program-lowering.md` §11, and therefore an internal compiler
/// error. The projection's *parse*, though, is not a validator: the
/// projection is TypeScript the user wrote with tt values replaced by
/// placeholders, so it can also fail because that TypeScript is not
/// TypeScript. That cause is a fact about the input and carries the source
/// byte to report it at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProgramSyntaxError {
    MissingSourceSpan {
        node: NodeId,
    },
    InvalidSourceSpan {
        start: SourceByte,
        end: SourceByte,
    },
    NodeCountOverflow,
    /// The projection stopped parsing at a byte copied verbatim from the
    /// source: the user's own TypeScript does not parse.
    SourceNotTypeScript {
        message: String,
        source: usize,
    },
    /// The projection stopped parsing at a byte this compiler generated.
    Parse {
        message: String,
        projection: String,
    },
    MissingOverlay {
        id: TtNodeId,
    },
    DuplicateOverlay {
        id: TtNodeId,
    },
    UnmappedEvaluationSpan {
        start: usize,
        end: usize,
    },
}

impl ProgramSyntax {
    /// Builds and validates the shadow program model.
    pub(crate) fn build(
        semantic: &SemanticFile,
        core: &CoreFile,
        source: &str,
        source_kind: crate::SourceKind,
    ) -> Result<Self, ProgramSyntaxError> {
        if let Some((span, message)) = crate::lexer::host_syntax_error(source, source_kind) {
            return Err(ProgramSyntaxError::SourceNotTypeScript {
                message: message.to_string(),
                source: span.start,
            });
        }
        let projection = ProjectionBuilder::new(semantic, core, source).build()?;
        let parsed = parse_module(&projection.code, &projection.source_segments, source_kind)?;
        let mut collector = ParentCollector::new(
            parsed.start,
            &projection.pending,
            &projection.source_segments,
            &projection.projection_only_protocol_parents,
            &projection.arm_blocks,
        );
        let mut path = AstNodePath::default();
        parsed.module.visit_with_ast_path(&mut collector, &mut path);
        let collected = collector.finish(projection.pending)?;
        let syntax = Self {
            source_len: source.len(),
            projection: projection.code,
            module: parsed.module,
            overlay: collected.overlay,
            owners: collected.owners,
            occupied_names: collected.occupied_names,
        };
        syntax.validate()?;
        Ok(syntax)
    }

    /// Returns the Core roots joined to their TypeScript host contexts.
    pub(crate) fn core_contexts(
        &self,
    ) -> impl Iterator<
        Item = (
            CoreRoot,
            TtNodeId,
            EvaluationContext,
            HostEvaluationProtocol,
            SourceSpan,
            HostOwner,
            Vec<HostExit>,
        ),
    > + '_ {
        self.overlay.iter().map(|entry| {
            (
                entry.core_root,
                entry.id,
                entry.context,
                entry.protocol.clone(),
                entry.source,
                entry.host_owner,
                entry.exits.clone(),
            )
        })
    }

    pub(crate) fn owners(&self) -> impl Iterator<Item = &HostOwnerSyntax> {
        self.owners.iter()
    }

    pub(crate) fn occupied_names(&self) -> impl Iterator<Item = &str> {
        self.occupied_names.iter().map(String::as_str)
    }

    fn validate(&self) -> Result<(), ProgramSyntaxError> {
        let _module_span = self.module.span;
        let projection_len = self.projection.len();
        for entry in &self.overlay {
            let start = entry.projected.start.0;
            let end = entry.projected.end.0;
            if start >= end || end > projection_len {
                return Err(ProgramSyntaxError::InvalidSourceSpan {
                    start: SourceByte(entry.source.start),
                    end: SourceByte(entry.source.end),
                });
            }
            if entry.parents.is_empty() {
                return Err(ProgramSyntaxError::MissingOverlay { id: entry.id });
            }
            match entry.category {
                SyntaxCategory::Expression
                | SyntaxCategory::Propagation
                | SyntaxCategory::Statement
                | SyntaxCategory::Item => {}
            }
            match entry.context.continuation {
                HostContinuation::Return
                | HostContinuation::ArrowReturn
                | HostContinuation::Initialize
                | HostContinuation::ForInitialize
                | HostContinuation::Discard
                | HostContinuation::Compose => {}
            }
            for step in entry.protocol.steps() {
                if step.parent.start >= step.parent.end || step.parent.end > self.source_len {
                    return Err(ProgramSyntaxError::InvalidSourceSpan {
                        start: SourceByte(step.parent.start),
                        end: SourceByte(step.parent.end),
                    });
                }
            }
        }
        for (index, owner) in self.owners.iter().enumerate() {
            if owner.owner.id.0 as usize != index || owner.roots.is_empty() {
                return Err(ProgramSyntaxError::NodeCountOverflow);
            }
            if owner.owner.span.start >= owner.owner.span.end
                || owner.owner.span.end > self.source_len
            {
                return Err(ProgramSyntaxError::InvalidSourceSpan {
                    start: SourceByte(owner.owner.span.start),
                    end: SourceByte(owner.owner.span.end),
                });
            }
        }
        Ok(())
    }
}

pub(super) struct Projection {
    pub(super) arm_blocks: HashMap<ProjectedSpan, BodyId>,
    pub(super) code: String,
    pub(super) pending: Vec<PendingOverlay>,
    pub(super) source_segments: Vec<ProjectionSourceSegment>,
    pub(super) projection_only_protocol_parents: Vec<ProjectedSpan>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ProjectionSourceSegment {
    pub(super) projected: ProjectedSpan,
    pub(super) source: SourceSpan,
    pub(super) kind: ProjectionSegmentKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProjectionSegmentKind {
    Copied,
    /// Compiler-written delimiter that closes a copied source fragment.
    /// A parser stopping here proves that the fragment immediately before
    /// it was incomplete; the delimiter itself is fixed syntax.
    SourceBoundary,
    Placeholder,
}

#[derive(Debug)]
pub(super) struct PendingOverlay {
    pub(super) id: TtNodeId,
    pub(super) category: SyntaxCategory,
    pub(super) source: SourceSpan,
    pub(super) projected: ProjectedSpan,
    pub(super) core_root: CoreRoot,
    pub(super) marker: OverlayMarker,
    pub(super) synthetic_return: Option<ProjectedSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OverlayMarker {
    Identifier,
    CallExpression,
    DecisionCallExpression,
}

pub(super) struct ProjectionBuilder<'a> {
    pub(super) arm_blocks: HashMap<ProjectedSpan, BodyId>,
    pub(super) semantic: &'a SemanticFile,
    pub(super) core: &'a CoreFile,
    pub(super) source: &'a str,
    pub(super) code: String,
    pub(super) pending: Vec<PendingOverlay>,
    pub(super) source_segments: Vec<ProjectionSourceSegment>,
    pub(super) projection_only_protocol_parents: Vec<ProjectedSpan>,
    pub(super) tokens: Vec<Token>,
}

impl<'a> ProjectionBuilder<'a> {
    pub(super) fn new(semantic: &'a SemanticFile, core: &'a CoreFile, source: &'a str) -> Self {
        Self {
            arm_blocks: HashMap::new(),
            semantic,
            core,
            source,
            code: String::with_capacity(source.len()),
            pending: Vec::new(),
            source_segments: Vec::new(),
            projection_only_protocol_parents: Vec::new(),
            tokens: crate::lexer::lex(source, 0, source.len()),
        }
    }

    pub(super) fn build(mut self) -> Result<Projection, ProgramSyntaxError> {
        self.emit_body(self.core.root)?;
        Ok(Projection {
            arm_blocks: self.arm_blocks,
            code: self.code,
            pending: self.pending,
            source_segments: self.source_segments,
            projection_only_protocol_parents: self.projection_only_protocol_parents,
        })
    }

    fn source_span(&self, node: NodeId) -> Result<SourceSpan, ProgramSyntaxError> {
        self.semantic
            .hir
            .source_map
            .node_span(node)
            .map(SourceSpan::from)
            .ok_or(ProgramSyntaxError::MissingSourceSpan { node })
    }

    fn push_source(&mut self, node: NodeId) -> Result<(), ProgramSyntaxError> {
        let span = self.source_span(node)?;
        let text =
            self.source
                .get(span.start..span.end)
                .ok_or(ProgramSyntaxError::InvalidSourceSpan {
                    start: SourceByte(span.start),
                    end: SourceByte(span.end),
                })?;
        let start = ProjectedByte(self.code.len());
        self.code.push_str(text);
        let end = ProjectedByte(self.code.len());
        self.source_segments.push(ProjectionSourceSegment {
            projected: ProjectedSpan { start, end },
            source: span,
            kind: ProjectionSegmentKind::Copied,
        });
        Ok(())
    }

    fn push_placeholder(
        &mut self,
        category: SyntaxCategory,
        source: SourceSpan,
        core_root: CoreRoot,
    ) -> Result<(), ProgramSyntaxError> {
        let ordinal =
            u32::try_from(self.pending.len()).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
        let id = TtNodeId(ordinal);
        let owner_start = ProjectedByte(self.code.len());
        match category {
            SyntaxCategory::Expression | SyntaxCategory::Propagation => self.code.push('('),
            SyntaxCategory::Statement => self.code.push('{'),
            SyntaxCategory::Item => self.code.push_str("const "),
        }
        let start = ProjectedByte(self.code.len());
        let prefix = match category {
            SyntaxCategory::Expression | SyntaxCategory::Propagation => "$tt_syntax_expr_",
            SyntaxCategory::Statement => "$tt_syntax_stmt_",
            SyntaxCategory::Item => "$tt_syntax_item_",
        };
        self.code.push_str(prefix);
        self.code.push_str(&ordinal.to_string());
        let end = ProjectedByte(self.code.len());
        match category {
            SyntaxCategory::Expression => self.code.push(')'),
            SyntaxCategory::Propagation => self.code.push_str(");"),
            SyntaxCategory::Statement => self.code.push_str(";}"),
            SyntaxCategory::Item => self.code.push_str(" = 0;"),
        }
        let owner_end = ProjectedByte(self.code.len());
        self.source_segments.push(ProjectionSourceSegment {
            projected: ProjectedSpan {
                start: owner_start,
                end: owner_end,
            },
            source,
            kind: ProjectionSegmentKind::Placeholder,
        });
        self.pending.push(PendingOverlay {
            id,
            category,
            source,
            projected: ProjectedSpan { start, end },
            core_root,
            marker: OverlayMarker::Identifier,
            synthetic_return: None,
        });
        Ok(())
    }

    /// Writes the fixed delimiter after a source fragment embedded in a
    /// generated owner and records the fragment as the parse cause when SWC
    /// stops on that delimiter. This is provenance, not diagnostic-message
    /// inference: only a copied segment ending exactly at this boundary can
    /// own it.
    fn push_source_boundary(&mut self, text: &str, segments_since: usize) {
        let start = ProjectedByte(self.code.len());
        let source = self.source_segments[segments_since..]
            .iter()
            .rev()
            .find(|segment| {
                segment.kind == ProjectionSegmentKind::Copied && segment.projected.end == start
            })
            .map(|segment| segment.source);
        self.code.push_str(text);
        if let Some(source) = source {
            self.source_segments.push(ProjectionSourceSegment {
                projected: ProjectedSpan {
                    start,
                    end: ProjectedByte(self.code.len()),
                },
                source: SourceSpan {
                    start: source.end.saturating_sub(1),
                    end: source.end,
                },
                kind: ProjectionSegmentKind::SourceBoundary,
            });
        }
    }

    fn emit_body(&mut self, body: BodyId) -> Result<(), ProgramSyntaxError> {
        for statement in &self.core.bodies[body.index()].statements {
            match statement {
                Statement::Opaque(node) => self.push_source(*node)?,
                Statement::Adt(adt) => self.emit_adt(adt)?,
                Statement::Import(import) => self.emit_import(import)?,
                Statement::Propagate(propagate) => self.emit_propagate(propagate)?,
                Statement::Decision(decision) => self.emit_statement_decision(decision)?,
                Statement::Expr(expr) => self.emit_expr(*expr)?,
            }
        }
        Ok(())
    }

    fn emit_adt(&mut self, adt: &Adt) -> Result<(), ProgramSyntaxError> {
        self.push_placeholder(
            SyntaxCategory::Item,
            self.source_span(adt.node)?,
            CoreRoot::Adt(adt.node),
        )
    }

    fn emit_import(&mut self, import: &Import) -> Result<(), ProgramSyntaxError> {
        self.push_source(import.specifier)
    }

    fn emit_propagate(&mut self, propagate: &Propagate) -> Result<(), ProgramSyntaxError> {
        // A declaration-form propagation can occur in a C-style `for`
        // initializer. Project it as an expression so that header remains
        // valid TypeScript; its typed continuation decides the eventual
        // statement shape in target lowering.
        if self.expr_contains_decision(propagate.value) {
            return self.emit_propagate_with_shadow(propagate);
        }
        self.preserve_concise_arrow_statement_boundary(self.source_span(propagate.owner)?.start);
        self.push_placeholder(
            SyntaxCategory::Propagation,
            self.source_span(propagate.owner)?,
            CoreRoot::Propagate(propagate.node),
        )
    }

    fn preserve_concise_arrow_statement_boundary(&mut self, source_start: usize) {
        let at = self
            .tokens
            .partition_point(|token| token.span.start < source_start);
        if crate::flow::concise_arrow_boundary_before(self.source, &self.tokens, at) {
            self.code.push(';');
        }
    }

    /// Keep the propagation as the statement's primary overlay while also
    /// exposing nested decisions to the TypeScript parent collector. The
    /// comma expression is projection-only; target lowering uses the two
    /// typed overlays to schedule the decision before the propagation.
    fn emit_propagate_with_shadow(
        &mut self,
        propagate: &Propagate,
    ) -> Result<(), ProgramSyntaxError> {
        let source = self.source_span(propagate.owner)?;
        let ordinal =
            u32::try_from(self.pending.len()).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
        let id = TtNodeId(ordinal);
        let pending_index = self.pending.len();
        self.pending.push(PendingOverlay {
            id,
            category: SyntaxCategory::Propagation,
            source,
            projected: ProjectedSpan {
                start: ProjectedByte(0),
                end: ProjectedByte(0),
            },
            core_root: CoreRoot::Propagate(propagate.node),
            marker: OverlayMarker::Identifier,
            synthetic_return: None,
        });
        let owner_start = ProjectedByte(self.code.len());
        self.code.push('(');
        self.emit_shadow_expr(propagate.value)?;
        self.code.push_str(", ");
        let start = ProjectedByte(self.code.len());
        self.code.push_str("$tt_syntax_expr_");
        self.code.push_str(&ordinal.to_string());
        let end = ProjectedByte(self.code.len());
        self.pending[pending_index].projected = ProjectedSpan { start, end };
        self.code.push(')');
        let owner_end = ProjectedByte(self.code.len());
        self.code.push(';');
        self.projection_only_protocol_parents.push(ProjectedSpan {
            start: ProjectedByte(owner_start.0 + 1),
            end: ProjectedByte(owner_end.0 - 1),
        });
        self.projection_only_protocol_parents.push(ProjectedSpan {
            start: owner_start,
            end: owner_end,
        });
        self.source_segments.push(ProjectionSourceSegment {
            projected: ProjectedSpan {
                start: ProjectedByte(owner_start.0 + 1),
                end: ProjectedByte(owner_end.0 - 1),
            },
            source,
            kind: ProjectionSegmentKind::Placeholder,
        });
        self.source_segments.push(ProjectionSourceSegment {
            projected: ProjectedSpan {
                start: owner_start,
                end: ProjectedByte(self.code.len()),
            },
            source,
            kind: ProjectionSegmentKind::Placeholder,
        });
        Ok(())
    }

    fn emit_statement_decision(&mut self, decision: &Decision) -> Result<(), ProgramSyntaxError> {
        self.push_placeholder(
            SyntaxCategory::Statement,
            self.source_span(decision.extent)?,
            CoreRoot::Decision(decision.extent),
        )?;
        self.emit_statement_decision_shadows(decision)
    }

    fn emit_statement_decision_shadows(
        &mut self,
        decision: &Decision,
    ) -> Result<(), ProgramSyntaxError> {
        for arm in &decision.arms {
            if let crate::core_ir::ArmAction::Yield { body, .. }
            | crate::core_ir::ArmAction::Execute(body) = arm.action
            {
                self.emit_shadow_body_island(body)?;
            }
        }
        self.emit_miss_shadow(&decision.miss)
    }

    fn emit_miss_shadow(
        &mut self,
        miss: &crate::core_ir::MissAction,
    ) -> Result<(), ProgramSyntaxError> {
        match miss {
            crate::core_ir::MissAction::Execute(body) => self.emit_shadow_body_island(*body),
            crate::core_ir::MissAction::Decision(decision) => {
                self.emit_statement_decision_shadows(decision)
            }
            crate::core_ir::MissAction::ThrowUnexpected(_)
            | crate::core_ir::MissAction::Nothing => Ok(()),
        }
    }

    fn emit_shadow_body_island(&mut self, body: BodyId) -> Result<(), ProgramSyntaxError> {
        if !self.body_contains_decision(body) {
            return Ok(());
        }
        self.code.push_str("\n(() => {");
        self.emit_shadow_body(body)?;
        self.code.push_str("});");
        Ok(())
    }

    fn emit_expr(&mut self, expr: ExprId) -> Result<(), ProgramSyntaxError> {
        match &self.core.exprs[expr.index()] {
            Expr::Opaque(node) => self.push_source(*node),
            Expr::Sequence(body) => self.emit_body(*body),
            Expr::Decision(decision) => self.emit_decision_region(expr, decision),
            Expr::Propagate(propagate) => self.push_placeholder(
                SyntaxCategory::Expression,
                self.source_span(propagate.node)?,
                CoreRoot::Expr(expr),
            ),
            Expr::Apply(apply) => self.emit_apply(expr, apply),
            Expr::ResultRegion(region) => self.emit_result_region(expr, region),
            Expr::Template(template) => self.emit_template(template),
        }
    }

    fn emit_apply(&mut self, expr: ExprId, apply: &Apply) -> Result<(), ProgramSyntaxError> {
        let mut source = self.source_span(apply.node)?;
        for step in &apply.steps {
            let step_span = self.source_span(step.node)?;
            source.start = source.start.min(step_span.start);
            source.end = source.end.max(step_span.end);
        }
        // SWC has no pipeline syntax, so the pipeline itself remains one
        // expression placeholder.  A step can nevertheless contain a tt
        // value whose TypeScript evaluation structure the pipeline
        // placeholder hides. Project that step beside the placeholder in a
        // valid comma expression so the parent collector retains its arrow
        // or conditional boundary.
        let shadow_steps: Vec<_> = apply
            .steps
            .iter()
            .enumerate()
            .filter(|step| {
                self.expr_contains_propagation(step.1.value)
                    || self.expr_contains_decision(step.1.value)
            })
            .collect();
        let shadow_head = apply.head.filter(|head| {
            self.expr_contains_propagation(*head) || self.expr_contains_decision(*head)
        });
        if shadow_head.is_none() && shadow_steps.is_empty() {
            return self.push_placeholder(SyntaxCategory::Expression, source, CoreRoot::Expr(expr));
        }
        let start = ProjectedByte(self.code.len());
        self.code.push('(');
        self.push_placeholder(SyntaxCategory::Expression, source, CoreRoot::Expr(expr))?;
        if let Some(head) = shadow_head {
            self.code.push_str(", (");
            self.emit_shadow_expr(head)?;
            self.code.push(')');
        }
        for (index, step) in shadow_steps {
            self.code.push_str(", (");
            if let Some(head) = apply.head
                && apply.steps[..=index]
                    .iter()
                    .all(|step| matches!(step.mode, crate::core_ir::ApplyMode::Postfix { .. }))
            {
                self.emit_shadow_expr(head)?;
                for prefix_step in &apply.steps[..=index] {
                    self.emit_shadow_expr(prefix_step.value)?;
                }
            } else {
                self.emit_shadow_expr(step.value)?;
            }
            self.code.push(')');
        }
        self.code.push(')');
        self.projection_only_protocol_parents.push(ProjectedSpan {
            start: ProjectedByte(start.0 + 1),
            end: ProjectedByte(self.code.len() - 1),
        });
        self.projection_only_protocol_parents.push(ProjectedSpan {
            start,
            end: ProjectedByte(self.code.len()),
        });
        self.source_segments.push(ProjectionSourceSegment {
            projected: ProjectedSpan {
                // The comma-expression shadow is the projection of the
                // complete pipeline value. Include its grouping parens so
                // a host edge whose operand retains those parens (notably
                // an assignment RHS) maps to the pipeline's source span.
                start,
                end: ProjectedByte(self.code.len()),
            },
            source,
            kind: ProjectionSegmentKind::Placeholder,
        });
        Ok(())
    }

    fn expr_contains_propagation(&self, expr: ExprId) -> bool {
        match &self.core.exprs[expr.index()] {
            Expr::Propagate(_) => true,
            Expr::Sequence(body) => self.body_contains_propagation(*body),
            Expr::Apply(apply) => apply
                .head
                .is_some_and(|head| self.expr_contains_propagation(head))
                || apply
                    .steps
                    .iter()
                    .any(|step| self.expr_contains_propagation(step.value)),
            Expr::Decision(decision) => decision.subjects.iter().any(|subject| {
                self.expr_contains_propagation(subject.value)
            }) || decision.arms.iter().any(|arm| {
                arm.guard.is_some_and(|guard| self.expr_contains_propagation(guard))
                    || matches!(arm.action, crate::core_ir::ArmAction::Yield { body, .. } | crate::core_ir::ArmAction::Execute(body) if self.core.bodies[body.index()].statements.iter().any(|statement| matches!(statement, Statement::Expr(expr) if self.expr_contains_propagation(*expr))))
            }),
            Expr::ResultRegion(region) => region.items.iter().any(|item| match item {
                crate::core_ir::ResultRegionItem::Statements(body) => {
                    self.body_contains_propagation(*body)
                }
            }) || region.value.is_some_and(|value| self.expr_contains_propagation(value)),
            Expr::Opaque(_) | Expr::Template(_) => false,
        }
    }

    fn body_contains_propagation(&self, body: BodyId) -> bool {
        self.core.bodies[body.index()]
            .statements
            .iter()
            .any(|statement| match statement {
                Statement::Propagate(_) => true,
                Statement::Expr(expr) => self.expr_contains_propagation(*expr),
                Statement::Decision(decision) => {
                    decision
                        .subjects
                        .iter()
                        .any(|subject| self.expr_contains_propagation(subject.value))
                        || decision.arms.iter().any(|arm| match arm.action {
                            crate::core_ir::ArmAction::Yield { body, .. }
                            | crate::core_ir::ArmAction::Execute(body) => {
                                self.body_contains_propagation(body)
                            }
                            crate::core_ir::ArmAction::BindThrough(_) => false,
                        })
                }
                Statement::Opaque(_) | Statement::Adt(_) | Statement::Import(_) => false,
            })
    }

    fn expr_contains_decision(&self, expr: ExprId) -> bool {
        match &self.core.exprs[expr.index()] {
            Expr::Decision(_) => true,
            Expr::Sequence(body) => self.core.bodies[body.index()].statements.iter().any(
                |statement| {
                    matches!(statement, Statement::Expr(expr) if self.expr_contains_decision(*expr))
                },
            ),
            Expr::Apply(apply) => {
                apply
                    .head
                    .is_some_and(|head| self.expr_contains_decision(head))
                    || apply
                        .steps
                        .iter()
                        .any(|step| self.expr_contains_decision(step.value))
            }
            Expr::Propagate(propagate) => self.expr_contains_decision(propagate.value),
            Expr::ResultRegion(region) => {
                region.items.iter().any(|item| match item {
                    crate::core_ir::ResultRegionItem::Statements(body) => self.core.bodies
                        [body.index()]
                    .statements
                    .iter()
                    .any(|statement| {
                        matches!(statement, Statement::Expr(expr) if self.expr_contains_decision(*expr))
                    }),
                }) || region
                    .value
                    .is_some_and(|value| self.expr_contains_decision(value))
            }
            Expr::Template(template) => template.parts.iter().any(|part| {
                matches!(part, TemplatePart::Interpolation(expr) if self.expr_contains_decision(*expr))
            }),
            Expr::Opaque(_) => false,
        }
    }

    fn body_contains_decision(&self, body: BodyId) -> bool {
        self.core.bodies[body.index()]
            .statements
            .iter()
            .any(|statement| match statement {
                Statement::Expr(expr) => self.expr_contains_decision(*expr),
                Statement::Decision(decision) => {
                    decision.arms.iter().any(|arm| match arm.action {
                        crate::core_ir::ArmAction::Yield { body, .. }
                        | crate::core_ir::ArmAction::Execute(body) => {
                            self.body_contains_decision(body)
                        }
                        crate::core_ir::ArmAction::BindThrough(_) => false,
                    }) || match &decision.miss {
                        crate::core_ir::MissAction::Execute(body) => {
                            self.body_contains_decision(*body)
                        }
                        crate::core_ir::MissAction::Decision(decision) => {
                            decision.arms.iter().any(|arm| match arm.action {
                                crate::core_ir::ArmAction::Yield { body, .. }
                                | crate::core_ir::ArmAction::Execute(body) => {
                                    self.body_contains_decision(body)
                                }
                                crate::core_ir::ArmAction::BindThrough(_) => false,
                            })
                        }
                        crate::core_ir::MissAction::ThrowUnexpected(_)
                        | crate::core_ir::MissAction::Nothing => false,
                    }
                }
                Statement::Opaque(_)
                | Statement::Adt(_)
                | Statement::Import(_)
                | Statement::Propagate(_) => false,
            })
    }

    fn emit_shadow_expr(&mut self, expr: ExprId) -> Result<(), ProgramSyntaxError> {
        match &self.core.exprs[expr.index()] {
            Expr::Opaque(node) => self.push_source(*node),
            Expr::Propagate(propagate) => self.push_placeholder(
                SyntaxCategory::Expression,
                self.source_span(propagate.node)?,
                CoreRoot::Expr(expr),
            ),
            Expr::Sequence(body) => self.emit_shadow_body(*body),
            // A nested pipeline is opaque to SWC for the same reason as its
            // parent. Its own projection retains the structural placeholder.
            Expr::Apply(apply) => self.emit_apply(expr, apply),
            Expr::Decision(_) | Expr::ResultRegion(_) | Expr::Template(_) => self.emit_expr(expr),
        }
    }

    fn emit_shadow_body(&mut self, body: BodyId) -> Result<(), ProgramSyntaxError> {
        for statement in &self.core.bodies[body.index()].statements {
            match statement {
                Statement::Opaque(node) => self.push_source(*node)?,
                Statement::Expr(expr) => self.emit_shadow_expr(*expr)?,
                Statement::Propagate(propagate) => self.push_placeholder(
                    SyntaxCategory::Propagation,
                    self.source_span(propagate.owner)?,
                    CoreRoot::Propagate(propagate.node),
                )?,
                Statement::Adt(_) | Statement::Import(_) | Statement::Decision(_) => {
                    return Err(ProgramSyntaxError::InvalidSourceSpan {
                        start: SourceByte(0),
                        end: SourceByte(0),
                    });
                }
            }
        }
        Ok(())
    }

    fn emit_result_region(
        &mut self,
        expr: ExprId,
        region: &ResultRegion,
    ) -> Result<(), ProgramSyntaxError> {
        let source = self.source_span(region.node)?;
        let ordinal =
            u32::try_from(self.pending.len()).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
        let id = TtNodeId(ordinal);
        let start = ProjectedByte(self.code.len());
        let pending_index = self.pending.len();
        self.pending.push(PendingOverlay {
            id,
            category: SyntaxCategory::Expression,
            source,
            projected: ProjectedSpan { start, end: start },
            core_root: CoreRoot::Expr(expr),
            marker: OverlayMarker::CallExpression,
            synthetic_return: None,
        });
        self.code.push('(');
        if region.is_async {
            self.code.push_str("async ");
        }
        self.code.push_str("() => {");
        for item in &region.items {
            match item {
                crate::core_ir::ResultRegionItem::Statements(body) => {
                    self.emit_result_body(*body)?
                }
            }
        }
        let synthetic_return_start = ProjectedByte(self.code.len());
        self.code.push_str("return ");
        if let Some(value) = region.value {
            self.emit_expr(value)?;
        } else {
            self.code.push_str("undefined");
        }
        self.code.push_str(";})()");
        let end = ProjectedByte(self.code.len());
        let projected = ProjectedSpan { start, end };
        self.source_segments.insert(
            0,
            ProjectionSourceSegment {
                projected,
                source,
                kind: ProjectionSegmentKind::Placeholder,
            },
        );
        self.pending[pending_index].projected = projected;
        self.pending[pending_index].synthetic_return = Some(ProjectedSpan {
            start: synthetic_return_start,
            end: ProjectedByte(self.code.len() - 4),
        });
        Ok(())
    }

    /// Result-owned `return` exits remain visible to the host projection
    /// through inline let-else and if-let bodies. Ordinary statement
    /// projection uses one placeholder for those tt decisions, but that
    /// would hide their source returns from the enclosing Result arrow.
    fn emit_result_body(&mut self, body: BodyId) -> Result<(), ProgramSyntaxError> {
        for statement in &self.core.bodies[body.index()].statements {
            match statement {
                Statement::Decision(decision) => self.emit_result_decision(decision)?,
                _ => self.emit_body_fragment(statement)?,
            }
        }
        Ok(())
    }

    fn emit_body_fragment(&mut self, statement: &Statement) -> Result<(), ProgramSyntaxError> {
        match statement {
            Statement::Opaque(node) => self.push_source(*node),
            Statement::Adt(adt) => self.emit_adt(adt),
            Statement::Import(import) => self.emit_import(import),
            Statement::Propagate(propagate) => self.emit_propagate(propagate),
            Statement::Decision(_) => unreachable!("Result body handles decisions separately"),
            Statement::Expr(expr) => self.emit_expr(*expr),
        }
    }

    fn emit_result_decision(&mut self, decision: &Decision) -> Result<(), ProgramSyntaxError> {
        match &decision.kind {
            crate::core_ir::DecisionKind::LetElse { .. } => {
                let crate::core_ir::MissAction::Execute(body) = decision.miss else {
                    crate::ice::bug!("let-else has no else body")
                };
                self.code.push_str("if (true) {");
                self.emit_result_body(body)?;
                self.code.push('}');
            }
            crate::core_ir::DecisionKind::IfLet => {
                let crate::core_ir::ArmAction::Execute(body) = decision.arms[0].action else {
                    crate::ice::bug!("if-let has no then body")
                };
                self.code.push_str("if (true) {");
                self.emit_result_body(body)?;
                self.code.push('}');
                match &decision.miss {
                    crate::core_ir::MissAction::Execute(body) => {
                        self.code.push_str(" else {");
                        self.emit_result_body(*body)?;
                        self.code.push('}');
                    }
                    crate::core_ir::MissAction::Decision(inner) => {
                        self.code.push_str(" else ");
                        self.emit_result_decision(inner)?;
                    }
                    crate::core_ir::MissAction::Nothing => {}
                    crate::core_ir::MissAction::ThrowUnexpected(_) => {
                        crate::ice::bug!("if-let has match miss action")
                    }
                }
            }
            crate::core_ir::DecisionKind::Match { .. } => {
                crate::ice::bug!("expression decision in Result statement body")
            }
        }
        Ok(())
    }

    fn emit_decision_region(
        &mut self,
        expr: ExprId,
        decision: &Decision,
    ) -> Result<(), ProgramSyntaxError> {
        let source = self.source_span(decision.extent)?;
        let ordinal =
            u32::try_from(self.pending.len()).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
        let id = TtNodeId(ordinal);
        let start = ProjectedByte(self.code.len());
        let pending_index = self.pending.len();
        self.pending.push(PendingOverlay {
            id,
            category: SyntaxCategory::Expression,
            source,
            projected: ProjectedSpan { start, end: start },
            core_root: CoreRoot::Expr(expr),
            marker: OverlayMarker::DecisionCallExpression,
            synthetic_return: None,
        });
        self.code.push('(');
        if decision.is_async {
            self.code.push_str("async ");
        }
        self.code.push_str("() => {");
        for subject in &decision.subjects {
            self.code.push('(');
            let segments_since = self.source_segments.len();
            self.emit_expr(subject.value)?;
            self.push_source_boundary(");", segments_since);
        }
        for arm in &decision.arms {
            if let Some(guard) = arm.guard {
                self.code.push('(');
                let segments_since = self.source_segments.len();
                self.emit_expr(guard)?;
                self.push_source_boundary(");", segments_since);
            }
            let crate::core_ir::ArmAction::Yield { body, kind } = arm.action else {
                continue;
            };
            match kind {
                hir::ArmBodyKind::Expression => {
                    self.code.push('(');
                    let segments_since = self.source_segments.len();
                    self.emit_body(body)?;
                    self.push_source_boundary(");", segments_since);
                }
                hir::ArmBodyKind::Block { .. } => {
                    let start = ProjectedByte(self.code.len());
                    self.code.push('{');
                    let segments_since = self.source_segments.len();
                    self.emit_body(body)?;
                    self.push_source_boundary("}", segments_since);
                    self.arm_blocks.insert(
                        ProjectedSpan {
                            start,
                            end: ProjectedByte(self.code.len()),
                        },
                        body,
                    );
                }
            }
        }
        self.code.push_str("0;})()");
        let end = ProjectedByte(self.code.len());
        let projected = ProjectedSpan { start, end };
        self.source_segments.insert(
            0,
            ProjectionSourceSegment {
                projected,
                source,
                kind: ProjectionSegmentKind::Placeholder,
            },
        );
        self.pending[pending_index].projected = projected;
        Ok(())
    }

    fn emit_template(&mut self, template: &Template) -> Result<(), ProgramSyntaxError> {
        for part in &template.parts {
            match part {
                TemplatePart::Raw(node) => self.push_source(*node)?,
                TemplatePart::Interpolation(expr) => {
                    self.code.push_str("${");
                    let segments_since = self.source_segments.len();
                    self.emit_expr(*expr)?;
                    self.push_source_boundary("}", segments_since);
                }
            }
        }
        Ok(())
    }
}
