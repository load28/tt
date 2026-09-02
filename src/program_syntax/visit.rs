//! SWC AST-path visitor implementation.

use super::*;

impl ParentCollector {
    pub(super) fn new(
        source_start: u32,
        pending: &[PendingOverlay],
        source_segments: &[ProjectionSourceSegment],
        projection_only_protocol_parents: &[ProjectedSpan],
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
        let synthetic_returns = pending
            .iter()
            .filter_map(|entry| entry.synthetic_return)
            .collect();
        Self {
            source_start,
            expected_identifiers,
            expected_calls,
            expected_exit_calls,
            synthetic_returns,
            found: HashMap::new(),
            duplicates: Vec::new(),
            source_segments: source_segments.to_vec(),
            projection_only_protocol_parents: projection_only_protocol_parents
                .iter()
                .copied()
                .collect(),
            host_owners: Vec::new(),
            protocol_frames: Vec::new(),
            occupied_names: HashSet::new(),
            function_depth: 0,
            function_targets: Vec::new(),
            contextual_types: Vec::new(),
            function_return_types: Vec::new(),
            function_return_async: Vec::new(),
            break_capture_depth: 0,
            exit_regions: Vec::new(),
        }
    }

    pub(super) fn record_overlay(&mut self, id: TtNodeId, path: &AstNodePath<'_>) {
        if self
            .found
            .insert(
                id,
                FoundOverlay {
                    parents: path.kinds().to_vec(),
                    host_owners: self.host_owners.clone(),
                    protocol_frames: self.protocol_frames.clone(),
                    exits: Vec::new(),
                    function_target: self.function_targets.last().copied(),
                    contextual_type: self.contextual_types.last().copied().flatten(),
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
        let mut overlay: Vec<OverlayEntry> = Vec::with_capacity(pending.len());
        let overlay_spans: Vec<_> = pending
            .iter()
            .map(|entry| (entry.id, entry.projected))
            .collect();
        for entry in pending {
            let found = self
                .found
                .remove(&entry.id)
                .ok_or(ProgramSyntaxError::MissingOverlay { id: entry.id })?;
            let (projected_owner, kind, span) = found
                .host_owners
                .iter()
                .rev()
                .filter(|owner| {
                    owner.span.start <= entry.projected.start
                        && entry.projected.end <= owner.span.end
                })
                .find_map(|owner| {
                    source_span_for_projection(&self.source_segments, owner.span)
                        .map(|span| (*owner, owner.kind, span))
                })
                .ok_or(ProgramSyntaxError::MissingOverlay { id: entry.id })?;
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
                    },
                    roots: Vec::new(),
                });
                owner_id
            };
            owners[owner_id.0 as usize].roots.push(entry.id);
            let enclosing_overlay = overlay_spans
                .iter()
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
                    found.function_target,
                    found
                        .contextual_type
                        .map(|span| map_evaluation_span(&self.source_segments, span))
                        .transpose()?,
                    found
                        .function_return_type
                        .map(|span| map_evaluation_span(&self.source_segments, span))
                        .transpose()?,
                    found.function_return_awaited,
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
                            statement: map_evaluation_span(&self.source_segments, exit.statement)?,
                            argument: exit
                                .argument
                                .map(|argument| {
                                    map_structural_span(&self.source_segments, argument)
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
        })
    }
}

impl VisitAstPath for ParentCollector {
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
        <VarDeclarator as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
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
        });
        <ModuleItem as VisitWithAstPath<Self>>::visit_children_with_ast_path(item, self, path);
        self.host_owners.pop();
    }

    fn visit_stmt<'ast: 'r, 'r>(&mut self, statement: &'ast Stmt, path: &mut AstNodePath<'r>) {
        self.host_owners.push(ProjectedHostOwner {
            kind: HostOwnerKind::Statement,
            span: projected_span(statement.span(), self.source_start),
            edge: path.kinds().len(),
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
                        projected_span(element.expr.span(), self.source_start),
                        expression_effects(&element.expr),
                    )
                })
                .collect(),
            kind: OrderedEvaluationKind::Array,
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
            positions: object_evaluation_positions(node, self.source_start),
            kind: OrderedEvaluationKind::Object,
        });
        <ObjectLit as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_assign_expr<'ast: 'r, 'r>(
        &mut self,
        node: &'ast AssignExpr,
        path: &mut AstNodePath<'r>,
    ) {
        self.protocol_frames.push(ProjectedProtocolFrame::Ordered {
            parent: projected_span(node.span, self.source_start),
            positions: vec![(
                projected_span(node.right.span(), self.source_start),
                expression_effects(&node.right),
            )],
            kind: OrderedEvaluationKind::Assignment,
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
                        projected_span(expression.span(), self.source_start),
                        expression_effects(expression),
                    )
                })
                .collect(),
            kind: OrderedEvaluationKind::Sequence,
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
                projected_span(node.arg.span(), self.source_start),
                expression_effects(&node.arg),
            )],
            kind: OrderedEvaluationKind::Unary,
        });
        <UnaryExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_bin_expr<'ast: 'r, 'r>(&mut self, node: &'ast BinExpr, path: &mut AstNodePath<'r>) {
        self.protocol_frames.push(ProjectedProtocolFrame::Binary {
            parent: projected_span(node.span, self.source_start),
            operator: node.op,
            left: (
                projected_span(node.left.span(), self.source_start),
                expression_effects(&node.left),
            ),
            right: projected_span(node.right.span(), self.source_start),
        });
        <BinExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_cond_expr<'ast: 'r, 'r>(&mut self, node: &'ast CondExpr, path: &mut AstNodePath<'r>) {
        self.protocol_frames
            .push(ProjectedProtocolFrame::Conditional {
                parent: projected_span(node.span, self.source_start),
                test: (
                    projected_span(node.test.span(), self.source_start),
                    expression_effects(&node.test),
                ),
                consequent: projected_span(node.cons.span(), self.source_start),
                alternate: projected_span(node.alt.span(), self.source_start),
            });
        <CondExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_call_expr<'ast: 'r, 'r>(&mut self, node: &'ast CallExpr, path: &mut AstNodePath<'r>) {
        let span = projected_span(node.span, self.source_start);
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
        let (callee_mode, callee_receiver) = match &node.callee {
            swc_ecma_ast::Callee::Expr(expression) => call_callee_mode(expression),
            swc_ecma_ast::Callee::Super(_) | swc_ecma_ast::Callee::Import(_) => {
                (EvaluationInputMode::MemberReference, None)
            }
        };
        self.protocol_frames.push(ProjectedProtocolFrame::Call {
            parent: span,
            callee: Some(projected_span(
                match &node.callee {
                    swc_ecma_ast::Callee::Expr(expression) => reference_value_span(expression),
                    swc_ecma_ast::Callee::Super(_) | swc_ecma_ast::Callee::Import(_) => {
                        node.callee.span()
                    }
                },
                self.source_start,
            )),
            callee_mode,
            callee_receiver: callee_receiver.map(|receiver| {
                (
                    projected_span(receiver.span(), self.source_start),
                    expression_effects(receiver),
                )
            }),
            arguments: argument_positions(&node.args, self.source_start),
            type_args: node
                .type_args
                .as_ref()
                .map(|args| projected_span(args.span(), self.source_start)),
            optional: false,
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
            test: projected_span(node.test.span(), self.source_start),
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
        self.function_targets.push(EvaluationOwner::FunctionBody);
        self.contextual_types.push(None);
        self.function_return_types.push(
            node.return_type
                .as_deref()
                .map(|annotation| projected_span(annotation.span, self.source_start)),
        );
        self.function_return_async.push(node.is_async);
        self.host_owners.push(ProjectedHostOwner {
            kind: HostOwnerKind::ArrowExpression,
            span: projected_span(node.body.span(), self.source_start),
            edge: path.kinds().len(),
        });
        <ArrowExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.host_owners.pop();
        self.function_return_types.pop();
        self.function_return_async.pop();
        self.contextual_types.pop();
        self.function_targets.pop();
        self.function_depth -= 1;
    }

    fn visit_function<'ast: 'r, 'r>(&mut self, node: &'ast Function, path: &mut AstNodePath<'r>) {
        self.function_depth += 1;
        self.function_targets.push(if node.is_generator {
            EvaluationOwner::Generator
        } else {
            EvaluationOwner::FunctionBody
        });
        self.contextual_types.push(None);
        self.function_return_types.push(
            node.return_type
                .as_deref()
                .map(|annotation| projected_span(annotation.span, self.source_start)),
        );
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
            found.exits.push(ProjectedHostExit {
                statement,
                argument: node
                    .arg
                    .as_ref()
                    .map(|argument| projected_span(argument.span(), self.source_start)),
                captured_break: self.break_capture_depth > region_break_depth,
                requires_block: path
                    .kinds()
                    .iter()
                    .rev()
                    .find(|parent| {
                        !matches!(parent, AstParentKind::Stmt(fields::StmtField::Return))
                    })
                    .is_some_and(|parent| {
                        matches!(
                            parent,
                            AstParentKind::IfStmt(
                                fields::IfStmtField::Cons | fields::IfStmtField::Alt
                            ) | AstParentKind::ForStmt(fields::ForStmtField::Body)
                                | AstParentKind::ForInStmt(fields::ForInStmtField::Body)
                                | AstParentKind::ForOfStmt(fields::ForOfStmtField::Body)
                                | AstParentKind::WhileStmt(fields::WhileStmtField::Body)
                                | AstParentKind::DoWhileStmt(fields::DoWhileStmtField::Body)
                                | AstParentKind::LabeledStmt(fields::LabeledStmtField::Body)
                                | AstParentKind::WithStmt(fields::WithStmtField::Body)
                        )
                    }),
            });
        }
        <ReturnStmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
    }

    fn visit_opt_call<'ast: 'r, 'r>(&mut self, node: &'ast OptCall, path: &mut AstNodePath<'r>) {
        let (callee_mode, callee_receiver) = call_callee_mode(&node.callee);
        self.protocol_frames.push(ProjectedProtocolFrame::Call {
            parent: projected_span(node.span, self.source_start),
            callee: Some(projected_span(
                reference_value_span(&node.callee),
                self.source_start,
            )),
            callee_mode,
            callee_receiver: callee_receiver.map(|receiver| {
                (
                    projected_span(receiver.span(), self.source_start),
                    expression_effects(receiver),
                )
            }),
            arguments: argument_positions(&node.args, self.source_start),
            type_args: node
                .type_args
                .as_ref()
                .map(|args| projected_span(args.span(), self.source_start)),
            optional: true,
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
                    .map(|args| argument_positions(args, self.source_start))
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
        let (tag_mode, tag_receiver) = call_callee_mode(&node.tag);
        self.protocol_frames
            .push(ProjectedProtocolFrame::TaggedTemplate {
                parent: projected_span(node.span, self.source_start),
                tag: projected_span(reference_value_span(&node.tag), self.source_start),
                tag_mode,
                tag_receiver: tag_receiver.map(|receiver| {
                    (
                        projected_span(receiver.span(), self.source_start),
                        expression_effects(receiver),
                    )
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
                        projected_span(expression.span(), self.source_start),
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
            value: Some(projected_span(node.arg.span(), self.source_start)),
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
        let start = ident.span.lo.0.saturating_sub(self.source_start) as usize;
        let end = ident.span.hi.0.saturating_sub(self.source_start) as usize;
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
