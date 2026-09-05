//! Host-owner, conditional-operation, and evaluation-schedule emission.

use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn emit_owner_slot_rewrite(&self, rewrite: &OwnerSlotRewrite) -> Rope<'a> {
        let _active = self.active_structured_exprs.enter(rewrite.expr);
        let anchored = self
            .emit_continued_expr(rewrite.expr, &ValueContinuation::assign(&rewrite.slot))
            .unwrap_or_else(|| {
                crate::ice::bug!("initializer rewrite is not structurally emit-able")
            });
        let mut out = Rope::new();
        out.push_lit(format!("let {}", rewrite.slot));
        self.push_contextual_type(
            &mut out,
            rewrite.contextual_type,
            rewrite.contextual_type_awaited,
        );
        out.push_lit(";");
        out.push_break(0);
        out.append(anchored);
        out.push_break(0);
        Rope::scoped(out)
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
        Rope::scoped(out)
    }

    pub(super) fn emit_for_initializer_payload(&self, propagate: &Propagate) -> Rope<'a> {
        let temp = temp_name(propagate.temporary);
        let mut out = Rope::new();
        if let Some(binding) = propagate.binding {
            out.push_lit(format!("{} ", binding_keyword(binding.mode)));
            out.append(self.source_rope(binding.node));
            out.push_lit(format!(" = {temp}.{};", propagate.layout.payload_field));
        }
        Rope::scoped(out)
    }

    pub(super) fn emit_compose_rewrite(&self, rewrite: &ComposeRewrite) -> Rope<'a> {
        let mut out = Rope::new();
        if rewrite.owner_kind == HostOwnerKind::ArrowExpression {
            out.push_lit("{");
            out.push_break(0);
        }
        for action in &rewrite.actions {
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
                        out.push_break(0);
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
                ComposeAction::Operation(operation) => self.value_slot_name(operation.result),
            };
            out.push_lit(format!("let {slot};"));
            out.push_break(0);
        }
        let mut captured = HashSet::new();
        for action in &rewrite.actions {
            match action {
                ComposeAction::Value(value) => {
                    if value.inline {
                        continue;
                    }
                    let _active = self.active_structured_exprs.enter(value.expr);
                    let mut lowered = if let Some(completion) = &value.call_completion {
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
                        region.append(
                            self.emit_continued_expr(
                                value.expr,
                                &ValueContinuation::invoke(
                                    &completion.invoke,
                                    completion.result.as_deref(),
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
                    for step in &value.steps {
                        lowered = self.emit_scheduled_step(step, lowered, &mut captured);
                    }
                    out.append(lowered);
                }
                ComposeAction::Operation(operation) => {
                    let mut lowered = self.emit_conditional_operation(operation, &mut captured);
                    for step in &operation.outer {
                        lowered = self.emit_scheduled_step(step, lowered, &mut captured);
                    }
                    out.append(lowered);
                }
            }
        }
        out.push_break(0);
        if rewrite.owner_kind == HostOwnerKind::ArrowExpression {
            out.push_lit("return ");
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
                ComposeAction::Operation(operation) => self.value_slot_name(operation.result),
            };
            out.push_lit(format!("let {slot};"));
            out.push_break(1);
        }
        self.loop_region_depth.set(self.loop_region_depth.get() + 1);
        let mut captured = HashSet::new();
        for action in &rewrite.actions {
            let lowered = match action {
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
        out.push_lit("if (!(");
        Rope::scoped(out)
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
        captured: &mut HashSet<crate::evaluation_ir::ValueSlotId>,
    ) -> Rope<'a> {
        self.conditional_region_depth
            .set(self.conditional_region_depth.get() + 1);
        let result = self.value_slot_name(operation.result);
        let mut out = Rope::new();
        let condition = self.emit_condition_capture(&operation.condition, captured, &mut out);
        let deliver_value = |expr: ExprId, target: &str| {
            self.emit_continued_expr(expr, &ValueContinuation::assign(target))
                .unwrap_or_else(|| {
                    crate::ice::bug!("conditional operation value is not structurally emit-able")
                })
        };
        let assign_condition = |out: &mut Rope<'a>| {
            out.push_lit(format!("{result} = {condition};"));
        };
        match &operation.kind {
            PlannedConditionalKind::LogicalAnd => {
                let value = operation.values[0];
                out.push_lit(format!("if ({condition}) {{"));
                out.push_break(1);
                out.append(Rope::indented(
                    1,
                    self.emit_conditional_active_branch(operation, value, result, captured),
                ));
                out.push_break(0);
                out.push_lit("} else {");
                out.push_break(1);
                assign_condition(&mut out);
                out.push_break(0);
                out.push_lit("}");
            }
            PlannedConditionalKind::LogicalOr => {
                let value = operation.values[0];
                out.push_lit(format!("if ({condition}) {{"));
                out.push_break(1);
                assign_condition(&mut out);
                out.push_break(0);
                out.push_lit("} else {");
                out.push_break(1);
                out.append(Rope::indented(
                    1,
                    self.emit_conditional_active_branch(operation, value, result, captured),
                ));
                out.push_break(0);
                out.push_lit("}");
            }
            PlannedConditionalKind::Nullish => {
                let value = operation.values[0];
                out.push_lit(format!("if ({condition} == null) {{"));
                out.push_break(1);
                out.append(Rope::indented(
                    1,
                    self.emit_conditional_active_branch(operation, value, result, captured),
                ));
                out.push_break(0);
                out.push_lit("} else {");
                out.push_break(1);
                assign_condition(&mut out);
                out.push_break(0);
                out.push_lit("}");
            }
            PlannedConditionalKind::Ternary {
                consequent,
                alternate,
            } => {
                let branch = |out: &mut Rope<'a>, content: &PlannedBranch| match content {
                    PlannedBranch::Value(expr) => {
                        out.append(Rope::indented(1, deliver_value(*expr, result)));
                    }
                    PlannedBranch::Source(span) => {
                        out.push_lit(format!("{result} = "));
                        let mut source = Rope::new();
                        source.push_src(&self.source[span.start..span.end], span.start);
                        push_grouped(out, source);
                        out.push_lit(";");
                    }
                };
                out.push_lit(format!("if ({condition}) {{"));
                out.push_break(1);
                branch(&mut out, consequent);
                out.push_break(0);
                out.push_lit("} else {");
                out.push_break(1);
                branch(&mut out, alternate);
                out.push_break(0);
                out.push_lit("}");
            }
            PlannedConditionalKind::OptionalCall {
                arguments,
                type_args,
            } => {
                out.push_lit(format!("if ({condition} != null) {{"));
                out.push_break(1);
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
                // A single whole-value argument with completable arms calls
                // the captured callee from each dispatch arm, keeping the
                // argument in the consumer's contextual position — the same
                // completion the plain-call path performs (TASK-327).
                if let [PlannedOperand::Value(expr)] = arguments.as_slice()
                    && type_args.is_none()
                    && completable_decision_arms(self.core, *expr, &self.exits_for_expr(*expr))
                {
                    let prefix = match receiver {
                        Some(receiver) => {
                            let mut text = format!("{condition}.call(");
                            match receiver {
                                PlannedReceiver::Captured { slot, .. } => {
                                    text.push_str(self.value_slot_name(slot));
                                }
                                PlannedReceiver::Stable { source } => {
                                    text.push_str(&self.source[source.start..source.end]);
                                }
                            }
                            text.push_str(", ");
                            text
                        }
                        None => format!("{condition}("),
                    };
                    let _active = self.active_structured_exprs.enter(*expr);
                    let body = self
                        .emit_continued_expr(
                            *expr,
                            &ValueContinuation::invoke(&prefix, Some(result)),
                        )
                        .unwrap_or_else(|| {
                            crate::ice::bug!("optional completed call lost its value decision")
                        });
                    out.append(Rope::indented(1, body));
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
                let mut body = Rope::new();
                for argument in arguments {
                    match argument {
                        PlannedOperand::Value(expr) => {
                            let name = self.value_name_of(*expr);
                            body.push_lit(format!("let {name};"));
                            body.push_break(0);
                            body.append(deliver_value(*expr, name));
                            body.push_break(0);
                        }
                        PlannedOperand::Source {
                            span,
                            capture: Some(slot),
                            ..
                        } => {
                            if captured.insert(*slot) {
                                body.push_lit(format!("const {} = (", self.value_slot_name(*slot)));
                                body.push_src(&self.source[span.start..span.end], span.start);
                                body.push_lit(");");
                                body.push_break(0);
                            }
                        }
                        PlannedOperand::Source { capture: None, .. } => {}
                    }
                }
                body.push_lit(format!("{result} = {condition}"));
                if let Some(span) = type_args {
                    body.push_src(&self.source[span.start..span.end], span.start);
                }
                match receiver {
                    Some(receiver) => {
                        body.push_lit(".call(");
                        self.push_planned_receiver(&receiver, false, &mut body);
                        for argument in arguments {
                            body.push_lit(", ");
                            self.push_operand(argument, &mut body);
                        }
                        body.push_lit(");");
                    }
                    None => {
                        body.push_lit("(");
                        for (index, argument) in arguments.iter().enumerate() {
                            if index > 0 {
                                body.push_lit(", ");
                            }
                            self.push_operand(argument, &mut body);
                        }
                        body.push_lit(");");
                    }
                }
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

    pub(super) fn emit_conditional_active_branch(
        &self,
        operation: &PlannedConditionalOperation,
        value: ExprId,
        result: &str,
        captured: &mut HashSet<crate::evaluation_ir::ValueSlotId>,
    ) -> Rope<'a> {
        let Some(branch) = operation.active_branch else {
            return self
                .emit_continued_expr(value, &ValueContinuation::assign(result))
                .unwrap_or_else(|| {
                    crate::ice::bug!("conditional operation value is not structurally emit-able")
                });
        };
        let value_slot = self.value_name_of(value);
        let mut out = Rope::new();
        out.push_lit(format!("let {value_slot};"));
        out.push_break(0);
        let mut lowered = self
            .emit_continued_expr(value, &ValueContinuation::assign(value_slot))
            .unwrap_or_else(|| {
                crate::ice::bug!("conditional branch value is not structurally emit-able")
            });
        for step in &operation.active_steps {
            lowered = self.emit_scheduled_step(step, lowered, captured);
        }
        out.append(lowered);
        out.push_break(0);
        out.push_lit(format!("{result} = "));
        push_grouped(
            &mut out,
            self.source_range_with_value_slots(branch, &operation.values),
        );
        out.push_lit(";");
        out
    }

    pub(super) fn source_range_with_value_slots(
        &self,
        span: SourceSpan,
        values: &[ExprId],
    ) -> Rope<'a> {
        let mut replacements: Vec<_> = values
            .iter()
            .map(|expr| {
                let (kind, start, head_end, extent) = self.value_anchor(*expr);
                (*expr, kind, SourceSpan { start, end: extent }, head_end)
            })
            .filter(|(_, _, value, _)| span.start <= value.start && value.end <= span.end)
            .collect();
        replacements.sort_unstable_by_key(|(_, _, value, _)| value.start);
        let mut out = Rope::new();
        let mut cursor = span.start;
        for (expr, kind, value, head_end) in replacements {
            if cursor < value.start {
                out.append(self.source_range_rope(hir::Span {
                    start: cursor,
                    end: value.start,
                }));
            }
            let mut slot = Rope::new();
            slot.push_lit(self.value_name_of(expr).to_owned());
            out.anchored(kind, value.start, head_end, value.end, slot);
            cursor = value.end;
        }
        if cursor < span.end {
            out.append(self.source_range_rope(hir::Span {
                start: cursor,
                end: span.end,
            }));
        }
        out
    }

    pub(super) fn source_range_with_nested_schedule(
        &self,
        span: SourceSpan,
        expr: ExprId,
        schedule: &EvaluationSchedule,
    ) -> Rope<'a> {
        let (kind, start, head_end, extent) = self.value_anchor(expr);
        let mut replacements: Vec<NestedSourceReplacement> = vec![(
            SourceSpan { start, end: extent },
            self.value_name_of(expr).to_owned(),
            Some((kind, head_end)),
        )];
        for step in schedule.steps() {
            for input in &step.inputs {
                if let PlannedEvaluationInput::Source { source, target, .. } = input {
                    replacements.push((*source, self.value_slot_name(*target).to_owned(), None));
                }
            }
        }
        replacements.retain(|(source, _, _)| span.start <= source.start && source.end <= span.end);
        replacements.sort_unstable_by_key(|(source, _, _)| source.start);
        let mut out = Rope::new();
        let mut cursor = span.start;
        for (source, slot, anchor) in replacements {
            if source.start < cursor {
                continue;
            }
            if cursor < source.start {
                out.append(self.source_range_rope(hir::Span::new(cursor, source.start)));
            }
            if let Some((kind, head_end)) = anchor {
                let mut rendered = Rope::new();
                rendered.push_lit(slot);
                out.anchored(kind, source.start, head_end, source.end, rendered);
            } else {
                out.push_lit(slot);
            }
            cursor = source.end;
        }
        if cursor < span.end {
            out.append(self.source_range_rope(hir::Span::new(cursor, span.end)));
        }
        out
    }

    /// One rebuilt argument of an optional call.
    pub(super) fn push_operand(&self, operand: &PlannedOperand, out: &mut Rope<'a>) {
        match operand {
            PlannedOperand::Value(expr) => {
                out.push_lit(self.value_name_of(*expr).to_owned());
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
                out.push_src(&self.source[span.start..span.end], span.start);
            }
        }
    }

    /// Captures the condition (or callee) of a conditional operation and
    /// returns the name the region tests and calls. A member callee keeps
    /// its receiver in a slot of its own — the rebuilt call goes through
    /// `.call(receiver, ...)`, so no `.bind` is written.
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
            PlannedEvaluationInput::Stable { source } => {
                format!("({})", &self.source[source.start..source.end])
            }
            PlannedEvaluationInput::Source {
                source,
                target,
                receiver: Some(receiver),
                mode: EvaluationInputMode::MemberReference,
            } => {
                if captured.insert(*target) {
                    let receiver_source = self.capture_planned_receiver(receiver, captured, out);
                    out.push_lit(format!("const {} = (", self.value_slot_name(*target)));
                    if source.start < receiver_source.start {
                        out.push_src(
                            &self.source[source.start..receiver_source.start],
                            source.start,
                        );
                    }
                    if !self.push_pipeline_member_reference(*source, receiver_source, receiver, out)
                    {
                        self.push_planned_receiver(receiver, true, out);
                        if receiver_source.end < source.end {
                            out.push_src(
                                &self.source[receiver_source.end..source.end],
                                receiver_source.end,
                            );
                        }
                    }
                    out.push_lit(");");
                    out.push_break(0);
                }
                self.value_slot_name(*target).to_owned()
            }
            PlannedEvaluationInput::Source { source, target, .. } => {
                if captured.insert(*target) {
                    out.push_lit(format!("const {} = (", self.value_slot_name(*target)));
                    out.push_src(&self.source[source.start..source.end], source.start);
                    out.push_lit(");");
                    out.push_break(0);
                }
                self.value_slot_name(*target).to_owned()
            }
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
                    out.push_lit(format!("const {} = (", self.value_slot_name(slot)));
                    out.push_src(&self.source[source.start..source.end], source.start);
                    out.push_lit(");");
                    out.push_break(0);
                }
                source
            }
            PlannedReceiver::Stable { source } => source,
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
        }
    }

    pub(super) fn push_pipeline_member_reference(
        &self,
        source: SourceSpan,
        receiver_source: SourceSpan,
        receiver: &PlannedReceiver,
        out: &mut Rope<'a>,
    ) -> bool {
        let suffix = self.core.exprs.iter().find_map(|expr| {
            let Expr::Apply(apply) = expr else {
                return None;
            };
            let head = apply.head?;
            let Expr::Opaque(head_node) = &self.core.exprs[head.index()] else {
                return None;
            };
            if SourceSpan::from(self.span(*head_node)) != receiver_source {
                return None;
            }
            apply.steps.iter().find_map(|step| {
                let ApplyMode::Postfix { optional: true } = step.mode else {
                    return None;
                };
                let step_span = SourceSpan::from(self.span(step.node));
                (source.start == receiver_source.start
                    && step_span.start < source.end
                    && source.end <= step_span.end)
                    .then_some(SourceSpan {
                        // Keep optional property access so a null receiver
                        // never reaches the member lookup. The explicit
                        // branch below controls argument evaluation.
                        start: step_span.start,
                        end: source.end,
                    })
            })
        });
        let Some(suffix) = suffix else {
            return false;
        };
        self.push_planned_receiver(receiver, true, out);
        out.push_src(&self.source[suffix.start..suffix.end], suffix.start);
        true
    }

    /// The join slot name of a tt value, from the plan.
    pub(super) fn value_name_of(&self, expr: ExprId) -> &str {
        self.value_slots
            .get(&expr)
            .map(String::as_str)
            .unwrap_or_else(|| crate::ice::bug!("conditional operation value has no slot"))
    }

    pub(super) fn emit_compose_suffix(&self, rewrite: &ComposeRewrite) -> Rope<'a> {
        debug_assert_eq!(rewrite.owner_kind, HostOwnerKind::ArrowExpression);
        let mut out = Rope::new();
        out.push_lit(";");
        out.push_break(0);
        out.push_lit("}");
        Rope::scoped(out)
    }

    pub(super) fn emit_scheduled_step(
        &self,
        step: &PlannedEvaluationStep,
        action: Rope<'a>,
        captured: &mut HashSet<crate::evaluation_ir::ValueSlotId>,
    ) -> Rope<'a> {
        let mut prefix = Rope::new();
        for input in &step.inputs {
            let PlannedEvaluationInput::Source {
                source,
                mode,
                target,
                receiver,
            } = input
            else {
                continue;
            };
            if !captured.insert(*target) {
                continue;
            }
            if *mode == EvaluationInputMode::MemberReference {
                let receiver = receiver
                    .unwrap_or_else(|| crate::ice::bug!("member reference has no receiver"));
                let receiver_source =
                    self.capture_planned_receiver(&receiver, captured, &mut prefix);
                let optional_reference = matches!(
                    step.operation,
                    HostEvaluationOperation::Conditional(ConditionalBranch::OptionalCallArgument(
                        _
                    ))
                );
                prefix.push_lit(format!(
                    "{} {} = (",
                    if optional_reference { "let" } else { "const" },
                    self.value_slot_name(*target)
                ));
                if source.start < receiver_source.start {
                    prefix.push_src(
                        &self.source[source.start..receiver_source.start],
                        source.start,
                    );
                }
                self.push_planned_receiver(&receiver, true, &mut prefix);
                if receiver_source.end < source.end {
                    prefix.push_src(
                        &self.source[receiver_source.end..source.end],
                        receiver_source.end,
                    );
                }
                if optional_reference {
                    let target_name = self.value_slot_name(*target);
                    prefix.push_lit(");");
                    prefix.push_break(0);
                    prefix.push_lit(format!("if ({target_name} != null) {{"));
                    prefix.push_break(1);
                    prefix.push_lit(format!("{target_name} = {target_name}.bind("));
                    self.push_planned_receiver(&receiver, false, &mut prefix);
                    prefix.push_lit(");");
                    prefix.push_break(0);
                    prefix.push_lit("}");
                    prefix.push_break(0);
                } else {
                    prefix.push_lit(").bind(");
                    self.push_planned_receiver(&receiver, false, &mut prefix);
                    prefix.push_lit(");");
                    prefix.push_break(0);
                }
            } else {
                prefix.push_lit(format!("const {} = (", self.value_slot_name(*target)));
                prefix.push_src(&self.source[source.start..source.end], source.start);
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
                    PlannedEvaluationInput::Stable { source } => {
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
                        prefix.push_lit(format!("{condition} != null"));
                    }
                    ConditionalBranch::LogicalOrRight | ConditionalBranch::Alternate => {
                        prefix.push_lit(format!("!({condition})"));
                    }
                    ConditionalBranch::NullishRight => {
                        prefix.push_lit(format!("{condition} == null"));
                    }
                }
                prefix.push_lit(") {");
                prefix.push_break(1);
                prefix.append(Rope::indented(1, action));
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

    pub(super) fn structured_value_slot(&self, expr: ExprId) -> Option<&String> {
        self.value_slots.get(&expr).or_else(|| {
            let Expr::Sequence(body) = &self.core.exprs[expr.index()] else {
                return None;
            };
            self.core
                .body_value_expr(*body)
                .and_then(|value| self.structured_value_slot(value))
        })
    }

    /// The slot of a structured value owned by the active structural parent.
    /// A sequence may wrap that value in grouping source. A value hosted by a
    /// nested function also has a slot, but is deliberately absent from
    /// `nested_values`; its own host rewrite must consume that slot instead.
    pub(super) fn nested_structured_value_slot(&self, expr: ExprId) -> Option<&String> {
        if self.structurally_nested_values.contains(&expr)
            && !matches!(self.core.exprs[expr.index()], Expr::ResultRegion(_))
        {
            return self.structured_value_slot(expr);
        }
        let Expr::Sequence(body) = &self.core.exprs[expr.index()] else {
            return None;
        };
        self.core
            .body_value_expr(*body)
            .and_then(|value| self.nested_structured_value_slot(value))
    }

    pub(super) fn value_anchor(&self, expr: ExprId) -> (AnchorKind, usize, usize, usize) {
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
                    .body_value_expr(*body)
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
        if rewrite.parenthesized {
            out.push_lit("(() => {");
        } else {
            out.push_lit("{");
        }
        out.push_break(1);
        out.push_lit(format!("let {}", rewrite.slot));
        self.push_contextual_type(
            &mut out,
            rewrite.contextual_type,
            rewrite.contextual_type_awaited,
        );
        out.push_lit(";");
        out.push_break(1);
        out.append(Rope::indented(1, anchored));
        out.push_break(1);
        out.push_lit(format!("return {};", rewrite.slot));
        out.push_break(0);
        out.push_lit(if rewrite.parenthesized { "})()" } else { "}" });
        Rope::scoped(out)
    }
}
