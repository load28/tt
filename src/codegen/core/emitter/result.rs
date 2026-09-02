//! Result-region and propagation emission.

use super::*;

impl<'a> Emitter<'a> {
    /// The lowering of one `try`. It opens its own layout scope: a
    /// structured propagation value writes block structure into it.
    pub(super) fn emit_propagate(&self, propagate: &Propagate) -> Rope<'a> {
        let temp = temp_name(propagate.temporary);
        let mut out = self.emit_propagate_input(propagate.value, &temp);
        out.push_break(0);
        out.push_lit(format!(
            "if ({}) {{",
            result_failure_test(&temp, propagate.layout)
        ));
        out.push_break(1);
        out.push_lit(format!("return {temp};"));
        out.push_break(0);
        out.push_lit("}");
        if let Some(binding) = propagate.binding {
            out.push_break(0);
            out.push_lit(format!("{} ", binding_keyword(binding.mode)));
            out.append(self.source_rope(binding.node));
            out.push_lit(format!(" = {temp}.{};", propagate.layout.payload_field));
        }
        Rope::scoped(out)
    }

    pub(super) fn emit_propagate_input(&self, value: ExprId, temp: &str) -> Rope<'a> {
        let mut out = Rope::new();
        let structured = self
            .core
            .has_statement_form(value)
            .then(|| self.structured_value_slot(value))
            .flatten()
            .and_then(|slot| {
                self.emit_continued_expr(value, &ValueContinuation::assign(slot))
                    .map(|continued| (slot, continued))
            });
        if let Some((slot, continued)) = structured {
            out.push_lit(format!("let {slot};"));
            out.push_break(0);
            out.append(continued);
            out.push_break(0);
            out.push_lit(format!("const {temp} = {slot};"));
        } else {
            out.push_lit(format!("const {temp} = "));
            push_grouped(&mut out, self.emit_expr(value).trim());
            out.push_lit(";");
        }
        out
    }

    pub(super) fn emit_result_region(&self, expr: ExprId, region: &ResultRegion) -> Rope<'a> {
        let mut out = Rope::new();
        self.used_expression_boundary.set(true);
        out.push_lit(if region.is_async {
            format!("(await {}(async () => {{", self.expression_boundary_name)
        } else {
            format!("{}(() => {{", self.expression_boundary_name)
        });
        out.push_break(1);
        for item in &region.items {
            let ResultRegionItem::Statements(body) = item;
            let exits = self.exits_for_expr(expr);
            let failure = ValueContinuation::returning();
            let success = failure.wrap_result_ok();
            out.append(Rope::indented(
                1,
                self.emit_result_body_with_exits(*body, &exits, &failure, &success, None)
                    .trim(),
            ));
            out.push_break(1);
        }
        // HIR computes `completes` with the conservative flow graph and sema
        // consumes that same stored fact when it rejects a fall-through
        // statement Result. Omitting the fallback is therefore sound: this
        // branch can disappear only when every reachable path already exits.
        if region.value.is_some() || !region.completes {
            out.push_lit("return { kind: \"Ok\" as const, value: ");
            if let Some(value) = region.value {
                push_grouped(
                    &mut out,
                    guard_line_comment(self.emit_expr(value).trim(), 0),
                );
            } else {
                out.push_lit("undefined");
            }
            out.push_lit(" };");
            out.push_break(0);
        }
        out.push_lit(if region.is_async { "}))" } else { "})" });
        Rope::scoped(out)
    }

    /// Result-owned returns inside the expression-boundary printer complete
    /// its lexical arrow. They must therefore wrap `Ok`, unlike ordinary
    /// function returns and unlike the statement-host continuation below.
    pub(super) fn emit_result_body_with_exits(
        &self,
        body: hir::BodyId,
        exits: &[HostExit],
        failure: &ValueContinuation<'_>,
        success: &ValueContinuation<'_>,
        exit_label: Option<&str>,
    ) -> Rope<'a> {
        let leave =
            exit_label.map_or_else(|| "break;".to_owned(), |label| format!("break {label};"));
        let mut edits = Vec::new();
        let mut propagating_returns = Vec::new();
        let mut structured_returns = Vec::new();
        for exit in exits {
            let line_start = self.source[..exit.statement.start]
                .rfind('\n')
                .map_or(0, |index| index + 1);
            let line_indent = &self.source[line_start..exit.statement.start];
            let starts_own_line = line_indent.bytes().all(|byte| matches!(byte, b' ' | b'\t'));
            let inner_indent = format!("{line_indent}  ");
            if let Some((expr, propagate)) = exit
                .argument
                .and_then(|argument| self.result_return_propagate(body, argument))
            {
                propagating_returns.push((
                    exit.statement,
                    exit.argument.expect("checked above"),
                    expr,
                    propagate,
                ));
                continue;
            }
            if let Some((argument, expr)) = exit.argument.and_then(|argument| {
                self.result_return_structured_expr(body, argument)
                    .map(|expr| (argument, expr))
            }) {
                structured_returns.push((exit.statement, argument, expr));
                continue;
            }
            match exit.argument {
                Some(argument) => {
                    let grouped = grouping_required(&self.source[argument.start..argument.end]);
                    edits.push(LocalSourceEdit {
                        span: SourceSpan {
                            start: exit.statement.start,
                            end: argument.start,
                        },
                        text: if starts_own_line {
                            format!("{{\n{inner_indent}{}", success.assignment_prefix(grouped))
                        } else {
                            format!("{{ {}", success.assignment_prefix(grouped))
                        },
                        result_return_mark: Some((argument, ResultReturnBoundary::Start)),
                    });
                    edits.push(LocalSourceEdit {
                        span: SourceSpan {
                            start: argument.end,
                            end: exit.statement.end,
                        },
                        text: if starts_own_line {
                            format!(
                                "{};{}\n{line_indent}}}",
                                success.assignment_suffix(grouped),
                                if success.assigns() {
                                    format!("\n{inner_indent}{leave}")
                                } else {
                                    String::new()
                                }
                            )
                        } else {
                            format!(
                                "{};{} }}",
                                success.assignment_suffix(grouped),
                                if success.assigns() {
                                    format!(" {leave}")
                                } else {
                                    String::new()
                                }
                            )
                        },
                        result_return_mark: Some((argument, ResultReturnBoundary::End)),
                    });
                }
                None => {
                    let mut text = if starts_own_line {
                        format!(
                            "{{\n{inner_indent}{}undefined{};",
                            success.assignment_prefix(false),
                            success.assignment_suffix(false),
                        )
                    } else {
                        format!(
                            "{{{}undefined{};",
                            success.assignment_prefix(false),
                            success.assignment_suffix(false),
                        )
                    };
                    if success.assigns() {
                        if starts_own_line {
                            text.push('\n');
                            text.push_str(&inner_indent);
                        } else {
                            text.push(' ');
                        }
                        text.push_str(&leave);
                    }
                    if starts_own_line {
                        text.push('\n');
                        text.push_str(line_indent);
                        text.push('}');
                    } else {
                        text.push_str(" }");
                    }
                    edits.push(LocalSourceEdit {
                        span: exit.statement,
                        text,
                        result_return_mark: None,
                    });
                }
            }
        }
        edits.sort_unstable_by_key(|edit| edit.span.start);
        let context = ResultEmissionContext {
            failure,
            success,
            exit_label,
        };
        self.emit_result_statements_with_exits(
            &self.core.bodies[body.index()].statements,
            exits,
            &edits,
            &propagating_returns,
            &structured_returns,
            &context,
        )
    }

    pub(super) fn result_return_structured_expr(
        &self,
        body: hir::BodyId,
        argument: SourceSpan,
    ) -> Option<ExprId> {
        self.core.bodies[body.index()]
            .statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Expr(expr) if self.core.has_statement_form(*expr) => Some(*expr),
                _ => None,
            })
            .find(|expr| structured_expr_span(self.semantic, self.core, *expr) == Some(argument))
    }

    /// Finds a `return try value;` whose `try` belongs directly to this
    /// Result region. The host parser gives the return argument span, while
    /// Core IR gives the propagation target; both facts are required before
    /// replacing an entire TypeScript statement.
    pub(super) fn result_return_propagate(
        &self,
        body: hir::BodyId,
        argument: SourceSpan,
    ) -> Option<(ExprId, &Propagate)> {
        fn find<'a>(
            emitter: &'a Emitter<'a>,
            expr: ExprId,
            argument: SourceSpan,
        ) -> Option<(ExprId, &'a Propagate)> {
            match &emitter.core.exprs[expr.index()] {
                Expr::Propagate(propagate)
                    if matches!(propagate.exit, ExitTarget::ResultRegion(_)) && {
                        let span = SourceSpan::from(emitter.span(propagate.node));
                        argument.start <= span.start && span.end <= argument.end
                    } =>
                {
                    Some((expr, propagate))
                }
                Expr::Sequence(body) => emitter.core.bodies[body.index()]
                    .statements
                    .iter()
                    .find_map(|statement| match statement {
                        Statement::Expr(inner) => find(emitter, *inner, argument),
                        _ => None,
                    }),
                _ => None,
            }
        }
        self.core.bodies[body.index()]
            .statements
            .iter()
            .find_map(|statement| {
                let Statement::Expr(expr) = statement else {
                    return None;
                };
                find(self, *expr, argument)
            })
    }

    pub(super) fn emit_result_statements_with_exits(
        &self,
        statements: &[Statement],
        exits: &[HostExit],
        edits: &[LocalSourceEdit],
        propagating_returns: &[(SourceSpan, SourceSpan, ExprId, &Propagate)],
        structured_returns: &[(SourceSpan, SourceSpan, ExprId)],
        context: &ResultEmissionContext<'_, '_>,
    ) -> Rope<'a> {
        let mut out = Rope::new();
        for statement in statements {
            match statement {
                Statement::Opaque(node) => {
                    // An opaque segment may contain both a nested function
                    // and the Result-owned return that follows it.  Dropping
                    // the whole segment would also drop the nested function's
                    // punctuation and source-backed propagation.  Erase only
                    // the return being rebuilt below; every surrounding byte
                    // remains pass-through.
                    let mut opaque_edits = edits.to_vec();
                    opaque_edits.extend(propagating_returns.iter().map(
                        |(return_span, _, _, _)| LocalSourceEdit {
                            span: SourceSpan {
                                start: return_span.start.max(self.span(*node).start),
                                end: return_span.end.min(self.span(*node).end),
                            },
                            text: String::new(),
                            result_return_mark: None,
                        },
                    ));
                    opaque_edits.extend(structured_returns.iter().map(|(return_span, _, _)| {
                        LocalSourceEdit {
                            span: SourceSpan {
                                start: return_span.start.max(self.span(*node).start),
                                end: return_span.end.min(self.span(*node).end),
                            },
                            text: String::new(),
                            result_return_mark: None,
                        }
                    }));
                    opaque_edits.retain(|edit| edit.span.start < edit.span.end);
                    opaque_edits.sort_unstable_by_key(|edit| edit.span.start);
                    out.append(self.source_rope_with_edits(*node, &opaque_edits));
                }
                Statement::Expr(expr) => {
                    if let Some((return_span, _, _)) = structured_returns
                        .iter()
                        .find(|(_, _, candidate)| candidate == expr)
                    {
                        let mut replacement = self
                            .emit_continued_expr(*expr, context.success)
                            .unwrap_or_else(|| {
                                crate::ice::bug!("structured result return was not emitted")
                            });
                        if context.success.assigns() {
                            push_control_break(&mut replacement, 0, context.exit_label);
                        }
                        let (kind, start, head_end, extent) = self.value_anchor(*expr);
                        out.anchored(
                            kind,
                            return_span.start.min(start),
                            head_end,
                            return_span.end.max(extent),
                            replacement,
                        );
                    } else if let Some((return_span, argument, _, propagate)) = propagating_returns
                        .iter()
                        .find(|(_, _, candidate, _)| candidate == expr)
                    {
                        let mut replacement = self.emit_region_propagate(
                            propagate,
                            context.failure,
                            context.exit_label,
                        );
                        let temp = temp_name(propagate.temporary);
                        let mut payload = Rope::new();
                        let try_span = self.span(propagate.node);
                        if argument.start < try_span.start {
                            payload.push_src(
                                &self.source[argument.start..try_span.start],
                                argument.start,
                            );
                        }
                        payload.push_lit(format!("{temp}.{}", propagate.layout.payload_field));
                        if try_span.end < argument.end {
                            payload
                                .push_src(&self.source[try_span.end..argument.end], try_span.end);
                        }
                        replacement.push_break(0);
                        replacement.append(self.emit_value_delivery_with_exit(
                            payload,
                            None,
                            context.success,
                            context.exit_label,
                            Some(0),
                        ));
                        out.anchored(
                            AnchorKind::Try,
                            return_span.start,
                            return_span.end,
                            return_span.end,
                            replacement,
                        );
                    } else {
                        self.emit_statement_expr(*expr, &mut out);
                    }
                }
                Statement::Adt(adt) => {
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
                    let span = self.span(propagate.node);
                    let emitted = if matches!(propagate.exit, ExitTarget::ResultRegion(_)) {
                        self.emit_region_propagate(propagate, context.failure, context.exit_label)
                    } else {
                        self.emit_propagate(propagate)
                    };
                    out.anchored(AnchorKind::Try, span.start, span.end, span.end, emitted);
                }
                Statement::Decision(decision) => self.emit_result_statement_decision(
                    decision,
                    exits,
                    context.failure,
                    context.success,
                    context.exit_label,
                    &mut out,
                ),
            }
        }
        out
    }

    pub(super) fn emit_result_statement_decision(
        &self,
        decision: &Decision,
        exits: &[HostExit],
        failure: &ValueContinuation<'_>,
        success: &ValueContinuation<'_>,
        exit_label: Option<&str>,
        out: &mut Rope<'a>,
    ) {
        let span = self.span(decision.head);
        let (kind, inner) = match &decision.kind {
            DecisionKind::LetElse { binding_mode, .. } => (
                AnchorKind::LetElse,
                self.emit_result_let_else(
                    decision,
                    *binding_mode,
                    exits,
                    failure,
                    success,
                    exit_label,
                ),
            ),
            DecisionKind::IfLet => (
                AnchorKind::IfLet,
                self.emit_result_if_let(decision, exits, failure, success, exit_label),
            ),
            DecisionKind::Match { .. } => {
                crate::ice::bug!("expression decision in a result statement body")
            }
        };
        out.anchored(kind, span.start, span.end, span.end, inner);
    }

    pub(super) fn emit_result_let_else(
        &self,
        decision: &Decision,
        mode: BindingMode,
        exits: &[HostExit],
        failure: &ValueContinuation<'_>,
        success: &ValueContinuation<'_>,
        exit_label: Option<&str>,
    ) -> Rope<'a> {
        let subject = &decision.subjects[0];
        let temp = temp_name(subject.temporary);
        let arm = &decision.arms[0];
        let mut out = self.emit_subject_initialization(subject, &temp, decision.head);
        out.push_break(0);
        out.push_lit("if (");
        let DecisionKind::LetElse {
            direct_variants, ..
        } = &decision.kind
        else {
            crate::ice::bug!("let-else has wrong Core decision kind")
        };
        if let Some(variants) = direct_variants {
            for (index, constructor) in variants.iter().enumerate() {
                if index > 0 {
                    out.push_lit(" && ");
                }
                out.push_lit(format!(
                    "{temp}.kind !== \"{}\"",
                    self.constructor_name(constructor)
                ));
            }
        } else {
            out.push_lit("!(");
            out.append(self.emit_condition(&arm.pattern, decision));
            out.push_lit(")");
        }
        out.push_lit(") {");
        let MissAction::Execute(body) = decision.miss else {
            crate::ice::bug!("let-else has no else body")
        };
        let body = self
            .emit_result_body_with_exits(body, exits, failure, success, exit_label)
            .trim();
        out.push_break(1);
        out.append(Rope::indented(1, body));
        out.push_break(0);
        out.push_lit("}");
        let mut recovery = BindingRecovery::new(self, &arm.pattern);
        out.push_break(0);
        out.append(
            self.emit_bindings(&arm.pattern, decision, Some(mode), &mut recovery, Some(0))
                .trim(),
        );
        Rope::scoped(out)
    }

    pub(super) fn emit_result_if_let(
        &self,
        decision: &Decision,
        exits: &[HostExit],
        failure: &ValueContinuation<'_>,
        success: &ValueContinuation<'_>,
        exit_label: Option<&str>,
    ) -> Rope<'a> {
        let subject = &decision.subjects[0];
        let temp = temp_name(subject.temporary);
        let arm = &decision.arms[0];
        let mut out = Rope::new();
        out.push_lit("{");
        out.push_break(1);
        out.append(self.emit_subject_initialization(subject, &temp, decision.head));
        out.push_break(1);
        out.push_lit("if (");
        out.append(self.emit_condition(&arm.pattern, decision));
        out.push_lit(") {");
        let mut recovery = BindingRecovery::new(self, &arm.pattern);
        let bindings = self.emit_bindings(&arm.pattern, decision, None, &mut recovery, Some(2));
        if !bindings.is_empty() {
            out.push_break(2);
            out.append(bindings.trim());
        }
        let ArmAction::Execute(body) = arm.action else {
            crate::ice::bug!("if-let has no then body")
        };
        out.push_break(2);
        out.append(Rope::indented(
            2,
            self.emit_result_body_with_exits(body, exits, failure, success, exit_label)
                .trim(),
        ));
        out.push_break(1);
        out.push_lit("}");
        match &decision.miss {
            MissAction::Execute(body) => {
                out.push_lit(" else {");
                out.push_break(2);
                out.append(Rope::indented(
                    2,
                    self.emit_result_body_with_exits(*body, exits, failure, success, exit_label)
                        .trim(),
                ));
                out.push_break(1);
                out.push_lit("}");
            }
            MissAction::Decision(inner) => {
                out.push_lit(" else ");
                out.append(self.emit_result_if_let(inner, exits, failure, success, exit_label));
            }
            MissAction::Nothing => {}
            MissAction::ThrowUnexpected(_) => {
                crate::ice::bug!("if-let has match miss action")
            }
        }
        out.push_break(0);
        out.push_lit("}");
        Rope::scoped(out)
    }

    pub(super) fn emit_result_region_continued(
        &self,
        expr: ExprId,
        region: &ResultRegion,
        continuation: &ValueContinuation<'_>,
    ) -> Rope<'a> {
        let mut out = Rope::new();
        // The block's own source follows, leading whitespace included, so
        // the opener does not break the line the region body starts on.
        let assignment_target = continuation.assignment_target();
        let distinct_label = assignment_target.and_then(|target| {
            let slot = self.structured_value_slot(expr)?;
            (slot != target).then(|| exit_label(slot))
        });
        let exit_label = distinct_label.as_deref().or(assignment_target);
        if let Some(label) = exit_label {
            out.push_lit(format!("{label}: {{"));
        } else {
            out.push_lit("{");
        }
        out.push_break(1);
        let success = continuation.wrap_result_ok();
        let exits = self.exits_for_expr(expr);
        for item in &region.items {
            let ResultRegionItem::Statements(body) = item;
            out.append(Rope::indented(
                1,
                self.emit_result_body_with_exits(*body, &exits, continuation, &success, exit_label)
                    .trim(),
            ));
        }
        if let Some(value) = region.value {
            out.push_break(1);
            if let Some(structured) = self.emit_continued_expr(value, &success) {
                out.append(Rope::indented(1, structured));
                if continuation.assigns() {
                    push_control_break(&mut out, 1, exit_label);
                }
            } else {
                out.append(Rope::indented(
                    1,
                    self.emit_value_delivery_with_exit(
                        guard_line_comment(self.emit_expr(value).trim(), 0),
                        None,
                        &success,
                        exit_label,
                        Some(0),
                    ),
                ));
            }
        }
        out.push_break(0);
        out.push_lit("}");
        let (binding_start, binding_end) = self.result_bind_anchor(region);
        let mut anchored = Rope::new();
        anchored.anchored(
            AnchorKind::Result,
            binding_start,
            binding_end,
            binding_end,
            Rope::scoped(out),
        );
        anchored
    }

    pub(super) fn emit_region_propagate(
        &self,
        propagate: &Propagate,
        continuation: &ValueContinuation<'_>,
        exit_label: Option<&str>,
    ) -> Rope<'a> {
        let temp = temp_name(propagate.temporary);
        let mut out = self.emit_propagate_input(propagate.value, &temp);
        out.push_break(0);
        out.push_lit(format!(
            "if ({}) {{",
            result_failure_test(&temp, propagate.layout)
        ));
        out.push_break(1);
        let mut value = Rope::new();
        value.push_lit(temp.clone());
        out.append(Rope::indented(
            1,
            self.emit_value_delivery_with_exit(value, None, continuation, exit_label, Some(0)),
        ));
        out.push_break(0);
        out.push_lit("}");
        if let Some(binding) = propagate.binding {
            out.push_break(0);
            out.push_lit(format!("{} ", binding_keyword(binding.mode)));
            out.append(self.source_rope(binding.node));
            out.push_lit(format!(" = {temp}.{};", propagate.layout.payload_field));
        }
        Rope::scoped(out)
    }
}
