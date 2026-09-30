//! Pattern decisions, arms, bindings, and tests.

use super::source::RECOVERED_VALUE;
use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn inline_subject_needs_storage(&self, decision: &Decision, index: usize) -> bool {
        fn reads(plan: &PatternPlan, index: usize) -> bool {
            match plan {
                PatternPlan::Any => false,
                PatternPlan::Bind(_) => true,
                PatternPlan::Test(
                    Test::Variant { place, .. }
                    | Test::Literal { place, .. }
                    | Test::InstanceOf { place, .. },
                ) => place.subject == index,
                PatternPlan::AllOf(parts) | PatternPlan::AnyOf(parts) => {
                    parts.iter().any(|part| reads(part, index))
                }
            }
        }
        !decision
            .arms
            .last()
            .is_some_and(DecisionArm::always_matches)
            || decision.arms.iter().any(|arm| reads(&arm.pattern, index))
    }

    pub(super) fn has_conditional_match_dispatch(&self, expr: ExprId) -> bool {
        matches!(
            &self.core.exprs[expr.index()],
            Expr::Decision(Decision {
                kind: DecisionKind::Match {
                    dispatch: MatchDispatch::Conditional,
                    ..
                },
                ..
            })
        )
    }

    /// Select a binding-free expression arm without moving its value out of
    /// the surrounding TypeScript expression. The plan proves that no other
    /// TT value intervenes between selection and value evaluation.
    pub(super) fn emit_arm_selector(&self, expr: ExprId, slot: &str) -> Rope<'a> {
        let Expr::Decision(decision) = &self.core.exprs[expr.index()] else {
            crate::ice::bug!("arm selector has no decision")
        };
        let mut out = Rope::new();
        if self.has_conditional_match_dispatch(expr) {
            for subject in &decision.subjects {
                out.append(self.emit_subject_initialization(
                    subject,
                    &self.temp_name(subject.temporary),
                    decision.head,
                ));
                out.push_break(0);
            }
            return out;
        }
        out.push_lit("{");
        for subject in &decision.subjects {
            out.push_break(1);
            out.append(self.emit_subject_initialization(
                subject,
                &self.temp_name(subject.temporary),
                decision.head,
            ));
        }
        let DecisionKind::Match { dispatch, .. } = decision.kind else {
            crate::ice::bug!("selector is not a match")
        };
        let temp = self.temp_name(decision.subjects[0].temporary);
        if dispatch == MatchDispatch::Conditional {
            crate::ice::bug!("a narrowing condition cannot be separated from its arm value");
        }
        out.push_break(1);
        out.push_lit(if dispatch == MatchDispatch::LiteralSwitch {
            format!("switch ({temp}) {{")
        } else {
            format!("switch ({temp}.kind) {{")
        });
        let mut wildcard = false;
        for (index, arm) in decision.arms.iter().enumerate() {
            out.push_break(2);
            if matches!(arm.pattern, PatternPlan::Any) {
                wildcard = true;
                out.push_lit("default");
            } else {
                for (alternative_index, alternative) in
                    pattern_alternatives(&arm.pattern).iter().enumerate()
                {
                    out.push_lit(if alternative_index == 0 {
                        "case "
                    } else {
                        ": case "
                    });
                    if dispatch == MatchDispatch::LiteralSwitch {
                        out.append(self.literal_label(alternative));
                    } else {
                        out.append(self.variant_label(alternative, decision));
                    }
                }
            }
            out.push_lit(format!(": {slot} = {index}; break;"));
        }
        if !wildcard {
            out.push_break(2);
            out.push_lit("default: ");
            out.push_lit(self.unexpected_throw(decision));
        }
        out.push_break(1);
        out.push_lit("}");
        out.push_break(0);
        out.push_lit("}");
        let mut anchored = Rope::new();
        let head = self.span(decision.head);
        anchored.anchored(
            AnchorKind::Match,
            head.start,
            head.end,
            self.span(decision.extent).end,
            Rope::scoped(out),
        );
        anchored
    }

    pub(super) fn emit_selected_arm_values(&self, expr: ExprId, slot: &str) -> Rope<'a> {
        let Expr::Decision(decision) = &self.core.exprs[expr.index()] else {
            crate::ice::bug!("selected arm values have no decision")
        };
        let mut out = Rope::new();
        out.push_lit("(");
        for (index, arm) in decision.arms.iter().enumerate() {
            let mut last = index + 1 == decision.arms.len();
            if !last {
                if !self.has_conditional_match_dispatch(expr) {
                    out.push_lit(format!("{slot} === {index} ? "));
                } else if let Some(test) = self.emit_arm_test(arm, decision) {
                    out.push_lit("(");
                    out.append(test);
                    out.push_lit(") ? ");
                } else {
                    last = true;
                }
            }
            let value = self.emit_deferred_arm_value(expr, &arm.action);
            push_grouped(
                &mut out,
                guard_line_comment(value.trim(), 0, self.source_kind),
                self.source_kind,
            );
            if last {
                break;
            }
            out.push_lit(" : ");
        }
        out.push_lit(")");
        out
    }

    fn emit_deferred_arm_value(&self, expr: ExprId, action: &ArmAction) -> Rope<'a> {
        match *action {
            ArmAction::Yield {
                body,
                kind: ArmBodyKind::Expression,
            } => self.emit_body(body),
            ArmAction::Yield {
                body,
                kind: ArmBodyKind::Block { completes: false },
            } => {
                let (span, exit) = single_return_arm_value(
                    self.semantic,
                    self.core,
                    body,
                    &self.exits_for_expr(expr),
                )
                .unwrap_or_else(|| crate::ice::bug!("deferred block has no single return value"));
                // HostExit is an AST ReturnStmt: erase only its keyword
                // and optional terminator, retaining authored trivia.
                let value_end = exit.statement.end
                    - usize::from(self.source.as_bytes()[exit.statement.end - 1] == b';');
                let mut value =
                    self.source_range_rope(hir::Span::new(span.start, exit.statement.start));
                value.append(self.source_range_rope(hir::Span::new(
                    exit.statement.start + "return".len(),
                    value_end,
                )));
                value.append(self.source_range_rope(hir::Span::new(exit.statement.end, span.end)));
                value
            }
            ArmAction::Yield {
                kind: ArmBodyKind::Missing,
                ..
            } => {
                let mut value = Rope::new();
                value.push_lit(RECOVERED_VALUE);
                value
            }
            _ => crate::ice::bug!("deferred match arm is not an expression value"),
        }
    }

    pub(super) fn emit_inline_match(&self, expr: ExprId) -> Rope<'a> {
        let Expr::Decision(decision) = &self.core.exprs[expr.index()] else {
            crate::ice::bug!("inline match has no decision")
        };
        let names = &self.inline_subjects[&decision.extent];
        let mut out = Rope::new();
        out.push_lit("(");
        for (index, (subject, name)) in decision.subjects.iter().zip(names).enumerate() {
            if self.inline_subject_needs_storage(decision, index) {
                out.push_lit(format!("{name} = "));
            }
            push_grouped(
                &mut out,
                self.emit_expr(subject.value).trim(),
                self.source_kind,
            );
            out.push_lit(", ");
        }
        let mut total = false;
        for arm in &decision.arms {
            if let Some(test) = self.emit_arm_test(arm, decision) {
                out.push_lit("(");
                out.append(test);
                out.push_lit(") ? ");
            } else {
                total = true;
            }
            push_grouped(
                &mut out,
                guard_line_comment(
                    self.emit_deferred_arm_value(expr, &arm.action).trim(),
                    0,
                    self.source_kind,
                ),
                self.source_kind,
            );
            if total {
                break;
            }
            out.push_lit(" : ");
        }
        if !total {
            self.used_match_raise.set(true);
            let (kind, value) = match decision.miss {
                MissAction::ThrowUnexpected(UnexpectedKind::Literal) => {
                    ("literal", self.shown(&names[0]))
                }
                MissAction::ThrowUnexpected(UnexpectedKind::Case) => {
                    ("case", self.shown(&names[0]))
                }
                MissAction::ThrowUnexpected(UnexpectedKind::Tuple) => {
                    ("case", self.shown_tuple(names))
                }
                _ => crate::ice::bug!("inline match has no failure completion"),
            };
            out.push_lit(format!(
                "{}(new {}(\"tt match: unexpected {kind} \" + {value}))",
                self.match_raise_name,
                self.host_error()
            ));
        }
        out.push_lit(")");
        out
    }

    pub(super) fn expression_is_inert(&self, expr: ExprId) -> bool {
        self.direct_apply_inputs.contains(&expr)
    }

    pub(super) fn emit_switch(
        &self,
        decision: &Decision,
        continuation: &ValueContinuation<'_>,
        exits: &[HostExit],
        exit_label: Option<&str>,
    ) -> Rope<'a> {
        let DecisionKind::Match { dispatch, .. } = decision.kind else {
            crate::ice::bug!("switch decision is not a match")
        };
        let literal = dispatch == MatchDispatch::LiteralSwitch;
        let temp = self.temp_name(decision.subjects[0].temporary);
        let mut out = Rope::new();
        out.push_break(0);
        out.push_lit(if literal {
            format!("switch ({temp}) {{")
        } else {
            format!("switch ({temp}.kind) {{")
        });
        let mut wildcard = false;
        for arm in &decision.arms {
            out.push_break(1);
            if matches!(arm.pattern, PatternPlan::Any) {
                wildcard = true;
                out.push_lit("default");
            } else {
                let alternatives = pattern_alternatives(&arm.pattern);
                for (index, alternative) in alternatives.iter().enumerate() {
                    out.push_lit(if index == 0 { "case " } else { ": case " });
                    if pattern_has_literal_test(alternative) {
                        out.append(self.literal_label(alternative));
                    } else {
                        out.append(self.variant_label(alternative, decision));
                    }
                }
            }
            let mut recovery = BindingRecovery::new(self, &arm.pattern);
            out.push_lit(": {");
            let bindings = self.emit_bindings(&arm.pattern, decision, None, &mut recovery, Some(2));
            if !bindings.is_empty() {
                out.push_break(2);
                out.append(bindings.trim());
            }
            out.push_break(2);
            self.emit_arm_action(
                arm,
                &ArmEmissionContext {
                    depth: 2,
                    chain: false,
                    continuation,
                    exits,
                    exit_label,
                    chain_exit_label: None,
                },
                &mut out,
            );
            out.push_break(1);
            out.push_lit("}");
        }
        if !wildcard {
            out.push_break(1);
            out.push_lit("default: {");
            out.push_break(2);
            out.push_lit(self.unexpected_throw(decision));
            out.push_break(1);
            out.push_lit("}");
        }
        out.push_break(0);
        out.push_lit("}");
        out
    }

    pub(super) fn emit_if_chain(
        &self,
        decision: &Decision,
        continuation: &ValueContinuation<'_>,
        exits: &[HostExit],
        exit_label: Option<&str>,
    ) -> Rope<'a> {
        let DecisionKind::Match { needs_label, .. } = decision.kind else {
            crate::ice::bug!("conditional decision is not a match")
        };
        let mut out = Rope::new();
        let mut depth = 0;
        let chain_exit_label = needs_label.then(|| self.generated_name("$tt_b"));
        let chain_exit_label = chain_exit_label.as_deref();
        if let Some(label) = chain_exit_label {
            out.push_break(depth);
            out.push_lit(format!("{label}: {{"));
            depth += 1;
        } else if continuation.assigns() {
            out.push_break(depth);
            out.push_lit("do {");
            depth += 1;
        }
        let mut unconditional = false;
        for arm in &decision.arms {
            let is_any = !arm.pattern.has_test();
            out.push_break(depth);
            if is_any {
                unconditional |= arm.guard.is_none();
            } else {
                out.push_lit("if (");
                out.append(self.emit_condition(&arm.pattern, decision));
                out.push_lit(") {");
                let mut recovery = BindingRecovery::new(self, &arm.pattern);
                let bindings = self.emit_bindings(
                    &arm.pattern,
                    decision,
                    None,
                    &mut recovery,
                    Some(depth + 1),
                );
                if !bindings.is_empty() {
                    out.push_break(depth + 1);
                    out.append(bindings.trim());
                }
                out.push_break(depth + 1);
            }
            self.emit_arm_action(
                arm,
                &ArmEmissionContext {
                    depth: if is_any { depth } else { depth + 1 },
                    chain: true,
                    continuation,
                    exits,
                    exit_label,
                    chain_exit_label,
                },
                &mut out,
            );
            if !is_any {
                out.push_break(depth);
                out.push_lit("}");
            }
        }
        if !unconditional {
            out.push_break(depth);
            out.push_lit(self.unexpected_throw(decision));
        }
        if needs_label {
            depth -= 1;
            out.push_break(depth);
            out.push_lit("}");
        } else if continuation.assigns() {
            depth -= 1;
            out.push_break(depth);
            out.push_lit("} while (false);");
        }
        out
    }

    pub(super) fn emit_arm_action(
        &self,
        arm: &DecisionArm,
        context: &ArmEmissionContext<'_, '_>,
        out: &mut Rope<'a>,
    ) {
        let depth = context.depth;
        let action_depth = depth + u16::from(arm.guard.is_some());
        let chain = context.chain;
        let continuation = context.continuation;
        let exits = context.exits;
        let exit_label = context.exit_label;
        let chain_exit_label = context.chain_exit_label;
        let ArmAction::Yield { body, kind } = arm.action else {
            crate::ice::bug!("match arm does not yield")
        };
        let structured_body = (matches!(kind, ArmBodyKind::Expression)
            && !continuation.is_expression())
        .then(|| match self.core.body_value_expr(body) {
            Some(expr) => self.emit_continued_expr(expr, continuation),
            None => self
                .emit_sequence_operand(body, continuation)
                .map(|(mut prelude, value)| {
                    let value = value.trim();
                    let close = value
                        .last_line_has_line_comment(self.source_kind)
                        .then_some(action_depth);
                    prelude.append(self.emit_value_delivery_control(
                        value,
                        close,
                        continuation,
                        None,
                        None,
                        false,
                    ));
                    Rope::scoped(prelude)
                }),
        })
        .flatten();
        // A block arm's body sits between braces this lowering writes, and
        // the author's own line break and indentation after their `{` is
        // the layout the rest of their block is written against — so it
        // stays (TASK-219). Every other body is spliced into a line.
        let block_layout = matches!(kind, ArmBodyKind::Block { .. }) && chain;
        let body = if structured_body.is_some() {
            Rope::new()
        } else if matches!(kind, ArmBodyKind::Missing) {
            let mut value = Rope::new();
            value.push_lit(RECOVERED_VALUE);
            value
        } else if matches!(kind, ArmBodyKind::Block { .. }) && continuation.assigns() {
            // Switch arms are indented as a generated case body after their
            // source is spliced in. Conditional chains retain the authored
            // source column, so their rewritten exits must not add that unit.
            let generated_indent = if chain { "" } else { "  " };
            let body =
                self.emit_body_with_exits(body, exits, continuation, exit_label, generated_indent);
            if block_layout {
                body.trim_end()
            } else {
                body.trim()
            }
        } else {
            let body = self.emit_body(body);
            if block_layout {
                body.trim_end()
            } else {
                body.trim()
            }
        };
        let mut action = Rope::new();
        match kind {
            ArmBodyKind::Expression | ArmBodyKind::Missing => {
                if let Some(structured) = structured_body {
                    action.append(structured);
                    if continuation.assigns() {
                        push_control_break(&mut action, action_depth, chain_exit_label);
                    }
                } else {
                    let close = body
                        .last_line_has_line_comment(self.source_kind)
                        .then_some(action_depth);
                    action.append(self.emit_value_delivery_with_exit(
                        body,
                        close,
                        continuation,
                        chain_exit_label,
                        Some(action_depth),
                    ));
                }
            }
            // A block that always leaves has written the arm's value on
            // every path it takes, so neither the fall-through to
            // `undefined` nor the exit after it can be reached.
            ArmBodyKind::Block { completes } if chain => {
                action.push_lit("{");
                action.append(body);
                if completes {
                    if continuation.assigns() {
                        action.push_break(action_depth + 1);
                        action.push_lit(format!(
                            "{} = undefined;",
                            // `assigns()` is exactly "this continuation has
                            // an assignment target", tested one line above.
                            continuation
                                .assignment_target()
                                .expect("an assigning continuation names its target")
                        ));
                    }
                    push_control_break(&mut action, action_depth + 1, chain_exit_label);
                }
                action.push_break(action_depth);
                action.push_lit("}");
            }
            ArmBodyKind::Block { completes } => {
                action.append(body);
                if completes {
                    if continuation.assigns() {
                        action.push_break(action_depth + 1);
                        action.push_lit(format!(
                            "{} = undefined;",
                            // `assigns()` is exactly "this continuation has
                            // an assignment target", tested one line above.
                            continuation
                                .assignment_target()
                                .expect("an assigning continuation names its target")
                        ));
                    }
                    action.push_break(action_depth + 1);
                    action.push_lit("break;");
                }
            }
        }
        if let Some(guard) = arm.guard {
            let (prelude, guard) = self.emit_guard(guard, depth);
            out.append(prelude);
            let guarded = guard.last_line_has_line_comment(self.source_kind);
            out.push_lit("if (");
            out.append(guard);
            if guarded {
                out.push_break(depth);
            }
            out.push_lit(") {");
            out.push_break(action_depth);
        }
        out.append(action);
        if arm.guard.is_some() {
            out.push_break(depth);
            out.push_lit("}");
        }
    }

    fn emit_guard(&self, guard: ExprId, depth: u16) -> (Rope<'a>, Rope<'a>) {
        let Expr::Sequence(body) = &self.core.exprs[guard.index()] else {
            return (Rope::new(), self.emit_expr(guard).trim());
        };
        let Some(node) = self.core.sequence_node(*body) else {
            return (Rope::new(), self.emit_expr(guard).trim());
        };
        let span = self.span(node);
        let mut prelude = Rope::new();
        for rewrite in self
            .owner_slot_index
            .starting_in(span.start, span.start.saturating_add(1))
            .into_iter()
            .map(|index| &self.owner_slot_rewrites[index])
        {
            if rewrite.owner == SourceSpan::from(span)
                && !self.emitted_owner_rewrites.contains(rewrite.expr)
            {
                self.emitted_owner_rewrites.mark(rewrite.expr);
                prelude.append(self.emit_owner_slot_rewrite(rewrite).trim_end());
                prelude.push_break(depth);
            }
        }
        for rewrite in self
            .compose_index
            .starting_in(span.start, span.start.saturating_add(1))
            .into_iter()
            .map(|index| &self.compose_rewrites[index])
        {
            if rewrite.owner == SourceSpan::from(span)
                && self.emitted_compose_rewrites.claim(rewrite.owner)
            {
                let lowered = self.emit_compose_rewrite(rewrite).trim_end();
                if !lowered.is_empty() {
                    prelude.append(lowered);
                    prelude.push_break(depth);
                }
            }
        }
        (prelude, self.emit_expr(guard).trim())
    }

    /// Delivers one value to its continuation. `close` breaks the line
    /// before the closing `);` at that depth — the delivered body ends with
    /// a `//` comment that would otherwise swallow it.
    pub(super) fn emit_value_delivery(
        &self,
        body: Rope<'a>,
        close: Option<u16>,
        continuation: &ValueContinuation<'_>,
    ) -> Rope<'a> {
        self.emit_value_delivery_control(body, close, continuation, None, None, true)
    }

    pub(super) fn emit_value_delivery_without_region_exit(
        &self,
        body: Rope<'a>,
        continuation: &ValueContinuation<'_>,
    ) -> Rope<'a> {
        self.emit_value_delivery_control(body, None, continuation, None, None, false)
    }

    pub(super) fn emit_value_delivery_with_exit(
        &self,
        body: Rope<'a>,
        close: Option<u16>,
        continuation: &ValueContinuation<'_>,
        break_label: Option<&str>,
        exit_depth: Option<u16>,
    ) -> Rope<'a> {
        self.emit_value_delivery_control(body, close, continuation, break_label, exit_depth, true)
    }

    pub(super) fn emit_value_delivery_control(
        &self,
        body: Rope<'a>,
        close: Option<u16>,
        continuation: &ValueContinuation<'_>,
        break_label: Option<&str>,
        exit_depth: Option<u16>,
        exit_after_assignment: bool,
    ) -> Rope<'a> {
        let mut value = body;
        for wrapper in continuation.wrappers.iter().rev() {
            match wrapper {
                ValueWrapper::ResultOk => {
                    let mut wrapped = Rope::new();
                    wrapped.push_lit("{ kind: \"Ok\" as const, value: ");
                    push_grouped(&mut wrapped, value, self.source_kind);
                    wrapped.push_lit(" }");
                    value = wrapped;
                }
            }
        }
        let grouped = needs_grouping(&value, self.source_kind);
        let mut out = Rope::new();
        match continuation.destination {
            ValueDestination::Expression | ValueDestination::Return => out.push_lit("return "),
            ValueDestination::Assign(target) => out.push_lit(format!("{target} = ")),
            ValueDestination::Invoke {
                prefix,
                result: Some(result),
                ..
            } => out.push_lit(format!("{result} = {prefix}")),
            ValueDestination::Invoke {
                prefix,
                result: None,
                ..
            } => out.push_lit(prefix.to_owned()),
        }
        let frame = match continuation.destination {
            ValueDestination::Invoke { frame, .. } => frame,
            _ => None,
        };
        // The literal the value was written inside, up to the value itself.
        if let Some((head, _)) = frame {
            out.push_src(&self.source[head.start..head.end], head.start);
        }
        if grouped {
            out.push_lit("(");
        }
        out.append(value);
        if let Some(depth) = close {
            out.push_break(depth);
        }
        if grouped {
            out.push_lit(")");
        }
        if let Some((_, tail)) = frame {
            out.push_src(&self.source[tail.start..tail.end], tail.start);
        }
        if matches!(continuation.destination, ValueDestination::Invoke { .. }) {
            out.push_lit(")");
        }
        out.push_lit(";");
        if continuation.assigns() && exit_after_assignment {
            if let Some(depth) = exit_depth {
                push_control_break(&mut out, depth, break_label);
            } else {
                push_region_break(&mut out, break_label);
            }
        }
        out
    }

    pub(super) fn emit_arm_test(&self, arm: &DecisionArm, decision: &Decision) -> Option<Rope<'a>> {
        let tested = arm.pattern.has_test();
        if !tested && arm.guard.is_none() {
            return None;
        }
        let mut out = Rope::new();
        if tested {
            out.append(self.emit_condition(&arm.pattern, decision));
        }
        if let Some(guard) = arm.guard {
            if tested {
                out.push_lit(" && ");
            }
            push_grouped(&mut out, self.emit_expr(guard).trim(), self.source_kind);
        }
        Some(out)
    }

    pub(super) fn emit_condition(&self, plan: &PatternPlan, decision: &Decision) -> Rope<'a> {
        match plan {
            PatternPlan::Any | PatternPlan::Bind(_) => Rope::new(),
            PatternPlan::Test(test) => self.emit_test(test, decision),
            PatternPlan::AllOf(parts) => {
                let mut out = Rope::new();
                let tests = parts
                    .iter()
                    .filter(|part| part.has_test())
                    .collect::<Vec<_>>();
                for (index, part) in tests.iter().enumerate() {
                    if index > 0 {
                        out.push_lit(" && ");
                    }
                    let parenthesize = matches!(part, PatternPlan::AnyOf(_));
                    if parenthesize {
                        out.push_lit("(");
                    }
                    out.append(self.emit_condition(part, decision));
                    if parenthesize {
                        out.push_lit(")");
                    }
                }
                out
            }
            PatternPlan::AnyOf(parts) => {
                let mut out = Rope::new();
                for (index, part) in parts.iter().enumerate() {
                    if index > 0 {
                        out.push_lit(" || ");
                    }
                    out.append(self.emit_condition(part, decision));
                }
                out
            }
        }
    }

    pub(super) fn emit_test(&self, test: &Test, decision: &Decision) -> Rope<'a> {
        match test {
            Test::Variant { place, constructor } => {
                let mut out = self.emit_place(place, decision, Some(constructor_node(constructor)));
                out.push_lit(format!(
                    ".kind === \"{}\"",
                    self.constructor_name(constructor)
                ));
                out
            }
            Test::Literal { place, pattern } => {
                let mut out = self.emit_place(place, decision, None);
                out.push_lit(" === ");
                let span = self
                    .semantic
                    .hir
                    .source_map
                    .pattern_span(*pattern)
                    .unwrap_or_else(|| crate::ice::bug!("literal has no span"));
                let (literal, at) = self.source_span(span);
                out.push_src(literal, at);
                out
            }
            Test::InstanceOf { place, constructor } => {
                let mut out = self.emit_place(place, decision, None);
                out.push_lit(" instanceof ");
                let span = self.span(*constructor);
                let (source, at) = self.source_span(span);
                out.push_src(source, at);
                out
            }
        }
    }

    pub(super) fn emit_place(
        &self,
        place: &Place,
        decision: &Decision,
        payload_for: Option<NodeId>,
    ) -> Rope<'a> {
        let mut out = Rope::new();
        out.push_lit(self.subject_reference(decision, place.subject));
        for (index, field) in place.fields.iter().enumerate() {
            out.push_lit(".");
            if index + 1 == place.fields.len()
                && let Some(node) = payload_for
            {
                out.push_payload_mark(self.span(node).start);
            }
            out.push_lit(self.field_name(field));
        }
        out
    }

    pub(super) fn emit_bindings(
        &self,
        plan: &PatternPlan,
        decision: &Decision,
        declaration: Option<BindingMode>,
        recovery: &mut BindingRecovery,
        separator_depth: Option<u16>,
    ) -> Rope<'a> {
        let selected = if let PatternPlan::AnyOf(parts) = plan {
            parts.first().unwrap_or(plan)
        } else {
            plan
        };
        let mut groups: Vec<BindingGroup<'_>> = Vec::new();
        collect_binding_groups(
            selected,
            matches!(plan, PatternPlan::AnyOf(_)).then_some(plan),
            &mut groups,
        );
        let mut out = Rope::new();
        for (group_index, (receiver, bindings)) in groups.into_iter().enumerate() {
            if group_index > 0
                && let Some(depth) = separator_depth
            {
                out.push_break(depth);
            }
            let keyword = match declaration {
                Some(mode) if group_index == 0 => format!(" {} ", binding_keyword(mode)),
                _ => "const ".to_string(),
            };
            out.push_lit(keyword);
            let list = bindings
                .iter()
                .map(|(binding, shared)| binding.list.filter(|_| shared.is_none()))
                .reduce(|left, right| left.filter(|_| left == right))
                .flatten()
                .map(|list| self.span(list));
            if let Some(list) = list {
                out.push_destructured_list_start(list.start);
            }
            out.push_lit("{ ");
            for (index, (binding, shared)) in bindings.iter().enumerate() {
                if index > 0 {
                    out.push_lit(", ");
                }
                self.emit_binding(binding, *shared, recovery, &mut out);
            }
            out.push_lit(" }");
            if let Some(list) = list {
                out.push_destructured_list_end(list.end);
            }
            out.push_lit(" = ");
            out.append(self.emit_place(&receiver, decision, None));
            out.push_lit(if declaration.is_some() || separator_depth.is_some() {
                ";"
            } else {
                "; "
            });
        }
        out
    }

    pub(super) fn emit_binding(
        &self,
        binding: &Bind,
        shared: Option<&PatternPlan>,
        recovery: &mut BindingRecovery,
        out: &mut Rope<'a>,
    ) {
        let field = binding
            .source
            .fields
            .last()
            .unwrap_or_else(|| crate::ice::bug!("binding has no source field"));
        let field_node = field_node(field);
        let field_text = self.field_name(field);
        let Some(alternatives) = shared else {
            let span = self.span(field_node);
            let (text, at) = self.source_span(span);
            out.push_src(text, at);
            if let Some(replacement) = recovery.replacement(self, binding) {
                out.push_lit(format!(": {replacement}"));
            } else if binding.binding != field_node {
                out.push_lit(": ");
                out.append(self.source_rope(binding.binding));
            }
            return;
        };
        if let Some(replacement) = recovery.replacement(self, binding) {
            out.push_lit(field_text);
            out.push_lit(format!(": {replacement}"));
            return;
        }
        let name = self.source_node(binding.binding).0;
        let mut every = Vec::new();
        every_binding(alternatives, &mut every);
        let occurrences: Vec<crate::BindingOccurrence> = every
            .into_iter()
            .filter(|other| self.source_node(other.binding).0 == name)
            .map(|other| {
                let span = self.span(other.binding);
                crate::BindingOccurrence {
                    src: span.start,
                    src_end: span.end,
                    shorthand: other
                        .source
                        .fields
                        .last()
                        .is_some_and(|field| helpers::field_node(field) == other.binding),
                }
            })
            .collect();
        if binding.binding == field_node {
            out.push_shared_binding(field_text, &occurrences);
        } else {
            out.push_lit(field_text);
            out.push_lit(": ");
            out.push_shared_binding(name.to_owned(), &occurrences);
        }
    }

    pub(super) fn constructor_name(&self, constructor: &Constructor) -> String {
        self.source_node(constructor_node(constructor)).0.to_owned()
    }

    pub(super) fn field_name(&self, field: &FieldAccess) -> String {
        self.source_node(field_node(field)).0.to_owned()
    }

    pub(super) fn literal_label(&self, plan: &PatternPlan) -> Rope<'a> {
        let PatternPlan::Test(Test::Literal { pattern, .. }) = plan else {
            crate::ice::bug!("switch literal alternative is not literal")
        };
        // Every pattern the lowering kept came from source text the HIR
        // recorded a span for; one without a span is a broken lowering, not
        // an input the user can write.
        let Some(span) = self.semantic.hir.source_map.pattern_span(*pattern) else {
            crate::ice::bug!("switch literal pattern has no source span")
        };
        let (text, at) = self.source_span(span);
        let mut out = Rope::new();
        out.push_src(text, at);
        out
    }

    pub(super) fn variant_label(&self, plan: &PatternPlan, decision: &Decision) -> Rope<'a> {
        let PatternPlan::AllOf(parts) = plan else {
            crate::ice::bug!("switch variant alternative is not constructor")
        };
        let constructor = parts.iter().find_map(|part| match part {
            PatternPlan::Test(Test::Variant { constructor, .. }) => Some(constructor),
            _ => None,
        });
        // A switch is only built over variant tests, so an alternative with
        // no variant part is a plan this emitter should never have been
        // handed.
        let Some(constructor) = constructor else {
            crate::ice::bug!("switch variant alternative tests no constructor")
        };
        let (tag, at) = self.source_node(constructor_node(constructor));
        let head = self.span(decision.head);
        let mut label = Rope::new();
        label.push_lit(format!("\"{tag}\""));
        let mut out = Rope::new();
        out.anchored_with_context(
            AnchorKind::Match,
            head.start,
            head.end,
            self.span(decision.extent).end,
            Some((at, at + tag.len())),
            label,
        );
        out
    }

    fn subject_reference(&self, decision: &Decision, subject: usize) -> String {
        self.inline_subjects
            .get(&decision.extent)
            .map(|names| names[subject].clone())
            .unwrap_or_else(|| self.temp_name(decision.subjects[subject].temporary))
    }

    pub(super) fn unexpected_throw(&self, decision: &Decision) -> String {
        let (kind, shown) = match decision.miss {
            MissAction::ThrowUnexpected(UnexpectedKind::Tuple) => {
                let temps = (0..decision.subjects.len())
                    .map(|subject| self.subject_reference(decision, subject))
                    .collect::<Vec<_>>();
                ("case", self.shown_tuple(&temps))
            }
            MissAction::ThrowUnexpected(UnexpectedKind::Literal) => {
                ("literal", self.shown(&self.subject_reference(decision, 0)))
            }
            MissAction::ThrowUnexpected(UnexpectedKind::Case) => {
                ("case", self.shown(&self.subject_reference(decision, 0)))
            }
            _ => crate::ice::bug!("match has non-match miss action"),
        };
        format!(
            "throw new {}(\"tt match: unexpected {kind} \" + {shown});",
            self.host_error()
        )
    }

    fn host_error(&self) -> &str {
        self.used_host_error.set(true);
        &self.host_error
    }

    fn shown(&self, value: &str) -> String {
        self.used_match_show.set(true);
        format!("{}({value})", self.match_show_name)
    }

    fn shown_tuple(&self, values: &[String]) -> String {
        let parts = values
            .iter()
            .map(|value| self.shown(value))
            .collect::<Vec<_>>();
        format!("\"[\" + {} + \"]\"", parts.join(" + \",\" + "))
    }
}
