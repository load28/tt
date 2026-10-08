//! Host-owner, conditional-operation, and evaluation-schedule emission.

use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn emit_owner_slot_rewrite(&self, rewrite: &OwnerSlotRewrite) -> Rope<'a> {
        self.within_owner_prelude(rewrite.owner, || self.emit_owner_slot_prelude(rewrite))
    }

    /// Writes one prelude hoisted to `owner`, first opening the owner's block
    /// when the owner is an unbraced body and no earlier prelude opened it.
    /// The opening brace carries its own layout scope, so the prelude and
    /// the rest of the owner's source indent one level inside it.
    pub(super) fn within_owner_prelude(
        &self,
        owner: SourceSpan,
        prelude: impl FnOnce() -> Rope<'a>,
    ) -> Rope<'a> {
        let mut out = Rope::new();
        if self.block_required_owners.contains(&owner) && self.opened_owner_blocks.claim(owner) {
            out.push_scope_open();
            out.push_lit("{");
            out.push_break(1);
        }
        self.emitting_owner_preludes.borrow_mut().push(owner);
        let prelude = prelude();
        self.emitting_owner_preludes.borrow_mut().pop();
        out.append(prelude);
        out
    }

    pub(super) fn open_declaration_blocks_at(&self, at: usize, out: &mut Rope<'a>) {
        for block in self
            .declarator_splits
            .iter()
            .filter_map(|split| split.block)
            .filter(|block| block.start == at)
        {
            if self.opened_owner_blocks.claim(block) {
                out.push_scope_open();
                out.push_lit("{");
                out.push_break(1);
            }
        }
        for split in &self.declarator_splits {
            if split.statement.start == at && self.opened_declaration_scopes.claim(split.statement)
            {
                out.push_scope_open();
            }
        }
    }

    /// Closes, innermost first, every opened owner block whose owner ends at
    /// `at`, unless that owner's own prelude is the one reaching its end.
    pub(super) fn close_owner_blocks_at(&self, at: usize, out: &mut Rope<'a>) {
        let mut ending: Vec<_> = self
            .block_required_by_end
            .get(&at)
            .into_iter()
            .flatten()
            .filter(|owner| {
                owner.end == at
                    && self.opened_owner_blocks.contains(**owner)
                    && !self.closed_owner_blocks.contains(**owner)
                    && !self.emitting_owner_preludes.borrow().contains(owner)
            })
            .copied()
            .collect();
        ending.sort_unstable_by_key(|owner| std::cmp::Reverse(owner.start));
        for owner in ending {
            self.closed_owner_blocks.claim(owner);
            *out = std::mem::take(out).trim_end();
            out.push_break(0);
            out.push_lit("}");
            out.push_scope_close();
        }
    }

    fn emit_owner_slot_prelude(&self, rewrite: &OwnerSlotRewrite) -> Rope<'a> {
        let _active = self.enter_active(rewrite.expr);
        let anchored = self
            .emit_continued_expr(rewrite.expr, &ValueContinuation::assign(&rewrite.slot))
            .unwrap_or_else(|| {
                crate::ice::bug!("initializer rewrite is not structurally emit-able")
            });
        let mut out = Rope::new();
        self.push_slot_declaration(
            &mut out,
            &rewrite.slot,
            rewrite.contextual_type,
            rewrite.contextual_type_awaited,
            rewrite.contextual_type_asserted,
        );
        out.push_break(0);
        out.append(anchored);
        out.push_break(0);
        Rope::scoped(out)
    }

    fn push_slot_declaration(
        &self,
        out: &mut Rope<'a>,
        slot: &str,
        contextual_type: Option<SourceSpan>,
        awaited: bool,
        asserted: bool,
    ) {
        match contextual_type {
            None => out.push_value_declaration(slot),
            Some(annotation) if asserted => out.push_asserted_declaration(
                slot,
                format!(": {}", &self.source[annotation.start..annotation.end]),
            ),
            Some(_) => {
                out.push_lit(format!("let {slot}"));
                self.push_contextual_type(out, contextual_type, awaited);
                out.push_lit(";");
            }
        }
    }

    pub(super) fn push_contextual_type(
        &self,
        out: &mut Rope<'a>,
        annotation: Option<SourceSpan>,
        awaited: bool,
    ) {
        let Some(annotation) = annotation else {
            return;
        };
        let authored = &self.source[annotation.start..annotation.end];
        if awaited {
            let ty = authored.strip_prefix(':').unwrap_or(authored);
            out.push_lit(format!(": Awaited<{ty}>"));
        } else {
            out.push_lit(authored.to_owned());
        }
    }

    pub(super) fn is_for_initializer_propagation(&self, node: NodeId) -> bool {
        self.for_initializer_propagations
            .iter()
            .any(|rewrite| rewrite.node == node)
    }

    pub(super) fn emit_for_initializer_propagation_prelude(
        &self,
        rewrite: &ForInitializerPropagationRewrite,
    ) -> Rope<'a> {
        self.within_owner_prelude(rewrite.owner, || {
            self.for_initializer_propagation_prelude(rewrite)
        })
    }

    fn for_initializer_propagation_prelude(
        &self,
        rewrite: &ForInitializerPropagationRewrite,
    ) -> Rope<'a> {
        let propagate = self
            .core
            .bodies
            .iter()
            .flat_map(|body| &body.statements)
            .find_map(|statement| match statement {
                Statement::Propagate(propagate) if propagate.node == rewrite.node => {
                    Some(propagate)
                }
                _ => None,
            })
            .unwrap_or_else(|| {
                crate::ice::bug!("for initializer propagation is missing from Core IR")
            });
        let temp = self.temp_name(propagate.temporary);
        let mut out = self.emit_propagate_input(propagate, &temp);
        out.push_break(0);
        self.push_failure_test(propagate, &temp, &mut out);
        out.push_break(1);
        out.append(self.emit_failure_exit(propagate, &temp));
        out.push_break(0);
        out.push_lit("}");
        out.push_break(0);
        Rope::scoped(out)
    }

    pub(super) fn emit_for_initializer_payload(&self, propagate: &Propagate) -> Rope<'a> {
        let temp = self.temp_name(propagate.temporary);
        let mut out = Rope::new();
        if let Some(binding) = propagate.binding {
            out.push_lit(format!("{} ", binding_keyword(binding.mode)));
            self.push_propagate_binding(binding.node, &mut out);
            self.push_propagate_payload(propagate, temp.as_str(), &mut out);
        }
        Rope::scoped(out)
    }

    pub(super) fn emit_compose_rewrite(&self, rewrite: &ComposeRewrite) -> Rope<'a> {
        self.within_owner_prelude(rewrite.owner, || self.emit_compose_prelude(rewrite))
    }

    fn emit_compose_prelude(&self, rewrite: &ComposeRewrite) -> Rope<'a> {
        let mut out = Rope::new();
        let depth = u16::from(rewrite.owner_kind == HostOwnerKind::ArrowExpression);
        if depth > 0 {
            out.push_lit("{");
            out.push_break(depth);
        }
        for action in &rewrite.actions {
            if let ComposeAction::Value(value) = action
                && value.defer_arm_values
                && !self.has_conditional_match_dispatch(value.expr)
            {
                out.push_selector_declaration(&value.slot);
                out.push_break(depth);
                continue;
            }
            let slot = match action {
                // A discarded completed call produces nothing; a consumed
                // one still fills the value's join slot with the result.
                ComposeAction::Value(value)
                    if value
                        .call_completion
                        .as_ref()
                        .is_some_and(|completion| completion.result.is_none()) =>
                {
                    continue;
                }
                ComposeAction::Value(value) if value.inline => {
                    let Expr::Decision(decision) = &self.core.exprs[value.expr.index()] else {
                        crate::ice::bug!("inline value lost its decision")
                    };
                    for (index, name) in self.inline_subjects[&decision.extent].iter().enumerate() {
                        if !self.inline_subject_needs_storage(decision, index) {
                            continue;
                        }
                        out.push_lit(format!("let {name};"));
                        out.push_break(depth);
                    }
                    continue;
                }
                ComposeAction::Value(value)
                    if value.defer_arm_values
                        && self.has_conditional_match_dispatch(value.expr) =>
                {
                    continue;
                }
                ComposeAction::Value(value) => &value.slot,
                ComposeAction::Operation(operation)
                    if matches!(
                        operation.kind,
                        PlannedConditionalKind::LogicalAssignment { .. }
                    ) =>
                {
                    continue;
                }
                ComposeAction::Operation(operation) => self.value_slot_name(operation.result),
            };
            out.push_value_declaration(slot);
            out.push_break(depth);
        }
        let mut emitted_steps = HashMap::new();
        let mut captured = CapturedSlots::default();
        let mut regions = 0usize;
        for action in &rewrite.actions {
            match action {
                ComposeAction::Value(value) => {
                    if value.inline {
                        continue;
                    }
                    let _active = self.enter_active(value.expr);
                    let mut lowered =
                        if let Some(completion) = &value.call_completion {
                            let mut region = Rope::new();
                            if let Some((name, type_args, callee)) = &completion.instantiation {
                                region.push_lit(format!("const {name} = {callee}"));
                                region.push_src(
                                    &self.source[type_args.start..type_args.end],
                                    type_args.start,
                                );
                                region.push_lit(";");
                                region.push_break(0);
                            }
                            for (name, source) in &completion.captures {
                                region.push_value_capture(name);
                                region.append(self.named_as_written(
                                    *source,
                                    self.captured_tail(*source, &captured),
                                ));
                                region.push_lit(");");
                                region.push_break(0);
                            }
                            region.append(
                                self.emit_continued_expr(
                                    value.expr,
                                    &ValueContinuation::invoke(
                                        &completion.invoke,
                                        &completion.close,
                                        completion.frame,
                                        completion.result.as_deref(),
                                        &completion.label,
                                    ),
                                )
                                .unwrap_or_else(|| {
                                    crate::ice::bug!("scoped call lost its value decision")
                                }),
                            );
                            region
                        } else if value.defer_arm_values {
                            self.emit_arm_selector(value.expr, &value.slot)
                        } else {
                            self.emit_continued_expr(
                                value.expr,
                                &ValueContinuation::assign(&value.slot),
                            )
                            .unwrap_or_else(|| {
                                crate::ice::bug!("compose value is not structurally emit-able")
                            })
                        };
                    for step in steps_to_emit(&value.steps, &mut emitted_steps) {
                        lowered = self.emit_scheduled_step(step, lowered, &mut captured);
                    }
                    if regions > 0 {
                        out.push_break(depth);
                    }
                    regions += 1;
                    out.append(Rope::indented(depth, lowered));
                }
                ComposeAction::Operation(operation)
                    if let Some(facts) = self.guarded_if_tests.get(&operation.parent) =>
                {
                    if regions > 0 {
                        out.push_break(depth);
                    }
                    regions += 1;
                    let lowered = self.emit_if_test_guard(operation, facts, &mut captured);
                    out.append(Rope::indented(depth, lowered));
                }
                ComposeAction::Operation(operation) => {
                    let mut lowered = self.emit_conditional_operation(operation, &mut captured);
                    for step in &operation.outer {
                        lowered = self.emit_scheduled_step(step, lowered, &mut captured);
                    }
                    if regions > 0 {
                        out.push_break(depth);
                    }
                    regions += 1;
                    out.append(Rope::indented(depth, lowered));
                }
            }
        }
        out.push_break(depth);
        if depth > 0 {
            out.push_lit("return ");
            let mut open = Rope::new();
            open.push_scope_open();
            open.append(out);
            return open;
        }
        Rope::scoped(out)
    }

    pub(super) fn emit_loop_test_prefix(&self, rewrite: &LoopTestRewrite) -> Rope<'a> {
        let mut out = Rope::new();
        match rewrite.kind {
            LoopTestKind::While => out.push_lit("while (true) {"),
            LoopTestKind::For => {
                out.push_lit("; ");
                if let Some(update) = rewrite.update {
                    out.push_src(&self.source[update.start..update.end], update.start);
                }
                out.push_lit(") {");
            }
        }
        out.push_break(1);
        for action in &rewrite.actions {
            let slot = match action {
                ComposeAction::Value(value) => &value.slot,
                ComposeAction::Operation(operation)
                    if matches!(
                        operation.kind,
                        PlannedConditionalKind::LogicalAssignment { .. }
                    ) =>
                {
                    continue;
                }
                ComposeAction::Operation(operation) => self.value_slot_name(operation.result),
            };
            out.push_value_declaration(slot);
            out.push_break(1);
        }
        self.loop_region_depth.set(self.loop_region_depth.get() + 1);
        let mut captured = CapturedSlots::default();
        let guarded = self.guarded_test_operation(rewrite);
        for action in &rewrite.actions {
            let lowered = match action {
                ComposeAction::Operation(operation)
                    if guarded.is_some_and(|guarded| std::ptr::eq(guarded, operation)) =>
                {
                    self.emit_test_guards(operation, &mut captured)
                }
                ComposeAction::Value(value) => {
                    let mut lowered = self
                        .emit_continued_expr(value.expr, &ValueContinuation::assign(&value.slot))
                        .unwrap_or_else(|| {
                            crate::ice::bug!("loop-test value is not structurally emit-able")
                        });
                    for step in &value.steps {
                        lowered = self.emit_scheduled_step(step, lowered, &mut captured);
                    }
                    lowered
                }
                ComposeAction::Operation(operation) => {
                    let mut lowered = self.emit_conditional_operation(operation, &mut captured);
                    for step in &operation.outer {
                        lowered = self.emit_scheduled_step(step, lowered, &mut captured);
                    }
                    lowered
                }
            };
            out.append(Rope::indented(1, lowered));
        }
        self.loop_region_depth.set(self.loop_region_depth.get() - 1);
        out.push_break(1);
        if guarded.is_none() {
            out.push_lit("if (!(");
        }
        Rope::scoped(out)
    }

    pub(super) fn guarded_test_operation<'r>(
        &self,
        rewrite: &'r LoopTestRewrite,
    ) -> Option<&'r PlannedConditionalOperation> {
        rewrite.actions.iter().find_map(|action| match action {
            ComposeAction::Operation(operation)
                if operation.parent == rewrite.test
                    && matches!(
                        operation.kind,
                        PlannedConditionalKind::LogicalAnd | PlannedConditionalKind::LogicalOr
                    ) =>
            {
                Some(operation)
            }
            _ => None,
        })
    }

    fn emit_if_test_guard(
        &self,
        operation: &PlannedConditionalOperation,
        facts: &crate::program_syntax::IfTestFacts,
        captured: &mut CapturedSlots,
    ) -> Rope<'a> {
        let result = self.value_slot_name(operation.result);
        let mut out = Rope::new();
        self.conditional_region_depth
            .set(self.conditional_region_depth.get() + 1);
        let condition = self.condition_test(&operation.condition, captured);
        let right = self.emit_conditional_active_branch(
            operation,
            &operation.values,
            result,
            None,
            operation.gaps.first().copied(),
            captured,
        );
        self.conditional_region_depth
            .set(self.conditional_region_depth.get() - 1);
        let mut closings = self.if_test_closings.borrow_mut();
        if operation.kind == PlannedConditionalKind::LogicalAnd {
            match facts.alternate {
                Some(_) => {
                    let flag = self.generated_name(&format!(
                        "$tt_f_{}",
                        result.strip_prefix("$tt_").unwrap_or(result)
                    ));
                    out.push_lit(format!("let {flag} = false;"));
                    out.push_break(0);
                    closings.push((facts.consequent.start, format!("{{ {flag} = true; ")));
                    closings.push((facts.consequent.end, format!(" }} }} if ({flag}) {{}}")));
                }
                None => closings.push((facts.consequent.end, " }".to_owned())),
            }
            out.push_lit("if (");
            out.append(condition);
            out.push_lit(") {");
            out.push_break(1);
            out.append(Rope::indented(1, right));
            out.push_break(1);
        } else {
            let label = self.exit_label(result);
            closings.push((facts.consequent.end, " }".to_owned()));
            out.push_lit(format!("{label}: {{"));
            out.push_break(1);
            out.push_lit("if (!(");
            out.append(condition);
            out.push_lit(")) {");
            out.push_break(2);
            out.append(Rope::indented(2, right));
            out.push_break(2);
            out.push_lit(format!("if (!({result})) break {label};"));
            out.push_break(1);
            out.push_lit("}");
            out.push_break(1);
        }
        drop(closings);
        self.delivered_conditional_values
            .borrow_mut()
            .retain(|value| !operation.values.contains(value));
        captured.insert(operation.result);
        out
    }

    pub(super) fn emit_if_test_closings(&self, at: usize, out: &mut Rope<'a>) {
        let mut closings = self.if_test_closings.borrow_mut();
        closings.retain(|(position, text)| {
            if *position == at {
                out.push_lit(text.clone());
                false
            } else {
                true
            }
        });
    }

    fn emit_test_guards(
        &self,
        operation: &PlannedConditionalOperation,
        captured: &mut CapturedSlots,
    ) -> Rope<'a> {
        let result = self.value_slot_name(operation.result);
        let mut out = Rope::new();
        self.conditional_region_depth
            .set(self.conditional_region_depth.get() + 1);
        let condition = self.condition_test(&operation.condition, captured);
        let right = self.emit_conditional_active_branch(
            operation,
            &operation.values,
            result,
            None,
            operation.gaps.first().copied(),
            captured,
        );
        self.conditional_region_depth
            .set(self.conditional_region_depth.get() - 1);
        let mut right_guard = Rope::new();
        right_guard.push_lit(format!("if (!({result})) break;"));
        out.push_break(0);
        match operation.kind {
            PlannedConditionalKind::LogicalAnd => {
                out.push_lit("if (!(");
                out.append(condition);
                out.push_lit(")) break;");
                out.push_break(0);
                out.append(right);
                out.push_break(0);
                out.append(right_guard);
            }
            _ => {
                out.push_lit("if (!(");
                out.append(condition);
                out.push_lit(")) {");
                out.push_break(1);
                out.append(Rope::indented(1, right));
                out.push_break(1);
                out.append(Rope::indented(1, right_guard));
                out.push_break(0);
                out.push_lit("}");
            }
        }
        self.delivered_conditional_values
            .borrow_mut()
            .retain(|value| !operation.values.contains(value));
        captured.insert(operation.result);
        out
    }

    /// Lowers one whole conditional operation (결정 17): evaluate the
    /// condition or callee once, branch, run the active branch's
    /// evaluations — tt regions included — in source order, and write every
    /// path's result into the operation's slot. All paths assign, so
    /// TypeScript sees the same definite-assignment correlation the original
    /// operation had, and an optional call's arguments evaluate only past
    /// its nullish check.
    pub(super) fn emit_conditional_operation(
        &self,
        operation: &PlannedConditionalOperation,
        captured: &mut CapturedSlots,
    ) -> Rope<'a> {
        let emitted =
            crate::stack::grow(|| self.emit_conditional_operation_grown(operation, captured));
        let mut delivered = self.delivered_conditional_values.borrow_mut();
        for value in &operation.values {
            delivered.remove(value);
        }
        captured.insert(operation.result);
        emitted
    }

    fn emit_conditional_operation_grown(
        &self,
        operation: &PlannedConditionalOperation,
        captured: &mut CapturedSlots,
    ) -> Rope<'a> {
        self.conditional_region_depth
            .set(self.conditional_region_depth.get() + 1);
        let result = self.value_slot_name(operation.result);
        let mut out = Rope::new();
        let deliver_value = |expr: ExprId, target: &str| {
            self.emit_continued_expr(expr, &ValueContinuation::assign(target))
                .unwrap_or_else(|| {
                    crate::ice::bug!("conditional operation value is not structurally emit-able")
                })
        };
        match &operation.kind {
            PlannedConditionalKind::LogicalAnd
            | PlannedConditionalKind::LogicalOr
            | PlannedConditionalKind::Nullish => {
                let (test, [left, stored]) =
                    self.condition_operand(&operation.condition, captured, &mut out);
                let nullish = matches!(operation.kind, PlannedConditionalKind::Nullish);
                if nullish {
                    out.push_lit("if ((");
                    out.append(test);
                    out.push_lit(") == null) {");
                } else {
                    out.push_lit("if (");
                    out.append(test);
                    out.push_lit(") {");
                }
                let operator = match operation.kind {
                    PlannedConditionalKind::LogicalAnd => "&&",
                    PlannedConditionalKind::LogicalOr => "||",
                    _ => "??",
                };
                let mut assign_left = Rope::new();
                assign_left.push_lit(format!("{result} = "));
                assign_left.append(left);
                assign_left.push_lit(";");
                let active = Rope::indented(
                    1,
                    self.emit_conditional_active_branch(
                        operation,
                        &operation.values,
                        result,
                        Some((stored, operator)),
                        operation.gaps.first().copied(),
                        captured,
                    ),
                );
                let (first, second) = if matches!(operation.kind, PlannedConditionalKind::LogicalOr)
                {
                    (Rope::indented(1, assign_left), active)
                } else {
                    (active, Rope::indented(1, assign_left))
                };
                out.push_break(1);
                out.append(first);
                out.push_break(0);
                out.push_lit("} else {");
                out.push_break(1);
                out.append(second);
                out.push_break(0);
                out.push_lit("}");
            }
            PlannedConditionalKind::LogicalAssignment { operator } => {
                let PlannedEvaluationInput::Source {
                    source: target,
                    receiver,
                    key,
                    ..
                } = &operation.condition
                else {
                    crate::ice::bug!(
                        "a logical assignment's target was not planned as its condition"
                    )
                };
                for part in [receiver, key].into_iter().flatten() {
                    self.capture_planned_receiver(part, captured, &mut out);
                }
                let target_text = || {
                    let mut text = Rope::new();
                    push_grouped(
                        &mut text,
                        self.captured_source(*target, captured),
                        self.source_kind,
                    );
                    text
                };
                out.push_lit("if (");
                match operator {
                    crate::program_syntax::LogicalAssignment::Nullish => {
                        out.append(target_text());
                        out.push_lit(" == null");
                    }
                    crate::program_syntax::LogicalAssignment::Or => {
                        out.push_lit("!");
                        out.append(target_text());
                    }
                    crate::program_syntax::LogicalAssignment::And => out.append(target_text()),
                }
                out.push_lit(") {");
                out.push_break(1);
                let mut assign = Rope::new();
                assign.append(self.captured_source(*target, captured));
                push_gap(
                    self.source,
                    &mut assign,
                    " = ",
                    operation.gaps.first().copied(),
                    "",
                );
                out.append(Rope::indented(
                    1,
                    self.emit_conditional_assignment_branch(operation, assign, captured),
                ));
                out.push_break(0);
                out.push_lit("}");
            }
            PlannedConditionalKind::Ternary {
                consequent,
                alternate,
            } => {
                let test = self.condition_test(&operation.condition, captured);
                let mut branch =
                    |out: &mut Rope<'a>, content: &PlannedBranch, gap: usize| match content {
                        PlannedBranch::Values(values) => {
                            out.append(Rope::indented(
                                1,
                                self.emit_conditional_active_branch(
                                    operation,
                                    values,
                                    result,
                                    None,
                                    operation.gaps.get(gap).copied(),
                                    captured,
                                ),
                            ));
                        }
                        PlannedBranch::Source(span) => {
                            out.push_lit(result.to_owned());
                            push_gap(
                                self.source,
                                out,
                                " = ",
                                operation.gaps.get(gap).copied(),
                                "",
                            );
                            push_grouped(
                                out,
                                self.named_as_written(*span, self.captured_tail(*span, captured)),
                                self.source_kind,
                            );
                            out.push_lit(";");
                        }
                    };
                out.push_lit("if (");
                out.append(test);
                out.push_lit(") {");
                out.push_break(1);
                branch(&mut out, consequent, 0);
                out.push_break(0);
                out.push_lit("} else {");
                out.push_break(1);
                branch(&mut out, alternate, 1);
                out.push_break(0);
                out.push_lit("}");
            }
            PlannedConditionalKind::OptionalCall {
                arguments,
                type_args,
                test,
            } => {
                // A call skipped at its callee's member link reads the callee only
                // past the receiver's test, as the chain does.
                let mut guarded_callee = Rope::new();
                let mut callee_text = None;
                let condition = match (&operation.kind, &operation.condition) {
                    (
                        PlannedConditionalKind::OptionalCall {
                            test: OptionalCallTest::Receiver,
                            ..
                        },
                        PlannedEvaluationInput::Source {
                            source,
                            target,
                            mode: EvaluationInputMode::MemberReference,
                            receiver: Some(receiver),
                            key,
                        },
                    ) => {
                        // The receiver is read before its test and the key once the
                        // test passed; the member is read by the call itself
                        // (`member_callee`).
                        if captured.insert(*target) {
                            self.capture_planned_receiver(receiver, captured, &mut out);
                            if let Some(key) = key {
                                self.capture_planned_receiver(key, captured, &mut guarded_callee);
                            }
                        }
                        callee_text =
                            Some(self.member_callee_text(*source, [Some(*receiver), *key]));
                        member_callee(self.source, *source, [Some(*receiver), *key], |slot| {
                            self.value_slot_name(slot)
                        })
                    }
                    _ => self.emit_condition_capture(&operation.condition, captured, &mut out),
                };
                let callee_text =
                    callee_text.unwrap_or_else(|| AuthoredText::generated(condition.clone()));
                let receiver = match &operation.condition {
                    PlannedEvaluationInput::Source {
                        mode: EvaluationInputMode::MemberReference,
                        receiver,
                        ..
                    } => Some(
                        receiver
                            .unwrap_or_else(|| crate::ice::bug!("member callee has no receiver")),
                    ),
                    _ => None,
                };
                // Only a callee the chain tests is a captured value; it is
                // called through its receiver. Every other callee is the
                // member call itself.
                let through = receiver.filter(|_| *test == OptionalCallTest::Callee);
                let tested = match test {
                    OptionalCallTest::Callee => AuthoredText::generated(condition.clone()),
                    OptionalCallTest::Receiver => self.planned_receiver_authored(
                        &receiver
                            .unwrap_or_else(|| crate::ice::bug!("receiver test has no receiver")),
                    ),
                    OptionalCallTest::Inner => {
                        crate::ice::bug!("an optional call skipped inside its callee was planned")
                    }
                };
                out.push_lit("if (");
                tested.push_to(self.source, &mut out);
                out.push_lit(" != null) {");
                out.push_break(1);
                // A single whole-value argument with completable arms calls
                // the captured callee from each dispatch arm, keeping the
                // argument in the consumer's contextual position — the same
                // completion the plain-call path performs (TASK-327).
                if let [PlannedOperand::Value(expr)] = arguments.as_slice()
                    && type_args.is_none()
                    && completable_decision_arms(self.core, *expr, &self.exits_for_expr(*expr))
                {
                    let mut prefix = match through {
                        Some(receiver) => {
                            let mut prefix = AuthoredText::generated(format!("{condition}.call("));
                            prefix.append(self.planned_receiver_authored(&receiver));
                            prefix
                        }
                        None => callee_text.clone(),
                    };
                    prefix.push_gap(
                        self.source,
                        if through.is_some() { ", " } else { "(" },
                        operation.gaps.first().copied(),
                        "",
                    );
                    let mut close = AuthoredText::default();
                    close.push_gap(self.source, "", operation.gaps.get(1).copied(), ")");
                    let _active = self.enter_active(*expr);
                    let body = self
                        .emit_continued_expr(
                            *expr,
                            &ValueContinuation::invoke(&prefix, &close, None, Some(result), result),
                        )
                        .unwrap_or_else(|| {
                            crate::ice::bug!("optional completed call lost its value decision")
                        });
                    let mut branch = std::mem::take(&mut guarded_callee);
                    branch.append(body);
                    out.append(Rope::indented(1, branch));
                    out.push_break(0);
                    out.push_lit("} else {");
                    out.push_break(1);
                    out.push_lit(format!("{result} = undefined;"));
                    out.push_break(0);
                    out.push_lit("}");
                    out.push_break(0);
                    self.conditional_region_depth
                        .set(self.conditional_region_depth.get() - 1);
                    let primary = operation.values[0];
                    let (kind, start, end, extent) = self.value_anchor(primary);
                    let mut anchored = Rope::new();
                    anchored.anchored(kind, start, end, extent, Rope::scoped(out));
                    return anchored;
                }
                // The active branch: arguments in source order — captures
                // for those before a tt value, regions for the values —
                // then the call, through the receiver when the callee is a
                // member reference.
                let mut body = std::mem::take(&mut guarded_callee);
                for argument in arguments {
                    match argument {
                        PlannedOperand::Value(expr) => {
                            let name = self.value_name_of(*expr);
                            body.push_value_declaration(name);
                            body.push_break(0);
                            body.append(deliver_value(*expr, name));
                            body.push_break(0);
                        }
                        PlannedOperand::Source {
                            span,
                            spread,
                            capture: Some(slot),
                        } => {
                            if captured.insert(*slot) {
                                let mode = if *spread {
                                    EvaluationInputMode::SpreadElement
                                } else {
                                    EvaluationInputMode::Value
                                };
                                let (open, close) = self.capture_form(mode);
                                if *spread {
                                    body.push_value_capture(self.value_slot_name(*slot));
                                } else {
                                    self.push_capture(
                                        self.value_slot_name(*slot),
                                        *span,
                                        &mut body,
                                    );
                                }
                                body.push_lit(open);
                                body.append(
                                    self.named_as_written(
                                        *span,
                                        self.captured_tail(*span, captured),
                                    ),
                                );
                                body.push_lit(close);
                                body.push_lit(");");
                                body.push_break(0);
                            }
                        }
                        PlannedOperand::Composed {
                            span,
                            values,
                            spread,
                            capture,
                        } => {
                            body.append(
                                self.emit_conditional_active_values(operation, values, captured),
                            );
                            if let Some(slot) = capture
                                && captured.insert(*slot)
                            {
                                let (open, close) = self.capture_form(if *spread {
                                    EvaluationInputMode::SpreadElement
                                } else {
                                    EvaluationInputMode::Value
                                });
                                body.push_value_capture(self.value_slot_name(*slot));
                                body.push_lit(open);
                                body.append(self.composed_operand(operation, *span, values));
                                body.push_lit(close);
                                body.push_lit(");");
                                body.push_break(0);
                            }
                        }
                        PlannedOperand::Source { capture: None, .. } => {}
                    }
                }
                body.push_lit(format!("{result} = "));
                callee_text.push_to(self.source, &mut body);
                if let Some(span) = type_args {
                    body.push_src(&self.source[span.start..span.end], span.start);
                }
                if let Some(receiver) = through {
                    body.push_lit(".call(");
                    self.push_planned_receiver(&receiver, true, &mut body);
                }
                for (index, argument) in arguments.iter().enumerate() {
                    let gap = operation.gaps.get(index).copied();
                    match gap.and_then(|gap| self.call_opener(gap)) {
                        Some(open) if index == 0 && through.is_none() => {
                            let gap = gap.unwrap_or_else(|| crate::ice::bug!("opener without gap"));
                            push_gap(
                                self.source,
                                &mut body,
                                "",
                                Some(SourceSpan {
                                    start: gap.start,
                                    end: open,
                                }),
                                "",
                            );
                            body.push_src(&self.source[open..open + 1], open);
                            push_gap(
                                self.source,
                                &mut body,
                                "",
                                Some(SourceSpan {
                                    start: open + 1,
                                    end: gap.end,
                                }),
                                "",
                            );
                        }
                        _ => {
                            let separator = if index > 0 || through.is_some() {
                                ", "
                            } else {
                                "("
                            };
                            push_gap(self.source, &mut body, separator, gap, "");
                        }
                    }
                    self.push_operand(operation, argument, &mut body);
                }
                if arguments.is_empty() && through.is_none() {
                    body.push_lit("(");
                }
                push_gap(
                    self.source,
                    &mut body,
                    "",
                    operation.gaps.get(arguments.len()).copied(),
                    ")",
                );
                body.push_lit(";");
                out.append(Rope::indented(1, body));
                out.push_break(0);
                out.push_lit("} else {");
                out.push_break(1);
                out.push_lit(format!("{result} = undefined;"));
                out.push_break(0);
                out.push_lit("}");
            }
        }
        out.push_break(0);
        self.conditional_region_depth
            .set(self.conditional_region_depth.get() - 1);
        let primary = operation.values[0];
        let (kind, start, end, extent) = self.value_anchor(primary);
        let mut anchored = Rope::new();
        anchored.anchored(kind, start, end, extent, Rope::scoped(out));
        anchored
    }

    /// The receiver a member callee is called through, as the expression
    /// that reads it.
    /// A member callee as written, its captured parts read from their
    /// slots and the rest copied from the source.
    fn member_callee_text(
        &self,
        callee: SourceSpan,
        parts: [Option<PlannedReceiver>; 2],
    ) -> AuthoredText {
        let mut text = AuthoredText::default();
        let mut cursor = callee.start;
        for part in parts.into_iter().flatten() {
            let PlannedReceiver::Captured { source: at, slot } = part else {
                continue;
            };
            text.push_source(SourceSpan {
                start: cursor,
                end: at.start,
            });
            text.push_generated(self.value_slot_name(slot));
            cursor = at.end;
        }
        text.push_source(SourceSpan {
            start: cursor,
            end: callee.end,
        });
        text
    }

    /// The `(` that opens a call's arguments: the last token of the
    /// authored text between the callee and the first argument.
    fn call_opener(&self, gap: SourceSpan) -> Option<usize> {
        crate::lexer::lex_with_kind(self.source, gap.start, gap.end, self.source_kind)
            .last()
            .filter(|token| matches!(token.kind, crate::lexer::TokenKind::Punct(b'(')))
            .map(|token| token.span.start)
    }

    /// A receiver read again: its slot, or its source text as written.
    fn planned_receiver_authored(&self, receiver: &PlannedReceiver) -> AuthoredText {
        match *receiver {
            PlannedReceiver::Stable { source } => {
                let mut text = AuthoredText::default();
                text.push_source(source);
                text
            }
            _ => AuthoredText::generated(self.planned_receiver_text(receiver)),
        }
    }

    fn planned_receiver_text(&self, receiver: &PlannedReceiver) -> String {
        match receiver {
            PlannedReceiver::Captured { slot, .. } => self.value_slot_name(*slot).to_owned(),
            PlannedReceiver::Stable { source } => self.source[source.start..source.end].to_owned(),
            PlannedReceiver::ThisOfSuper { .. } => "this".to_owned(),
        }
    }

    /// One branch of a conditional operation: a value that is the whole
    /// branch delivers straight into the result slot; otherwise the branch's
    /// values run in source order and the branch is rebuilt around them.
    ///
    /// The right operand of a logical operation is written into the result
    /// slot as the operation itself, over `left`, the left operand's
    /// storage as the test narrowed it, so the result has the type
    /// TypeScript gives the operation (`checkBinaryLikeExpressionWorker` in
    /// the checker): the left operand's type alone when the branch cannot
    /// be taken (`true || v`, `o ?? v`).
    pub(super) fn emit_conditional_active_branch(
        &self,
        operation: &PlannedConditionalOperation,
        values: &[ExprId],
        result: &str,
        left: Option<(Rope<'a>, &str)>,
        gap: Option<SourceSpan>,
        captured: &mut CapturedSlots,
    ) -> Rope<'a> {
        let entries: Vec<_> = values
            .iter()
            .filter_map(|value| {
                operation
                    .active
                    .iter()
                    .find(|active| active.value == *value)
            })
            .collect();
        let Some(first) = entries.first() else {
            let [value] = values else {
                crate::ice::bug!("a conditional branch of several values has no active plan")
            };
            if left.is_some() {
                crate::ice::bug!("a logical operation's right operand has no active plan")
            }
            let _active = self.enter_active(*value);
            return self
                .emit_continued_expr(*value, &ValueContinuation::assign(result))
                .unwrap_or_else(|| {
                    crate::ice::bug!("conditional operation value is not structurally emit-able")
                });
        };
        let branch = first.branch;
        let mut out = self.emit_conditional_active_values(operation, values, captured);
        out.push_lit(result.to_owned());
        let binary = left.is_some();
        if let Some((operand, operator)) = left {
            out.push_lit(" = ");
            out.append(operand);
            push_gap(self.source, &mut out, &format!(" {operator} "), gap, "");
        } else {
            push_gap(self.source, &mut out, " = ", gap, "");
        }
        let steps: Vec<_> = entries.iter().flat_map(|active| &active.steps).collect();
        let operand =
            self.source_range_with_scheduled_values(branch, &operation.values, &steps, &[]);
        if binary
            && self.authored_in_parentheses(branch)
            && !operand.resolved_text().is_some_and(|text| {
                crate::lexer::is_primary_expression(&text, 0, text.len(), self.source_kind)
            })
        {
            out.push_lit("(");
            out.append(operand);
            out.push_lit(")");
        } else {
            push_grouped(&mut out, operand, self.source_kind);
        }
        out.push_lit(";");
        out
    }

    fn authored_in_parentheses(&self, span: SourceSpan) -> bool {
        let bytes = self.source.as_bytes();
        let mut before = span.start;
        while before > 0 && bytes[before - 1].is_ascii_whitespace() {
            before -= 1;
        }
        let (after, _) = crate::scanner::skip_trivia(bytes, span.end, bytes.len());
        if before == 0 || bytes[before - 1] != b'(' || bytes.get(after) != Some(&b')') {
            return false;
        }
        let tokens = crate::lexer::lex(self.source, before - 1, after + 1);
        crate::parser::find_close_at(&tokens, 0) == Some(tokens.len() - 1)
    }

    fn emit_conditional_assignment_branch(
        &self,
        operation: &PlannedConditionalOperation,
        assign: Rope<'a>,
        captured: &mut CapturedSlots,
    ) -> Rope<'a> {
        let entries: Vec<_> = operation
            .values
            .iter()
            .filter_map(|value| {
                operation
                    .active
                    .iter()
                    .find(|active| active.value == *value)
            })
            .collect();
        let Some(first) = entries.first() else {
            crate::ice::bug!("a logical assignment's right operand has no active plan")
        };
        let branch = first.branch;
        let mut out = self.emit_conditional_active_values(operation, &operation.values, captured);
        out.append(assign);
        let steps: Vec<_> = entries.iter().flat_map(|active| &active.steps).collect();
        push_grouped(
            &mut out,
            self.source_range_with_scheduled_values(branch, &operation.values, &steps, &[]),
            self.source_kind,
        );
        out.push_lit(";");
        out
    }

    /// Evaluates the given values of a conditional operation, in source
    /// order, each into its own slot followed by the evaluation steps between
    /// it and the branch or argument that holds it.
    pub(super) fn emit_conditional_active_values(
        &self,
        operation: &PlannedConditionalOperation,
        values: &[ExprId],
        captured: &mut CapturedSlots,
    ) -> Rope<'a> {
        let mut out = Rope::new();
        for value in values {
            let _active = self.enter_active(*value);
            let Some(active) = operation
                .active
                .iter()
                .find(|active| active.value == *value)
            else {
                crate::ice::bug!("a composed conditional value has no active plan")
            };
            let value_slot = self.value_name_of(*value);
            out.push_value_declaration(value_slot);
            out.push_break(0);
            let mut lowered = self
                .emit_continued_expr(*value, &ValueContinuation::assign(value_slot))
                .unwrap_or_else(|| {
                    crate::ice::bug!("conditional branch value is not structurally emit-able")
                });
            for step in &active.steps {
                lowered = self.emit_scheduled_step(step, lowered, captured);
            }
            out.append(lowered);
            out.push_break(0);
            self.delivered_conditional_values
                .borrow_mut()
                .insert(*value);
        }
        out
    }

    pub(super) fn carried_by_capture(&self, start: usize, end: usize) -> bool {
        let (values, exprs) = self.slot_value_order();
        let Some(value) = values.containing(start, end, true).next() else {
            return false;
        };
        let (_, value_start, _, value_end) = self.value_anchor(exprs[value]);
        self.replacement_order
            .containing(value_start, value_end, true)
            .map(|index| &self.source_replacements[index])
            .any(|captured| {
                captured.anchor.is_none()
                    && (captured.source.start < start || end < captured.source.end)
                    && !self.capture_is_active(captured.source)
            })
    }

    fn nested_input_order(&self) -> &(crate::span_index::NestedOrder, Vec<NestedInput>) {
        self.nested_input_order.get_or_init(|| {
            let mut covered = HashMap::new();
            let mut read = HashSet::new();
            let mut spans = Vec::new();
            let mut list = Vec::new();
            let mut exprs: Vec<&ExprId> = self.nested_schedules.keys().collect();
            exprs.sort_unstable_by_key(|expr| expr.index());
            for expr in exprs {
                for step in self.nested_schedules[expr].steps().fresh(&mut covered) {
                    for input in step.inputs.fresh(&mut read) {
                        let PlannedEvaluationInput::Source {
                            source: dependency,
                            mode,
                            ..
                        } = input
                        else {
                            continue;
                        };
                        spans.push((dependency.start, dependency.end));
                        list.push(NestedInput {
                            step: step.clone(),
                            input: *input,
                            comma: false,
                        });
                        if *mode == EvaluationInputMode::Discarded {
                            let comma = discarded_operand_comma(self.source, *dependency);
                            spans.push((comma.start, comma.end));
                            list.push(NestedInput {
                                step: step.clone(),
                                input: *input,
                                comma: true,
                            });
                        }
                    }
                }
            }
            (crate::span_index::NestedOrder::new(spans), list)
        })
    }

    fn slot_value_order(&self) -> &(crate::span_index::NestedOrder, Vec<ExprId>) {
        self.slot_value_order.get_or_init(|| {
            let mut exprs: Vec<ExprId> = self.slot_exprs.keys().copied().collect();
            exprs.sort_unstable_by_key(|expr| expr.index());
            let order = crate::span_index::NestedOrder::new(exprs.iter().map(|expr| {
                let (_, start, _, extent) = self.value_anchor(*expr);
                (start, extent)
            }));
            (order, exprs)
        })
    }

    pub(super) fn inside_captured_value(
        &self,
        captured: SourceSpan,
        start: usize,
        end: usize,
    ) -> bool {
        let (values, exprs) = self.slot_value_order();
        values
            .containing(start, end, true)
            .next()
            .is_some_and(|value| {
                let (_, value_start, _, extent) = self.value_anchor(exprs[value]);
                captured.start <= value_start && extent <= captured.end
            })
    }

    fn captured_source(
        &self,
        source: SourceSpan,
        captured: &HashSet<crate::evaluation_ir::ValueSlotId>,
    ) -> Rope<'a> {
        self.captured_range(source, captured, self.piped_value_at(source.start))
    }

    fn captured_tail(
        &self,
        source: SourceSpan,
        captured: &HashSet<crate::evaluation_ir::ValueSlotId>,
    ) -> Rope<'a> {
        self.captured_range(source, captured, None)
    }

    fn captured_range(
        &self,
        source: SourceSpan,
        captured: &HashSet<crate::evaluation_ir::ValueSlotId>,
        piped: Option<Rope<'a>>,
    ) -> Rope<'a> {
        // Compose the capture from already materialized dependencies and Core
        // expressions. Source bytes belonging to a dependency are never evaluated
        // again; expression-only tt nodes are lowered at this evaluation site.
        enum Part<'b, 'r> {
            Captured(&'b str),
            Read(String),
            Value(ExprId),
            Statement(&'b Statement),
            Piped(Rope<'r>),
        }
        enum Admit<'b> {
            Always,
            Unclaimed(Option<SourceSpan>),
            CapturedReplacement(&'b str),
        }
        let mut eager = Vec::new();
        if let Some(piped) = piped {
            eager.push((
                SourceSpan {
                    start: source.start,
                    end: source.start,
                },
                Part::Piped(piped),
                Admit::Always,
            ));
        }
        eager.sort_by_key(|(span, ..)| (span.start, std::cmp::Reverse(span.end)));
        let (values, value_exprs) = self.value_order();
        let (statements, statement_list) = self.statement_order();
        let (inputs, input_list) = self.nested_input_order();
        let mut input_at = 0;
        self.active_capture_sources.borrow_mut().push(source);
        self.rebuilt_sources.borrow_mut().push(source);
        let mut out = Rope::new();
        let mut cursor = source.start;
        let (mut eager_at, mut replacement_at, mut value_at, mut statement_at) = (0, 0, 0, 0);
        let mut eager = eager.into_iter().map(Some).collect::<Vec<_>>();
        loop {
            while eager_at < eager.len()
                && eager[eager_at]
                    .as_ref()
                    .is_none_or(|(span, ..)| span.start < cursor)
            {
                eager_at += 1;
            }
            let replacement = loop {
                let Some(position) =
                    self.replacement_order
                        .next_within(cursor, source.end, replacement_at)
                else {
                    break None;
                };
                let replacement = &self.source_replacements[self.replacement_order.index(position)];
                if (replacement.anchor.is_none() || !replacement.claim)
                    && (replacement.source != source || replacement.anchor.is_some())
                {
                    break Some(position);
                }
                replacement_at = position + 1;
            };
            let input = loop {
                let Some(position) = inputs.next_within(cursor, source.end, input_at) else {
                    break None;
                };
                let entry = &input_list[inputs.index(position)];
                if let PlannedEvaluationInput::Source {
                    source: dependency,
                    target,
                    ..
                } = entry.input
                    && dependency != source
                    && captured.contains(&target)
                {
                    break Some(position);
                }
                input_at = position + 1;
            };
            let value = values.next_within(cursor, source.end, value_at);
            let statement = statements.next_within(cursor, source.end, statement_at);
            let span_of = |(start, end): (usize, usize)| SourceSpan { start, end };
            let mut candidates: Vec<(SourceSpan, u8)> = Vec::with_capacity(4);
            if let Some(position) = replacement {
                candidates.push((span_of(self.replacement_order.span(position)), 0));
            }
            if let Some(Some((span, ..))) = eager.get(eager_at) {
                candidates.push((*span, 1));
            }
            if let Some(position) = input {
                candidates.push((span_of(inputs.span(position)), 1));
            }
            if let Some(position) = value {
                candidates.push((span_of(values.span(position)), 2));
            }
            if let Some(position) = statement {
                candidates.push((span_of(statements.span(position)), 3));
            }
            let Some(&(span, kind)) = candidates
                .iter()
                .enumerate()
                .min_by_key(|(order, (span, kind))| {
                    (span.start, std::cmp::Reverse(span.end), *kind, *order)
                })
                .map(|(_, candidate)| candidate)
            else {
                break;
            };
            let kind = if kind == 1
                && input.is_some_and(|position| span_of(inputs.span(position)) == span)
                && !eager
                    .get(eager_at)
                    .is_some_and(|entry| entry.as_ref().is_some_and(|(other, ..)| *other == span))
            {
                4
            } else {
                kind
            };
            let (part, admit) = match kind {
                0 => {
                    let position = replacement.expect("a replacement candidate");
                    replacement_at = position + 1;
                    let replacement =
                        &self.source_replacements[self.replacement_order.index(position)];
                    (
                        Part::Captured(replacement.written()),
                        Admit::CapturedReplacement(&replacement.slot),
                    )
                }
                1 => {
                    let (_, part, admit) = eager[eager_at].take().expect("an eager candidate");
                    eager_at += 1;
                    (part, admit)
                }
                4 => {
                    let position = input.expect("an input candidate");
                    input_at = position + 1;
                    let entry = &input_list[inputs.index(position)];
                    let part = match entry.input {
                        PlannedEvaluationInput::Source {
                            mode: EvaluationInputMode::Discarded,
                            ..
                        } => Part::Read(String::new()),
                        _ if entry.comma => Part::Read(String::new()),
                        _ => Part::Piped(self.captured_reading_rope(&entry.step, &entry.input)),
                    };
                    (part, Admit::Always)
                }
                2 => {
                    let position = value.expect("a value candidate");
                    value_at = position + 1;
                    (
                        Part::Value(value_exprs[values.index(position)]),
                        Admit::Unclaimed(Some(source)),
                    )
                }
                _ => {
                    let position = statement.expect("a statement candidate");
                    statement_at = position + 1;
                    (
                        Part::Statement({
                            let (body, at) = statement_list[statements.index(position)];
                            &self.core.bodies[body].statements[at]
                        }),
                        Admit::Unclaimed(Some(source)),
                    )
                }
            };
            let admitted = match admit {
                Admit::Always => true,
                Admit::Unclaimed(within) => !self.inside_claimed_frame(span, within),
                Admit::CapturedReplacement(slot) => {
                    !self.inside_claimed_frame(span, None)
                        && self
                            .slots_named
                            .get_or_init(|| {
                                let mut named = HashMap::<String, Vec<_>>::new();
                                for (slot, name) in &self.scheduled_slots {
                                    named.entry(name.clone()).or_default().push(*slot);
                                }
                                named
                            })
                            .get(slot)
                            .is_some_and(|slots| {
                                crate::work::tick_by("captured slot checks", slots.len());
                                slots.iter().any(|slot| captured.contains(slot))
                            })
                }
            };
            if !admitted {
                continue;
            }
            if cursor < span.start {
                out.append(self.source_range_rope(hir::Span {
                    start: cursor,
                    end: span.start,
                }));
            }
            match part {
                Part::Captured(name) => out.push_lit(name.to_owned()),
                Part::Read(text) => out.push_lit(text),
                Part::Piped(piped) => out.append(piped),
                Part::Statement(statement) => {
                    out.append(self.emit_statements(std::slice::from_ref(statement)))
                }
                Part::Value(expr) if self.discarded_values.contains(&expr) => {}
                Part::Value(expr) => {
                    let (kind, start, head_end, extent) = self.value_anchor(expr);
                    let delivered = self.delivered_conditional_values.borrow().contains(&expr);
                    if let Some(name) = self
                        .slot_exprs
                        .get(&expr)
                        .or_else(|| delivered.then(|| &self.value_slots[&expr]))
                    {
                        let slot = if self.defers_arm_values(expr) {
                            self.emit_selected_arm_values(expr, name)
                        } else {
                            let mut slot = Rope::new();
                            slot.push_relocated_operand(name.clone(), start, extent);
                            slot
                        };
                        out.anchored(kind, start, head_end, extent, slot);
                    } else {
                        let _active = self.enter_active(expr);
                        out.append(self.emit_expr(expr));
                    }
                }
            }
            cursor = span.end;
        }
        if cursor < source.end {
            out.append(self.source_range_rope(hir::Span {
                start: cursor,
                end: source.end,
            }));
        }
        self.active_capture_sources.borrow_mut().pop();
        self.rebuilt_sources.borrow_mut().pop();
        out
    }

    fn value_order(&self) -> &(crate::span_index::NestedOrder, Vec<ExprId>) {
        self.value_order.get_or_init(|| {
            let mut exprs: Vec<ExprId> = self.value_slots.keys().copied().collect();
            exprs.sort_unstable_by_key(|expr| expr.index());
            let order = crate::span_index::NestedOrder::new(exprs.iter().map(|expr| {
                let (_, start, _, extent) = self.value_anchor(*expr);
                (start, extent)
            }));
            (order, exprs)
        })
    }

    fn statement_order(&self) -> &(crate::span_index::NestedOrder, Vec<(usize, usize)>) {
        self.statement_order.get_or_init(|| {
            let mut spans = Vec::new();
            let mut places = Vec::new();
            for (body, statements) in self.core.bodies.iter().enumerate() {
                for (at, statement) in statements.statements.iter().enumerate() {
                    let node = match statement {
                        Statement::Decision(decision) => decision.extent,
                        Statement::Propagate(propagate) => propagate.owner,
                        Statement::Adt(adt) => adt.node,
                        _ => continue,
                    };
                    let span = self
                        .semantic
                        .hir
                        .source_map
                        .node_extent(node)
                        .expect("statement extent");
                    spans.push((span.start, span.end));
                    places.push((body, at));
                }
            }
            (crate::span_index::NestedOrder::new(spans), places)
        })
    }

    fn expressions_within(&self, source: SourceSpan) -> impl Iterator<Item = ExprId> + '_ {
        self.core.bodies.iter().flat_map(move |body| {
            body.statements.iter().filter_map(move |statement| {
                let Statement::Expr(expr) = statement else {
                    return None;
                };
                structured_expr_span(self.semantic, self.core, *expr)
                    .is_some_and(|span| source.start <= span.start && span.end <= source.end)
                    .then_some(*expr)
            })
        })
    }

    fn statements_within(
        &self,
        source: SourceSpan,
    ) -> impl Iterator<Item = (SourceSpan, &'a Statement)> + '_ {
        self.core.bodies.iter().flat_map(move |body| {
            body.statements.iter().filter_map(move |statement| {
                let node = match statement {
                    Statement::Decision(decision) => decision.extent,
                    Statement::Propagate(propagate) => propagate.owner,
                    Statement::Adt(adt) => adt.node,
                    _ => return None,
                };
                let span = SourceSpan::from(
                    self.semantic
                        .hir
                        .source_map
                        .node_extent(node)
                        .expect("statement extent"),
                );
                (source.start <= span.start && span.end <= source.end).then_some((span, statement))
            })
        })
    }

    /// [`Self::captured_reading`], with the authored text of a member
    /// callee read at its call kept as mapped source.
    fn captured_reading_rope(
        &self,
        step: &PlannedEvaluationStep,
        input: &PlannedEvaluationInput,
    ) -> Rope<'a> {
        let mut out = Rope::new();
        match input {
            PlannedEvaluationInput::Source {
                source,
                mode: EvaluationInputMode::MemberReference,
                receiver,
                key,
                ..
            } if !callee_tested_step(step) => {
                let mut cursor = source.start;
                for part in [*receiver, *key].into_iter().flatten() {
                    let PlannedReceiver::Captured { source: at, slot } = part else {
                        continue;
                    };
                    if cursor < at.start {
                        out.push_src(&self.source[cursor..at.start], cursor);
                    }
                    out.push_lit(self.value_slot_name(slot).to_owned());
                    cursor = at.end;
                }
                if cursor < source.end {
                    out.push_src(&self.source[cursor..source.end], cursor);
                }
            }
            PlannedEvaluationInput::Source { source, .. } => {
                out.push_relocated_operand(
                    self.captured_reading(step, input),
                    source.start,
                    source.end,
                );
            }
            _ => out.push_lit(self.captured_reading(step, input)),
        }
        out
    }

    /// The text that reads a captured input where its source stood: its
    /// slot, or, for a method, the method bound to its receiver
    /// (`bound_callee`).
    fn captured_reading(
        &self,
        step: &PlannedEvaluationStep,
        input: &PlannedEvaluationInput,
    ) -> String {
        match input {
            PlannedEvaluationInput::Source {
                source,
                mode: EvaluationInputMode::MemberReference,
                receiver,
                key,
                ..
            } if !callee_tested_step(step) => {
                member_callee(self.source, *source, [*receiver, *key], |slot| {
                    self.value_slot_name(slot)
                })
            }
            PlannedEvaluationInput::Source {
                source,
                target,
                mode: EvaluationInputMode::ShorthandProperty,
                ..
            } => format!(
                "{}: {}",
                &self.source[source.start..source.end],
                self.value_slot_name(*target)
            ),
            PlannedEvaluationInput::Source { target, .. } => {
                self.value_slot_name(*target).to_owned()
            }
            PlannedEvaluationInput::Slot { slot, .. } => self.value_slot_name(*slot).to_owned(),
            PlannedEvaluationInput::Stable { source, .. } => {
                self.source[source.start..source.end].to_owned()
            }
        }
    }

    pub(super) fn source_range_with_value_slots(
        &self,
        span: SourceSpan,
        values: &[ExprId],
    ) -> Rope<'a> {
        self.source_range_with_scheduled_values(span, values, &[], &[])
    }

    pub(super) fn scheduled_steps_within(
        &self,
        expr: ExprId,
        span: SourceSpan,
    ) -> Vec<&PlannedEvaluationStep> {
        self.nested_schedules
            .get(&expr)
            .map(|schedule| {
                schedule
                    .steps()
                    .iter()
                    .take_while(|step| {
                        span.start <= step.parent.start && step.parent.end <= span.end
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(super) fn source_range_with_scheduled_values(
        &self,
        span: SourceSpan,
        values: &[ExprId],
        steps: &[&PlannedEvaluationStep],
        operations: &[&PlannedConditionalOperation],
    ) -> Rope<'a> {
        self.source_range_with_scheduled_values_and(span, values, steps, operations, &[])
    }

    pub(super) fn source_range_with_scheduled_values_and(
        &self,
        span: SourceSpan,
        values: &[ExprId],
        steps: &[&PlannedEvaluationStep],
        operations: &[&PlannedConditionalOperation],
        parts: &[(SourceSpan, String)],
    ) -> Rope<'a> {
        let mut replacements: Vec<(SourceSpan, Rope<'a>)> = parts
            .iter()
            .map(|(source, name)| {
                let mut rendered = Rope::new();
                rendered.push_lit(name.clone());
                (*source, rendered)
            })
            .collect();
        replacements.extend(operations.iter().map(|operation| {
            let primary = operation
                .values
                .first()
                .copied()
                .unwrap_or_else(|| crate::ice::bug!("conditional operation has no value"));
            let (kind, start, head_end, extent) = self.value_anchor(primary);
            let mut slot = Rope::new();
            slot.push_lit(self.value_slot_name(operation.result).to_owned());
            let mut rendered = Rope::new();
            rendered.anchored(kind, start, head_end, extent, slot);
            (operation.parent, rendered)
        }));
        let discarded: Vec<SourceSpan> = steps
            .iter()
            .flat_map(|step| &step.inputs)
            .filter_map(|input| match input {
                PlannedEvaluationInput::Source {
                    source,
                    mode: EvaluationInputMode::Discarded,
                    ..
                } => Some(*source),
                _ => None,
            })
            .collect();
        replacements.extend(values.iter().filter_map(|expr| {
            let (kind, start, head_end, extent) = self.value_anchor(*expr);
            let source = SourceSpan { start, end: extent };
            if discarded
                .iter()
                .any(|operand| operand.start <= source.start && source.end <= operand.end)
            {
                return None;
            }
            if self.discarded_values.contains(expr) {
                return Some((source, Rope::new()));
            }
            let covered = self
                .replacements_covering(source.start, source.end)
                .any(|captured| {
                    captured.anchor.is_none()
                        && span.start <= captured.source.start
                        && captured.source.start <= source.start
                        && source.end <= captured.source.end
                        && captured.source != source
                });
            (!covered).then(|| {
                let mut slot = Rope::new();
                slot.push_relocated_operand(self.value_name_of(*expr).to_owned(), start, extent);
                let mut rendered = Rope::new();
                rendered.anchored(kind, start, head_end, extent, slot);
                (source, rendered)
            })
        }));
        replacements.extend(steps.iter().flat_map(|step| {
            step.inputs.iter().flat_map(move |input| match input {
                PlannedEvaluationInput::Source {
                    source,
                    target,
                    mode: EvaluationInputMode::CompoundAssignmentTarget { operator },
                    ..
                } => {
                    let mut rendered = Rope::new();
                    rendered.push_lit(format!("= {} {operator}", self.value_slot_name(*target)));
                    vec![(
                        compound_assignment_operator(self.source, *source, operator),
                        rendered,
                    )]
                }
                PlannedEvaluationInput::Source {
                    source,
                    mode: EvaluationInputMode::Discarded,
                    ..
                } => vec![
                    (*source, Rope::new()),
                    (discarded_operand_comma(self.source, *source), Rope::new()),
                ],
                PlannedEvaluationInput::Source { source, .. } => {
                    let rendered = self.captured_reading_rope(step, input);
                    vec![(*source, rendered)]
                }
                PlannedEvaluationInput::Slot { .. } | PlannedEvaluationInput::Stable { .. } => {
                    Vec::new()
                }
            })
        }));
        let mut nested: Vec<(SourceSpan, Option<&Statement>, Option<ExprId>)> = self
            .statements_within(span)
            .map(|(source, statement)| (source, Some(statement), None))
            .collect();
        let within = |inner: SourceSpan, outer: SourceSpan| {
            inner != outer && outer.start <= inner.start && inner.end <= outer.end
        };
        nested.extend(self.expressions_within(span).filter_map(|expr| {
            (!values.contains(&expr) && self.core.expr_requires_host(expr))
                .then(|| structured_expr_span(self.semantic, self.core, expr))
                .flatten()
                .filter(|source| {
                    !replacements
                        .iter()
                        .any(|(replaced, _)| within(*replaced, *source))
                })
                .map(|source| (source, None, Some(expr)))
        }));
        let statements: Vec<_> = nested
            .iter()
            .filter(|(source, ..)| {
                !replacements
                    .iter()
                    .any(|(replaced, _)| *replaced == *source || within(*source, *replaced))
                    && !nested.iter().any(|(outer, ..)| within(*source, *outer))
            })
            .map(|(source, statement, expr)| {
                let rendered = match (statement, expr) {
                    (Some(statement), _) => self.emit_statements(std::slice::from_ref(*statement)),
                    (None, Some(expr)) => self.emit_expr(*expr),
                    (None, None) => Rope::new(),
                };
                (*source, rendered)
            })
            .collect();
        replacements.extend(statements);
        replacements.extend(self.piped_value_at(span.start).map(|piped| {
            (
                SourceSpan {
                    start: span.start,
                    end: span.start,
                },
                piped,
            )
        }));
        replacements.retain(|(source, _)| span.start <= source.start && source.end <= span.end);
        replacements.sort_by_key(|(source, _)| (source.start, usize::MAX - source.end));
        self.rebuilt_sources.borrow_mut().push(span);
        let mut out = Rope::new();
        let mut cursor = span.start;
        for (source, rendered) in replacements {
            if source.start < cursor {
                continue;
            }
            if cursor < source.start {
                out.append(self.source_range_rope(hir::Span::new(cursor, source.start)));
            }
            out.append(rendered);
            cursor = source.end;
        }
        if cursor < span.end {
            out.append(self.source_range_rope(hir::Span::new(cursor, span.end)));
        }
        self.rebuilt_sources.borrow_mut().pop();
        out
    }

    fn composed_operand(
        &self,
        operation: &PlannedConditionalOperation,
        span: SourceSpan,
        values: &[ExprId],
    ) -> Rope<'a> {
        let steps: Vec<_> = operation
            .active
            .iter()
            .filter(|active| values.contains(&active.value))
            .flat_map(|active| &active.steps)
            .collect();
        self.source_range_with_scheduled_values(span, &operation.values, &steps, &[])
    }

    /// One rebuilt argument of an optional call.
    pub(super) fn push_operand(
        &self,
        operation: &PlannedConditionalOperation,
        operand: &PlannedOperand,
        out: &mut Rope<'a>,
    ) {
        match operand {
            PlannedOperand::Value(expr) => {
                out.push_lit(self.value_name_of(*expr).to_owned());
            }
            PlannedOperand::Composed {
                spread,
                capture: Some(slot),
                ..
            } => {
                if *spread {
                    out.push_lit("...");
                }
                out.push_lit(self.value_slot_name(*slot).to_owned());
            }
            PlannedOperand::Composed {
                span,
                spread,
                values,
                capture: None,
            } => {
                if *spread {
                    out.push_lit("...");
                }
                out.append(self.composed_operand(operation, *span, values));
            }
            PlannedOperand::Source {
                spread,
                capture: Some(slot),
                ..
            } => {
                if *spread {
                    out.push_lit("...");
                }
                out.push_lit(self.value_slot_name(*slot).to_owned());
            }
            PlannedOperand::Source {
                span,
                spread,
                capture: None,
            } => {
                if *spread {
                    out.push_lit("...");
                }
                out.append(self.captured_tail(*span, &HashSet::new()));
            }
        }
    }

    /// Captures the condition (or callee) of a conditional operation and
    /// returns the name the region tests and calls. A member callee an
    /// optional call tests keeps its receiver in a slot of its own — the
    /// rebuilt call goes through `.call(receiver, ...)`, so no `.bind` is
    /// written.
    pub(super) fn emit_condition_capture(
        &self,
        condition: &PlannedEvaluationInput,
        captured: &mut HashSet<crate::evaluation_ir::ValueSlotId>,
        out: &mut Rope<'a>,
    ) -> String {
        match condition {
            PlannedEvaluationInput::Slot { slot, .. } => self.value_slot_name(*slot).to_owned(),
            // An inert condition needs no capture; re-reading it in each
            // branch is unobservable and yields the same value.
            PlannedEvaluationInput::Stable { source, .. } => {
                format!("({})", &self.source[source.start..source.end])
            }
            PlannedEvaluationInput::Source {
                source,
                target,
                receiver: Some(receiver),
                mode: EvaluationInputMode::MemberReference,
                ..
            } => {
                if captured.insert(*target) {
                    let receiver_source = self.capture_planned_receiver(receiver, captured, out);
                    out.push_value_capture(self.value_slot_name(*target));
                    if source.start < receiver_source.start {
                        out.push_src(
                            &self.source[source.start..receiver_source.start],
                            source.start,
                        );
                    }
                    self.push_planned_receiver(receiver, true, out);
                    if receiver_source.end < source.end {
                        out.push_src(
                            &self.source[receiver_source.end..source.end],
                            receiver_source.end,
                        );
                    }
                    out.push_lit(");");
                    out.push_break(0);
                }
                self.value_slot_name(*target).to_owned()
            }
            PlannedEvaluationInput::Source { source, target, .. } => {
                if captured.insert(*target) {
                    self.push_capture(self.value_slot_name(*target), *source, out);
                    out.append(self.captured_source(*source, captured));
                    out.push_lit(");");
                    out.push_break(0);
                }
                self.value_slot_name(*target).to_owned()
            }
        }
    }

    fn condition_test(
        &self,
        condition: &PlannedEvaluationInput,
        captured: &HashSet<crate::evaluation_ir::ValueSlotId>,
    ) -> Rope<'a> {
        let mut out = Rope::new();
        match condition {
            PlannedEvaluationInput::Slot { slot, .. } => {
                out.push_lit(self.value_slot_name(*slot).to_owned());
            }
            PlannedEvaluationInput::Source { target, .. } if captured.contains(target) => {
                out.push_lit(self.value_slot_name(*target).to_owned());
            }
            PlannedEvaluationInput::Source {
                mode: EvaluationInputMode::MemberReference,
                ..
            } => crate::ice::bug!("a condition was planned as a member callee"),
            PlannedEvaluationInput::Source { source, .. } => {
                out.append(self.captured_source(*source, captured));
            }
            PlannedEvaluationInput::Stable { source, .. } => {
                out.push_lit("(");
                out.push_src(&self.source[source.start..source.end], source.start);
                out.push_lit(")");
            }
        }
        out
    }

    fn condition_operand(
        &self,
        condition: &PlannedEvaluationInput,
        captured: &mut HashSet<crate::evaluation_ir::ValueSlotId>,
        out: &mut Rope<'a>,
    ) -> (Rope<'a>, [Rope<'a>; 2]) {
        match condition {
            PlannedEvaluationInput::Source {
                source,
                target,
                mode,
                ..
            } if !captured.contains(target) => {
                if *mode == EvaluationInputMode::MemberReference {
                    crate::ice::bug!("a condition was planned as a member callee")
                }
                let name = self.value_slot_name(*target);
                out.push_operand_declaration(name);
                out.push_break(0);
                let mut test = Rope::new();
                test.push_lit(format!("{name} = "));
                push_grouped(
                    &mut test,
                    self.named_as_written(*source, self.captured_source(*source, captured)),
                    self.source_kind,
                );
                captured.insert(*target);
                let left = || {
                    let mut left = Rope::new();
                    left.push_lit(name.to_owned());
                    left
                };
                (test, [left(), left()])
            }
            _ => (
                self.condition_test(condition, captured),
                [
                    self.condition_test(condition, captured),
                    self.condition_test(condition, captured),
                ],
            ),
        }
    }

    pub(super) fn capture_planned_receiver(
        &self,
        receiver: &PlannedReceiver,
        captured: &mut HashSet<crate::evaluation_ir::ValueSlotId>,
        out: &mut Rope<'a>,
    ) -> SourceSpan {
        match *receiver {
            PlannedReceiver::Captured { source, slot } => {
                if captured.insert(slot) {
                    // A receiver retains its inferred members. The `this`
                    // parameter at a later bind is not its contextual type.
                    let name = self.value_slot_name(slot);
                    match self.type_query(source) {
                        Some(query) => {
                            out.push_lit(format!("const {name}: "));
                            out.append(query);
                            out.push_lit(" = (");
                        }
                        None => out.push_lit(format!("const {name} = (")),
                    }
                    out.append(self.captured_source(source, captured));
                    out.push_lit(");");
                    out.push_break(0);
                }
                source
            }
            PlannedReceiver::Stable { source } | PlannedReceiver::ThisOfSuper { source } => source,
        }
    }

    pub(super) fn push_planned_receiver(
        &self,
        receiver: &PlannedReceiver,
        mapped: bool,
        out: &mut Rope<'a>,
    ) {
        match *receiver {
            PlannedReceiver::Captured { slot, .. } => {
                out.push_lit(self.value_slot_name(slot).to_owned());
            }
            PlannedReceiver::Stable { source } => {
                let text = &self.source[source.start..source.end];
                if mapped {
                    out.push_src(text, source.start);
                } else {
                    out.push_lit(text.to_owned());
                }
            }
            PlannedReceiver::ThisOfSuper { source } if mapped => {
                out.push_src(&self.source[source.start..source.end], source.start);
            }
            PlannedReceiver::ThisOfSuper { .. } => out.push_lit("this"),
        }
    }

    pub(super) fn piped_value_at(&self, position: usize) -> Option<Rope<'a>> {
        let (expr, step) = *self
            .piped_steps
            .get_or_init(|| {
                let mut steps = HashMap::new();
                for (index, expr) in self.core.exprs.iter().enumerate() {
                    crate::work::tick("piped step lookups");
                    let Expr::Apply(apply) = expr else {
                        continue;
                    };
                    if apply.head.is_none() {
                        continue;
                    }
                    let id = ExprId::new(index);
                    let Some(piped) = self.piped_slots.get(&id) else {
                        continue;
                    };
                    let mut seen = HashSet::new();
                    for (step, planned) in apply.steps.iter().enumerate() {
                        if !matches!(planned.mode, ApplyMode::Postfix { .. }) {
                            continue;
                        }
                        let start = self.span(planned.node).start;
                        if seen.insert(start) && piped.get(step).is_some() {
                            steps.entry(start).or_insert((id, step));
                        }
                    }
                }
                steps
            })
            .get(&position)?;
        let Expr::Apply(apply) = &self.core.exprs[expr.index()] else {
            crate::ice::bug!("a piped step belongs to an application");
        };
        Some(self.pipe_input(apply, step, &self.piped_slots[&expr][step]))
    }

    pub(super) fn pipe_input(&self, apply: &Apply, step: usize, piped: &str) -> Rope<'a> {
        let end = apply.steps.last().map_or_else(
            || self.span(apply.node).end,
            |step| self.span(step.node).end,
        );
        let produced = step.checked_sub(1).map_or_else(
            || self.span(apply.node),
            |previous| self.span(apply.steps[previous].node),
        );
        let step_span = self.span(apply.steps[step].node);
        let mut input = Rope::new();
        input.push_lit(piped.to_owned());
        let mut out = Rope::new();
        out.anchored_with_context(
            AnchorKind::Pipe,
            step_span.start,
            step_span.end,
            end,
            Some((produced.start, produced.end)),
            input,
        );
        out
    }

    /// The join slot name of a tt value, from the plan.
    pub(super) fn value_name_of(&self, expr: ExprId) -> &str {
        self.value_slots
            .get(&expr)
            .map(String::as_str)
            .unwrap_or_else(|| crate::ice::bug!("conditional operation value has no slot"))
    }

    /// Closes the block a compose rewrite opened for a concise arrow body.
    ///
    /// The block is opened once, by [`Self::emit_compose_rewrite`], and ends
    /// where the arrow body's own source range ends. The structured-value
    /// path reaches that point when the value *is* the whole body; otherwise
    /// the source walk reaches it while emitting the rest of the body. Both
    /// ask here, and the first one to arrive writes the brace.
    pub(super) fn emit_compose_suffix(&self, rewrite: &ComposeRewrite) -> Rope<'a> {
        debug_assert_eq!(rewrite.owner_kind, HostOwnerKind::ArrowExpression);
        if !self.closed_compose_blocks.claim(rewrite.owner) {
            return Rope::new();
        }
        let mut out = Rope::new();
        out.push_lit(";");
        out.push_break(0);
        out.push_lit("}");
        out.push_scope_close();
        out
    }

    pub(super) fn emit_scheduled_step(
        &self,
        step: &PlannedEvaluationStep,
        action: Rope<'a>,
        captured: &mut CapturedSlots,
    ) -> Rope<'a> {
        crate::stack::grow(|| self.emit_scheduled_step_grown(step, action, captured))
    }

    fn emit_scheduled_step_grown(
        &self,
        step: &PlannedEvaluationStep,
        action: Rope<'a>,
        captured: &mut CapturedSlots,
    ) -> Rope<'a> {
        let optional_reference = matches!(
            step.operation,
            HostEvaluationOperation::Conditional(ConditionalBranch::OptionalCallArgument(_))
        );
        // A call skipped at its callee's member link reads the callee only
        // past the receiver's test, as the chain does.
        let receiver_test = optional_reference
            && step
                .conditional
                .as_ref()
                .and_then(|facts| facts.optional_test)
                == Some(OptionalCallTest::Receiver);
        let mut prefix = Rope::new();
        let mut guarded = Rope::new();
        let (skipped, unwritten) = captured.unwritten(&step.inputs);
        crate::work::tick_by("scheduled input visits", unwritten.len());
        for (index, input) in (skipped..).zip(unwritten) {
            let PlannedEvaluationInput::Source {
                source,
                mode,
                target,
                receiver,
                key,
            } = input
            else {
                continue;
            };
            if !captured.insert(*target) {
                continue;
            }
            if *mode == EvaluationInputMode::MemberReference && !callee_tested_step(step) {
                // The reference's parts run before the arguments; the member
                // is read at the call (`member_callee`). A receiver an
                // optional call tests is read before its test, and a key
                // only once the test passed, as the chain evaluates them.
                if let Some(receiver) = receiver {
                    self.capture_planned_receiver(receiver, captured, &mut prefix);
                }
                if let Some(key) = key {
                    let at = if receiver_test && index == 0 {
                        &mut guarded
                    } else {
                        &mut prefix
                    };
                    self.capture_planned_receiver(key, captured, at);
                }
            } else if *mode == EvaluationInputMode::MemberReference {
                let receiver = receiver
                    .unwrap_or_else(|| crate::ice::bug!("member reference has no receiver"));
                let receiver_source =
                    self.capture_planned_receiver(&receiver, captured, &mut prefix);
                let mut callee = Rope::new();
                callee.push_lit(format!("let {} = (", self.value_slot_name(*target)));
                if source.start < receiver_source.start {
                    callee.append(self.captured_source(
                        SourceSpan {
                            start: source.start,
                            end: receiver_source.start,
                        },
                        captured,
                    ));
                }
                self.push_planned_receiver(&receiver, true, &mut callee);
                if receiver_source.end < source.end {
                    callee.append(self.captured_tail(
                        SourceSpan {
                            start: receiver_source.end,
                            end: source.end,
                        },
                        captured,
                    ));
                }
                let target_name = self.value_slot_name(*target);
                callee.push_lit(");");
                callee.push_break(0);
                callee.push_lit(format!("if ({target_name} != null) {{"));
                callee.push_break(1);
                callee.push_lit(format!("{target_name} = {target_name}.bind("));
                self.push_planned_receiver(&receiver, false, &mut callee);
                callee.push_lit(");");
                callee.push_break(0);
                callee.push_lit("}");
                callee.push_break(0);
                prefix.append(callee);
            } else {
                if let EvaluationInputMode::CompoundAssignmentTarget { .. } = mode {
                    prefix.push_lit(format!("let {} = (", self.value_slot_name(*target)));
                } else if *mode == EvaluationInputMode::Discarded {
                    prefix.push_lit("(");
                } else if matches!(
                    mode,
                    EvaluationInputMode::Value | EvaluationInputMode::DirectReference
                ) {
                    self.push_capture(self.value_slot_name(*target), *source, &mut prefix);
                } else {
                    prefix.push_value_capture(self.value_slot_name(*target));
                }
                let (open, close) = self.capture_form(*mode);
                prefix.push_lit(open);
                prefix.append(
                    self.named_as_written(*source, self.captured_source(*source, captured)),
                );
                prefix.push_lit(close);
                prefix.push_lit(");");
                prefix.push_break(0);
            }
        }
        match step.operation {
            HostEvaluationOperation::Eager(_) | HostEvaluationOperation::Suspend(_) => {
                prefix.append(action);
                prefix
            }
            HostEvaluationOperation::Reference(_) => {
                prefix.append(action);
                prefix
            }
            HostEvaluationOperation::Conditional(branch) => {
                let input = step
                    .inputs
                    .first()
                    .unwrap_or_else(|| crate::ice::bug!("conditional schedule has no condition"));
                let condition = match input {
                    PlannedEvaluationInput::Source { target, .. }
                    | PlannedEvaluationInput::Slot { slot: target, .. } => {
                        self.value_slot_name(*target).to_owned()
                    }
                    PlannedEvaluationInput::Stable { source, .. } => {
                        format!("({})", &self.source[source.start..source.end])
                    }
                };
                let condition = condition.as_str();
                prefix.push_lit("if (");
                match branch {
                    ConditionalBranch::LogicalAndRight | ConditionalBranch::Consequent => {
                        prefix.push_lit(condition.to_owned());
                    }
                    ConditionalBranch::OptionalCallArgument(_) => {
                        let test = step
                            .conditional
                            .as_ref()
                            .and_then(|facts| facts.optional_test);
                        let receiver = match input {
                            PlannedEvaluationInput::Source {
                                mode: EvaluationInputMode::MemberReference,
                                receiver: Some(receiver),
                                ..
                            } => Some(*receiver),
                            _ => None,
                        };
                        match (test, receiver) {
                            (Some(OptionalCallTest::Callee), _) => {
                                prefix.push_lit(format!("{condition} != null"));
                            }
                            (Some(OptionalCallTest::Receiver), Some(receiver)) => {
                                prefix.push_lit(format!(
                                    "{} != null",
                                    self.planned_receiver_text(&receiver)
                                ));
                            }
                            _ => crate::ice::bug!(
                                "an optional call's schedule has no test for its short-circuit"
                            ),
                        }
                    }
                    ConditionalBranch::LogicalOrRight | ConditionalBranch::Alternate => {
                        prefix.push_lit(format!("!({condition})"));
                    }
                    ConditionalBranch::NullishRight => {
                        prefix.push_lit(format!("{condition} == null"));
                    }
                    ConditionalBranch::LogicalAssignmentRight { .. } => {
                        crate::ice::bug!(
                            "a logical assignment reached step-level conditional emission"
                        )
                    }
                }
                prefix.push_lit(") {");
                prefix.push_break(1);
                guarded.append(action);
                prefix.append(Rope::indented(1, guarded));
                prefix.push_break(0);
                prefix.push_lit("}");
                prefix.push_break(0);
                prefix
            }
            HostEvaluationOperation::LoopTest => {
                crate::ice::bug!("loop-test schedule reached ordinary step emission")
            }
        }
    }

    pub(super) fn value_slot_name(&self, slot: crate::evaluation_ir::ValueSlotId) -> &str {
        self.scheduled_slots
            .get(&slot)
            .map(String::as_str)
            .unwrap_or_else(|| crate::ice::bug!("scheduled value slot has no generated name"))
    }

    /// The slot that holds `expr`'s value. A sequence has the slot of the
    /// value it consists of; a sequence that computes something else from
    /// its values (`f(match ...)`) has none, since that slot holds the
    /// inner value.
    pub(super) fn structured_value_slot(&self, expr: ExprId) -> Option<&String> {
        crate::stack::grow(|| self.structured_value_slot_grown(expr))
    }

    fn structured_value_slot_grown(&self, expr: ExprId) -> Option<&String> {
        self.value_slots.get(&expr).or_else(|| {
            let Expr::Sequence(body) = &self.core.exprs[expr.index()] else {
                return None;
            };
            self.grouped_sequence_value(*body)
                .and_then(|value| self.structured_value_slot(value))
        })
    }

    /// The one value a sequence consists of, apart from trivia and the
    /// parentheses that group it.
    fn grouped_sequence_value(&self, body: hir::BodyId) -> Option<ExprId> {
        let statements = &self.core.bodies[body.index()].statements;
        let index = statements
            .iter()
            .position(|statement| matches!(statement, Statement::Expr(_)))?;
        let Statement::Expr(value) = statements[index] else {
            return None;
        };
        let only = |statements: &[Statement], paren: u8| {
            statements.iter().all(|statement| {
                let Statement::Opaque(node) = statement else {
                    return false;
                };
                let span = self.span(*node);
                let bytes = self.source.as_bytes();
                let mut at = span.start;
                loop {
                    at = crate::scanner::skip_ws_comments(bytes, at, span.end);
                    if at == span.end {
                        return true;
                    }
                    if bytes[at] != paren {
                        return false;
                    }
                    at += 1;
                }
            })
        };
        (only(&statements[..index], b'(') && only(&statements[index + 1..], b')')).then_some(value)
    }

    /// The slot of a structured value owned by the active structural parent.
    /// A sequence may wrap that value in grouping source. A value hosted by a
    /// nested function also has a slot, but is deliberately absent from
    /// `nested_values`; its own host rewrite must consume that slot instead.
    pub(super) fn nested_structured_value_slot(&self, expr: ExprId) -> Option<&String> {
        crate::stack::grow(|| self.nested_structured_value_slot_grown(expr))
    }

    fn nested_structured_value_slot_grown(&self, expr: ExprId) -> Option<&String> {
        if !self.core.has_statement_form(expr) {
            return None;
        }
        if self.structurally_nested_values.contains(&expr)
            && !matches!(self.core.exprs[expr.index()], Expr::ResultRegion(_))
        {
            return self.structured_value_slot(expr);
        }
        let Expr::Sequence(body) = &self.core.exprs[expr.index()] else {
            return None;
        };
        self.core
            .body_tail_expr(*body)
            .and_then(|value| self.nested_structured_value_slot(value))
    }

    pub(super) fn value_anchor(&self, expr: ExprId) -> (AnchorKind, usize, usize, usize) {
        crate::stack::grow(|| self.value_anchor_grown(expr))
    }

    fn value_anchor_grown(&self, expr: ExprId) -> (AnchorKind, usize, usize, usize) {
        crate::work::tick("value anchors");
        match &self.core.exprs[expr.index()] {
            Expr::Decision(decision) => {
                let head = self.span(decision.head);
                let extent = self.span(decision.extent);
                (AnchorKind::Match, head.start, head.end, extent.end)
            }
            Expr::Propagate(propagate) => {
                let span = self.span(propagate.node);
                (AnchorKind::Try, span.start, span.end, span.end)
            }
            Expr::ResultRegion(region) => {
                let (start, end) = self.result_bind_anchor(region);
                (AnchorKind::Result, start, end, end)
            }
            Expr::Sequence(body) => {
                let value = self
                    .core
                    .body_tail_expr(*body)
                    .unwrap_or_else(|| crate::ice::bug!("slotted sequence has no value"));
                self.value_anchor(value)
            }
            Expr::Apply(apply) => {
                let start = self.span(apply.node).start;
                let end = apply.steps.last().map_or_else(
                    || self.span(apply.node).end,
                    |step| self.span(step.node).end,
                );
                (AnchorKind::Pipe, start, end, end)
            }
            Expr::Template(template) => template
                .parts
                .iter()
                .find_map(|part| match part {
                    TemplatePart::Interpolation(inner) if self.core.has_statement_form(*inner) => {
                        Some(self.value_anchor(*inner))
                    }
                    TemplatePart::Raw(_) | TemplatePart::Interpolation(_) => None,
                })
                .unwrap_or_else(|| crate::ice::bug!("unstructured template owns a join slot")),
            Expr::Opaque(_) => crate::ice::bug!("unstructured expression owns a join slot"),
        }
    }

    pub(super) fn result_bind_anchor(&self, region: &ResultRegion) -> (usize, usize) {
        let span = self.span(region.node);
        (span.start, span.end)
    }

    pub(super) fn emit_arrow_return_rewrite(&self, rewrite: &ArrowReturnRewrite) -> Rope<'a> {
        let anchored = self
            .emit_continued_expr(rewrite.expr, &ValueContinuation::assign(&rewrite.slot))
            .unwrap_or_else(|| {
                crate::ice::bug!("arrow return rewrite is not structurally emit-able")
            });
        let mut out = Rope::new();
        out.push_lit("{");
        out.push_break(1);
        self.push_slot_declaration(
            &mut out,
            &rewrite.slot,
            rewrite.contextual_type,
            rewrite.contextual_type_awaited,
            rewrite.contextual_type_asserted,
        );
        out.push_break(1);
        out.append(Rope::indented(1, anchored));
        out.push_break(1);
        out.push_lit(format!("return {};", rewrite.slot));
        out.push_break(0);
        out.push_lit("}");
        Rope::scoped(out)
    }

    fn type_query(&self, source: SourceSpan) -> Option<Rope<'a>> {
        let text = &self.source[source.start..source.end];
        crate::program_syntax::source_entity_name(text, self.source_kind).then(|| {
            let mut query = Rope::new();
            query.push_lit("typeof ");
            query.push_restatement(text);
            query
        })
    }

    fn push_capture(&self, name: &str, source: SourceSpan, out: &mut Rope<'a>) {
        match self.type_query(source) {
            Some(query) => {
                out.push_lit(format!("const {name}: "));
                out.append(query);
                out.push_lit(" = (");
            }
            None => out.push_value_capture(name),
        }
    }

    fn inside_claimed_frame(&self, span: SourceSpan, within: Option<SourceSpan>) -> bool {
        crate::work::tick("claimed frame queries");
        let low = within.map_or(0, |within| within.start);
        if low > span.start {
            return false;
        }
        self.claimed_frames
            .range(low..=span.start)
            .any(|(_, frames)| {
                let from = frames.partition_point(|&(end, _)| end < span.end);
                frames[from..]
                    .iter()
                    .take_while(|&&(end, _)| within.is_none_or(|within| end <= within.end))
                    .any(|&(_, index)| {
                        crate::work::tick("claimed frame checks");
                        !self.source_replacements[index]
                            .anchor
                            .is_some_and(|expr| self.active_structured_exprs.contains(expr))
                    })
            })
    }

    fn capture_form(&self, mode: EvaluationInputMode) -> (String, &'static str) {
        match mode {
            EvaluationInputMode::SpreadElement => {
                self.used_spread.set(true);
                (format!("{}(", self.spread_name), ")")
            }
            EvaluationInputMode::ObjectSpread => ("{ ...".to_owned(), " }"),
            EvaluationInputMode::TemplateSubstitution => ("`${".to_owned(), "}`"),
            _ => (String::new(), ""),
        }
    }
}

/// The steps of `steps` a value of a compose rewrite still has to write.
/// A run of steps an earlier value of the rewrite already wrote captured
/// every input it reads, so a later value sharing that run adds nothing by
/// writing it again, unless the run holds a conditional step, whose guard
/// each value is written under.
fn steps_to_emit<'s>(
    steps: &'s crate::chain::ChainSlice<PlannedEvaluationStep>,
    emitted: &mut HashMap<usize, (usize, bool)>,
) -> Vec<&'s PlannedEvaluationStep> {
    let mut fresh = Vec::new();
    let mut rest_conditional = false;
    for (identity, remaining, step) in steps.links() {
        if let Some(&(seen, conditional)) = emitted.get(&identity)
            && seen >= remaining
        {
            rest_conditional = conditional;
            break;
        }
        fresh.push((identity, remaining, step));
    }
    let mut conditional = rest_conditional;
    for &(identity, remaining, step) in fresh.iter().rev() {
        conditional |= matches!(step.operation, HostEvaluationOperation::Conditional(_));
        emitted.insert(identity, (remaining, conditional));
    }
    if rest_conditional {
        return steps.iter().collect();
    }
    fresh.into_iter().map(|(_, _, step)| step).collect()
}
