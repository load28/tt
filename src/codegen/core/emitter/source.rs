//! Source-preserving body, statement, and expression traversal.

use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn exits_for_expr(&self, expr: ExprId) -> Vec<HostExit> {
        self.value_exits.get(&expr).cloned().unwrap_or_default()
    }

    pub(in super::super) fn result_return_rewrite_spans(&self) -> Vec<SourceSpan> {
        self.core
            .exprs
            .iter()
            .enumerate()
            .flat_map(|(index, expr)| {
                let Expr::ResultRegion(region) = expr else {
                    return Vec::new();
                };
                let expr = ExprId::new(index);
                let exits = self
                    .value_exits
                    .get(&expr)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                region
                    .items
                    .iter()
                    .map(|item| {
                        let ResultRegionItem::Statements(body) = item;
                        *body
                    })
                    .flat_map(|body| {
                        exits.iter().filter_map(move |exit| {
                            exit.argument
                                .and_then(|argument| self.result_return_propagate(body, argument))
                                .map(|_| exit.statement)
                        })
                    })
                    .collect()
            })
            .collect()
    }

    pub(super) fn span(&self, node: NodeId) -> hir::Span {
        self.semantic
            .hir
            .source_map
            .node_span(node)
            .unwrap_or_else(|| crate::ice::bug!("target node has no source span"))
    }

    pub(super) fn source_node(&self, node: NodeId) -> (&'a str, usize) {
        let span = self.span(node);
        (&self.source[span.start..span.end], span.start)
    }

    pub(super) fn source_span(&self, span: hir::Span) -> (&'a str, usize) {
        (&self.source[span.start..span.end], span.start)
    }

    pub(super) fn source_rope(&self, node: NodeId) -> Rope<'a> {
        let span = self.span(node);
        self.source_range_rope(span)
    }

    pub(super) fn source_range_rope(&self, span: hir::Span) -> Rope<'a> {
        let mut rope = Rope::new();
        let mut insertions = self
            .owner_slot_rewrites
            .iter()
            .filter(|rewrite| span.start <= rewrite.owner.start && rewrite.owner.start < span.end)
            .peekable();
        let mut propagation_insertions = self
            .for_initializer_propagations
            .iter()
            .filter(|rewrite| span.start <= rewrite.owner.start && rewrite.owner.start < span.end)
            .peekable();
        let mut compose_insertions = self
            .compose_rewrites
            .iter()
            .filter(|rewrite| {
                span.start <= rewrite.owner.start
                    && rewrite.owner.start < span.end
                    && !rewrite.actions.iter().any(|action| match action {
                        ComposeAction::Value(value) => {
                            self.active_structured_exprs.contains(value.expr)
                        }
                        ComposeAction::Operation(operation) => operation
                            .values
                            .iter()
                            .any(|expr| self.active_structured_exprs.contains(*expr)),
                    })
            })
            .peekable();
        let mut compose_endings = self
            .compose_rewrites
            .iter()
            .filter(|rewrite| {
                rewrite.owner_kind == HostOwnerKind::ArrowExpression
                    && span.start < rewrite.owner.end
                    && rewrite.owner.end <= span.end
                    && !rewrite.actions.iter().any(|action| match action {
                        ComposeAction::Value(value) => {
                            self.active_structured_exprs.contains(value.expr)
                        }
                        ComposeAction::Operation(operation) => operation
                            .values
                            .iter()
                            .any(|expr| self.active_structured_exprs.contains(*expr)),
                    })
            })
            .peekable();
        let mut loop_endings: Vec<_> = self
            .loop_test_rewrites
            .iter()
            .filter(|rewrite| span.start < rewrite.body.end && rewrite.body.end <= span.end)
            .collect();
        loop_endings.sort_unstable_by_key(|rewrite| rewrite.body.end);
        let mut loop_endings = loop_endings.into_iter().peekable();
        let mut cursor = span.start;
        while cursor < span.end {
            while let Some(_rewrite) = loop_endings.next_if(|rewrite| rewrite.body.end == cursor) {
                rope.push_lit("}");
            }
            while let Some(rewrite) = compose_endings.next_if(|rewrite| rewrite.owner.end == cursor)
            {
                rope.append(self.emit_compose_suffix(rewrite));
            }
            while let Some(rewrite) = insertions.next_if(|rewrite| rewrite.owner.start == cursor) {
                if !self.emitted_owner_rewrites.contains(rewrite.expr) {
                    self.emitted_owner_rewrites.mark(rewrite.expr);
                    rope.append(self.emit_owner_slot_rewrite(rewrite));
                }
            }
            while let Some(rewrite) =
                propagation_insertions.next_if(|rewrite| rewrite.owner.start == cursor)
            {
                rope.append(self.emit_for_initializer_propagation_prelude(rewrite));
            }
            while let Some(rewrite) =
                compose_insertions.next_if(|rewrite| rewrite.owner.start == cursor)
            {
                rope.append(self.emit_compose_rewrite(rewrite));
            }
            if let Some(rewrite) = self.loop_test_rewrites.iter().find(|rewrite| {
                rewrite.kind == LoopTestKind::While
                    && rewrite.owner.start <= cursor
                    && cursor < rewrite.test.start
            }) {
                if cursor == rewrite.owner.start {
                    rope.append(self.emit_loop_test_prefix(rewrite));
                }
                cursor = rewrite.test.start.min(span.end);
                continue;
            }
            if let Some(rewrite) = self
                .loop_test_rewrites
                .iter()
                .find(|rewrite| rewrite.kind == LoopTestKind::For && cursor == rewrite.test.start)
            {
                rope.append(self.emit_loop_test_prefix(rewrite));
            }
            if self.loop_region_depth.get() == 0
                && let Some(operation) = self
                    .loop_test_rewrites
                    .iter()
                    .flat_map(|rewrite| &rewrite.actions)
                    .filter_map(|action| match action {
                        ComposeAction::Operation(operation) => Some(operation),
                        ComposeAction::Value(_) => None,
                    })
                    .find(|operation| {
                        operation.parent.start < cursor && cursor < operation.parent.end
                    })
            {
                cursor = operation.parent.end.min(span.end);
                continue;
            }
            if let Some(rewrite) = self
                .loop_test_rewrites
                .iter()
                .find(|rewrite| rewrite.test.end <= cursor && cursor < rewrite.body.start)
            {
                if cursor == rewrite.test.end {
                    rope.push_lit(")) break; ");
                }
                cursor = rewrite.body.start.min(span.end);
                continue;
            }
            if let Some(replacement) = self.source_replacements.iter().find(|replacement| {
                if replacement.anchor.is_some() {
                    self.conditional_region_depth.get() == 0
                        && self.loop_region_depth.get() == 0
                        && !replacement
                            .anchor
                            .is_some_and(|expr| self.active_structured_exprs.contains(expr))
                        && replacement.source.start <= cursor
                        && cursor < replacement.source.end
                } else {
                    replacement.source.start <= cursor && cursor < replacement.source.end
                }
            }) {
                if cursor == replacement.source.start {
                    if replacement.jsx_child {
                        rope.push_lit("{");
                    }
                    match replacement.anchor {
                        Some(expr) => {
                            let (kind, start, end, extent) = self.value_anchor(expr);
                            let mut name = Rope::new();
                            name.push_lit(replacement.slot.clone());
                            rope.anchored(kind, start, end, extent, name);
                        }
                        None => rope.push_lit(replacement.slot.clone()),
                    }
                    if replacement.jsx_child {
                        rope.push_lit("}");
                    }
                }
                cursor = replacement.source.end.min(span.end);
                continue;
            }
            let next_insertion = insertions
                .peek()
                .map_or(span.end, |rewrite| rewrite.owner.start);
            let next_compose = compose_insertions
                .peek()
                .map_or(span.end, |rewrite| rewrite.owner.start);
            let next_propagation = propagation_insertions
                .peek()
                .map_or(span.end, |rewrite| rewrite.owner.start);
            let next_compose_end = compose_endings
                .peek()
                .map_or(span.end, |rewrite| rewrite.owner.end);
            let next_loop_boundary = self
                .loop_test_rewrites
                .iter()
                .flat_map(|rewrite| {
                    [
                        rewrite.owner.start,
                        rewrite.test.start,
                        rewrite.test.end,
                        rewrite.body.start,
                        rewrite.body.end,
                    ]
                })
                .filter(|boundary| cursor < *boundary && *boundary < span.end)
                .min()
                .unwrap_or(span.end);
            let next_replacement = self
                .source_replacements
                .iter()
                .filter(|replacement| {
                    (replacement.anchor.is_none()
                        || (self.conditional_region_depth.get() == 0
                            && self.loop_region_depth.get() == 0
                            && !replacement
                                .anchor
                                .is_some_and(|expr| self.active_structured_exprs.contains(expr))))
                        && cursor < replacement.source.start
                        && replacement.source.start < span.end
                })
                .map(|replacement| replacement.source.start)
                .min()
                .unwrap_or(span.end);
            let next = next_insertion
                .min(next_compose)
                .min(next_propagation)
                .min(next_compose_end)
                .min(next_loop_boundary)
                .min(next_replacement)
                .min(span.end);
            if cursor < next {
                rope.push_src(&self.source[cursor..next], cursor);
                cursor = next;
            }
        }
        while let Some(rewrite) = compose_endings.next_if(|rewrite| rewrite.owner.end == span.end) {
            rope.append(self.emit_compose_suffix(rewrite));
        }
        while let Some(_rewrite) = loop_endings.next_if(|rewrite| rewrite.body.end == span.end) {
            rope.push_lit("}");
        }
        rope
    }

    pub(super) fn source_rope_with_edits(
        &self,
        node: NodeId,
        edits: &[LocalSourceEdit],
    ) -> Rope<'a> {
        let span = self.span(node);
        let mut cursor = span.start;
        let mut out = Rope::new();
        for edit in edits
            .iter()
            .filter(|edit| span.start <= edit.span.start && edit.span.end <= span.end)
        {
            if cursor < edit.span.start {
                out.append(self.source_range_rope(hir::Span::new(cursor, edit.span.start)));
            }
            match edit.result_return_mark {
                Some((mark, ResultReturnBoundary::Start)) => {
                    // The prefix ends immediately before the authored value.
                    out.push_lit(edit.text.clone());
                    out.push_result_return_start(mark.start);
                }
                Some((mark, ResultReturnBoundary::End)) => {
                    // The suffix begins immediately after the authored value.
                    out.push_result_return_end(mark.start);
                    out.push_lit(edit.text.clone());
                }
                None => out.push_lit(edit.text.clone()),
            }
            cursor = edit.span.end;
        }
        if cursor < span.end {
            out.append(self.source_range_rope(hir::Span::new(cursor, span.end)));
        }
        out
    }

    pub(in super::super) fn emit_body(&self, body: hir::BodyId) -> Rope<'a> {
        self.emit_statements(&self.core.bodies[body.index()].statements)
    }

    pub(super) fn emit_body_with_exits(
        &self,
        body: hir::BodyId,
        exits: &[HostExit],
        continuation: &ValueContinuation<'_>,
        label: Option<&str>,
        generated_indent: &str,
    ) -> Rope<'a> {
        // Without a label the region's own dispatch is the nearest `break`
        // target already ([`HostExit::captured_break`]).
        let leave = label.map_or_else(|| "break;".to_owned(), |label| format!("break {label};"));
        let mut edits = Vec::new();
        for exit in exits {
            let line_start = self.source[..exit.statement.start]
                .rfind('\n')
                .map_or(0, |index| index + 1);
            let line_indent = &self.source[line_start..exit.statement.start];
            let starts_own_line = line_indent.bytes().all(|byte| matches!(byte, b' ' | b'\t'));
            match exit.argument {
                Some(argument) => {
                    let grouped =
                        grouping_required(self.source[argument.start..argument.end].trim());
                    edits.push(LocalSourceEdit {
                        span: SourceSpan {
                            start: exit.statement.start,
                            end: argument.start,
                        },
                        text: format!(
                            "{}{}{}",
                            if starts_own_line {
                                generated_indent
                            } else {
                                ""
                            },
                            if exit.requires_block { "{ " } else { "" },
                            continuation.assignment_prefix(grouped)
                        ),
                        result_return_mark: None,
                    });
                    edits.push(LocalSourceEdit {
                        span: SourceSpan {
                            start: argument.end,
                            end: exit.statement.end,
                        },
                        text: if starts_own_line {
                            format!(
                                "{};\n{line_indent}{generated_indent}{leave}",
                                continuation.assignment_suffix(grouped),
                            )
                        } else {
                            format!(
                                "{}; {leave}{}",
                                continuation.assignment_suffix(grouped),
                                if exit.requires_block { " }" } else { "" }
                            )
                        },
                        result_return_mark: None,
                    });
                }
                None => edits.push(LocalSourceEdit {
                    span: exit.statement,
                    text: if starts_own_line {
                        format!(
                            "{generated_indent}{}undefined{};\n{line_indent}{generated_indent}{leave}",
                            continuation.assignment_prefix(false),
                            continuation.assignment_suffix(false)
                        )
                    } else {
                        format!(
                            "{}{}undefined{}; {leave}{}",
                            if exit.requires_block { "{ " } else { "" },
                            continuation.assignment_prefix(false),
                            continuation.assignment_suffix(false),
                            if exit.requires_block { " }" } else { "" }
                        )
                    },
                    result_return_mark: None,
                }),
            }
        }
        edits.sort_unstable_by_key(|edit| edit.span.start);
        self.emit_statements_with_edits(&self.core.bodies[body.index()].statements, &edits)
    }

    pub(super) fn emit_statements(&self, statements: &[Statement]) -> Rope<'a> {
        self.emit_statements_with_edits(statements, &[])
    }

    pub(super) fn emit_statements_with_edits(
        &self,
        statements: &[Statement],
        edits: &[LocalSourceEdit],
    ) -> Rope<'a> {
        let mut out = Rope::new();
        for statement in statements {
            match statement {
                Statement::Opaque(node) => out.append(self.source_rope_with_edits(*node, edits)),
                Statement::Adt(adt) => {
                    // The union type and constructor object are this
                    // declaration's glue: a frame inside a generated
                    // constructor belongs to the `variant` that wrote it.
                    let span = self.span(adt.node);
                    out.anchored(
                        AnchorKind::Variant,
                        span.start,
                        span.end,
                        span.end,
                        emit_adt(adt),
                    );
                }
                Statement::Import(import) => self.emit_import(import, &mut out),
                Statement::Propagate(propagate) => {
                    let owner = self.span(propagate.owner);
                    if let Some(rewrite) = self
                        .compose_rewrites
                        .iter()
                        .find(|rewrite| rewrite.owner == SourceSpan::from(owner))
                    {
                        out.append(self.emit_compose_rewrite(rewrite));
                    }
                    let span = self.span(propagate.node);
                    let emitted = if self.is_for_initializer_propagation(propagate.node) {
                        self.emit_for_initializer_payload(propagate)
                    } else {
                        self.emit_propagate(propagate)
                    };
                    out.anchored(AnchorKind::Try, span.start, span.end, span.end, emitted);
                }
                Statement::Decision(decision) => self.emit_statement_decision(decision, &mut out),
                Statement::Expr(expr) if self.statement_expr_requires_lowering(*expr) => {
                    self.emit_statement_expr(*expr, &mut out);
                }
                Statement::Expr(expr) => out.append(self.emit_expr(*expr)),
            }
        }
        out
    }

    pub(super) fn statement_expr_requires_lowering(&self, expr: ExprId) -> bool {
        self.owner_slot_rewrites.iter().any(|rewrite| {
            rewrite.expr == expr && rewrite.continuation == HostContinuation::Discard
        }) || (matches!(self.core.exprs[expr.index()], Expr::Decision(_))
            && !self
                .owner_slot_rewrites
                .iter()
                .any(|rewrite| rewrite.expr == expr)
            && !self.value_slots.contains_key(&expr))
    }

    pub(super) fn emit_statement_expr(&self, expr: ExprId, out: &mut Rope<'a>) {
        // A tt value that is itself an expression statement has no opaque
        // source owner around it where `source_range_rope` could insert the
        // planned statement region. Consume that plan here before the inline
        // occurrence is replaced by its join slot.
        if let Some(rewrite) = self.owner_slot_rewrites.iter().find(|rewrite| {
            rewrite.expr == expr && rewrite.continuation == HostContinuation::Discard
        }) {
            if self.emitted_owner_rewrites.contains(expr) {
                out.append(self.emit_expr(expr));
            } else {
                self.emitted_owner_rewrites.mark(expr);
                out.append(self.emit_owner_slot_rewrite(rewrite));
            }
            return;
        }
        if self.emitted_owner_rewrites.contains(expr) {
            out.append(self.emit_expr(expr));
            return;
        }
        if matches!(self.core.exprs[expr.index()], Expr::Decision(_)) {
            if let Some(slot) = self.value_slots.get(&expr) {
                out.push_lit(format!("let {slot};"));
                out.push_break(0);
                out.append(
                    self.emit_continued_expr(expr, &ValueContinuation::assign(slot))
                        .unwrap_or_else(|| {
                            crate::ice::bug!("statement match has no structured emission")
                        }),
                );
                return;
            }
            // An incomplete editor buffer can leave the surrounding
            // TypeScript owner unparsable even though the tt match itself is
            // complete. There is then no safe source owner to rewrite and no
            // planned slot. Keep the structurally parsed match available to
            // the language service through the existing expression boundary.
            out.push_lit("(() => { let $tt_recovery; ");
            out.append(
                self.emit_continued_expr(expr, &ValueContinuation::assign("$tt_recovery"))
                    .unwrap_or_else(|| {
                        crate::ice::bug!("statement match has no expression-boundary emission")
                    }),
            );
            out.push_lit(" return $tt_recovery; })()");
            return;
        }
        out.append(self.emit_expr(expr));
    }

    pub(super) fn emit_sequence_continued(
        &self,
        body: hir::BodyId,
        continuation: &ValueContinuation<'_>,
    ) -> Option<Rope<'a>> {
        let statements = &self.core.bodies[body.index()].statements;
        if let Some((value_index, value)) = crate::core_ir::sequence_value(statements)
            && statements
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != value_index)
                .all(|(_, statement)| match statement {
                    Statement::Opaque(node) => {
                        let span = self.span(*node);
                        self.source[span.start..span.end].trim().is_empty()
                    }
                    _ => false,
                })
        {
            let mut out = self.emit_statements(&statements[..value_index]);
            out.append(self.emit_continued_expr(value, continuation)?);
            out.append(self.emit_statements(&statements[value_index + 1..]));
            return Some(out);
        }

        let nested: Vec<_> = statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Expr(inner)
                    if self.core.has_statement_form(*inner)
                        && !self.slot_exprs.contains_key(inner) =>
                {
                    self.structured_value_slot(*inner)
                        .map(|slot| (*inner, slot.clone()))
                }
                _ => None,
            })
            .collect();
        if nested.is_empty() {
            return None;
        }
        let nested_exprs: Vec<_> = nested.iter().map(|(inner, _)| *inner).collect();
        let mut out = Rope::new();
        for (inner, slot) in nested {
            if self.emitted_owner_rewrites.contains(inner) {
                continue;
            }
            if !continuation.is_unwrapped_assignment_to(&slot) {
                out.push_lit(format!("let {slot};"));
                out.push_break(0);
            }
            out.append(self.emit_continued_expr(inner, &ValueContinuation::assign(&slot))?);
            out.push_break(0);
        }
        let sequence_node = self
            .core
            .sequence_node(body)
            .unwrap_or_else(|| crate::ice::bug!("embedded sequence has no source extent"));
        let span = SourceSpan::from(self.span(sequence_node));
        out.append(self.emit_value_delivery_without_region_exit(
            self.source_range_with_value_slots(span, &nested_exprs),
            continuation,
        ));
        Some(Rope::scoped(out))
    }

    pub(super) fn emit_expr(&self, expr: ExprId) -> Rope<'a> {
        // A value a conditional operation consumed is emitted by the
        // operation's region; its inline position sits inside the replaced
        // operation span and prints nothing.
        if self.consumed_exprs.contains(&expr) {
            return Rope::new();
        }
        // An opaque TypeScript owner consumes compose rewrites through
        // `source_range_rope`. When the owner is itself one structured Core
        // value, emission reaches that value directly and there is no opaque
        // frame to insert the prelude. Consume the same owner plan here and
        // reconstruct only the host frame outside the value.
        if !self.active_structured_exprs.contains(expr)
            && let Some((rewrite, value)) = self.compose_rewrites.iter().find_map(|rewrite| {
                if rewrite.actions.len() != 1 {
                    return None;
                }
                let ComposeAction::Value(value) = &rewrite.actions[0] else {
                    return None;
                };
                (value.expr == expr && rewrite.owner.start == value.source.start)
                    .then_some((rewrite, value))
            })
        {
            let _active = self.active_structured_exprs.enter(expr);
            let mut out = self.emit_compose_rewrite(rewrite);
            if value.defer_arm_values {
                out.append(self.emit_selected_arm_values(value.expr, &value.slot));
            } else {
                out.push_lit(value.slot.clone());
            }
            match rewrite.owner_kind {
                HostOwnerKind::ArrowExpression => {
                    out.append(self.emit_compose_suffix(rewrite));
                }
                // The Core body retains a trailing statement/module frame
                // (normally the authored semicolon) outside the direct
                // expression and emits it after this value.
                HostOwnerKind::Statement | HostOwnerKind::ModuleItem => {}
            }
            return out;
        }
        if let Some(rewrite) = self
            .arrow_return_rewrites
            .iter()
            .find(|rewrite| rewrite.expr == expr)
        {
            return self.emit_arrow_return_rewrite(rewrite);
        }
        if let Some(slot) = self.slot_exprs.get(&expr) {
            if let Expr::Decision(decision) = &self.core.exprs[expr.index()]
                && self.inline_subjects.contains_key(&decision.extent)
            {
                let (kind, start, end, extent) = self.value_anchor(expr);
                let mut out = Rope::new();
                out.anchored(kind, start, end, extent, self.emit_inline_match(expr));
                return out;
            }
            if self.compose_rewrites.iter().flat_map(|rewrite| &rewrite.actions).any(|action| {
                matches!(action, ComposeAction::Value(value) if value.expr == expr && value.defer_arm_values)
            }) {
                let (kind, start, end, extent) = self.value_anchor(expr);
                let mut out = Rope::new();
                out.anchored(kind, start, end, extent, self.emit_selected_arm_values(expr, slot));
                return out;
            }
            let (kind, start, end, extent) = self.value_anchor(expr);
            let mut out = Rope::new();
            let mut rendered_slot = slot.as_str();
            if let Some(rewrite) = self.loop_test_rewrites.iter().find(|rewrite| {
                rewrite.first_expr == expr && rewrite.first_source.start == rewrite.test.start
            }) {
                if rewrite.kind == LoopTestKind::For {
                    out.append(self.emit_loop_test_prefix(rewrite));
                }
                if let Some(operation) = rewrite.actions.iter().find_map(|action| match action {
                    ComposeAction::Operation(operation)
                        if operation.parent.start == rewrite.first_source.start =>
                    {
                        Some(operation)
                    }
                    ComposeAction::Operation(_) | ComposeAction::Value(_) => None,
                }) {
                    rendered_slot = self.value_slot_name(operation.result);
                }
            }
            let mut generated = Rope::new();
            generated.push_lit(rendered_slot.to_owned());
            out.anchored(kind, start, end, extent, generated);
            return out;
        }
        if self.nested_values.contains(&expr)
            && matches!(
                self.core.exprs[expr.index()],
                Expr::Decision(_) | Expr::ResultRegion(_) | Expr::Propagate(_)
            )
            && let Some(slot) = self.value_slots.get(&expr)
        {
            let (kind, start, end, extent) = self.value_anchor(expr);
            let mut generated = Rope::new();
            generated.push_lit(slot.clone());
            let mut out = Rope::new();
            out.anchored(kind, start, end, extent, generated);
            return out;
        }
        match &self.core.exprs[expr.index()] {
            Expr::Opaque(node) => self.source_rope(*node),
            Expr::Sequence(body) => self.emit_body(*body),
            Expr::Decision(decision) => {
                let head = self.span(decision.head);
                let extent = self.span(decision.extent);
                let inner =
                    self.emit_value_decision(decision, &ValueContinuation::expression(), &[]);
                let mut out = Rope::new();
                out.anchored(AnchorKind::Match, head.start, head.end, extent.end, inner);
                out
            }
            Expr::Propagate(propagate) => {
                if matches!(propagate.exit, ExitTarget::ResultRegion(_)) {
                    let span = self.span(propagate.node);
                    let mut out = Rope::new();
                    out.anchored(
                        AnchorKind::Try,
                        span.start,
                        span.end,
                        span.end,
                        self.emit_propagate(propagate),
                    );
                    return out;
                }
                if !self.recovered_propagations.contains(&expr) {
                    crate::ice::bug!("unscheduled expression try reached inline emission");
                }
                let span = self.span(propagate.node);
                let mut generated = Rope::new();
                generated.push_lit("undefined");
                let mut out = Rope::new();
                out.anchored(AnchorKind::Try, span.start, span.end, span.end, generated);
                out
            }
            Expr::Apply(apply) => self.emit_apply(apply),
            Expr::ResultRegion(region) => self.emit_result_region(expr, region),
            Expr::Template(template) => self.emit_template(template),
        }
    }
}
