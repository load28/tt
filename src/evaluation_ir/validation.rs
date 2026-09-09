//! Evaluation IR ordering, reference, and structural validation.

use super::*;

impl EvaluationFile {
    pub(super) fn has_differently_hosted_descendant(
        &self,
        core: &CoreFile,
        expr: ExprId,
        owner: HostOwner,
    ) -> bool {
        self.expr_has_differently_hosted_descendant(core, expr, owner)
    }

    /// Whether `expr` structurally owns a nested value that needs statement
    /// lowering before the surrounding source expression can be evaluated.
    ///
    /// Region ancestry is the ownership boundary. A value below another
    /// `Host` region belongs to that host (for example a concise-arrow body)
    /// and must not make the outer Apply consume its rewrite. A value reached
    /// only through `Nested` regions belongs to the Apply and requires its
    /// statement form even when a sibling value belongs to another host.
    pub(super) fn has_owned_nested_statement_descendant(
        &self,
        core: &CoreFile,
        expr: ExprId,
    ) -> bool {
        let Some(ancestor) = self.regions.iter().position(|region| {
            region.root == Some(CoreRoot::Expr(expr))
                && matches!(region.placement, RegionPlacement::Host { .. })
        }) else {
            return false;
        };
        self.regions.iter().enumerate().any(|(index, region)| {
            let Some(CoreRoot::Expr(child)) = region.root else {
                return false;
            };
            child != expr
                && core.has_statement_form(child)
                && self.region_descends_from(index, ancestor)
        })
    }

    pub(super) fn region_descends_from(&self, mut region: usize, ancestor: usize) -> bool {
        while let RegionPlacement::Nested { parent, .. } = self.regions[region].placement {
            region = parent.0 as usize;
            if region == ancestor {
                return true;
            }
        }
        false
    }

    pub(super) fn expr_has_differently_hosted_descendant(
        &self,
        core: &CoreFile,
        expr: ExprId,
        owner: HostOwner,
    ) -> bool {
        let nested = |child| {
            self.regions.iter().any(|region| {
                region.root == Some(CoreRoot::Expr(child))
                    && matches!(
                        region.placement,
                        RegionPlacement::Host { host_owner, .. } if host_owner != owner
                    )
            }) || self.expr_has_differently_hosted_descendant(core, child, owner)
        };
        match &core.exprs[expr.index()] {
            Expr::Opaque(_) | Expr::Propagate(_) => false,
            Expr::Sequence(body) => self.body_has_differently_hosted_descendant(core, *body, owner),
            Expr::Decision(decision) => {
                decision
                    .subjects
                    .iter()
                    .any(|subject| nested(subject.value))
                    || decision.arms.iter().any(|arm| {
                        arm.guard.is_some_and(nested)
                            || match arm.action {
                                ArmAction::Yield { body, .. } | ArmAction::Execute(body) => {
                                    self.body_has_differently_hosted_descendant(core, body, owner)
                                }
                                ArmAction::BindThrough(_) => false,
                            }
                    })
            }
            Expr::Apply(apply) => {
                apply.head.is_some_and(nested) || apply.steps.iter().any(|step| nested(step.value))
            }
            Expr::ResultRegion(region) => {
                region.items.iter().any(|item| {
                    let ResultRegionItem::Statements(body) = item;
                    self.body_has_differently_hosted_descendant(core, *body, owner)
                }) || region.value.is_some_and(nested)
            }
            Expr::Template(template) => template.parts.iter().any(|part| match part {
                crate::core_ir::TemplatePart::Raw(_) => false,
                crate::core_ir::TemplatePart::Interpolation(expr) => nested(*expr),
            }),
        }
    }

    pub(super) fn body_has_differently_hosted_descendant(
        &self,
        core: &CoreFile,
        body: BodyId,
        owner: HostOwner,
    ) -> bool {
        core.bodies[body.index()]
            .statements
            .iter()
            .any(|statement| match statement {
                Statement::Expr(expr) => {
                    self.regions.iter().any(|region| {
                        region.root == Some(CoreRoot::Expr(*expr))
                            && matches!(
                                region.placement,
                                RegionPlacement::Host { host_owner, .. } if host_owner != owner
                            )
                    }) || self.expr_has_differently_hosted_descendant(core, *expr, owner)
                }
                Statement::Opaque(_)
                | Statement::Adt(_)
                | Statement::Import(_)
                | Statement::Propagate(_)
                | Statement::Decision(_) => false,
            })
    }

    /// Checks the plan's evaluation order and count contracts
    /// (`docs/design/program-lowering.md` §11, `validate_order`).
    ///
    /// The checks re-derive each contract from the plan itself rather than
    /// trusting [`target_capability`]'s decision: a bug in the decision is
    /// exactly what this stage exists to catch, and the mutation tests break
    /// the decision to prove it does.
    pub(crate) fn validate_order(&self, plan: &LoweringPlan) -> Result<(), LoweringError> {
        use crate::ice::{InternalCompilerError, Invariant, LoweringStage};
        let stage = LoweringStage::EvaluationOrder;
        for rewrite in plan.owners() {
            let operation_of: HashMap<_, _> = rewrite
                .operations
                .iter()
                .flat_map(|operation| operation.values.iter().map(move |expr| (*expr, operation)))
                .collect();
            let mut produced: HashSet<ValueSlotId> = HashSet::new();
            let mut last_start = None;
            // Capture spans in the order they reach the target. A span is
            // materialized once per owner, at its first occurrence in
            // emission order (the target dedups later occurrences), and the
            // target writes a value's steps outermost first because each
            // step wraps the accumulated action.
            let mut materialized: Vec<SourceSpan> = Vec::new();
            for value in &rewrite.values {
                let subject =
                    LoweringSubject::owner(rewrite.owner).with_root(CoreRoot::Expr(value.expr));
                if value.capability != TargetCapability::StatementRegion {
                    continue;
                }
                match value.context.owner_reach {
                    OwnerReach::Same => {}
                    OwnerReach::Repeated => {
                        if !value
                            .schedule
                            .steps()
                            .iter()
                            .any(|step| step.operation == HostEvaluationOperation::LoopTest)
                        {
                            return Err(InternalCompilerError::new(
                                stage,
                                Invariant::RepetitionRegionLeft,
                                subject,
                            )
                            .at(value.source));
                        }
                    }
                    OwnerReach::UnmodeledConditional => {
                        return Err(InternalCompilerError::new(
                            stage,
                            Invariant::EvaluationCountChanged,
                            subject,
                        )
                        .at(value.source));
                    }
                }
                if last_start.is_some_and(|start| value.source.start < start) {
                    return Err(InternalCompilerError::new(
                        stage,
                        Invariant::EvaluationOrderChanged,
                        subject,
                    )
                    .at(value.source));
                }
                last_start = Some(value.source.start);
                let steps = value.schedule.steps();
                for step in steps {
                    for input in &step.inputs {
                        if let PlannedEvaluationInput::Slot { slot, .. } = input
                            && !produced.contains(slot)
                        {
                            return Err(InternalCompilerError::new(
                                stage,
                                Invariant::ValueReadBeforeItIsProduced,
                                subject.with_slot(*slot),
                            )
                            .at(value.source));
                        }
                    }
                }
                for index in (0..steps.len()).rev() {
                    let step = &steps[index];
                    let conditional_after = steps[index + 1..].iter().any(|later| {
                        matches!(later.operation, HostEvaluationOperation::Conditional(_))
                    });
                    for input in &step.inputs {
                        let PlannedEvaluationInput::Source { source, target, .. } = input else {
                            continue;
                        };
                        if materialized.contains(source) {
                            continue;
                        }
                        if conditional_after {
                            if operation_of.contains_key(&value.expr) {
                                continue;
                            }
                            return Err(InternalCompilerError::new(
                                stage,
                                Invariant::ConditionalRegionLeft,
                                subject.with_slot(*target),
                            )
                            .at(*source));
                        }
                        for span in &self.tt_spans {
                            if overlaps(*source, *span)
                                && !(span.start <= value.source.start
                                    && value.source.end <= span.end)
                            {
                                return Err(InternalCompilerError::new(
                                    stage,
                                    Invariant::EvaluationCountChanged,
                                    subject.with_slot(*target),
                                )
                                .at(*source)
                                .with_origin(vec![*span]));
                            }
                        }
                        for earlier in &materialized {
                            if overlaps(*source, *earlier) {
                                return Err(InternalCompilerError::new(
                                    stage,
                                    Invariant::EvaluationCountChanged,
                                    subject.with_slot(*target),
                                )
                                .at(*source)
                                .with_origin(vec![*earlier]));
                            }
                            if source.end <= earlier.start {
                                return Err(InternalCompilerError::new(
                                    stage,
                                    Invariant::EvaluationOrderChanged,
                                    subject.with_slot(*target),
                                )
                                .at(*source)
                                .with_origin(vec![*earlier]));
                            }
                        }
                        materialized.push(*source);
                    }
                }
                let ValueTarget::Slot(slot) = value.target;
                produced.insert(slot);
                if let Some(operation) = operation_of.get(&value.expr)
                    && operation.values.first() == Some(&value.expr)
                {
                    produced.insert(operation.result);
                }
            }
        }
        Ok(())
    }

    /// Checks the plan's JavaScript reference contracts
    /// (`docs/design/program-lowering.md` §11, `validate_reference`).
    pub(crate) fn validate_reference(&self, plan: &LoweringPlan) -> Result<(), LoweringError> {
        use crate::ice::{InternalCompilerError, Invariant, LoweringStage};
        let stage = LoweringStage::EvaluationReference;
        for rewrite in plan.owners() {
            let operation_values: HashSet<ExprId> = rewrite
                .operations
                .iter()
                .flat_map(|operation| operation.values.iter().copied())
                .collect();
            for value in &rewrite.values {
                if value.capability != TargetCapability::StatementRegion {
                    continue;
                }
                let subject =
                    LoweringSubject::owner(rewrite.owner).with_root(CoreRoot::Expr(value.expr));
                for step in value.schedule.steps() {
                    let optional_argument = matches!(
                        step.operation,
                        HostEvaluationOperation::Conditional(
                            ConditionalBranch::OptionalCallArgument(_)
                        )
                    );
                    for input in &step.inputs {
                        match input {
                            PlannedEvaluationInput::Source {
                                mode: EvaluationInputMode::MemberReference,
                                receiver,
                                source,
                                target,
                            } => {
                                if receiver.is_none() {
                                    return Err(InternalCompilerError::new(
                                        stage,
                                        Invariant::ReceiverLost,
                                        subject.with_slot(*target),
                                    )
                                    .at(*source));
                                }
                                // A member callee of an optional call keeps
                                // its receiver only when the whole operation
                                // is a planned region calling through
                                // `.call(receiver, ...)`.
                                if optional_argument && !operation_values.contains(&value.expr) {
                                    return Err(InternalCompilerError::new(
                                        stage,
                                        Invariant::ReferenceModeUnsupported,
                                        subject.with_slot(*target),
                                    )
                                    .at(*source));
                                }
                            }
                            PlannedEvaluationInput::Slot {
                                mode: EvaluationInputMode::MemberReference,
                                slot,
                            } => {
                                return Err(InternalCompilerError::new(
                                    stage,
                                    Invariant::ReferenceDemoted,
                                    subject.with_slot(*slot),
                                )
                                .at(value.source));
                            }
                            PlannedEvaluationInput::Source { .. }
                            | PlannedEvaluationInput::Slot { .. }
                            | PlannedEvaluationInput::Stable { .. } => {}
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn validate(&self) -> Result<(), EvaluationError> {
        for region in &self.regions {
            if region.entry.0 as usize >= region.blocks.len() {
                return Err(EvaluationError::InvalidEntry { region: region.id });
            }
            let core_operations = region
                .blocks
                .iter()
                .flat_map(|block| &block.statements)
                .filter(|statement| {
                    matches!(statement, EvalStatement::Core(operation) if *operation == region.operation)
                })
                .count();
            if core_operations != 1 {
                return Err(EvaluationError::CoreOperationMismatch { region: region.id });
            }
            let mut reachable = HashSet::new();
            let mut states = HashSet::new();
            let mut pending = vec![(region.entry, false)];
            while let Some((block, produced_before)) = pending.pop() {
                if !states.insert((block, produced_before)) {
                    continue;
                }
                reachable.insert(block);
                let Some(current) = region.blocks.get(block.0 as usize) else {
                    return Err(EvaluationError::InvalidTarget {
                        region: region.id,
                        target: block,
                    });
                };
                let mut produced = produced_before;
                for statement in &current.statements {
                    if let EvalStatement::Produce(value) = statement {
                        if region.result != Some(*value) {
                            return Err(EvaluationError::UnexpectedResultDefinition {
                                region: region.id,
                                value: *value,
                            });
                        }
                        produced = true;
                    }
                }
                match &current.terminator {
                    EvalTerminator::Goto(target) => pending.push((*target, produced)),
                    EvalTerminator::Branch { success, failure } => {
                        pending.push((*success, produced));
                        pending.push((*failure, produced));
                    }
                    EvalTerminator::Switch { arms, fallback } => {
                        pending.extend(arms.iter().copied().map(|target| (target, produced)));
                        pending.push((*fallback, produced));
                    }
                    EvalTerminator::Complete if region.result.is_some() && !produced => {
                        return Err(EvaluationError::MissingResultDefinition {
                            region: region.id,
                            block,
                        });
                    }
                    EvalTerminator::Exit | EvalTerminator::Complete => {}
                }
            }
            for index in 0..region.blocks.len() {
                let block =
                    EvalBlockId(u32::try_from(index).map_err(|_| EvaluationError::IdOverflow)?);
                if !reachable.contains(&block) {
                    return Err(EvaluationError::UnreachableBlock {
                        region: region.id,
                        block,
                    });
                }
            }
            let _operation = region.operation;
            let _result = region.result;
        }
        Ok(())
    }
}
