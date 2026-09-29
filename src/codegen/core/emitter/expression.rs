//! Pipeline, template, import, and continued-expression emission.

use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn emit_apply(&self, apply: &Apply) -> Rope<'a> {
        let start = self.span(apply.node).start;
        let end = apply.steps.last().map_or_else(
            || self.span(apply.node).end,
            |step| self.span(step.node).end,
        );
        let inner = match apply.head {
            Some(head) => {
                let mut acc = guard_line_comment(self.emit_expr(head).trim(), 0, self.source_kind);
                let mut accumulator_is_inert = self.expression_is_inert(head);
                // Where the value flowing into the current step was
                // produced: the head, then each step in turn — the place a
                // label on a rejected value points back at.
                let mut produced = self.span(apply.node);
                for step in &apply.steps {
                    let step_span = self.span(step.node);
                    let context = Some((produced.start, produced.end));
                    let body =
                        guard_line_comment(self.emit_expr(step.value).trim(), 0, self.source_kind);
                    let mut next = Rope::new();
                    // The value flowing into a step occupies a position the
                    // checker types against that step, so a diagnostic that
                    // lands on it belongs to the step that rejected the
                    // value — each piped-value position is anchored to the
                    // step consuming it. Verbatim spans still resolve
                    // exactly; only glue-crossing spans re-home here.
                    let mut input = Rope::new();
                    match step.mode {
                        ApplyMode::Postfix { .. } => {
                            push_receiver(&mut input, acc, self.source_kind);
                            next.anchored_with_context(
                                AnchorKind::Pipe,
                                step_span.start,
                                step_span.end,
                                end,
                                context,
                                input,
                            );
                            next.append(body);
                        }
                        ApplyMode::Call => {
                            if accumulator_is_inert {
                                push_receiver(&mut next, body, self.source_kind);
                                next.push_lit("(");
                                push_grouped(&mut input, acc, self.source_kind);
                                next.anchored_with_context(
                                    AnchorKind::Pipe,
                                    step_span.start,
                                    step_span.end,
                                    end,
                                    context,
                                    input,
                                );
                                next.push_lit(")");
                            } else if let Some(member) =
                                self.member_apply_steps.get(&step.value).copied()
                            {
                                push_grouped(&mut input, acc, self.source_kind);
                                let mut call = Rope::new();
                                call.anchored_with_context(
                                    AnchorKind::Pipe,
                                    step_span.start,
                                    step_span.end,
                                    end,
                                    context,
                                    input,
                                );
                                next.anchored_with_context(
                                    AnchorKind::Pipe,
                                    step_span.start,
                                    step_span.end,
                                    end,
                                    context,
                                    self.emit_member_step(step.value, member, call),
                                );
                            } else {
                                self.used_pipe.set(true);
                                next.push_lit(format!("{}(", self.generated_name("$tt_ap")));
                                push_grouped(&mut input, acc, self.source_kind);
                                next.anchored_with_context(
                                    AnchorKind::Pipe,
                                    step_span.start,
                                    step_span.end,
                                    end,
                                    context,
                                    input,
                                );
                                next.push_lit(", ");
                                push_grouped(&mut next, body, self.source_kind);
                                next.push_lit(")");
                            }
                        }
                    }
                    acc = next;
                    // A call or member operation can return any value and
                    // can have arbitrary effects. Only the original head's
                    // syntax proof can authorize inline reordering.
                    accumulator_is_inert = false;
                    produced = step_span;
                }
                acc
            }
            None => self.emit_flow(apply, end),
        };
        let mut out = Rope::new();
        out.anchored(AnchorKind::Pipe, start, end, end, inner);
        out
    }

    fn emit_member_step(
        &self,
        value: ExprId,
        member: crate::program_syntax::MemberCallee,
        input: Rope<'a>,
    ) -> Rope<'a> {
        let mut out = Rope::new();
        let input_name = self.generated_name("$tt_v");
        let mut names = vec![input_name.clone()];
        names.extend(self.member_operand_names(member));
        out.push_lit(format!("(({}) => ", names.join(", ")));
        if member.grouped {
            out.push_lit("(");
            out.append(self.member_callee_body(value, member));
            out.push_lit(")");
        } else {
            out.append(self.member_callee_body(value, member));
        }
        out.push_lit(format!("({input_name}))("));
        out.append(input);
        self.push_member_operands(&mut out, member, true);
        out.push_lit(")");
        out
    }

    fn member_operand_names(&self, member: crate::program_syntax::MemberCallee) -> Vec<String> {
        member
            .receiver
            .map(|_| self.generated_name("$tt_r"))
            .into_iter()
            .chain(member.key.map(|_| self.generated_name("$tt_k")))
            .collect()
    }

    fn emit_flow_function(&self, value: ExprId) -> Rope<'a> {
        match self.member_apply_steps.get(&value).copied() {
            Some(member) if !member.optional => self.emit_bound_member(value, member),
            Some(_) | None => guard_line_comment(self.emit_expr(value).trim(), 0, self.source_kind),
        }
    }

    fn member_callee_body(
        &self,
        value: ExprId,
        member: crate::program_syntax::MemberCallee,
    ) -> Rope<'a> {
        let Expr::Opaque(node) = &self.core.exprs[value.index()] else {
            crate::ice::bug!("a member pipeline step is not source text");
        };
        let callee = self.span(*node);
        let mut substitutions: Vec<(SourceSpan, String)> = member
            .receiver
            .map(|receiver| (receiver, self.generated_name("$tt_r")))
            .into_iter()
            .chain(member.key.map(|key| (key, self.generated_name("$tt_k"))))
            .collect();
        substitutions.sort_by_key(|(span, _)| span.start);
        let mut body = Rope::new();
        let mut cursor = callee.start;
        for (span, name) in &substitutions {
            body.append(self.source_range_rope(hir::Span {
                start: cursor,
                end: span.start,
            }));
            body.push_lit(name.clone());
            cursor = span.end;
        }
        body.append(self.source_range_rope(hir::Span {
            start: cursor,
            end: callee.end,
        }));
        guard_line_comment(body, 0, self.source_kind)
    }

    fn push_member_operands(
        &self,
        out: &mut Rope<'a>,
        member: crate::program_syntax::MemberCallee,
        leading_separator: bool,
    ) {
        for (index, span) in [member.receiver, member.key]
            .into_iter()
            .flatten()
            .enumerate()
        {
            out.push_lit(if leading_separator || index > 0 {
                ", ("
            } else {
                "("
            });
            out.append(guard_line_comment(
                self.source_range_rope(hir::Span {
                    start: span.start,
                    end: span.end,
                }),
                0,
                self.source_kind,
            ));
            out.push_lit(")");
        }
    }

    fn emit_bound_member(
        &self,
        value: ExprId,
        member: crate::program_syntax::MemberCallee,
    ) -> Rope<'a> {
        let mut out = Rope::new();
        out.push_lit("((");
        out.push_lit(self.member_operand_names(member).join(", "));
        out.push_lit(") => (");
        out.append(self.member_callee_body(value, member));
        out.push_lit(match member.receiver {
            Some(_) => format!(").bind({}))(", self.generated_name("$tt_r")),
            None => ").bind(this))(".to_owned(),
        });
        self.push_member_operands(&mut out, member, false);
        out.push_lit(")");
        out
    }

    pub(super) fn emit_flow(&self, apply: &Apply, owner_end: usize) -> Rope<'a> {
        let mut steps = apply.steps.iter();
        let first = steps
            .next()
            .unwrap_or_else(|| crate::ice::bug!("flow has no step"));
        let mut acc = Rope::new();
        push_grouped(
            &mut acc,
            self.emit_flow_function(first.value),
            self.source_kind,
        );
        let mut produced = self.span(first.node);
        for step in steps {
            self.used_flow.set(true);
            let step_span = self.span(step.node);
            let body = self.emit_flow_function(step.value);
            let mut next = Rope::new();
            next.push_lit(format!("{}(", self.generated_name("$tt_fl")));
            // The composition built so far is what this step composes onto;
            // a mismatch on it means this step rejected it (see
            // `emit_apply`).
            next.anchored_with_context(
                AnchorKind::Pipe,
                step_span.start,
                step_span.end,
                owner_end,
                Some((produced.start, produced.end)),
                acc,
            );
            match step.mode {
                ApplyMode::Postfix { .. } => {
                    let input_name = self.generated_name("$tt_v");
                    next.push_lit(format!(", (({input_name}) => ({input_name})"));
                    next.append(body);
                    next.push_lit("))");
                }
                ApplyMode::Call => {
                    next.push_lit(", ");
                    push_grouped(&mut next, body, self.source_kind);
                    next.push_lit(")");
                }
            }
            acc = next;
            produced = step_span;
        }
        acc
    }

    pub(super) fn emit_template(&self, template: &Template) -> Rope<'a> {
        let mut out = Rope::new();
        for part in &template.parts {
            match part {
                TemplatePart::Raw(node) => out.append(self.source_rope(*node)),
                TemplatePart::Interpolation(expr) => {
                    out.append(self.emit_expr(*expr));
                }
            }
        }
        out
    }

    pub(super) fn emit_import(&self, import: &Import, out: &mut Rope<'a>) {
        let (specifier, at) = self.source_node(import.specifier);
        if let hir::ImportKind::Std(module) = import.kind {
            if !self.imported_std.borrow().contains(&module) {
                self.imported_std.borrow_mut().push(module);
            }
            match self.std_imports.get(module) {
                Some(path) => {
                    let quote = &specifier[..1];
                    out.push_lit(format!("{quote}{path}{quote}"));
                }
                None => out.push_src(specifier, at),
            }
            return;
        }
        match self.rewrite_imports {
            ImportRewrite::Off => out.push_src(specifier, at),
            ImportRewrite::Js => {
                let hir::ImportKind::Relative(kind) = import.kind else {
                    unreachable!("standard-library imports returned above")
                };
                let extension = if kind.is_tsx() { "jsx" } else { "js" };
                let suffix_len = if kind.is_tsx() { 5 } else { 4 };
                out.push_src(&specifier[..specifier.len() - suffix_len], at);
                out.push_lit(format!(".{extension}{}", &specifier[specifier.len() - 1..]));
            }
            ImportRewrite::Ts => {
                let hir::ImportKind::Relative(kind) = import.kind else {
                    unreachable!("standard-library imports returned above")
                };
                let extension = kind.output_extension();
                let suffix_len = if kind.is_tsx() { 5 } else { 4 };
                out.push_src(&specifier[..specifier.len() - suffix_len], at);
                out.push_lit(format!(".{extension}{}", &specifier[specifier.len() - 1..]));
            }
        }
    }

    pub(super) fn emit_statement_decision(
        &self,
        decision: &Decision,
        out: &mut Rope<'a>,
        body: &dyn Fn(hir::BodyId) -> Rope<'a>,
    ) {
        let span = self.span(decision.head);
        let (kind, inner) = match &decision.kind {
            DecisionKind::LetElse { binding_mode, .. } => {
                let mut inner = self.emit_let_else(decision, *binding_mode, body);
                if self.block_required_statements.contains(&decision.extent) {
                    inner = Rope::braced(inner);
                }
                (AnchorKind::LetElse, inner)
            }
            DecisionKind::IfLet => (AnchorKind::IfLet, self.emit_if_let(decision, body)),
            DecisionKind::Match { .. } => {
                crate::ice::bug!("expression decision in a statement body")
            }
        };
        out.anchored(kind, span.start, span.end, span.end, inner);
    }

    pub(super) fn emit_subject_initialization(
        &self,
        subject: &crate::core_ir::Subject,
        temp: &str,
        mark: NodeId,
    ) -> Rope<'a> {
        let mut out = Rope::new();
        let continued = self
            .core
            .has_statement_form(subject.value)
            .then(|| self.emit_continued_expr(subject.value, &ValueContinuation::assign(temp)))
            .flatten();
        if let Some(continued) = continued {
            out.push_lit("let ");
            out.push_mark(self.span(mark).start);
            out.push_lit(format!("{temp};"));
            out.push_lit(" ");
            out.append(continued);
        } else {
            out.push_lit("const ");
            out.push_mark(self.span(mark).start);
            out.push_lit(format!("{temp} = "));
            push_grouped(
                &mut out,
                self.emit_expr(subject.value).trim(),
                self.source_kind,
            );
            out.push_lit(";");
        }
        out
    }

    pub(super) fn emit_let_else(
        &self,
        decision: &Decision,
        mode: BindingMode,
        emit_body: &dyn Fn(hir::BodyId) -> Rope<'a>,
    ) -> Rope<'a> {
        let subject = &decision.subjects[0];
        let temp = self.temp_name(subject.temporary);
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
        out.push_break(1);
        out.append(Rope::indented(1, emit_body(body).trim()));
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

    pub(super) fn emit_if_let(
        &self,
        decision: &Decision,
        emit_body: &dyn Fn(hir::BodyId) -> Rope<'a>,
    ) -> Rope<'a> {
        let subject = &decision.subjects[0];
        let temp = self.temp_name(subject.temporary);
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
        out.append(Rope::indented(2, emit_body(body).trim()));
        out.push_break(1);
        out.push_lit("}");
        match &decision.miss {
            MissAction::Execute(body) => {
                out.push_lit(" else {");
                out.push_break(2);
                out.append(Rope::indented(2, emit_body(*body).trim()));
                out.push_break(1);
                out.push_lit("}");
            }
            MissAction::Decision(inner) => {
                out.push_lit(" else ");
                out.append(self.emit_if_let(inner, emit_body));
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

    pub(super) fn emit_value_decision(
        &self,
        decision: &Decision,
        continuation: &ValueContinuation<'_>,
        exits: &[HostExit],
    ) -> Rope<'a> {
        let DecisionKind::Match { dispatch, .. } = decision.kind else {
            crate::ice::bug!("value decision is not a match")
        };
        let mut out = Rope::new();
        // The region needs a label exactly when a rewritten exit sits
        // inside a loop or `switch` the arm body wrote, which would swallow
        // the `break` the rewrite emits. Otherwise the dispatch the region
        // already generates — an if-chain's `do { … } while (false)`, or
        // the `switch` itself — is the nearest `break` target, so the
        // labeled block around it would be a second exit target for the
        // same region (TASK-199, TASK-160 §6).
        let label = continuation
            .assignment_target()
            .filter(|_| decision_has_block_arm(decision))
            .filter(|_| exits.iter().any(|exit| exit.captured_break))
            .map(|target| self.exit_label(target));
        if let Some(label) = &label {
            out.push_lit(format!("{label}: "));
        }
        if continuation.is_expression() {
            crate::ice::bug!("match reached expression emission without a host rewrite")
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
        out.append(Rope::indented(
            1,
            match dispatch {
                MatchDispatch::Conditional => {
                    self.emit_if_chain(decision, continuation, exits, label.as_deref())
                }
                MatchDispatch::VariantSwitch | MatchDispatch::LiteralSwitch => {
                    self.emit_switch(decision, continuation, exits, label.as_deref())
                }
            },
        ));
        out.push_break(0);
        out.push_lit("}");
        Rope::scoped(out)
    }

    pub(super) fn emit_continued_expr(
        &self,
        expr: ExprId,
        continuation: &ValueContinuation<'_>,
    ) -> Option<Rope<'a>> {
        if !self.core.has_statement_form(expr) {
            return None;
        }
        // Structural parents may consume a child's owner rewrite directly
        // (for example a Result body's declaration initializer). Mark that
        // plan at the common entry point so a later source-range walk only
        // emits the child's inline slot and never schedules its statement
        // region a second time.
        if self
            .owner_slots_of(expr)
            .any(|rewrite| rewrite.expr == expr)
        {
            self.emitted_owner_rewrites.mark(expr);
        }
        match &self.core.exprs[expr.index()] {
            Expr::Decision(decision) => {
                let head = self.span(decision.head);
                let extent = self.span(decision.extent);
                let exits = self
                    .value_exits
                    .get(&expr)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let lowered = self.emit_value_decision(decision, continuation, exits);
                let mut out = Rope::new();
                out.anchored(AnchorKind::Match, head.start, head.end, extent.end, lowered);
                Some(out)
            }
            Expr::ResultRegion(region) => {
                Some(self.emit_result_region_continued(expr, region, continuation))
            }
            Expr::Propagate(propagate) => {
                Some(self.emit_expression_propagate(propagate, continuation))
            }
            Expr::Sequence(body) => self.emit_sequence_continued(*body, continuation),
            Expr::Apply(apply) => self.emit_apply_continued(expr, apply, continuation),
            Expr::Template(template) => {
                let (mut out, value) = self.emit_template_operand(expr, template, continuation)?;
                out.append(self.emit_value_delivery_without_region_exit(value, continuation));
                Some(Rope::scoped(out))
            }
            Expr::Opaque(_) => None,
        }
    }

    pub(super) fn emit_nested_operand(&self, expr: ExprId) -> Option<(Rope<'a>, Rope<'a>)> {
        match &self.core.exprs[expr.index()] {
            Expr::Sequence(body) => {
                self.emit_sequence_operand(*body, &ValueContinuation::expression())
            }
            Expr::Template(template) if self.core.has_statement_form(expr) => {
                self.emit_template_operand(expr, template, &ValueContinuation::expression())
            }
            _ => None,
        }
    }

    pub(super) fn emit_expression_propagate(
        &self,
        propagate: &Propagate,
        continuation: &ValueContinuation<'_>,
    ) -> Rope<'a> {
        let temp = self.temp_name(propagate.temporary);
        let mut out = self.emit_propagate_input(propagate.value, &temp);
        out.push_break(0);
        out.push_lit(format!(
            "if ({}) {{",
            result_failure_test(&temp, propagate.layout)
        ));
        out.push_break(1);
        out.append(self.emit_failure_exit(propagate, &temp));
        out.push_break(0);
        out.push_lit("}");
        out.push_break(0);
        let grouped = false;
        out.push_lit(continuation.assignment_prefix(grouped));
        out.push_lit(format!("{temp}.{}", propagate.layout.payload_field));
        out.push_lit(continuation.assignment_suffix(grouped));
        out.push_lit(";");
        let span = self.span(propagate.node);
        let mut anchored = Rope::new();
        anchored.anchored(
            AnchorKind::Try,
            span.start,
            span.end,
            span.end,
            Rope::scoped(out),
        );
        anchored
    }

    pub(super) fn emit_apply_continued(
        &self,
        expr: ExprId,
        apply: &Apply,
        continuation: &ValueContinuation<'_>,
    ) -> Option<Rope<'a>> {
        if !continuation.assigns() {
            return None;
        }
        let head = apply.head?;
        let start = self.span(apply.node).start;
        let end = apply.steps.last().map_or_else(
            || self.span(apply.node).end,
            |step| self.span(step.node).end,
        );
        let accumulator = self
            .value_slots
            .get(&expr)
            .unwrap_or_else(|| crate::ice::bug!("structured apply has no value slot"));
        let accumulator_is_host_slot = self
            .slot_exprs
            .get(&expr)
            .is_some_and(|slot| slot == accumulator);
        let mut inner = Rope::new();
        inner.push_lit("do {");
        if !accumulator_is_host_slot && !continuation.is_unwrapped_assignment_to(accumulator) {
            inner.push_break(1);
            inner.push_value_declaration(accumulator);
        }
        let piped = self
            .piped_slots
            .get(&expr)
            .filter(|slots| slots.len() == apply.steps.len())
            .unwrap_or_else(|| crate::ice::bug!("structured apply has no piped slots"));
        let push_target = |inner: &mut Rope<'a>, index: usize| match piped.get(index + 1) {
            Some(next) => inner.push_value_definition(next),
            None => inner.push_lit(format!("{accumulator} = ")),
        };
        inner.push_break(1);
        if self.nested_structured_value_slot(head).is_some() {
            inner.push_value_declaration(&piped[0]);
            inner.push_break(1);
            inner.append(Rope::indented(
                1,
                self.emit_continued_expr(head, &ValueContinuation::assign(&piped[0]))
                    .unwrap_or_else(|| crate::ice::bug!("structured apply head was not emitted")),
            ));
        } else {
            let value = match self.emit_nested_operand(head) {
                Some((prelude, value)) => {
                    inner.append(Rope::indented(1, prelude.trim_end()));
                    inner.push_break(1);
                    value
                }
                None => self.emit_expr(head),
            };
            inner.push_value_definition(&piped[0]);
            push_grouped(
                &mut inner,
                guard_line_comment(value.trim(), 1, self.source_kind),
                self.source_kind,
            );
            inner.push_lit(";");
        }
        for (index, step) in apply.steps.iter().enumerate() {
            let conditionally_reached = matches!(step.mode, ApplyMode::Postfix { optional: true });
            let operand = match (step.mode, self.emit_nested_operand(step.value)) {
                (ApplyMode::Postfix { .. }, Some((prelude, value))) => {
                    inner.push_break(1);
                    inner.append(Rope::indented(1, prelude.trim_end()));
                    inner.push_break(1);
                    push_target(&mut inner, index);
                    inner.append(guard_line_comment(value.trim(), 1, self.source_kind));
                    inner.push_lit(";");
                    continue;
                }
                (_, operand) => operand,
            };
            let step_value = if let Some((prelude, value)) = operand {
                inner.push_break(1);
                inner.append(Rope::indented(1, prelude.trim_end()));
                guard_line_comment(value.trim(), 1, self.source_kind)
            } else if let Some(slot) = self
                .nested_structured_value_slot(step.value)
                .filter(|_| !conditionally_reached)
            {
                inner.push_break(1);
                inner.push_value_declaration(slot);
                inner.push_break(1);
                inner.append(Rope::indented(
                    1,
                    self.emit_continued_expr(step.value, &ValueContinuation::assign(slot))
                        .unwrap_or_else(|| {
                            crate::ice::bug!("structured apply step was not emitted")
                        }),
                ));
                let mut value = Rope::new();
                value.push_lit(slot.clone());
                value
            } else {
                guard_line_comment(self.emit_expr(step.value).trim(), 1, self.source_kind)
            };
            inner.push_break(1);
            let input = self.pipe_input(apply, index, &piped[index]);
            push_target(&mut inner, index);
            match step.mode {
                ApplyMode::Postfix { .. } => {
                    inner.append(input);
                    inner.append(step_value);
                    inner.push_lit(";");
                }
                ApplyMode::Call => {
                    push_grouped(&mut inner, step_value, self.source_kind);
                    inner.push_lit("(");
                    inner.append(input);
                    inner.push_lit(");");
                }
            }
        }
        inner.push_break(1);
        if continuation.is_unwrapped_assignment_to(accumulator) {
            inner.push_lit("break;");
        } else {
            let mut value = Rope::new();
            value.push_lit(accumulator.clone());
            inner.append(self.emit_value_delivery(value, None, continuation));
        }
        inner.push_break(0);
        inner.push_lit("} while (false);");
        let mut out = Rope::new();
        out.anchored(AnchorKind::Pipe, start, end, end, Rope::scoped(inner));
        Some(out)
    }
}
