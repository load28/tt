//! Source-preserving body, statement, and expression traversal.

use super::*;

/// What a tt value that has no lowering stands as: TypeScript's error type,
/// so nothing the checker says past it is a consequence of the stand-in.
/// The typed projection's recovery writes the same expression.
pub(super) const RECOVERED_VALUE: &str = "(undefined as any)";

fn push_source_edit<'a>(out: &mut Rope<'a>, edit: &LocalSourceEdit) {
    match edit.result_return_mark {
        Some((mark, ResultReturnBoundary::Start)) => {
            out.push_lit(edit.text.clone());
            out.push_result_return_start(mark.start);
        }
        Some((mark, ResultReturnBoundary::End)) => {
            out.push_result_return_end(mark.start);
            out.push_lit(edit.text.clone());
        }
        None => out.push_lit(edit.text.clone()),
    }
}

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

    pub(super) fn replacements_covering(
        &self,
        start: usize,
        end: usize,
    ) -> impl Iterator<Item = &SourceReplacement> {
        self.replacement_index
            .covering(start, end)
            .into_iter()
            .map(|index| &self.source_replacements[index])
    }

    pub(super) fn owner_slots_of(&self, expr: ExprId) -> impl Iterator<Item = &OwnerSlotRewrite> {
        self.owner_slots_by_expr
            .get(&expr)
            .into_iter()
            .flatten()
            .map(|&index| &self.owner_slot_rewrites[index])
    }

    pub(super) fn source_rope(&self, node: NodeId) -> Rope<'a> {
        let span = self.span(node);
        self.source_range_rope(span)
    }

    // A host replacement consumes the authored occurrence, never the source
    // used to emit a value inside that occurrence. In particular a logical
    // operation also contains its separately evaluated condition value.
    pub(super) fn replacement_contains_active_value(&self, source: SourceSpan) -> bool {
        self.active_structured_exprs
            .exprs
            .borrow()
            .iter()
            .any(|expr| {
                let (_, start, _, end) = self.value_anchor(*expr);
                source.start <= start && end <= source.end
            })
    }

    fn capture_is_active(&self, replacement: SourceSpan) -> bool {
        self.active_capture_sources
            .borrow()
            .iter()
            .any(|active| replacement.start <= active.start && active.end <= replacement.end)
    }

    pub(super) fn source_range_rope(&self, span: hir::Span) -> Rope<'a> {
        let mut rope = Rope::new();
        let mut insertions = self
            .owner_slot_index
            .starting_in(span.start, span.end)
            .into_iter()
            .map(|index| &self.owner_slot_rewrites[index])
            .peekable();
        let mut propagation_insertions = self
            .propagation_index
            .starting_in(span.start, span.end)
            .into_iter()
            .map(|index| &self.for_initializer_propagations[index])
            .peekable();
        let mut compose_insertions = self
            .compose_index
            .starting_in(span.start, span.end)
            .into_iter()
            .map(|index| &self.compose_rewrites[index])
            .filter(|rewrite| {
                span.start <= rewrite.owner.start
                    && rewrite.owner.start < span.end
                    && !self.emitted_compose_rewrites.contains(rewrite.owner)
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
            .compose_index
            .ending_in(span.start, span.end.saturating_add(1))
            .into_iter()
            .map(|index| &self.compose_rewrites[index])
            .filter(|rewrite| {
                // `<=` on the left as well: when the body's last token is a
                // tt value, the source that follows begins exactly where the
                // body ends, and that span is the only one that can carry
                // the brace. Writing it twice is prevented by the registry
                // the suffix claims, not by this range.
                rewrite.owner_kind == HostOwnerKind::ArrowExpression
                    && span.start <= rewrite.owner.end
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
            .loop_body_index
            .ending_in(span.start.saturating_add(1), span.end.saturating_add(1))
            .into_iter()
            .map(|index| &self.loop_test_rewrites[index])
            .collect();
        loop_endings.sort_unstable_by_key(|rewrite| rewrite.body.end);
        let mut loop_endings = loop_endings.into_iter().peekable();
        let mut cursor = span.start;
        while cursor < span.end {
            self.close_owner_blocks_at(cursor, &mut rope);
            while let Some(_rewrite) = loop_endings.next_if(|rewrite| rewrite.body.end == cursor) {
                rope.push_lit("}");
            }
            while let Some(rewrite) = compose_endings.next_if(|rewrite| rewrite.owner.end == cursor)
            {
                rope.append(self.emit_compose_suffix(rewrite));
            }
            if let Some(documentation) = self.documentation_starts.get(&cursor)
                && documentation.end <= span.end
                && !self.emitted_documentation.contains(*documentation)
            {
                cursor = documentation.end;
                continue;
            }
            self.open_declaration_blocks_at(cursor, &mut rope);
            if let Some(split) = self.declarator_splits.iter().find(|split| {
                split.separator.start == cursor
                    && self.emitted_declarator_separators.claim(split.separator)
            }) {
                rope.push_lit(";");
                cursor = split.separator.end;
                continue;
            }
            let split_head = self.declarator_splits.iter().find(|split| {
                split.at == cursor
                    && self.emitted_declarator_separators.contains(split.separator)
                    && self.emitted_declarator_heads.claim(split.separator)
            });
            if let Some(split) = split_head {
                rope = rope.trim_end();
                if self.opened_declaration_scopes.contains(split.statement) {
                    rope.push_break(0);
                } else {
                    let mut separation = Rope::new();
                    separation.push_break(0);
                    rope.append(Rope::scoped(separation));
                }
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
                if self.emitted_compose_rewrites.claim(rewrite.owner) {
                    rope.append(self.emit_compose_rewrite(rewrite));
                }
            }
            if let Some(documentation) = self.relocated_documentation(cursor) {
                rope.append(documentation);
            }
            if let Some(split) = split_head {
                rope.push_lit(split.head.clone());
                if split.last
                    && self.opened_declaration_scopes.contains(split.statement)
                    && self.closed_declaration_scopes.claim(split.statement)
                {
                    rope.push_scope_close();
                }
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
                && self.emitted_loop_tests.claim(rewrite.owner)
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
            if let Some(replacement) = self
                .replacement_index
                .containing(cursor)
                .into_iter()
                .map(|index| &self.source_replacements[index])
                .find(|replacement| {
                    if replacement.anchor.is_some() {
                        !replacement
                            .anchor
                            .is_some_and(|expr| self.active_structured_exprs.contains(expr))
                            && self.conditional_region_depth.get() == 0
                            && self.loop_region_depth.get() == 0
                            && !self.replacement_contains_active_value(replacement.source)
                            && replacement.source.start <= cursor
                            && cursor < replacement.source.end
                    } else {
                        !self.capture_is_active(replacement.source)
                            && replacement.source.start <= cursor
                            && cursor < replacement.source.end
                            && !self.replacement_contains_active_value(replacement.source)
                            && !self.inside_captured_value(replacement.source, span.start, span.end)
                    }
                })
            {
                if cursor == replacement.source.start {
                    if replacement.jsx_child {
                        rope.push_lit("{");
                    }
                    match replacement.anchor {
                        Some(expr) => {
                            let (kind, start, end, extent) = self.value_anchor(expr);
                            let mut name = Rope::new();
                            name.push_lit(replacement.written().to_owned());
                            rope.anchored(kind, start, end, extent, name);
                        }
                        None => rope.push_lit(replacement.written().to_owned()),
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
                .replacement_index
                .starting_after(cursor)
                .map(|index| &self.source_replacements[index])
                .take_while(|replacement| replacement.source.start < span.end)
                .find(|replacement| {
                    replacement.anchor.is_none()
                        || (self.conditional_region_depth.get() == 0
                            && self.loop_region_depth.get() == 0
                            && !self.replacement_contains_active_value(replacement.source))
                })
                .map_or(span.end, |replacement| replacement.source.start);
            let next_owner_end = self
                .block_required_by_end
                .range(cursor.saturating_add(1)..span.end.max(cursor.saturating_add(1)))
                .next()
                .map_or(span.end, |(end, _)| *end);
            let next_split = self
                .declarator_splits
                .iter()
                .flat_map(|split| {
                    [split.statement.start, split.separator.start, split.at]
                        .into_iter()
                        .chain(split.block.map(|block| block.start))
                })
                .filter(|boundary| cursor < *boundary && *boundary < span.end)
                .min()
                .unwrap_or(span.end);
            let next_documentation = self
                .documentation_starts
                .range(cursor.saturating_add(1)..span.end.max(cursor.saturating_add(1)))
                .next()
                .map_or(span.end, |(start, _)| *start);
            let next = next_insertion
                .min(next_documentation)
                .min(next_split)
                .min(next_owner_end)
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
        self.close_owner_blocks_at(span.end, &mut rope);
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
            // An edit that rewrites the head of a host owner (a return a
            // value region turns into its exit) still runs the owner's
            // prelude first.
            if let Some(rewrite) = self.compose_rewrites.iter().find(|rewrite| {
                rewrite.owner.start == edit.span.start
                    && !self.emitted_compose_rewrites.contains(rewrite.owner)
            }) {
                self.emitted_compose_rewrites.claim(rewrite.owner);
                out.append(self.emit_compose_rewrite(rewrite));
            }
            push_source_edit(&mut out, edit);
            cursor = edit.span.end;
        }
        if cursor < span.end {
            out.append(self.source_range_rope(hir::Span::new(cursor, span.end)));
        }
        out
    }

    /// Emits the file's root body. Every planned host prelude is written by
    /// the one owner that consumes it; a prelude left unwritten would leave
    /// its slots unassigned in the output.
    pub(in super::super) fn emit_file(&self, root: hir::BodyId) -> Rope<'a> {
        let out = self.emit_body(root);
        if let Some(rewrite) = self
            .compose_rewrites
            .iter()
            .find(|rewrite| !self.emitted_compose_rewrites.contains(rewrite.owner))
        {
            crate::ice::bug!(
                "the planned prelude of the host owner at {}..{} was not emitted",
                rewrite.owner.start,
                rewrite.owner.end
            );
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
        let mut structured_returns = Vec::new();
        for exit in exits {
            if let Some(expr) = exit
                .value_argument
                .and_then(|argument| self.returned_structured_expr(body, argument))
            {
                structured_returns.push((exit, expr));
                continue;
            }
            let line_start = crate::lines::line_start_before(self.source, exit.statement.start);
            let line_indent = &self.source[line_start..exit.statement.start];
            let starts_own_line = line_indent.bytes().all(|byte| matches!(byte, b' ' | b'\t'));
            match exit.argument {
                Some(argument) => {
                    let grouped =
                        grouping_required(self.source[argument.start..argument.end].trim(), self.source_kind);
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
        let mut out = Rope::new();
        let statements = &self.core.bodies[body.index()].statements;
        for (index, statement) in statements.iter().enumerate() {
            match statement {
                Statement::Opaque(node) if !structured_returns.is_empty() => {
                    let span = self.span(*node);
                    let mut local = edits.clone();
                    for (exit, _) in &structured_returns {
                        let clipped = SourceSpan {
                            start: span.start.max(exit.statement.start),
                            end: span.end.min(exit.statement.end),
                        };
                        if clipped.start < clipped.end {
                            local.push(LocalSourceEdit {
                                span: clipped,
                                text: String::new(),
                                result_return_mark: None,
                            });
                        }
                    }
                    local.sort_unstable_by_key(|edit| edit.span.start);
                    out.append(self.source_rope_with_edits(*node, &local));
                }
                Statement::Expr(expr)
                    if structured_returns
                        .iter()
                        .any(|(_, returned)| returned == expr) =>
                {
                    let (exit, _) = structured_returns
                        .iter()
                        .find(|(_, returned)| returned == expr)
                        .expect("matched above");
                    if exit.requires_block {
                        out.push_lit("{ ");
                    }
                    out.append(self.emit_returned_structured_value(
                        *expr,
                        exit.argument.expect("structured return has an argument"),
                        continuation,
                    ));
                    out.push_break(0);
                    out.push_lit(leave.clone());
                    if exit.requires_block {
                        out.push_lit(" }");
                    }
                }
                Statement::Decision(decision) => {
                    self.emit_statement_decision(decision, &mut out, &|body| {
                        self.emit_body_with_exits(
                            body,
                            exits,
                            continuation,
                            label,
                            generated_indent,
                        )
                    })
                }
                _ => {
                    if self.emit_statement_with_edits(statement, &edits, &mut out) {
                        out.append(self.edits_after_statement(statements, index, &edits));
                    }
                }
            }
        }
        out
    }

    /// Deliver a structured return argument while retaining authored wrappers.
    pub(super) fn emit_returned_structured_value(
        &self,
        expr: ExprId,
        argument: SourceSpan,
        continuation: &ValueContinuation<'_>,
    ) -> Rope<'a> {
        if structured_expr_span(self.semantic, self.core, expr) == Some(argument) {
            return self
                .emit_continued_expr(expr, continuation)
                .unwrap_or_else(|| crate::ice::bug!("returned structured value was not emitted"));
        }
        let slot = self
            .structured_value_slot(expr)
            .unwrap_or_else(|| crate::ice::bug!("wrapped returned value has no storage"));
        let mut out = Rope::new();
        out.push_value_declaration(slot);
        out.push_break(0);
        out.append(
            self.emit_continued_expr(expr, &ValueContinuation::assign(slot))
                .unwrap_or_else(|| crate::ice::bug!("wrapped returned value was not emitted")),
        );
        out.push_break(0);
        out.append(self.emit_value_delivery_without_region_exit(
            self.source_range_with_value_slots(argument, &[expr]),
            continuation,
        ));
        out
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
        for (index, statement) in statements.iter().enumerate() {
            if self.emit_statement_with_edits(statement, edits, &mut out) {
                out.append(self.edits_after_statement(statements, index, edits));
            }
        }
        out
    }

    pub(super) fn emit_statement_with_edits(
        &self,
        statement: &Statement,
        edits: &[LocalSourceEdit],
        out: &mut Rope<'a>,
    ) -> bool {
        {
            let relocated_node = match statement {
                Statement::Decision(decision) => Some(decision.extent),
                Statement::Propagate(propagate) => Some(propagate.owner),
                Statement::Adt(adt) => Some(adt.node),
                _ => None,
            };
            if let Some(node) = relocated_node {
                let span = self
                    .semantic
                    .hir
                    .source_map
                    .node_extent(node)
                    .expect("statement extent");
                if self
                    .replacements_covering(span.start, span.end)
                    .any(|capture| {
                        (capture.anchor.is_none()
                            || (!capture.claim
                                && self.conditional_region_depth.get() == 0
                                && self.loop_region_depth.get() == 0))
                            && capture.source.start <= span.start
                            && span.end <= capture.source.end
                            && !self.capture_is_active(capture.source)
                            && !self.replacement_contains_active_value(capture.source)
                            && !capture
                                .anchor
                                .is_some_and(|expr| self.active_structured_exprs.contains(expr))
                    })
                {
                    return false;
                }
            }
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
                        emit_adt(
                            adt,
                            self.source,
                            |node| self.span(node),
                            self.ambient_items.contains(&adt.node),
                            self.source_kind,
                        ),
                    );
                }
                Statement::Import(import) => self.emit_import(import, out),
                Statement::Propagate(propagate) => {
                    out.append(self.emit_propagate_owner_prelude(propagate));
                    let span = self.span(propagate.node);
                    let mut emitted = if self.is_for_initializer_propagation(propagate.node) {
                        self.emit_for_initializer_payload(propagate)
                    } else {
                        self.emit_propagate(propagate)
                    };
                    if self.block_required_statements.contains(&propagate.node) {
                        emitted = Rope::braced(emitted);
                    }
                    out.anchored(AnchorKind::Try, span.start, span.end, span.end, emitted);
                }
                Statement::Decision(decision) => {
                    self.emit_statement_decision(decision, out, &|body| {
                        self.emit_statements_with_edits(
                            &self.core.bodies[body.index()].statements,
                            edits,
                        )
                    })
                }
                Statement::Expr(expr) if self.statement_expr_requires_lowering(*expr) => {
                    self.emit_statement_expr(*expr, out);
                }
                Statement::Expr(expr) => out.append(self.emit_expr(*expr)),
            }
        }
        true
    }

    pub(super) fn edits_after_statement(
        &self,
        statements: &[Statement],
        index: usize,
        edits: &[LocalSourceEdit],
    ) -> Rope<'a> {
        let mut out = Rope::new();
        let end = match &statements[index] {
            Statement::Expr(expr) => structured_expr_span(self.semantic, self.core, *expr),
            Statement::Decision(decision) => Some(self.span(decision.extent).into()),
            Statement::Propagate(propagate) => Some(self.span(propagate.owner).into()),
            Statement::Adt(adt) => Some(self.span(adt.node).into()),
            Statement::Opaque(_) | Statement::Import(_) => None,
        }
        .map(|span: SourceSpan| span.end);
        let Some(end) = end else {
            return out;
        };
        let held = statements.iter().any(|statement| {
            matches!(statement, Statement::Opaque(node) if {
                let span = self.span(*node);
                span.start <= end && end <= span.end
            })
        });
        if held {
            return out;
        }
        for edit in edits
            .iter()
            .filter(|edit| edit.span.start == end && edit.span.end == end)
        {
            push_source_edit(&mut out, edit);
        }
        out
    }

    /// The values a `try` statement's operand holds run in the statement's
    /// prelude, before the operand is read into the propagation temporary.
    pub(super) fn emit_propagate_owner_prelude(&self, propagate: &Propagate) -> Rope<'a> {
        let owner = SourceSpan::from(self.span(propagate.owner));
        match self
            .compose_rewrites
            .iter()
            .find(|rewrite| rewrite.owner == owner)
        {
            Some(rewrite) if self.emitted_compose_rewrites.claim(rewrite.owner) => {
                self.emit_compose_rewrite(rewrite)
            }
            _ => Rope::new(),
        }
    }

    pub(super) fn statement_expr_requires_lowering(&self, expr: ExprId) -> bool {
        self.owner_slots_of(expr).any(|rewrite| {
            rewrite.expr == expr && rewrite.continuation == HostContinuation::Discard
        }) || (matches!(self.core.exprs[expr.index()], Expr::Decision(_))
            && !self
                .owner_slots_of(expr)
                .any(|rewrite| rewrite.expr == expr)
            && !self.value_slots.contains_key(&expr))
    }

    pub(super) fn emit_statement_expr(&self, expr: ExprId, out: &mut Rope<'a>) {
        // A tt value that is itself an expression statement has no opaque
        // source owner around it where `source_range_rope` could insert the
        // planned statement region. Consume that plan here before the inline
        // occurrence is replaced by its join slot.
        if let Some(rewrite) = self.owner_slots_of(expr).find(|rewrite| {
            rewrite.expr == expr && rewrite.continuation == HostContinuation::Discard
        }) {
            if self.emitted_owner_rewrites.contains(expr) {
                out.append(self.emit_expr(expr));
            } else {
                self.emitted_owner_rewrites.mark(expr);
                out.append(self.emit_owner_slot_rewrite(rewrite));
            }
            self.close_owner_blocks_at(rewrite.source.end, out);
            return;
        }
        if self.emitted_owner_rewrites.contains(expr) {
            out.append(self.emit_expr(expr));
            return;
        }
        if matches!(self.core.exprs[expr.index()], Expr::Decision(_)) {
            if let Some(slot) = self.value_slots.get(&expr) {
                out.push_value_declaration(slot);
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
            let recovery = self.generated_name("$tt_recovery");
            out.push_lit(format!("(() => {{ let {recovery}; "));
            out.append(
                self.emit_continued_expr(expr, &ValueContinuation::assign(&recovery))
                    .unwrap_or_else(|| {
                        crate::ice::bug!("statement match has no expression-boundary emission")
                    }),
            );
            out.push_lit(format!(" return {recovery}; }})()"));
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
        if let Some((value_index, value)) = self.core.bodies[body.index()].value
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

        let (mut out, value) = self.emit_sequence_operand(body, continuation)?;
        out.append(self.emit_value_delivery_without_region_exit(value, continuation));
        Some(Rope::scoped(out))
    }

    pub(super) fn emit_sequence_operand(
        &self,
        body: hir::BodyId,
        continuation: &ValueContinuation<'_>,
    ) -> Option<(Rope<'a>, Rope<'a>)> {
        let mut nested = Vec::new();
        self.collect_operand_values(body, &mut nested);
        self.emit_operand(self.body_extent(body), &nested, continuation)
    }

    pub(super) fn emit_template_operand(
        &self,
        expr: ExprId,
        template: &Template,
        continuation: &ValueContinuation<'_>,
    ) -> Option<(Rope<'a>, Rope<'a>)> {
        let mut nested = Vec::new();
        for part in &template.parts {
            if let TemplatePart::Interpolation(inner) = part {
                self.collect_operand_value(*inner, &mut nested);
            }
        }
        let span = structured_expr_span(self.semantic, self.core, expr)
            .unwrap_or_else(|| crate::ice::bug!("a template has no source extent"));
        self.emit_operand(span, &nested, continuation)
    }

    fn emit_operand(
        &self,
        span: SourceSpan,
        nested: &[(ExprId, String)],
        continuation: &ValueContinuation<'_>,
    ) -> Option<(Rope<'a>, Rope<'a>)> {
        if nested.is_empty() {
            return None;
        }
        let mut out = Rope::new();
        let mut captured = HashSet::new();
        let mut scheduled = Vec::new();
        let mut operations: Vec<&PlannedConditionalOperation> = Vec::new();
        for (inner, slot) in nested {
            if self.emitted_owner_rewrites.contains(*inner) {
                continue;
            }
            if let Some(operation) = self
                .nested_operations
                .iter()
                .find(|operation| operation.values.contains(inner))
            {
                if operations.contains(&operation) {
                    continue;
                }
                operations.push(operation);
                out.push_value_declaration(self.value_slot_name(operation.result));
                out.push_break(0);
                let mut lowered = self.emit_conditional_operation(operation, &mut captured);
                for step in operation.outer.iter().take_while(|step| {
                    span.start <= step.parent.start && step.parent.end <= span.end
                }) {
                    lowered = self.emit_scheduled_step(step, lowered, &mut captured);
                    scheduled.push(step);
                }
                out.append(lowered.trim_end());
                out.push_break(0);
                continue;
            }
            if !continuation.is_unwrapped_assignment_to(slot) {
                out.push_value_declaration(slot);
                out.push_break(0);
            }
            let steps = self.scheduled_steps_within(*inner, span);
            let mut action = self.emit_continued_expr(*inner, &ValueContinuation::assign(slot))?;
            for step in steps {
                action = self.emit_scheduled_step(step, action, &mut captured);
                scheduled.push(step);
            }
            out.append(action.trim_end());
            out.push_break(0);
        }
        let values: Vec<_> = nested.iter().map(|(inner, _)| *inner).collect();
        Some((
            out,
            self.source_range_with_scheduled_values(span, &values, &scheduled, &operations),
        ))
    }

    fn body_extent(&self, body: hir::BodyId) -> SourceSpan {
        if let Some(node) = self.core.sequence_node(body) {
            return SourceSpan::from(self.span(node));
        }
        self.core.bodies[body.index()]
            .statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Opaque(node) => Some(SourceSpan::from(self.span(*node))),
                Statement::Expr(expr) => structured_expr_span(self.semantic, self.core, *expr),
                Statement::Adt(_)
                | Statement::Import(_)
                | Statement::Propagate(_)
                | Statement::Decision(_) => None,
            })
            .reduce(|extent, span| SourceSpan {
                start: extent.start.min(span.start),
                end: extent.end.max(span.end),
            })
            .unwrap_or_else(|| crate::ice::bug!("an operand body has no source extent"))
    }

    fn collect_operand_values(&self, body: hir::BodyId, out: &mut Vec<(ExprId, String)>) {
        for statement in &self.core.bodies[body.index()].statements {
            let Statement::Expr(inner) = statement else {
                continue;
            };
            self.collect_operand_value(*inner, out);
        }
    }

    fn collect_operand_value(&self, expr: ExprId, out: &mut Vec<(ExprId, String)>) {
        if !self.core.has_statement_form(expr) || self.slot_exprs.contains_key(&expr) {
            return;
        }
        match &self.core.exprs[expr.index()] {
            Expr::Sequence(body) => self.collect_operand_values(*body, out),
            Expr::Template(template) => {
                for part in &template.parts {
                    if let TemplatePart::Interpolation(inner) = part {
                        self.collect_operand_value(*inner, out);
                    }
                }
            }
            Expr::Decision(_) | Expr::Propagate(_) | Expr::Apply(_) => {
                if self.structurally_nested_values.contains(&expr)
                    && let Some(slot) = self.value_slots.get(&expr)
                {
                    out.push((expr, slot.clone()));
                }
            }
            Expr::ResultRegion(_) | Expr::Opaque(_) => {}
        }
    }

    pub(super) fn emit_expr(&self, expr: ExprId) -> Rope<'a> {
        // A structured expression can own the first byte of a host region.
        // Enter that region before substituting any captured source inside it,
        // just as the opaque-source traversal does at the same boundary.
        if let Some(span) = structured_expr_span(self.semantic, self.core, expr)
            && let Some(rewrite) = self.compose_rewrites.iter().find(|rewrite| {
                rewrite.owner.start == span.start
                    && !self.emitted_compose_rewrites.contains(rewrite.owner)
                    && !rewrite.actions.iter().any(|action| match action {
                        ComposeAction::Value(value) => {
                            self.active_structured_exprs.contains(value.expr)
                        }
                        ComposeAction::Operation(operation) => operation
                            .values
                            .iter()
                            .any(|value| self.active_structured_exprs.contains(*value)),
                    })
            })
        {
            self.emitted_compose_rewrites.claim(rewrite.owner);
            let mut out = self.emit_compose_rewrite(rewrite);
            out.append(self.emit_expr(expr));
            if rewrite.owner_kind == HostOwnerKind::ArrowExpression && span.end == rewrite.owner.end
            {
                out.append(self.emit_compose_suffix(rewrite));
            }
            return out;
        }
        // A completed call owns its entire authored frame, including tt
        // expressions in earlier arguments. Those arguments are emitted at
        // their capture sites, not again beside the completed call's result.
        if !self.active_structured_exprs.contains(expr)
            && let Some(span) = structured_expr_span(self.semantic, self.core, expr)
            && self
                .replacements_covering(span.start, span.end)
                .any(|frame| {
                    frame.claim
                        && frame.source.start <= span.start
                        && span.end <= frame.source.end
                        && !frame
                            .anchor
                            .is_some_and(|value| self.active_structured_exprs.contains(value))
                        && !self.capture_is_active(frame.source)
                        && !self.replacement_contains_active_value(frame.source)
                })
        {
            return Rope::new();
        }
        // A source capture owns complete tt expressions as well as opaque
        // chunks. Substitute the value at its authored occurrence, rather
        // than reconstructing its operators around already captured children.
        if !matches!(
            self.core.exprs[expr.index()],
            Expr::Opaque(_) | Expr::Sequence(_)
        ) && !self.active_structured_exprs.contains(expr)
            && let Some(span) = structured_expr_span(self.semantic, self.core, expr)
            && let Some(capture) =
                self.replacements_covering(span.start, span.end)
                    .find(|capture| {
                        (capture.anchor.is_none()
                            || (!capture.claim
                                && self.conditional_region_depth.get() == 0
                                && self.loop_region_depth.get() == 0))
                            && capture.source.start <= span.start
                            && span.end <= capture.source.end
                            && !self.capture_is_active(capture.source)
                            && !self.replacement_contains_active_value(capture.source)
                    })
        {
            let mut out = Rope::new();
            if span.start == capture.source.start {
                let mut written = Rope::new();
                written.push_lit(capture.written().to_owned());
                match capture.anchor {
                    Some(value) => {
                        let (kind, start, end, extent) = self.value_anchor(value);
                        out.anchored(kind, start, end, extent, written);
                    }
                    None => out.append(written),
                }
            }
            return out;
        }
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
                if rewrite.actions.len() != 1
                    || self.emitted_compose_rewrites.contains(rewrite.owner)
                {
                    return None;
                }
                let ComposeAction::Value(value) = &rewrite.actions[0] else {
                    return None;
                };
                (value.expr == expr && rewrite.owner.start == value.source.start)
                    .then_some((rewrite, value))
            })
        {
            self.emitted_compose_rewrites.claim(rewrite.owner);
            let _active = self.active_structured_exprs.enter(expr);
            let mut out = self.emit_compose_rewrite(rewrite);
            if value.defer_arm_values {
                out.append(self.emit_selected_arm_values(value.expr, &value.slot));
            } else {
                out.push_lit(value.slot.clone());
            }
            match rewrite.owner_kind {
                // The block closes where the arrow body ends. That is here
                // only when the value *is* the whole body; when source
                // follows it, the walk over that source closes the block
                // after it, so closing here would leave the rest outside
                // the arrow.
                HostOwnerKind::ArrowExpression if value.source.end == rewrite.owner.end => {
                    out.append(self.emit_compose_suffix(rewrite));
                }
                HostOwnerKind::ArrowExpression => {}
                // The Core body retains a trailing statement/module frame
                // (normally the authored semicolon) outside the direct
                // expression and emits it after this value.
                HostOwnerKind::Statement
                | HostOwnerKind::ModuleItem
                | HostOwnerKind::Declarator => {}
            }
            return out;
        }
        if let Some(rewrite) = self
            .arrow_returns_by_expr
            .get(&expr)
            .map(|&index| &self.arrow_return_rewrites[index])
        {
            return self.emit_arrow_return_rewrite(rewrite);
        }
        if let Some(slot) = self.slot_exprs.get(&expr) {
            let (_, anchor_start, _, anchor_end) = self.value_anchor(expr);
            if self.carried_by_capture(anchor_start, anchor_end) {
                return Rope::new();
            }
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
            Expr::Decision(decision) if self.recovered_matches.contains(&expr) => {
                let head = self.span(decision.head);
                let extent = self.span(decision.extent);
                let mut generated = Rope::new();
                generated.push_lit(RECOVERED_VALUE);
                let mut out = Rope::new();
                out.anchored(
                    AnchorKind::Match,
                    head.start,
                    head.end,
                    extent.end,
                    generated,
                );
                out
            }
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
                if self.owner_model && !self.recovered_propagations.contains(&expr) {
                    crate::ice::bug!(
                        "unscheduled expression try reached inline emission: {:?} {:?}",
                        expr,
                        self.span(propagate.node)
                    );
                }
                let span = self.span(propagate.node);
                let mut generated = Rope::new();
                if self.owner_model {
                    generated.push_lit(RECOVERED_VALUE);
                } else {
                    // No owner to hold the early exit: the operand is still
                    // the user's expression, evaluated where it stands as
                    // the argument, and the glue only reads its success
                    // payload under the Result ABI a statement `try` tests.
                    self.recovered_sources
                        .borrow_mut()
                        .push(SourceSpan::from(span));
                    let result = self.generated_name("$tt_result");
                    generated.push_lit(format!(
                        "(({result}) => {{ if ({}) throw {result}; return {result}.{}; }})(",
                        result_failure_test(&result, propagate.layout),
                        propagate.layout.payload_field,
                    ));
                    generated.append(self.emit_expr(propagate.value));
                    generated.push_lit(")");
                }
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
