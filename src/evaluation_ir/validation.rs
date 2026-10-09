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
        crate::stack::grow(|| self.expr_has_differently_hosted_descendant_grown(core, expr, owner))
    }

    fn expr_has_differently_hosted_descendant_grown(
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
        crate::stack::grow(|| self.body_has_differently_hosted_descendant_grown(core, body, owner))
    }

    fn body_has_differently_hosted_descendant_grown(
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
            let mut materialized: HashSet<SourceSpan> = HashSet::new();
            let mut roots: Vec<SourceSpan> = Vec::new();
            let mut read_checked: HashMap<usize, usize> = HashMap::new();
            let mut read_segments: HashSet<usize> = HashSet::new();
            let mut captured: HashMap<usize, (usize, bool)> = HashMap::new();
            let mut captured_segments: HashSet<usize> = HashSet::new();
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
                for (link, left, step) in steps.links() {
                    if read_checked
                        .get(&link)
                        .is_some_and(|checked| *checked >= left)
                    {
                        break;
                    }
                    for input in step.inputs.fresh(&mut read_segments) {
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
                    read_checked.insert(link, left);
                }
                let mut pending = Vec::new();
                let mut conditional_after = false;
                for (link, left, step) in steps.links() {
                    if let Some((done, conditional)) = captured.get(&link)
                        && *done >= left
                    {
                        conditional_after = *conditional;
                        break;
                    }
                    pending.push((link, left, step));
                }
                let mut complete = true;
                let mut suffix_conditional = conditional_after;
                for (link, left, step) in pending.into_iter().rev() {
                    let conditional =
                        matches!(step.operation, HostEvaluationOperation::Conditional(_));
                    let (segments, inputs) = step.inputs.unseen(&captured_segments);
                    for input in inputs {
                        let PlannedEvaluationInput::Source { source, target, .. } = input else {
                            continue;
                        };
                        if materialized.contains(source) {
                            continue;
                        }
                        if conditional_after {
                            if operation_of.contains_key(&value.expr) {
                                complete = false;
                                continue;
                            }
                            return Err(InternalCompilerError::new(
                                stage,
                                Invariant::ConditionalRegionLeft,
                                subject.with_slot(*target),
                            )
                            .at(*source));
                        }
                        let encloses_value = |span: &SourceSpan| {
                            span.start <= value.source.start && value.source.end <= span.end
                        };
                        let mut violations: Vec<(usize, SourceSpan)> = self
                            .tt_spans
                            .straddling(*source)
                            .into_iter()
                            .filter(|(_, span)| !encloses_value(span))
                            .collect();
                        if source.end > value.source.start {
                            violations.extend(self.tt_spans.within(*source).into_iter().filter(
                                |(_, span)| {
                                    overlaps(*source, *span)
                                        && span.end > value.source.start
                                        && !encloses_value(span)
                                },
                            ));
                        }
                        if let Some(&(_, span)) = violations.iter().min_by_key(|(at, _)| *at) {
                            {
                                return Err(InternalCompilerError::new(
                                    stage,
                                    Invariant::EvaluationCountChanged,
                                    subject.with_slot(*target),
                                )
                                .at(*source)
                                .with_origin(vec![span]));
                            }
                        }
                        while let Some(earlier) = roots.last().copied()
                            && earlier.end > source.start
                        {
                            // A completed child capture is a dependency of a later
                            // enclosing capture: emission reads its slot, not its source.
                            if plan.capture_depends_on(*target, earlier) {
                                roots.pop();
                                continue;
                            }
                            if overlaps(*source, earlier) {
                                return Err(InternalCompilerError::new(
                                    stage,
                                    Invariant::EvaluationCountChanged,
                                    subject.with_slot(*target),
                                )
                                .at(*source)
                                .with_origin(vec![earlier]));
                            }
                            return Err(InternalCompilerError::new(
                                stage,
                                Invariant::EvaluationOrderChanged,
                                subject.with_slot(*target),
                            )
                            .at(*source)
                            .with_origin(vec![earlier]));
                        }
                        materialized.insert(*source);
                        roots.push(*source);
                    }
                    if complete {
                        captured_segments.extend(segments);
                    }
                    conditional_after |= conditional;
                    suffix_conditional |= conditional;
                    if complete {
                        captured.insert(link, (left, suffix_conditional));
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
            let mut checked: [HashMap<usize, usize>; 2] = [HashMap::new(), HashMap::new()];
            let mut checked_segments: [HashSet<usize>; 4] = Default::default();
            for value in &rewrite.values {
                if value.capability != TargetCapability::StatementRegion {
                    continue;
                }
                let subject =
                    LoweringSubject::owner(rewrite.owner).with_root(CoreRoot::Expr(value.expr));
                let in_operation = operation_values.contains(&value.expr);
                for (link, left, step) in value.schedule.steps().links() {
                    if checked[usize::from(in_operation)]
                        .get(&link)
                        .is_some_and(|done| *done >= left)
                    {
                        break;
                    }
                    let optional_argument = matches!(
                        step.operation,
                        HostEvaluationOperation::Conditional(
                            ConditionalBranch::OptionalCallArgument(_)
                        )
                    );
                    let segments = &mut checked_segments
                        [usize::from(in_operation) * 2 + usize::from(optional_argument)];
                    for input in step.inputs.fresh(segments) {
                        match input {
                            PlannedEvaluationInput::Source {
                                mode: EvaluationInputMode::MemberReference,
                                receiver,
                                source,
                                target,
                                ..
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
                                // is a planned region making the call.
                                if optional_argument && !in_operation {
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
                    checked[usize::from(in_operation)].insert(link, left);
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
