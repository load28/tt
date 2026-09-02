//! Pattern decisions, arms, bindings, and tests.

use super::*;

impl<'a> Emitter<'a> {
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
        let temp = temp_name(decision.subjects[0].temporary);
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
                        out.push_lit(format!("\"{}\"", self.variant_label(alternative)));
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
        let chain_exit_label = needs_label.then_some("$tt_b");
        if needs_label {
            out.push_break(depth);
            out.push_lit("$tt_b: {");
            depth += 1;
        } else if continuation.assigns() {
            out.push_break(depth);
            out.push_lit("do {");
            depth += 1;
        }
        let mut unconditional = false;
        for arm in &decision.arms {
            let is_any = !pattern_has_test(&arm.pattern);
            out.push_break(depth);
            if is_any && arm.guard.is_none() {
                unconditional = true;
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
            if !is_any || arm.guard.is_some() {
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
        let body_expr = self.core.body_value_expr(body);
        let structured_body = matches!(kind, ArmBodyKind::Expression)
            .then(|| {
                body_expr.and_then(|expr| {
                    (!continuation.is_expression())
                        .then(|| self.emit_continued_expr(expr, continuation))
                        .flatten()
                })
            })
            .flatten();
        // A block arm's body sits between braces this lowering writes, and
        // the author's own line break and indentation after their `{` is
        // the layout the rest of their block is written against — so it
        // stays (TASK-219). Every other body is spliced into a line.
        let block_layout = matches!(kind, ArmBodyKind::Block { .. }) && chain;
        let body = if structured_body.is_some() {
            Rope::new()
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
            ArmBodyKind::Expression => {
                if let Some(structured) = structured_body {
                    action.append(structured);
                    if continuation.assigns() {
                        push_control_break(&mut action, action_depth, chain_exit_label);
                    }
                } else {
                    let close = body.last_line_has_line_comment().then_some(action_depth);
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
            let guard = self.emit_expr(guard).trim();
            let guarded = guard.last_line_has_line_comment();
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
                    push_grouped(&mut wrapped, value);
                    wrapped.push_lit(" }");
                    value = wrapped;
                }
            }
        }
        let grouped = needs_grouping(&value);
        let mut out = Rope::new();
        match continuation.destination {
            ValueDestination::Expression | ValueDestination::Return => out.push_lit("return "),
            ValueDestination::Assign(target) => out.push_lit(format!("{target} = ")),
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

    pub(super) fn emit_condition(&self, plan: &PatternPlan, decision: &Decision) -> Rope<'a> {
        match plan {
            PatternPlan::Any | PatternPlan::Bind(_) => Rope::new(),
            PatternPlan::Test(test) => self.emit_test(test, decision),
            PatternPlan::AllOf(parts) => {
                let mut out = Rope::new();
                let tests = parts
                    .iter()
                    .filter(|part| pattern_has_test(part))
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
        out.push_lit(temp_name(decision.subjects[place.subject].temporary));
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
            !matches!(plan, PatternPlan::AnyOf(_)),
            &mut groups,
        );
        let mut out = Rope::new();
        for (group_index, (receiver, bindings)) in groups.into_iter().enumerate() {
            if group_index > 0
                && let Some(depth) = separator_depth
            {
                out.push_break(depth);
            }
            if group_index == 0 {
                if let Some(mode) = declaration {
                    out.push_lit(format!(" {} {{ ", binding_keyword(mode)));
                } else {
                    out.push_lit("const { ");
                }
            } else {
                out.push_lit("const { ");
            }
            for (index, (binding, mapped)) in bindings.iter().enumerate() {
                if index > 0 {
                    out.push_lit(", ");
                }
                self.emit_binding(binding, *mapped, recovery, &mut out);
            }
            out.push_lit(" } = ");
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
        mapped: bool,
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
        if mapped {
            let span = self.span(field_node);
            let (text, at) = self.source_span(span);
            out.push_src(text, at);
        } else {
            out.push_lit(field_text);
        }
        if let Some(replacement) = recovery.replacement(self, binding) {
            out.push_lit(format!(": {replacement}"));
        } else if binding.binding != field_node {
            out.push_lit(": ");
            if mapped {
                out.append(self.source_rope(binding.binding));
            } else {
                out.push_lit(self.source_node(binding.binding).0.to_owned());
            }
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

    pub(super) fn variant_label(&self, plan: &PatternPlan) -> String {
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
        self.constructor_name(constructor)
    }

    pub(super) fn unexpected_throw(&self, decision: &Decision) -> String {
        match decision.miss {
            MissAction::ThrowUnexpected(UnexpectedKind::Tuple) => {
                let temps = decision
                    .subjects
                    .iter()
                    .map(|subject| temp_name(subject.temporary))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "throw new Error(\"tt match: unexpected case \" + JSON.stringify([{temps}]));"
                )
            }
            MissAction::ThrowUnexpected(UnexpectedKind::Literal) => {
                "throw new Error(\"tt match: unexpected literal \" + JSON.stringify($tt_m));"
                    .to_owned()
            }
            MissAction::ThrowUnexpected(UnexpectedKind::Case) => {
                "throw new Error(\"tt match: unexpected case \" + JSON.stringify($tt_m));"
                    .to_owned()
            }
            _ => crate::ice::bug!("match has non-match miss action"),
        }
    }
}
