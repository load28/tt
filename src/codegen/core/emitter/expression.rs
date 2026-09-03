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
                            push_receiver(&mut input, acc);
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
                                push_grouped(&mut next, body);
                                next.push_lit("(");
                                push_grouped(&mut input, acc);
                                next.anchored_with_context(
                                    AnchorKind::Pipe,
                                    step_span.start,
                                    step_span.end,
                                    end,
                                    context,
                                    input,
                                );
                                next.push_lit(")");
                            } else {
                                self.used_pipe.set(true);
                                next.push_lit("$tt_ap(");
                                push_grouped(&mut input, acc);
                                next.anchored_with_context(
                                    AnchorKind::Pipe,
                                    step_span.start,
                                    step_span.end,
                                    end,
                                    context,
                                    input,
                                );
                                next.push_lit(", ");
                                push_grouped(&mut next, body);
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

    pub(super) fn emit_flow(&self, apply: &Apply, owner_end: usize) -> Rope<'a> {
        let mut steps = apply.steps.iter();
        let first = steps
            .next()
            .unwrap_or_else(|| crate::ice::bug!("flow has no step"));
        let mut acc = Rope::new();
        push_grouped(
            &mut acc,
            guard_line_comment(self.emit_expr(first.value).trim(), 0, self.source_kind),
        );
        let mut produced = self.span(first.node);
        for step in steps {
            self.used_flow.set(true);
            let step_span = self.span(step.node);
            let body = guard_line_comment(self.emit_expr(step.value).trim(), 0, self.source_kind);
            let mut next = Rope::new();
            next.push_lit("$tt_fl(");
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
                    next.push_lit(", (($tt_v) => ($tt_v)");
                    next.append(body);
                    next.push_lit("))");
                }
                ApplyMode::Call => {
                    next.push_lit(", ");
                    push_grouped(&mut next, body);
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
                    out.push_lit("${");
                    out.append(self.emit_expr(*expr));
                    out.push_lit("}");
                }
            }
        }
        out
    }

    pub(super) fn emit_import(&self, import: &Import, out: &mut Rope<'a>) {
        let (specifier, at) = self.source_node(import.specifier);
        if let hir::ImportKind::Std(module) = import.kind {
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

    pub(super) fn emit_statement_decision(&self, decision: &Decision, out: &mut Rope<'a>) {
        let span = self.span(decision.head);
        let (kind, inner) = match &decision.kind {
            DecisionKind::LetElse { binding_mode, .. } => (
                AnchorKind::LetElse,
                self.emit_let_else(decision, *binding_mode),
            ),
            DecisionKind::IfLet => (AnchorKind::IfLet, self.emit_if_let(decision)),
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
            push_grouped(&mut out, self.emit_expr(subject.value).trim());
            out.push_lit(";");
        }
        out
    }

    pub(super) fn emit_let_else(&self, decision: &Decision, mode: BindingMode) -> Rope<'a> {
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
        out.push_break(1);
        out.append(Rope::indented(1, self.emit_body(body).trim()));
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

    pub(super) fn emit_if_let(&self, decision: &Decision) -> Rope<'a> {
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
        out.append(Rope::indented(2, self.emit_body(body).trim()));
        out.push_break(1);
        out.push_lit("}");
        match &decision.miss {
            MissAction::Execute(body) => {
                out.push_lit(" else {");
                out.push_break(2);
                out.append(Rope::indented(2, self.emit_body(*body).trim()));
                out.push_break(1);
                out.push_lit("}");
            }
            MissAction::Decision(inner) => {
                out.push_lit(" else ");
                out.append(self.emit_if_let(inner));
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
            .map(exit_label);
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
                &temp_name(subject.temporary),
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
        let structural_slot = self.structured_value_slot(expr);
        for rewrite in &self.owner_slot_rewrites {
            // A grouping/sequence expression can be the structural value
            // while the host rewrite belongs to the decision it wraps. The
            // generated slot is their ownership identity across that Core
            // boundary, so consume the rewrite through the shared slot too.
            if rewrite.expr == expr
                || structural_slot.is_some_and(|slot| slot == &rewrite.slot)
            {
                self.emitted_owner_rewrites.mark(rewrite.expr);
            }
        }
        if let Some(schedule) = self.nested_schedules.get(&expr)
            && !schedule.steps().is_empty()
            && !self.active_scheduled_exprs.contains(expr)
        {
            let slot = self
                .value_slots
                .get(&expr)
                .unwrap_or_else(|| crate::ice::bug!("scheduled nested value has no slot"));
            let _active = self.active_scheduled_exprs.enter(expr);
            let mut action = self.emit_continued_expr(expr, &ValueContinuation::assign(slot))?;
            let mut captured = HashSet::new();
            for step in schedule.steps() {
                action = self.emit_scheduled_step(step, action, &mut captured);
            }
            let parent = schedule
                .steps()
                .last()
                .map(|step| step.parent)
                .unwrap_or_else(|| crate::ice::bug!("nested schedule lost its parent"));
            let mut out = Rope::new();
            out.push_lit(format!("let {slot};"));
            out.push_break(0);
            out.append(action);
            out.push_break(0);
            out.append(self.emit_value_delivery(
                self.source_range_with_nested_schedule(parent, expr, schedule),
                None,
                continuation,
            ));
            return Some(Rope::scoped(out));
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
                let mut out = Rope::new();
                for part in &template.parts {
                    let TemplatePart::Interpolation(inner) = part else {
                        continue;
                    };
                    if !self.core.has_statement_form(*inner) {
                        continue;
                    }
                    let slot = self
                        .structured_value_slot(*inner)
                        .unwrap_or_else(|| crate::ice::bug!("nested template value has no slot"));
                    out.push_lit(format!("let {slot};"));
                    out.push_break(0);
                    out.append(self.emit_continued_expr(*inner, &ValueContinuation::assign(slot))?);
                    out.push_break(0);
                }
                out.append(self.emit_value_delivery(
                    self.emit_template(template),
                    None,
                    continuation,
                ));
                Some(Rope::scoped(out))
            }
            Expr::Opaque(_) => None,
        }
    }

    pub(super) fn emit_expression_propagate(
        &self,
        propagate: &Propagate,
        continuation: &ValueContinuation<'_>,
    ) -> Rope<'a> {
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
        if !accumulator_is_host_slot {
            inner.push_break(1);
            inner.push_lit(format!("let {accumulator};"));
        }
        inner.push_break(1);
        if self.nested_structured_value_slot(head).is_some() {
            inner.append(Rope::indented(
                1,
                self.emit_continued_expr(head, &ValueContinuation::assign(accumulator))
                    .unwrap_or_else(|| crate::ice::bug!("structured apply head was not emitted")),
            ));
        } else {
            inner.push_lit(format!("{accumulator} = "));
            push_grouped(
                &mut inner,
                guard_line_comment(self.emit_expr(head).trim(), 1, self.source_kind),
            );
            inner.push_lit(";");
        }
        let mut produced = self.span(apply.node);
        for step in &apply.steps {
            let conditionally_reached = matches!(step.mode, ApplyMode::Postfix { optional: true });
            let step_value = if let Some(slot) = self
                .nested_structured_value_slot(step.value)
                .filter(|_| !conditionally_reached)
            {
                inner.push_break(1);
                inner.push_lit(format!("let {slot};"));
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
            let step_span = self.span(step.node);
            let context = Some((produced.start, produced.end));
            // The re-piped accumulator is the value this step consumes — a
            // mismatch on it belongs to this step (see `emit_apply`).
            let mut input = Rope::new();
            input.push_lit(accumulator.clone());
            match step.mode {
                ApplyMode::Postfix { .. } => {
                    inner.push_lit(format!("{accumulator} = "));
                    inner.anchored_with_context(
                        AnchorKind::Pipe,
                        step_span.start,
                        step_span.end,
                        end,
                        context,
                        input,
                    );
                    inner.append(step_value);
                    inner.push_lit(";");
                }
                ApplyMode::Call => {
                    // The accumulator has already been evaluated into a
                    // collision-free compiler slot. Reading that slot is
                    // unobservable, so the callee can occupy its natural
                    // call position without changing source evaluation.
                    inner.push_lit(format!("{accumulator} = "));
                    push_grouped(&mut inner, step_value);
                    inner.push_lit("(");
                    inner.anchored_with_context(
                        AnchorKind::Pipe,
                        step_span.start,
                        step_span.end,
                        end,
                        context,
                        input,
                    );
                    inner.push_lit(");");
                }
            }
            produced = step_span;
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
