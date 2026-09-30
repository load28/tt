//! SWC AST-path visitor implementation.

use super::*;

impl ParentCollector {
    /// The declared annotation of the value a `return` in this function
    /// delivers, or `None` when the declaration does not name that type.
    ///
    /// Lowering copies the annotation onto the slot a `return match ...`
    /// assigns through, which is sound only while the declared return type
    /// *is* the returned value's type. A generator declares the iterator it
    /// produces, whose returned value is one of its type arguments, and a
    /// type predicate or assertion signature is not a type at all — both
    /// leave the slot to be inferred from the value it is assigned.
    fn returned_value_type(
        &self,
        annotation: Option<&TsTypeAnn>,
        is_generator: bool,
    ) -> Option<ProjectedSpan> {
        if is_generator {
            return None;
        }
        let annotation = annotation?;
        if matches!(*annotation.type_ann, TsType::TsTypePredicate(_)) {
            return None;
        }
        Some(projected_span(annotation.span, self.source_start))
    }

    pub(super) fn new(
        source_start: HostOrigin,
        pending: &[PendingOverlay],
        source_segments: &[ProjectionSourceSegment],
        projection_only_protocol_parents: &[ProjectedSpan],
        arm_blocks: &HashMap<ProjectedSpan, BodyId>,
        tt_bindings: &projection::TtBindings,
    ) -> Self {
        let expected_identifiers = pending
            .iter()
            .filter(|entry| entry.marker == OverlayMarker::Identifier)
            .map(|entry| (entry.projected, entry.id))
            .collect();
        let expected_calls = pending
            .iter()
            .filter(|entry| {
                matches!(
                    entry.marker,
                    OverlayMarker::CallExpression | OverlayMarker::DecisionCallExpression
                )
            })
            .map(|entry| (entry.projected, entry.id))
            .collect();
        let expected_exit_calls = pending
            .iter()
            .filter(|entry| {
                matches!(
                    entry.marker,
                    OverlayMarker::CallExpression | OverlayMarker::DecisionCallExpression
                )
            })
            .map(|entry| entry.id)
            .collect();
        let decision_calls = pending
            .iter()
            .filter(|entry| entry.marker == OverlayMarker::DecisionCallExpression)
            .map(|entry| entry.projected)
            .collect();
        let synthetic_returns = pending
            .iter()
            .filter_map(|entry| entry.synthetic_return)
            .collect();
        let placeholders = pending.iter().map(|entry| entry.projected).collect();
        Self {
            placeholders,
            arm_blocks: arm_blocks.clone(),
            tt_bindings: tt_bindings.clone(),
            single_return_bodies: HashMap::new(),
            source_start,
            expected_identifiers,
            expected_calls,
            expected_exit_calls,
            synthetic_returns,
            found: HashMap::new(),
            duplicates: Vec::new(),
            source_segments: ProjectionSegments::new(source_segments.to_vec()),
            projection_only_protocol_parents: projection_only_protocol_parents
                .iter()
                .copied()
                .collect(),
            host_owners: Vec::new(),
            protocol_frames: Vec::new(),
            occupied_names: HashSet::new(),
            function_depth: 0,
            function_targets: Vec::new(),
            decision_calls,
            decision_functions: HashSet::new(),
            contextual_types: Vec::new(),
            assertions: Vec::new(),
            function_return_types: Vec::new(),
            function_return_async: Vec::new(),
            break_capture_depth: 0,
            exit_regions: Vec::new(),
            arm_block_scopes: Vec::new(),
            global_statements: HashMap::new(),
        }
    }

    fn loop_head_reads(&self, head: &swc_ecma_ast::ForStmt) -> bool {
        let Some(swc_ecma_ast::VarDeclOrExpr::VarDecl(declaration)) = &head.init else {
            return false;
        };
        if declaration.kind == swc_ecma_ast::VarDeclKind::Var {
            return false;
        }
        let Some(initializer) = declaration
            .decls
            .first()
            .and_then(|declarator| declarator.init.as_deref())
        else {
            return false;
        };
        let mut names = Vec::new();
        for declarator in &declaration.decls {
            scopes::pattern_names(&declarator.name, &mut names);
        }
        let names = names.into_iter().collect();
        scopes::reads_outer(initializer, &names, &self.tt_bindings, &|position| {
            self.source_start.byte(position)
        })
    }

    /// The function target a function's body gives what it holds: its own,
    /// or for a decision's stand-in function, the target around the match.
    fn function_target_of(&self, span: swc_common::Span, own: EvaluationOwner) -> EvaluationOwner {
        if self
            .decision_functions
            .contains(&projected_span(span, self.source_start))
        {
            self.function_targets.last().copied().unwrap_or(own)
        } else {
            own
        }
    }

    pub(super) fn record_overlay(&mut self, id: TtNodeId, path: &AstNodePath<'_>) {
        let ambient = path.iter().any(|parent| {
            matches!(parent, swc_ecma_visit::AstParentNodeRef::TsModuleDecl(decl, _) if decl.declare)
        });
        let decorated_classes = path
            .iter()
            .enumerate()
            .filter_map(|(index, parent)| match parent {
                swc_ecma_visit::AstParentNodeRef::Class(class, _)
                    if !class.decorators.is_empty() =>
                {
                    Some(index)
                }
                _ => None,
            })
            .collect();
        let decision_functions = path
            .iter()
            .enumerate()
            .filter_map(|(index, parent)| {
                let span = match parent {
                    swc_ecma_visit::AstParentNodeRef::ArrowExpr(
                        arrow,
                        swc_ecma_visit::fields::ArrowExprField::Body,
                    ) => arrow.span,
                    swc_ecma_visit::AstParentNodeRef::Function(
                        function,
                        swc_ecma_visit::fields::FunctionField::Body,
                    ) => function.span,
                    _ => return None,
                };
                self.decision_functions
                    .contains(&projected_span(span, self.source_start))
                    .then_some(index)
            })
            .collect();
        let loop_head_reads = path
            .iter()
            .rev()
            .find_map(|parent| match parent {
                swc_ecma_visit::AstParentNodeRef::ForStmt(
                    head,
                    swc_ecma_visit::fields::ForStmtField::Init,
                ) => Some(*head),
                _ => None,
            })
            .is_some_and(|head| self.loop_head_reads(head));
        if self
            .found
            .insert(
                id,
                FoundOverlay {
                    loop_head_reads,
                    ambient,
                    decorated_classes,
                    decision_functions,
                    parents: path.kinds().to_vec(),
                    host_owners: self.host_owners.clone(),
                    protocol_frames: self.protocol_frames.clone(),
                    exits: Vec::new(),
                    function_target: self.function_targets.last().copied(),
                    contextual_type: self.contextual_types.last().copied().flatten(),
                    assertion: self.assertions.last().copied(),
                    function_return_type: self.function_return_types.last().copied().flatten(),
                    function_return_awaited: self
                        .function_return_async
                        .last()
                        .copied()
                        .unwrap_or(false),
                },
            )
            .is_some()
        {
            self.duplicates.push(id);
        }
    }

    pub(super) fn finish(
        mut self,
        pending: Vec<PendingOverlay>,
    ) -> Result<CollectedProgramSyntax, ProgramSyntaxError> {
        if let Some(id) = self.duplicates.into_iter().next() {
            return Err(ProgramSyntaxError::DuplicateOverlay { id });
        }
        let mut owner_ids: HashMap<ProjectedHostOwner, HostOwnerId> = HashMap::new();
        let mut owners: Vec<HostOwnerSyntax> = Vec::new();
        let mut globals = HashMap::new();
        let mut overlay: Vec<OverlayEntry> = Vec::with_capacity(pending.len());
        let mut owner_sources: HashMap<ProjectedSpan, Option<SourceSpan>> = HashMap::new();
        let overlay_spans: Vec<_> = pending
            .iter()
            .map(|entry| (entry.id, entry.projected))
            .collect();
        let overlay_index = crate::span_index::SpanIndex::new(
            overlay_spans
                .iter()
                .map(|(_, span)| (span.start.0, span.end.0)),
        );
        for entry in &pending {
            let found = self
                .found
                .remove(&entry.id)
                .ok_or(ProgramSyntaxError::MissingOverlay { id: entry.id })?;
            let (owner_index, projected_owner, kind, span) = found
                .host_owners
                .iter()
                .enumerate()
                .rev()
                .filter(|(_, owner)| {
                    owner.span.start <= entry.projected.start
                        && entry.projected.end <= owner.span.end
                })
                .find_map(|(index, owner)| {
                    owner_source(&mut owner_sources, &self.source_segments, owner.span)
                        .map(|span| (index, *owner, owner.kind, span))
                })
                .ok_or(ProgramSyntaxError::MissingOverlay { id: entry.id })?;
            let projected_anchor =
                prelude_anchor(&found.host_owners[..=owner_index], &found.parents);
            let anchor = owner_source(
                &mut owner_sources,
                &self.source_segments,
                projected_anchor.span,
            )
            .ok_or(ProgramSyntaxError::MissingOverlay { id: entry.id })?;
            let statement_index = found.host_owners[..=owner_index]
                .iter()
                .rposition(|owner| owner.kind != HostOwnerKind::Declarator)
                .unwrap_or(owner_index);
            let projected_statement =
                prelude_anchor(&found.host_owners[..=statement_index], &found.parents);
            let statement = owner_source(
                &mut owner_sources,
                &self.source_segments,
                projected_statement.span,
            )
            .ok_or(ProgramSyntaxError::MissingOverlay { id: entry.id })?;
            let split = projected_owner
                .split
                .map(|split| {
                    source_span_for_projection(&self.source_segments, split.previous)
                        .map(|previous| DeclaratorSplit {
                            previous_end: previous.end,
                            kind: split.kind,
                            exported: split.exported,
                            declared: split.declared,
                        })
                        .ok_or(ProgramSyntaxError::MissingOverlay { id: entry.id })
                })
                .transpose()?;
            let requires_block = projected_statement.kind == HostOwnerKind::Statement
                && is_unbraced_body(
                    &found.parents[..projected_statement.edge.min(found.parents.len())],
                );
            let owner_id = if let Some(owner_id) = owner_ids.get(&projected_owner).copied() {
                owner_id
            } else {
                let owner_id = HostOwnerId(
                    u32::try_from(owners.len())
                        .map_err(|_| ProgramSyntaxError::NodeCountOverflow)?,
                );
                owner_ids.insert(projected_owner, owner_id);
                owners.push(HostOwnerSyntax {
                    owner: HostOwner {
                        id: owner_id,
                        kind,
                        span,
                        anchor_start: anchor.start,
                        statement,
                        split,
                    },
                    roots: Vec::new(),
                });
                owner_id
            };
            owners[owner_id.0 as usize].roots.push(entry.id);
            if let Some(global) = self.global_statements.get(&projected_statement.span) {
                globals.insert(
                    owners[owner_id.0 as usize].owner.statement(),
                    global.clone(),
                );
            }
            let enclosing_overlay = overlay_index
                .covering(entry.projected.start.0, entry.projected.end.0)
                .into_iter()
                .map(|index| &overlay_spans[index])
                .filter(|(id, span)| {
                    *id != entry.id
                        && span.start <= entry.projected.start
                        && entry.projected.end <= span.end
                })
                .min_by_key(|(_, span)| span.end.0 - span.start.0)
                .map(|(_, span)| *span);
            overlay.push(OverlayEntry {
                id: entry.id,
                category: entry.category,
                source: entry.source,
                projected: entry.projected,
                context: EvaluationContext::from_path(
                    entry.category,
                    &found.parents,
                    projected_owner.edge,
                    requires_block,
                    OverlayFacts {
                        function_target: found.function_target,
                        contextual_type: found
                            .contextual_type
                            .map(|span| map_evaluation_span(&self.source_segments, span))
                            .transpose()?,
                        function_return_type: found
                            .function_return_type
                            .map(|span| map_evaluation_span(&self.source_segments, span))
                            .transpose()?,
                        loop_head_reads: found.loop_head_reads,
                        assertion: found
                            .assertion
                            .map(|span| {
                                span.map(|span| map_evaluation_span(&self.source_segments, span))
                                    .transpose()
                            })
                            .transpose()?,
                        function_return_awaited: found.function_return_awaited,
                        ambient: found.ambient,
                        decorated_classes: found.decorated_classes,
                        decision_functions: found.decision_functions,
                        value_is_owner: span == entry.source,
                    },
                ),
                // A frame outside the host owner is not this owner's
                // evaluation obligation: a statement (or a concise arrow
                // body) can sit inside an outer expression only across a
                // function boundary, and the rewrite happens where the owner
                // executes, not where the enclosing expression does.
                protocol: evaluation_protocol(
                    &self.source_segments,
                    entry.projected,
                    entry.source,
                    &found
                        .protocol_frames
                        .iter()
                        .filter(|frame| {
                            projected_contains(projected_owner.span, frame.parent())
                                && !self
                                    .projection_only_protocol_parents
                                    .contains(&frame.parent())
                                // A source operation outside an enclosing TT
                                // value belongs to that value's protocol. If
                                // the nested value inherited it as well, both
                                // lowering schedules would own and emit the
                                // same source range.
                                && !enclosing_overlay.is_some_and(|ancestor| {
                                    projected_contains(frame.parent(), ancestor)
                                })
                                && (!matches!(frame, ProjectedProtocolFrame::LoopTest { .. })
                                    || entry.marker == OverlayMarker::DecisionCallExpression)
                        })
                        .cloned()
                        .collect::<Vec<_>>(),
                )?,
                core_root: entry.core_root,
                parents: found.parents,
                host_owner: owners[owner_id.0 as usize].owner,
                exits: found
                    .exits
                    .into_iter()
                    .map(|exit| {
                        Ok(HostExit {
                            body: exit.body,
                            call_safe: exit.call_safe,
                            single_return_body: exit.single_return_body,
                            statement: map_evaluation_span(&self.source_segments, exit.statement)?,
                            argument: exit
                                .argument
                                .map(|argument| {
                                    map_structural_span(&self.source_segments, argument)
                                })
                                .transpose()?,
                            value_argument: exit
                                .value_argument
                                .map(|argument| {
                                    if let Some(value) =
                                        pending.iter().find(|value| value.projected == argument)
                                    {
                                        Ok(value.source)
                                    } else {
                                        map_structural_span(&self.source_segments, argument)
                                    }
                                })
                                .transpose()?,
                            captured_break: exit.captured_break,
                            requires_block: exit.requires_block,
                        })
                    })
                    .collect::<Result<Vec<_>, ProgramSyntaxError>>()?,
            });
        }
        Ok(CollectedProgramSyntax {
            overlay,
            owners,
            occupied_names: self.occupied_names,
            globals,
        })
    }
}

fn owner_source(
    cache: &mut HashMap<ProjectedSpan, Option<SourceSpan>>,
    segments: &ProjectionSegments,
    owner: ProjectedSpan,
) -> Option<SourceSpan> {
    *cache
        .entry(owner)
        .or_insert_with(|| source_span_for_projection(segments, owner))
}

impl VisitAstPath for ParentCollector {
    fn visit_expr<'ast: 'r, 'r>(
        &mut self,
        node: &'ast swc_ecma_ast::Expr,
        path: &mut AstNodePath<'r>,
    ) {
        crate::stack::grow(|| {
            <swc_ecma_ast::Expr as VisitWithAstPath<Self>>::visit_children_with_ast_path(
                node, self, path,
            );
        });
    }

    fn visit_var_declarator<'ast: 'r, 'r>(
        &mut self,
        node: &'ast VarDeclarator,
        path: &mut AstNodePath<'r>,
    ) {
        let annotation = match &node.name {
            Pat::Ident(pattern) => pattern.type_ann.as_deref(),
            Pat::Array(pattern) => pattern.type_ann.as_deref(),
            Pat::Object(pattern) => pattern.type_ann.as_deref(),
            Pat::Rest(pattern) => pattern.type_ann.as_deref(),
            Pat::Assign(pattern) => match pattern.left.as_ref() {
                Pat::Ident(pattern) => pattern.type_ann.as_deref(),
                Pat::Array(pattern) => pattern.type_ann.as_deref(),
                Pat::Object(pattern) => pattern.type_ann.as_deref(),
                Pat::Rest(pattern) => pattern.type_ann.as_deref(),
                Pat::Assign(_) | Pat::Invalid(_) | Pat::Expr(_) => None,
            },
            Pat::Invalid(_) | Pat::Expr(_) => None,
        }
        .map(|annotation| projected_span(annotation.span, self.source_start));
        self.contextual_types.push(annotation);
        let split = declarator_split(path, self.source_start);
        if let Some(split) = split {
            self.host_owners.push(ProjectedHostOwner {
                kind: HostOwnerKind::Declarator,
                span: projected_span(node.span, self.source_start),
                edge: path.kinds().len(),
                split: Some(split),
            });
        }
        <VarDeclarator as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        if split.is_some() {
            self.host_owners.pop();
        }
        self.contextual_types.pop();
    }

    fn visit_module_item<'ast: 'r, 'r>(
        &mut self,
        item: &'ast ModuleItem,
        path: &mut AstNodePath<'r>,
    ) {
        self.host_owners.push(ProjectedHostOwner {
            kind: HostOwnerKind::ModuleItem,
            span: projected_span(item.span(), self.source_start),
            edge: path.kinds().len(),
            split: None,
        });
        <ModuleItem as VisitWithAstPath<Self>>::visit_children_with_ast_path(item, self, path);
        self.host_owners.pop();
    }

    fn visit_stmt<'ast: 'r, 'r>(&mut self, statement: &'ast Stmt, path: &mut AstNodePath<'r>) {
        self.host_owners.push(ProjectedHostOwner {
            kind: HostOwnerKind::Statement,
            span: projected_span(statement.span(), self.source_start),
            edge: path.kinds().len(),
            split: None,
        });
        <Stmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(statement, self, path);
        self.host_owners.pop();
    }

    fn visit_array_lit<'ast: 'r, 'r>(&mut self, node: &'ast ArrayLit, path: &mut AstNodePath<'r>) {
        self.protocol_frames.push(ProjectedProtocolFrame::Ordered {
            parent: projected_span(node.span, self.source_start),
            positions: node
                .elems
                .iter()
                .flatten()
                .map(|element| {
                    (
                        operand_span(
                            &element.expr,
                            self.source_start,
                            &self.placeholders,
                            &self.source_segments,
                        ),
                        expression_effects(&element.expr),
                    )
                })
                .collect(),
            kind: OrderedEvaluationKind::Array,
            spread_free: node.elems.iter().flatten().all(|e| e.spread.is_none()),
        });
        <ArrayLit as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_object_lit<'ast: 'r, 'r>(
        &mut self,
        node: &'ast ObjectLit,
        path: &mut AstNodePath<'r>,
    ) {
        self.protocol_frames.push(ProjectedProtocolFrame::Ordered {
            parent: projected_span(node.span, self.source_start),
            positions: object_evaluation_positions(
                node,
                self.source_start,
                &self.placeholders,
                &self.source_segments,
            ),
            kind: OrderedEvaluationKind::Object,
            spread_free: !node
                .props
                .iter()
                .any(|property| matches!(property, PropOrSpread::Spread(_))),
        });
        <ObjectLit as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_assign_expr<'ast: 'r, 'r>(
        &mut self,
        node: &'ast AssignExpr,
        path: &mut AstNodePath<'r>,
    ) {
        self.protocol_frames
            .push(ProjectedProtocolFrame::Assignment {
                parent: projected_span(node.span, self.source_start),
                operator: node.op,
                target: projected_span(node.left.span(), self.source_start),
                reference: assignment_reference(&node.left)
                    .map(|expression| {
                        (
                            operand_span(
                                expression,
                                self.source_start,
                                &self.placeholders,
                                &self.source_segments,
                            ),
                            expression_effects(expression),
                        )
                    })
                    .collect(),
                right: operand_span(
                    &node.right,
                    self.source_start,
                    &self.placeholders,
                    &self.source_segments,
                ),
            });
        <AssignExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_seq_expr<'ast: 'r, 'r>(&mut self, node: &'ast SeqExpr, path: &mut AstNodePath<'r>) {
        self.protocol_frames.push(ProjectedProtocolFrame::Ordered {
            parent: projected_span(node.span, self.source_start),
            positions: node
                .exprs
                .iter()
                .map(|expression| {
                    (
                        operand_span(
                            expression,
                            self.source_start,
                            &self.placeholders,
                            &self.source_segments,
                        ),
                        expression_effects(expression),
                    )
                })
                .collect(),
            kind: OrderedEvaluationKind::Sequence,
            spread_free: true,
        });
        <SeqExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_unary_expr<'ast: 'r, 'r>(
        &mut self,
        node: &'ast UnaryExpr,
        path: &mut AstNodePath<'r>,
    ) {
        self.protocol_frames.push(ProjectedProtocolFrame::Ordered {
            parent: projected_span(node.span, self.source_start),
            positions: vec![(
                operand_span(
                    &node.arg,
                    self.source_start,
                    &self.placeholders,
                    &self.source_segments,
                ),
                expression_effects(&node.arg),
            )],
            kind: OrderedEvaluationKind::Unary,
            spread_free: true,
        });
        <UnaryExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_bin_expr<'ast: 'r, 'r>(&mut self, node: &'ast BinExpr, path: &mut AstNodePath<'r>) {
        self.protocol_frames.push(ProjectedProtocolFrame::Binary {
            parent: projected_span(node.span, self.source_start),
            operator: node.op,
            left: (
                operand_span(
                    &node.left,
                    self.source_start,
                    &self.placeholders,
                    &self.source_segments,
                ),
                expression_effects(&node.left),
            ),
            right: operand_span(
                &node.right,
                self.source_start,
                &self.placeholders,
                &self.source_segments,
            ),
        });
        <BinExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_cond_expr<'ast: 'r, 'r>(&mut self, node: &'ast CondExpr, path: &mut AstNodePath<'r>) {
        self.protocol_frames
            .push(ProjectedProtocolFrame::Conditional {
                parent: projected_span(node.span, self.source_start),
                test: (
                    operand_span(
                        &node.test,
                        self.source_start,
                        &self.placeholders,
                        &self.source_segments,
                    ),
                    expression_effects(&node.test),
                ),
                consequent: operand_span(
                    &node.cons,
                    self.source_start,
                    &self.placeholders,
                    &self.source_segments,
                ),
                alternate: operand_span(
                    &node.alt,
                    self.source_start,
                    &self.placeholders,
                    &self.source_segments,
                ),
            });
        <CondExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_call_expr<'ast: 'r, 'r>(&mut self, node: &'ast CallExpr, path: &mut AstNodePath<'r>) {
        let span = projected_span(node.span, self.source_start);
        if self.decision_calls.contains(&span)
            && let swc_ecma_ast::Callee::Expr(callee) = &node.callee
        {
            let mut callee = &**callee;
            while let swc_ecma_ast::Expr::Paren(inner) = callee {
                callee = &inner.expr;
            }
            match callee {
                swc_ecma_ast::Expr::Arrow(arrow) => {
                    self.decision_functions
                        .insert(projected_span(arrow.span, self.source_start));
                }
                swc_ecma_ast::Expr::Fn(function) => {
                    self.decision_functions
                        .insert(projected_span(function.function.span, self.source_start));
                }
                _ => {}
            }
        }
        if let Some(id) = self.expected_calls.get(&span).copied() {
            self.record_overlay(id, path);
            let collects_exits = self.expected_exit_calls.contains(&id);
            if collects_exits {
                self.exit_regions
                    .push((id, self.function_depth + 1, self.break_capture_depth));
            }
            <CallExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
            if collects_exits {
                self.exit_regions.pop();
            }
            return;
        }
        let (callee_mode, callee_parts) = match &node.callee {
            swc_ecma_ast::Callee::Expr(expression) => call_callee_mode(expression),
            swc_ecma_ast::Callee::Super(_) | swc_ecma_ast::Callee::Import(_) => {
                (EvaluationInputMode::MemberReference, [None, None])
            }
        };
        self.protocol_frames.push(ProjectedProtocolFrame::Call {
            discarded: path
                .kinds()
                .iter()
                .rev()
                .find(|kind| !matches!(kind, AstParentKind::Expr(fields::ExprField::Call)))
                .is_some_and(|kind| matches!(kind, AstParentKind::ExprStmt(_))),
            parent: span,
            callee: Some(projected_span(
                match &node.callee {
                    // A method is captured with the TypeScript wrappers
                    // around it (`o.m!`, `(o.m as F)`), so the call binds
                    // the method as its author typed it.
                    swc_ecma_ast::Callee::Expr(expression)
                        if callee_mode == EvaluationInputMode::MemberReference =>
                    {
                        expression.span()
                    }
                    swc_ecma_ast::Callee::Expr(expression) => reference_value_span(expression),
                    swc_ecma_ast::Callee::Super(_) | swc_ecma_ast::Callee::Import(_) => {
                        node.callee.span()
                    }
                },
                self.source_start,
            )),
            callee_mode,
            callee_reference: (callee_mode == EvaluationInputMode::MemberReference).then(|| {
                projected_member_reference(callee_parts, self.source_start, &self.source_segments)
            }),
            arguments: argument_positions(
                &node.args,
                self.source_start,
                &self.placeholders,
                &self.source_segments,
            ),
            type_args: node
                .type_args
                .as_ref()
                .map(|args| projected_span(args.span(), self.source_start)),
            optional: None,
        });
        <CallExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_for_stmt<'ast: 'r, 'r>(
        &mut self,
        node: &'ast swc_ecma_ast::ForStmt,
        path: &mut AstNodePath<'r>,
    ) {
        self.break_capture_depth += 1;
        if let Some(test) = &node.test {
            self.protocol_frames.push(ProjectedProtocolFrame::LoopTest {
                parent: projected_span(node.span, self.source_start),
                kind: LoopTestKind::For,
                test: projected_span(test.span(), self.source_start),
                body: projected_span(node.body.span(), self.source_start),
                update: node
                    .update
                    .as_ref()
                    .map(|update| projected_span(update.span(), self.source_start)),
            });
        }
        <swc_ecma_ast::ForStmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(
            node, self, path,
        );
        if node.test.is_some() {
            self.protocol_frames.pop();
        }
        self.break_capture_depth -= 1;
    }

    fn visit_for_in_stmt<'ast: 'r, 'r>(
        &mut self,
        node: &'ast swc_ecma_ast::ForInStmt,
        path: &mut AstNodePath<'r>,
    ) {
        self.break_capture_depth += 1;
        <swc_ecma_ast::ForInStmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(
            node, self, path,
        );
        self.break_capture_depth -= 1;
    }

    fn visit_for_of_stmt<'ast: 'r, 'r>(
        &mut self,
        node: &'ast swc_ecma_ast::ForOfStmt,
        path: &mut AstNodePath<'r>,
    ) {
        self.break_capture_depth += 1;
        <swc_ecma_ast::ForOfStmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(
            node, self, path,
        );
        self.break_capture_depth -= 1;
    }

    fn visit_while_stmt<'ast: 'r, 'r>(
        &mut self,
        node: &'ast swc_ecma_ast::WhileStmt,
        path: &mut AstNodePath<'r>,
    ) {
        self.break_capture_depth += 1;
        self.protocol_frames.push(ProjectedProtocolFrame::LoopTest {
            parent: projected_span(node.span, self.source_start),
            kind: LoopTestKind::While,
            test: operand_span(
                &node.test,
                self.source_start,
                &self.placeholders,
                &self.source_segments,
            ),
            body: projected_span(node.body.span(), self.source_start),
            update: None,
        });
        <swc_ecma_ast::WhileStmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(
            node, self, path,
        );
        self.protocol_frames.pop();
        self.break_capture_depth -= 1;
    }

    fn visit_do_while_stmt<'ast: 'r, 'r>(
        &mut self,
        node: &'ast swc_ecma_ast::DoWhileStmt,
        path: &mut AstNodePath<'r>,
    ) {
        self.break_capture_depth += 1;
        <swc_ecma_ast::DoWhileStmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(
            node, self, path,
        );
        self.break_capture_depth -= 1;
    }

    fn visit_switch_stmt<'ast: 'r, 'r>(
        &mut self,
        node: &'ast swc_ecma_ast::SwitchStmt,
        path: &mut AstNodePath<'r>,
    ) {
        self.break_capture_depth += 1;
        <swc_ecma_ast::SwitchStmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(
            node, self, path,
        );
        self.break_capture_depth -= 1;
    }

    fn visit_arrow_expr<'ast: 'r, 'r>(
        &mut self,
        node: &'ast ArrowExpr,
        path: &mut AstNodePath<'r>,
    ) {
        self.function_depth += 1;
        let target = self.function_target_of(node.span, EvaluationOwner::FunctionBody);
        self.function_targets.push(target);
        self.contextual_types.push(None);
        self.function_return_types
            .push(self.returned_value_type(node.return_type.as_deref(), false));
        self.function_return_async.push(node.is_async);
        self.host_owners.push(ProjectedHostOwner {
            kind: HostOwnerKind::ArrowExpression,
            span: projected_span(node.body.span(), self.source_start),
            edge: path.kinds().len(),
            split: None,
        });
        <ArrowExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.host_owners.pop();
        self.function_return_types.pop();
        self.function_return_async.pop();
        self.contextual_types.pop();
        self.function_targets.pop();
        self.function_depth -= 1;
    }

    fn visit_ts_as_expr<'ast: 'r, 'r>(
        &mut self,
        node: &'ast swc_ecma_ast::TsAsExpr,
        path: &mut AstNodePath<'r>,
    ) {
        self.assertions.push(None);
        <swc_ecma_ast::TsAsExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(
            node, self, path,
        );
        self.assertions.pop();
    }

    fn visit_ts_type_assertion<'ast: 'r, 'r>(
        &mut self,
        node: &'ast swc_ecma_ast::TsTypeAssertion,
        path: &mut AstNodePath<'r>,
    ) {
        self.assertions.push(None);
        <swc_ecma_ast::TsTypeAssertion as VisitWithAstPath<Self>>::visit_children_with_ast_path(
            node, self, path,
        );
        self.assertions.pop();
    }

    fn visit_ts_satisfies_expr<'ast: 'r, 'r>(
        &mut self,
        node: &'ast swc_ecma_ast::TsSatisfiesExpr,
        path: &mut AstNodePath<'r>,
    ) {
        self.assertions.push(Some(projected_span(
            node.type_ann.span(),
            self.source_start,
        )));
        <swc_ecma_ast::TsSatisfiesExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(
            node, self, path,
        );
        self.assertions.pop();
    }

    fn visit_function<'ast: 'r, 'r>(&mut self, node: &'ast Function, path: &mut AstNodePath<'r>) {
        self.function_depth += 1;
        let target = self.function_target_of(
            node.span,
            if node.is_generator {
                EvaluationOwner::Generator
            } else {
                EvaluationOwner::FunctionBody
            },
        );
        self.function_targets.push(target);
        self.contextual_types.push(None);
        self.function_return_types
            .push(self.returned_value_type(node.return_type.as_deref(), node.is_generator));
        self.function_return_async.push(node.is_async);
        <Function as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.function_return_types.pop();
        self.function_return_async.pop();
        self.contextual_types.pop();
        self.function_targets.pop();
        self.function_depth -= 1;
    }

    fn visit_constructor<'ast: 'r, 'r>(
        &mut self,
        node: &'ast Constructor,
        path: &mut AstNodePath<'r>,
    ) {
        self.function_depth += 1;
        self.function_targets.push(EvaluationOwner::Constructor);
        self.contextual_types.push(None);
        self.function_return_types.push(None);
        self.function_return_async.push(false);
        <Constructor as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.function_return_types.pop();
        self.function_return_async.pop();
        self.contextual_types.pop();
        self.function_targets.pop();
        self.function_depth -= 1;
    }

    fn visit_block_stmt<'ast: 'r, 'r>(
        &mut self,
        node: &'ast BlockStmt,
        path: &mut AstNodePath<'r>,
    ) {
        let block = projected_span(node.span, self.source_start);
        if let Some(body) = self.arm_blocks.get(&block)
            && let [Stmt::Return(statement)] = node.stmts.as_slice()
            && statement.arg.is_some()
        {
            self.single_return_bodies
                .insert(projected_span(statement.span, self.source_start), *body);
        }
        let arm_scope = self.arm_blocks.get(&block).copied();
        if let Some(body) = arm_scope {
            self.arm_block_scopes.push((
                body,
                statements_are_cleanup_free(&node.stmts),
                self.function_depth,
            ));
        }
        <BlockStmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        if arm_scope.is_some() {
            self.arm_block_scopes.pop();
        }
    }

    fn visit_return_stmt<'ast: 'r, 'r>(
        &mut self,
        node: &'ast ReturnStmt,
        path: &mut AstNodePath<'r>,
    ) {
        let statement = projected_span(node.span, self.source_start);
        if !self.synthetic_returns.contains(&statement)
            && let Some((id, target_depth, region_break_depth)) = self.exit_regions.last().copied()
            && target_depth == self.function_depth
            && let Some(found) = self.found.get_mut(&id)
        {
            let arm_block = self
                .arm_block_scopes
                .last()
                .copied()
                .filter(|(_, _, depth)| *depth == self.function_depth);
            found.exits.push(ProjectedHostExit {
                body: arm_block.map(|(body, _, _)| body),
                call_safe: arm_block.is_some_and(|(_, cleanup_free, _)| cleanup_free),
                single_return_body: self.single_return_bodies.get(&statement).copied(),
                statement,
                argument: node
                    .arg
                    .as_ref()
                    .map(|argument| projected_span(argument.span(), self.source_start)),
                value_argument: node.arg.as_ref().map(|argument| {
                    projected_span(reference_value_span(argument), self.source_start)
                }),
                captured_break: self.break_capture_depth > region_break_depth,
                requires_block: is_unbraced_body(match &path.kinds()[..] {
                    [above @ .., AstParentKind::Stmt(fields::StmtField::Return)] => above,
                    kinds => kinds,
                }),
            });
        }
        <ReturnStmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
    }

    fn visit_opt_call<'ast: 'r, 'r>(&mut self, node: &'ast OptCall, path: &mut AstNodePath<'r>) {
        let (callee_mode, callee_parts) = call_callee_mode(&node.callee);
        let own_link = path.iter().rev().find_map(|parent| match parent {
            swc_ecma_visit::AstParentNodeRef::OptChainExpr(chain, _) => Some(chain.optional),
            _ => None,
        });
        let optional = optional_call_test(own_link == Some(true), &node.callee);
        self.protocol_frames.push(ProjectedProtocolFrame::Call {
            discarded: false,
            parent: projected_span(node.span, self.source_start),
            callee: Some(projected_span(
                reference_value_span(&node.callee),
                self.source_start,
            )),
            callee_mode,
            callee_reference: (callee_mode == EvaluationInputMode::MemberReference).then(|| {
                projected_member_reference(callee_parts, self.source_start, &self.source_segments)
            }),
            arguments: argument_positions(
                &node.args,
                self.source_start,
                &self.placeholders,
                &self.source_segments,
            ),
            type_args: node
                .type_args
                .as_ref()
                .map(|args| projected_span(args.span(), self.source_start)),
            optional: Some(optional),
        });
        <OptCall as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_member_expr<'ast: 'r, 'r>(
        &mut self,
        node: &'ast MemberExpr,
        path: &mut AstNodePath<'r>,
    ) {
        let property = match &node.prop {
            MemberProp::Computed(property) => {
                Some(projected_span(property.expr.span(), self.source_start))
            }
            MemberProp::Ident(_) | MemberProp::PrivateName(_) => None,
        };
        self.protocol_frames.push(ProjectedProtocolFrame::Member {
            parent: projected_span(node.span, self.source_start),
            object: (
                projected_span(node.obj.span(), self.source_start),
                expression_effects(&node.obj),
            ),
            property,
        });
        <MemberExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_new_expr<'ast: 'r, 'r>(&mut self, node: &'ast NewExpr, path: &mut AstNodePath<'r>) {
        self.protocol_frames
            .push(ProjectedProtocolFrame::Construct {
                parent: projected_span(node.span, self.source_start),
                callee: projected_span(reference_value_span(&node.callee), self.source_start),
                arguments: node
                    .args
                    .as_deref()
                    .map(|args| {
                        argument_positions(
                            args,
                            self.source_start,
                            &self.placeholders,
                            &self.source_segments,
                        )
                    })
                    .unwrap_or_default(),
            });
        <NewExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_tagged_tpl<'ast: 'r, 'r>(
        &mut self,
        node: &'ast TaggedTpl,
        path: &mut AstNodePath<'r>,
    ) {
        let (tag_mode, tag_parts) = call_callee_mode(&node.tag);
        self.protocol_frames
            .push(ProjectedProtocolFrame::TaggedTemplate {
                parent: projected_span(node.span, self.source_start),
                tag: projected_span(reference_value_span(&node.tag), self.source_start),
                tag_mode,
                tag_reference: (tag_mode == EvaluationInputMode::MemberReference).then(|| {
                    projected_member_reference(tag_parts, self.source_start, &self.source_segments)
                }),
                expressions: node
                    .tpl
                    .exprs
                    .iter()
                    .map(|expression| {
                        (
                            projected_span(expression.span(), self.source_start),
                            expression_effects(expression),
                        )
                    })
                    .collect(),
            });
        <TaggedTpl as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_tpl<'ast: 'r, 'r>(&mut self, node: &'ast Tpl, path: &mut AstNodePath<'r>) {
        self.protocol_frames.push(ProjectedProtocolFrame::Template {
            parent: projected_span(node.span, self.source_start),
            expressions: node
                .exprs
                .iter()
                .map(|expression| {
                    (
                        operand_span(
                            expression,
                            self.source_start,
                            &self.placeholders,
                            &self.source_segments,
                        ),
                        expression_effects(expression),
                    )
                })
                .collect(),
        });
        <Tpl as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_jsx_element<'ast: 'r, 'r>(
        &mut self,
        node: &'ast JSXElement,
        path: &mut AstNodePath<'r>,
    ) {
        self.protocol_frames.push(ProjectedProtocolFrame::Jsx {
            parent: projected_span(node.span, self.source_start),
            expressions: jsx_evaluation_positions(node, self.source_start)
                .into_iter()
                .map(|(span, child)| (span, Effects::ANY, child))
                .collect(),
        });
        <JSXElement as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_jsx_fragment<'ast: 'r, 'r>(
        &mut self,
        node: &'ast JSXFragment,
        path: &mut AstNodePath<'r>,
    ) {
        self.protocol_frames.push(ProjectedProtocolFrame::Jsx {
            parent: projected_span(node.span, self.source_start),
            expressions: jsx_fragment_positions(node, self.source_start)
                .into_iter()
                .map(|(span, child)| (span, Effects::ANY, child))
                .collect(),
        });
        <JSXFragment as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_await_expr<'ast: 'r, 'r>(
        &mut self,
        node: &'ast AwaitExpr,
        path: &mut AstNodePath<'r>,
    ) {
        self.protocol_frames.push(ProjectedProtocolFrame::Suspend {
            parent: projected_span(node.span, self.source_start),
            kind: SuspensionKind::Await,
            value: Some(operand_span(
                &node.arg,
                self.source_start,
                &self.placeholders,
                &self.source_segments,
            )),
        });
        <AwaitExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_yield_expr<'ast: 'r, 'r>(
        &mut self,
        node: &'ast YieldExpr,
        path: &mut AstNodePath<'r>,
    ) {
        self.protocol_frames.push(ProjectedProtocolFrame::Suspend {
            parent: projected_span(node.span, self.source_start),
            kind: if node.delegate {
                SuspensionKind::YieldDelegate
            } else {
                SuspensionKind::Yield
            },
            value: node
                .arg
                .as_ref()
                .map(|value| projected_span(value.span(), self.source_start)),
        });
        <YieldExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_ident<'ast: 'r, 'r>(&mut self, ident: &'ast Ident, path: &mut AstNodePath<'r>) {
        self.occupied_names.insert(ident.sym.to_string());
        let start = self.source_start.byte(ident.span.lo);
        let end = self.source_start.byte(ident.span.hi);
        let projected = ProjectedSpan {
            start: ProjectedByte(start),
            end: ProjectedByte(end),
        };
        let Some(id) = self.expected_identifiers.get(&projected).copied() else {
            return;
        };
        self.record_overlay(id, path);
    }
}

/// Whether a projected arm block contains no cleanup boundary — no `try`,
/// `with`, or `using` declaration — anywhere in its statement tree outside
/// nested functions. Every rewritten `return` in such a block may carry the
/// consuming call: nothing can catch the consumer's exceptions, and no
/// finalizer or disposal is scheduled to run between the authored value and
/// the match's completion. Statements are the complete search space here: a
/// statement can only appear inside an expression through a function body,
/// and exits are never collected across a function boundary.
pub(super) fn statements_are_cleanup_free(statements: &[Stmt]) -> bool {
    statements.iter().all(statement_is_cleanup_free)
}

fn statement_is_cleanup_free(statement: &Stmt) -> bool {
    use swc_ecma_ast::{Decl, ForHead, VarDeclOrExpr};
    match statement {
        Stmt::Try(_) | Stmt::With(_) => false,
        Stmt::Decl(Decl::Using(_)) => false,
        Stmt::Decl(_) | Stmt::Expr(_) | Stmt::Empty(_) | Stmt::Debugger(_) => true,
        Stmt::Return(_) | Stmt::Break(_) | Stmt::Continue(_) | Stmt::Throw(_) => true,
        Stmt::Block(block) => statements_are_cleanup_free(&block.stmts),
        Stmt::If(node) => {
            statement_is_cleanup_free(&node.cons)
                && node.alt.as_deref().is_none_or(statement_is_cleanup_free)
        }
        Stmt::Labeled(node) => statement_is_cleanup_free(&node.body),
        Stmt::While(node) => statement_is_cleanup_free(&node.body),
        Stmt::DoWhile(node) => statement_is_cleanup_free(&node.body),
        Stmt::For(node) => {
            let init_is_clean = match &node.init {
                Some(VarDeclOrExpr::VarDecl(_)) | Some(VarDeclOrExpr::Expr(_)) | None => true,
                Some(VarDeclOrExpr::UsingDecl(_)) => false,
            };
            init_is_clean && statement_is_cleanup_free(&node.body)
        }
        Stmt::ForIn(node) => {
            !matches!(node.left, ForHead::UsingDecl(_)) && statement_is_cleanup_free(&node.body)
        }
        Stmt::ForOf(node) => {
            !matches!(node.left, ForHead::UsingDecl(_)) && statement_is_cleanup_free(&node.body)
        }
        Stmt::Switch(node) => node
            .cases
            .iter()
            .all(|case| statements_are_cleanup_free(&case.cons)),
    }
}

fn declarator_split(
    path: &AstNodePath<'_>,
    source_start: HostOrigin,
) -> Option<ProjectedDeclaratorSplit> {
    use swc_ecma_ast::VarDeclKind;
    use swc_ecma_visit::AstParentNodeRef;

    let mut parents = path.iter().rev();
    let (previous, kind, declared, field) = match parents.next()? {
        AstParentNodeRef::VarDecl(declaration, fields::VarDeclField::Decls(index))
            if *index > 0 =>
        {
            (
                declaration.decls[*index - 1].span,
                match declaration.kind {
                    VarDeclKind::Var => DeclarationKind::Var,
                    VarDeclKind::Let => DeclarationKind::Let,
                    VarDeclKind::Const => DeclarationKind::Const,
                },
                declaration.declare,
                fields::DeclField::Var,
            )
        }
        AstParentNodeRef::UsingDecl(declaration, fields::UsingDeclField::Decls(index))
            if *index > 0 =>
        {
            (
                declaration.decls[*index - 1].span,
                if declaration.is_await {
                    DeclarationKind::AwaitUsing
                } else {
                    DeclarationKind::Using
                },
                false,
                fields::DeclField::Using,
            )
        }
        _ => return None,
    };
    match parents.next()? {
        AstParentNodeRef::Decl(_, parent) if *parent == field => {}
        _ => return None,
    }
    let exported = matches!(
        parents.next(),
        Some(AstParentNodeRef::ExportDecl(
            _,
            fields::ExportDeclField::Decl
        ))
    );
    Some(ProjectedDeclaratorSplit {
        previous: projected_span(previous, source_start),
        kind,
        exported,
        declared,
    })
}
