//! Evaluation-file construction and lowering-plan assembly.

use super::*;

impl EvaluationFile {
    pub(crate) fn primary_source(&self) -> SourceSpan {
        self.tt_spans
            .iter()
            .copied()
            .min_by_key(|span| span.start)
            .expect("EvaluationFile has at least one tt source span")
    }

    pub(crate) fn build(syntax: &ProgramSyntax, core: &CoreFile) -> Result<Self, EvaluationError> {
        let declared_owners: HashSet<HostOwner> =
            syntax.owners().map(|owner| owner.owner).collect();
        let mut hosts = HashMap::new();
        for (root, syntax_id, context, protocol, source, host_owner, exits) in
            syntax.core_contexts()
        {
            if !declared_owners.contains(&host_owner) {
                return Err(EvaluationError::InvalidHostOwner {
                    root,
                    owner: host_owner,
                    value: source,
                });
            }
            if hosts
                .insert(
                    root,
                    HostBinding {
                        syntax: syntax_id,
                        context,
                        protocol,
                        source,
                        owner: host_owner,
                        exits,
                    },
                )
                .is_some()
            {
                return Err(EvaluationError::DuplicateHost { root });
            }
        }
        let mut builder = EvaluationBuilder {
            core,
            hosts,
            regions: Vec::new(),
            seen: HashSet::new(),
            next_value: 0,
            nested_owners: HashMap::new(),
            enclosing: HashMap::new(),
        };
        builder.walk_body(core.root, None)?;
        if let Some(root) = builder.hosts.keys().copied().next() {
            return Err(EvaluationError::OrphanHost { root });
        }
        let file = Self {
            regions: builder.regions,
            occupied_names: syntax.occupied_names().map(str::to_owned).collect(),
            declared_names: syntax.declared_names(),
            module_declared_names: syntax.module_declared_names(),
            if_tests: syntax.if_tests().to_vec(),
            directive_prologue_end: syntax.directive_prologue_end(),
            tt_spans: syntax
                .core_contexts()
                .map(|(_, _, _, _, source, _, _)| source)
                .collect(),
            script: syntax.is_script(),
            commonjs: syntax.uses_commonjs_syntax(),
            globals: syntax.globals().clone(),
        };
        file.validate()?;
        Ok(file)
    }

    fn host_anchor(&self, region: &EvalRegion) -> Option<SourceSpan> {
        let mut region = region;
        loop {
            match &region.placement {
                RegionPlacement::Host { host_owner, .. } => return Some(host_owner.statement()),
                RegionPlacement::Nested { parent, .. } => {
                    region = &self.regions[parent.0 as usize];
                }
                RegionPlacement::SourceEdit => return None,
            }
        }
    }

    pub(crate) fn lowering_plan(&self, core: &CoreFile) -> Result<LoweringPlan, EvaluationError> {
        let mut owners: HashMap<HostOwner, Vec<PendingPlannedValue>> = HashMap::new();
        for region in &self.regions {
            let Some(CoreRoot::Propagate(_)) = region.root else {
                continue;
            };
            let RegionPlacement::Host {
                context, source, ..
            } = &region.placement
            else {
                continue;
            };
            if context.owner_reach == OwnerReach::Repeated {
                return Err(EvaluationError::RepeatedPropagation { source: *source });
            }
        }
        for region in &self.regions {
            let Some(CoreRoot::Expr(expr)) = region.root else {
                continue;
            };
            let (owner, value, context, protocol, exits) = match &region.placement {
                RegionPlacement::Host {
                    context:
                        context @ EvaluationContext {
                            continuation: HostContinuation::Return,
                            ..
                        },
                    source,
                    host_owner,
                    protocol,
                    exits,
                    ..
                } => (
                    *host_owner,
                    *source,
                    *context,
                    protocol.clone(),
                    exits.clone(),
                ),
                RegionPlacement::Host {
                    source,
                    host_owner,
                    context,
                    protocol,
                    exits,
                    ..
                } => (
                    *host_owner,
                    *source,
                    *context,
                    protocol.clone(),
                    exits.clone(),
                ),
                RegionPlacement::Nested { .. } | RegionPlacement::SourceEdit => continue,
            };
            // A pipeline remains an expression only when a nested value has
            // crossed into a different source-backed owner, such as a
            // concise arrow step. A hosted value in the same owner (for
            // example a match in the pipeline head) is part of the Apply's
            // own statement form and must be planned with it.
            if matches!(core.exprs[expr.index()], Expr::Apply(_))
                && self.has_differently_hosted_descendant(core, expr, owner)
                && !self.has_owned_nested_statement_descendant(core, expr)
            {
                continue;
            }
            if region.result.is_none() {
                continue;
            }
            if owner.span.start > value.start || value.end > owner.span.end {
                return Err(EvaluationError::InvalidHostOwner {
                    root: CoreRoot::Expr(expr),
                    owner,
                    value,
                });
            }
            owners.entry(owner).or_default().push(PendingPlannedValue {
                expr,
                source: value,
                context,
                protocol,
                exits,
            });
        }
        let mut owners: Vec<_> = owners
            .into_iter()
            .map(|(owner, mut values)| {
                values.sort_unstable_by_key(|value| value.source.start);
                (owner, values)
            })
            .collect();
        owners.sort_unstable_by_key(|(owner, _)| owner.span.start);
        // The heads of the let-else and `if let` statements each owner
        // hosts: the bounds of the values their subjects contain.
        let mut statement_decisions: HashMap<HostOwner, Vec<SourceSpan>> = HashMap::new();
        for region in &self.regions {
            let Some(CoreRoot::Decision(_)) = region.root else {
                continue;
            };
            let source = match &region.placement {
                RegionPlacement::Host { source, .. } => *source,
                RegionPlacement::Nested {
                    source: Some(source),
                    ..
                } => *source,
                RegionPlacement::Nested { source: None, .. } | RegionPlacement::SourceEdit => {
                    continue;
                }
            };
            let mut host = region;
            let owner = loop {
                match &host.placement {
                    RegionPlacement::Host { host_owner, .. } => break Some(*host_owner),
                    RegionPlacement::Nested { parent, .. } => {
                        host = &self.regions[parent.0 as usize];
                    }
                    RegionPlacement::SourceEdit => break None,
                }
            };
            if let Some(owner) = owner {
                statement_decisions.entry(owner).or_default().push(source);
            }
        }
        let nested_propagation_sources: Vec<SourceSpan> = self
            .regions
            .iter()
            .filter_map(|region| match (region.root, &region.placement) {
                (
                    Some(CoreRoot::Expr(expr)),
                    RegionPlacement::Nested {
                        source: Some(source),
                        ..
                    },
                ) if matches!(core.exprs[expr.index()], Expr::Propagate(_)) => Some(*source),
                _ => None,
            })
            .collect();
        let mut statement_decision_sources: Vec<SourceSpan> =
            statement_decisions.values().flatten().copied().collect();
        statement_decision_sources.sort_unstable_by_key(|span| (span.start, span.end));
        let mut next_slot = 0u32;
        let mut occupied_names = self.occupied_names.clone();
        let mut slot_names = Vec::new();
        let mut slot_anchors: Vec<Option<SourceSpan>> = Vec::new();
        let mut value_slots = HashMap::new();
        let mut capture_dependencies = HashMap::new();
        let mut rewrites = Vec::with_capacity(owners.len());
        let mut structurally_owned_children = HashSet::new();
        let mut owned_child_schedules = Vec::new();
        let mut owned_child_exits = Vec::new();
        let mut nested_operations = Vec::new();
        let mut unsupported_owned_children = Vec::new();
        for (owner, values) in owners {
            let assigned = values
                .into_iter()
                .map(|value| {
                    // A host value always crosses the Core/TypeScript boundary through a
                    // named join slot. A return still owns its original TypeScript return
                    // statement; the slot merely makes every Core exit converge before that
                    // statement consumes the value. Besides avoiding expression wrappers,
                    // this preserves the checker's contextual type for the value as a whole.
                    let slot =
                        allocate_value_slot(&mut next_slot, &mut slot_names, &mut occupied_names)?;
                    value_slots.insert(value.expr, slot);
                    let target = ValueTarget::Slot(slot);
                    Ok((value, target))
                })
                .collect::<Result<Vec<_>, EvaluationError>>()?;
            let slots: HashMap<_, _> = assigned
                .iter()
                .filter(|(value, _)| core.has_statement_form(value.expr))
                .map(|(value, target)| match target {
                    ValueTarget::Slot(slot) => (value.source, *slot),
                })
                .collect();
            let mut source_slots = HashMap::new();
            let values = assigned
                .into_iter()
                .map(|(value, target)| {
                    let schedule = resolve_schedule(
                        value.protocol,
                        &self.tt_spans,
                        &slots,
                        &mut source_slots,
                        &mut next_slot,
                        &mut slot_names,
                        &mut occupied_names,
                    )?;
                    let capability = target_capability(
                        core,
                        &self.tt_spans,
                        value.expr,
                        value.source,
                        &value.context,
                        &schedule,
                    );
                    Ok(PlannedValue {
                        expr: value.expr,
                        source: value.source,
                        target,
                        context: value.context,
                        schedule,
                        exits: value.exits,
                        capability,
                    })
                })
                .collect::<Result<Vec<_>, EvaluationError>>()?;
            for (source, slot) in &source_slots {
                capture_dependencies.insert(
                    slot.target,
                    source_slots
                        .iter()
                        .filter(|(child, dependency)| {
                            **child != *source
                                && source.start <= child.start
                                && child.end <= source.end
                                && dependency.target.0 < slot.target.0
                        })
                        .map(|(child, dependency)| (*child, dependency.target))
                        .collect(),
                );
            }
            let mut values = values;
            // A statement-capable outer Core value owns same-host tt values
            // lexically nested inside it. Its structural emitter evaluates
            // those children at their exact position and writes their
            // already allocated slots; planning the children again as
            // sibling owner actions would either run them twice or consume
            // authored syntax that still contains the outer construct. A
            // let-else or `if let` owns the values in its subject the same
            // way: it evaluates the subject into its own temporary.
            let outers: Vec<SourceSpan> = values
                .iter()
                .filter(|outer| outer.capability == TargetCapability::StatementRegion)
                .map(|outer| outer.source)
                .chain(
                    statement_decisions
                        .get(&owner)
                        .into_iter()
                        .flatten()
                        .copied(),
                )
                .collect();
            let owned_children: HashSet<_> = values
                .iter()
                .filter(|child| {
                    outers.iter().any(|outer| {
                        outer.start <= child.source.start
                            && child.source.end <= outer.end
                            && (outer.start < child.source.start || child.source.end < outer.end)
                    })
                })
                .map(|child| child.expr)
                .collect();
            structurally_owned_children.extend(owned_children.iter().copied());
            owned_child_exits.extend(
                values
                    .iter()
                    .filter(|value| owned_children.contains(&value.expr) && !value.exits.is_empty())
                    .map(|value| (value.expr, value.exits.clone())),
            );
            let mut owned_groups: Vec<(SourceSpan, Vec<PlannedValue>)> = Vec::new();
            for child in values
                .iter()
                .filter(|value| owned_children.contains(&value.expr))
            {
                let Some(outer) = outers
                    .iter()
                    .copied()
                    .chain(nested_propagation_sources.iter().copied())
                    .filter(|outer| {
                        *outer != child.source
                            && outer.start <= child.source.start
                            && child.source.end <= outer.end
                    })
                    .min_by_key(|outer| outer.end - outer.start)
                else {
                    continue;
                };
                let steps: Vec<_> = child
                    .schedule
                    .steps()
                    .iter()
                    .take_while(|step| {
                        outer.start <= step.parent.start
                            && step.parent.end <= outer.end
                            && step.parent != outer
                    })
                    .cloned()
                    .collect();
                let schedule = EvaluationSchedule {
                    steps,
                    call_completion: None,
                };
                let capability = target_capability(
                    core,
                    &self.tt_spans,
                    child.expr,
                    child.source,
                    &child.context,
                    &schedule,
                );
                let owned = PlannedValue {
                    schedule,
                    capability,
                    ..child.clone()
                };
                match owned_groups.iter_mut().find(|(group, _)| *group == outer) {
                    Some((_, group)) => group.push(owned),
                    None => owned_groups.push((outer, vec![owned])),
                }
            }
            for (_, mut group) in owned_groups {
                let operations = plan_conditional_operations(
                    &mut group,
                    &self.tt_spans,
                    &mut next_slot,
                    &mut slot_names,
                    &mut occupied_names,
                )?;
                let consumed: HashSet<_> = operations
                    .iter()
                    .flat_map(|operation| operation.values.iter().copied())
                    .collect();
                for child in group {
                    if let TargetCapability::ExpressionBoundary(reason) = child.capability {
                        unsupported_owned_children.push((
                            child.expr,
                            child.source,
                            child.context.owner,
                            reason,
                        ));
                    } else if !consumed.contains(&child.expr) && !child.schedule.steps.is_empty() {
                        owned_child_schedules.push((child.expr, child.schedule));
                    }
                }
                nested_operations.extend(operations);
            }
            values.retain(|value| !owned_children.contains(&value.expr));
            let operations = plan_conditional_operations(
                &mut values,
                &self.tt_spans,
                &mut next_slot,
                &mut slot_names,
                &mut occupied_names,
            )?;
            // A later host value may consume an earlier conditional
            // operation as one ordered input. Once that operation has a
            // join slot, depend on the slot rather than trying to capture
            // its original source (which contains tt syntax by definition).
            let operation_slots: HashMap<_, _> = operations
                .iter()
                .map(|operation| (operation.parent, operation.result))
                .collect();
            for value in &mut values {
                let mut changed = false;
                for step in &mut value.schedule.steps {
                    for input in &mut step.inputs {
                        let PlannedEvaluationInput::Source { source, mode, .. } = *input else {
                            continue;
                        };
                        let Some(slot) = operation_slots.get(&source).copied() else {
                            continue;
                        };
                        *input = PlannedEvaluationInput::Slot { slot, mode };
                        changed = true;
                    }
                }
                if changed {
                    value.capability = target_capability(
                        core,
                        &self.tt_spans,
                        value.expr,
                        value.source,
                        &value.context,
                        &value.schedule,
                    );
                }
            }
            slot_anchors.resize(slot_names.len(), Some(owner.statement()));
            rewrites.push(HostRewrite {
                owner,
                values,
                operations,
            });
        }
        for region in &self.regions {
            let Some(CoreRoot::Expr(expr)) = region.root else {
                continue;
            };
            if region.result.is_none() || value_slots.contains_key(&expr) {
                continue;
            }
            let slot = allocate_value_slot(&mut next_slot, &mut slot_names, &mut occupied_names)?;
            slot_anchors.resize(slot_names.len(), self.host_anchor(region));
            value_slots.insert(expr, slot);
        }
        let mut pipelines: Vec<_> = value_slots
            .iter()
            .filter_map(|(expr, slot)| match &core.exprs[expr.index()] {
                Expr::Apply(apply) if apply.head.is_some() && core.has_statement_form(*expr) => {
                    Some((*slot, *expr, apply.steps.len()))
                }
                _ => None,
            })
            .collect();
        pipelines.sort_unstable_by_key(|(slot, ..)| slot.0);
        let mut piped_slots = HashMap::new();
        for (_, expr, steps) in pipelines {
            let slots = (0..steps)
                .map(|_| allocate_value_slot(&mut next_slot, &mut slot_names, &mut occupied_names))
                .collect::<Result<Vec<_>, _>>()?;
            slot_anchors.resize(slot_names.len(), None);
            piped_slots.insert(expr, slots);
        }
        let nested_sources: HashMap<_, _> = self
            .regions
            .iter()
            .filter_map(|region| {
                let Some(CoreRoot::Expr(expr)) = region.root else {
                    return None;
                };
                let RegionPlacement::Nested {
                    source: Some(source),
                    ..
                } = region.placement
                else {
                    return None;
                };
                value_slots.get(&expr).map(|slot| (source, *slot))
            })
            .collect();
        let mut nested_source_slots = HashMap::new();
        let mut nested_schedules: HashMap<_, _> = owned_child_schedules.into_iter().collect();
        let mut nested_groups: Vec<(SourceSpan, Option<SourceSpan>, Vec<PlannedValue>)> =
            Vec::new();
        let planned_sources: HashMap<_, _> = rewrites
            .iter()
            .flat_map(|owner| owner.values.iter().map(|value| (value.expr, value.source)))
            .collect();
        for region in &self.regions {
            let Some(CoreRoot::Expr(expr)) = region.root else {
                continue;
            };
            if !matches!(
                core.exprs[expr.index()],
                Expr::ResultRegion(_) | Expr::Propagate(_)
            ) {
                continue;
            }
            let RegionPlacement::Nested {
                parent,
                protocol,
                source,
                exits,
                ..
            } = &region.placement
            else {
                continue;
            };
            let mut ancestor = &self.regions[parent.0 as usize];
            let planned_boundary = loop {
                if let Some(CoreRoot::Expr(parent_expr)) = ancestor.root
                    && let Some(source) = planned_sources.get(&parent_expr)
                {
                    break Some(*source);
                }
                if let (Some(CoreRoot::Decision(_)), RegionPlacement::Host { source, .. }) =
                    (ancestor.root, &ancestor.placement)
                {
                    break Some(*source);
                }
                let RegionPlacement::Nested { parent, .. } = ancestor.placement else {
                    break None;
                };
                ancestor = &self.regions[parent.0 as usize];
            };
            let mut host = ancestor;
            while let RegionPlacement::Nested { parent, .. } = host.placement {
                host = &self.regions[parent.0 as usize];
            }
            let step_count = planned_boundary.map_or(protocol.steps().len(), |boundary| {
                protocol
                    .steps()
                    .iter()
                    .take_while(|step| {
                        boundary.start <= step.parent.start
                            && step.parent.end <= boundary.end
                            && step.parent != boundary
                    })
                    .count()
            });
            if step_count == 0 {
                continue;
            }
            let schedule = resolve_schedule_steps(
                &protocol.steps()[..step_count],
                Elision {
                    tt_spans: &self.tt_spans,
                    reserve_names: false,
                },
                &nested_sources,
                &mut nested_source_slots,
                &mut next_slot,
                &mut slot_names,
                &mut occupied_names,
            )?;
            slot_anchors.resize(slot_names.len(), self.host_anchor(region));
            if let (
                Expr::Propagate(_),
                Some(boundary),
                Some(source),
                RegionPlacement::Host { context, .. },
                Some(slot),
            ) = (
                &core.exprs[expr.index()],
                planned_boundary,
                source,
                &host.placement,
                value_slots.get(&expr),
            ) {
                let capability =
                    target_capability(core, &self.tt_spans, expr, *source, context, &schedule);
                let value = PlannedValue {
                    expr,
                    source: *source,
                    target: ValueTarget::Slot(*slot),
                    context: *context,
                    schedule,
                    exits: exits.clone(),
                    capability,
                };
                match nested_groups
                    .iter_mut()
                    .find(|(group, ..)| *group == boundary)
                {
                    Some((.., group)) => group.push(value),
                    None => nested_groups.push((boundary, self.host_anchor(region), vec![value])),
                }
                continue;
            }
            nested_schedules.insert(expr, schedule);
        }
        for (_, anchor, mut group) in nested_groups {
            let operations = plan_conditional_operations(
                &mut group,
                &self.tt_spans,
                &mut next_slot,
                &mut slot_names,
                &mut occupied_names,
            )?;
            slot_anchors.resize(slot_names.len(), anchor);
            let consumed: HashSet<_> = operations
                .iter()
                .flat_map(|operation| operation.values.iter().copied())
                .collect();
            for value in group {
                if let TargetCapability::ExpressionBoundary(reason) = value.capability {
                    unsupported_owned_children.push((
                        value.expr,
                        value.source,
                        value.context.owner,
                        reason,
                    ));
                } else if !consumed.contains(&value.expr) {
                    nested_schedules.insert(value.expr, value.schedule);
                }
            }
            nested_operations.extend(operations);
        }
        let direct_capabilities: HashMap<_, _> = rewrites
            .iter()
            .flat_map(|rewrite| &rewrite.values)
            .map(|value| (value.expr, value.capability))
            .collect();
        for region in &self.regions {
            let Some(CoreRoot::Expr(expr)) = region.root else {
                continue;
            };
            if !matches!(core.exprs[expr.index()], Expr::ResultRegion(_)) {
                continue;
            }
            let RegionPlacement::Host {
                context, source, ..
            } = &region.placement
            else {
                continue;
            };
            if context.continuation == HostContinuation::Discard {
                return Err(EvaluationError::DiscardedResult { source: *source });
            }
        }
        let mut unsupported_expression_propagations = Vec::new();
        for region in &self.regions {
            let Some(CoreRoot::Expr(expr)) = region.root else {
                continue;
            };
            let Expr::Propagate(propagate) = &core.exprs[expr.index()] else {
                continue;
            };
            let exits_result = matches!(propagate.exit, ExitTarget::ResultRegion(_));
            let mut host_region = region;
            let mut covered_by_parent_propagation = false;
            while let RegionPlacement::Nested { parent, .. } = host_region.placement {
                host_region = &self.regions[parent.0 as usize];
                covered_by_parent_propagation |= host_region.root.is_some_and(|root| {
                    matches!(root, CoreRoot::Propagate(_))
                        || matches!(root, CoreRoot::Expr(parent_expr)
                            if matches!(core.exprs[parent_expr.index()], Expr::Propagate(_)))
                });
            }
            let RegionPlacement::Host { context, .. } = &host_region.placement else {
                continue;
            };
            if let Some(CoreRoot::Expr(host_expr)) = host_region.root
                && matches!(core.exprs[host_expr.index()], Expr::ResultRegion(_))
            {
                // The current Result language rejects value-form `try` in
                // semantic analysis. Its Result-owned diagnostic is the
                // one public result; do not add a second host-capability
                // error after the projection has supplied the inner host.
                continue;
            }
            let capability = match host_region.root {
                Some(CoreRoot::Expr(host_expr)) => direct_capabilities
                    .get(&host_expr)
                    .copied()
                    .unwrap_or(TargetCapability::StatementRegion),
                _ => TargetCapability::StatementRegion,
            };
            // A `try` that exits to its `result` is written as a statement of
            // its own host owner; only an owner that takes no statements has
            // nowhere to write it.
            if exits_result
                && capability == TargetCapability::StatementRegion
                && !matches!(
                    context.owner,
                    EvaluationOwner::ParameterInitializer
                        | EvaluationOwner::ClassInitializer
                        | EvaluationOwner::ClassDefinition
                        | EvaluationOwner::EnumInitializer
                )
            {
                continue;
            }
            let reason = match (context.owner, capability) {
                (EvaluationOwner::FunctionBody, TargetCapability::StatementRegion) => {
                    continue;
                }
                (_, TargetCapability::ExpressionBoundary(reason)) => reason,
                (_, TargetCapability::StatementRegion) => {
                    ExpressionBoundaryReason::OwnerTakesNoStatements
                }
            };
            let source = match &region.placement {
                RegionPlacement::Host { source, .. } => Some(*source),
                RegionPlacement::Nested { source, .. } => *source,
                RegionPlacement::SourceEdit => None,
            };
            let Some(source) = source else {
                if covered_by_parent_propagation {
                    continue;
                }
                return Err(EvaluationError::MissingHost {
                    root: CoreRoot::Expr(expr),
                });
            };
            unsupported_expression_propagations.push(UnsupportedExpressionPropagation {
                expr,
                source,
                owner: context.owner,
                reason,
            });
        }
        unsupported_expression_propagations.extend(
            unsupported_owned_children
                .iter()
                .filter(|(expr, ..)| matches!(core.exprs[expr.index()], Expr::Propagate(_)))
                .map(
                    |&(expr, source, owner, reason)| UnsupportedExpressionPropagation {
                        expr,
                        source,
                        owner,
                        reason,
                    },
                ),
        );
        let for_initializer_propagations = self
            .regions
            .iter()
            .filter_map(|region| {
                let CoreRoot::Propagate(node) = region.root? else {
                    return None;
                };
                let (context, owner, source) = match &region.placement {
                    RegionPlacement::Host {
                        context,
                        host_owner,
                        source,
                        ..
                    } => (context, host_owner, source),
                    RegionPlacement::Nested {
                        context: Some(context),
                        owner: Some(owner),
                        source: Some(source),
                        ..
                    } => (context, owner, source),
                    _ => return None,
                };
                (context.continuation == HostContinuation::ForInitialize).then_some(
                    ForInitializerPropagation {
                        node,
                        owner: *owner,
                        source: *source,
                    },
                )
            })
            .collect();
        let mut unsupported_matches: Vec<_> = rewrites
            .iter()
            .flat_map(|rewrite| &rewrite.values)
            .filter_map(|value| {
                let Expr::Decision(_) = &core.exprs[value.expr.index()] else {
                    return None;
                };
                let TargetCapability::ExpressionBoundary(reason) = value.capability else {
                    return None;
                };
                Some(UnsupportedMatch {
                    expr: value.expr,
                    source: value.source,
                    owner: value.context.owner,
                    reason,
                })
            })
            .collect();
        // A statement-form match can be nested inside an expression-form
        // outer value. The outer value's host capability governs the whole
        // region: if that host has no statement position, the nested match
        // must report the same placement diagnostic instead of surviving as
        // an unassigned nested slot in expression emission. A Result region
        // is an isolated statement boundary and owns its nested matches.
        unsupported_matches.extend(self.regions.iter().filter_map(|region| {
            let Some(CoreRoot::Expr(expr)) = region.root else {
                return None;
            };
            if !matches!(core.exprs[expr.index()], Expr::Decision(_)) {
                return None;
            }
            let RegionPlacement::Nested {
                parent,
                source: Some(source),
                ..
            } = region.placement
            else {
                return None;
            };
            let mut ancestor = &self.regions[parent.0 as usize];
            loop {
                if let Some(CoreRoot::Expr(parent_expr)) = ancestor.root
                    && matches!(core.exprs[parent_expr.index()], Expr::ResultRegion(_))
                {
                    return None;
                }
                let RegionPlacement::Nested { parent, .. } = ancestor.placement else {
                    break;
                };
                ancestor = &self.regions[parent.0 as usize];
            }
            let RegionPlacement::Host { context, .. } = &ancestor.placement else {
                return None;
            };
            let Some(CoreRoot::Expr(host_expr)) = ancestor.root else {
                return None;
            };
            let reason = match direct_capabilities.get(&host_expr).copied() {
                Some(TargetCapability::ExpressionBoundary(reason)) => reason,
                Some(TargetCapability::StatementRegion) => return None,
                None => match context.owner {
                    EvaluationOwner::ParameterInitializer
                    | EvaluationOwner::ClassInitializer
                    | EvaluationOwner::ClassDefinition
                    | EvaluationOwner::EnumInitializer => {
                        ExpressionBoundaryReason::OwnerTakesNoStatements
                    }
                    _ => ExpressionBoundaryReason::ValueHasNoStatementForm,
                },
            };
            Some(UnsupportedMatch {
                expr,
                source,
                owner: context.owner,
                reason,
            })
        }));
        unsupported_matches.extend(
            unsupported_owned_children
                .iter()
                .filter(|(expr, ..)| matches!(core.exprs[expr.index()], Expr::Decision(_)))
                .map(|&(expr, source, owner, reason)| UnsupportedMatch {
                    expr,
                    source,
                    owner,
                    reason,
                }),
        );
        let expression_boundary_name = allocate_generated_name("$tt_expr", &mut occupied_names)?;
        let match_raise_name = allocate_generated_name("$tt_raise", &mut occupied_names)?;
        let match_show_name = allocate_generated_name("$tt_show", &mut occupied_names)?;
        let spread_name = allocate_generated_name("$tt_spread", &mut occupied_names)?;
        let global_bindings: HashMap<SourceSpan, &str> = self
            .globals
            .iter()
            .filter_map(|(anchor, global)| match global {
                GlobalStatement::Binding(binding) => Some((*anchor, binding.as_str())),
                GlobalStatement::Enclose => None,
            })
            .collect();
        slot_anchors.resize(slot_names.len(), None);
        for (name, anchor) in slot_names.iter_mut().zip(&slot_anchors) {
            if let Some(binding) = anchor.and_then(|anchor| global_bindings.get(&anchor)) {
                *name = globalize_name(name, binding, &mut occupied_names)?;
            }
        }
        let mut match_subject_names = HashMap::new();
        let mut taken_subject_names = 0;
        for rewrite in &rewrites {
            let binding = global_bindings.get(&rewrite.owner.statement()).copied();
            for value in &rewrite.values {
                if let Expr::Decision(decision) = &core.exprs[value.expr.index()] {
                    let names = decision
                        .subjects
                        .iter()
                        .map(|_| {
                            let name = crate::generated_names::allocate_after(
                                "$tt_subject",
                                &mut occupied_names,
                                &mut taken_subject_names,
                            )
                            .ok_or(EvaluationError::GeneratedNameOverflow)?;
                            match binding {
                                Some(binding) => {
                                    globalize_name(&name, binding, &mut occupied_names)
                                }
                                None => Ok(name),
                            }
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    match_subject_names.insert(value.expr, names);
                }
            }
        }
        let adt_names: HashSet<&str> = core
            .bodies
            .iter()
            .flat_map(|body| &body.statements)
            .filter_map(|statement| match statement {
                Statement::Adt(adt) => Some(adt.name.as_str()),
                _ => None,
            })
            .collect();
        let declared = |name: &str| self.declared_names.contains(name) || adt_names.contains(name);
        let module_declared =
            |name: &str| self.module_declared_names.contains(name) || adt_names.contains(name);
        let shadowed_globals: HashSet<String> = ["Error", "JSON", "String"]
            .into_iter()
            .filter(|name| declared(name))
            .map(str::to_owned)
            .collect();
        let mut host_global_aliases = HashMap::new();
        if declared("globalThis") {
            for name in ["Error", "JSON", "String"] {
                if !shadowed_globals.contains(name) {
                    continue;
                }
                let capture = if !module_declared("globalThis") {
                    format!("globalThis.{name}")
                } else if !module_declared(name) {
                    name.to_owned()
                } else {
                    continue;
                };
                let alias = allocate_generated_name(&format!("$tt_{name}"), &mut occupied_names)?;
                host_global_aliases.insert(
                    name.to_owned(),
                    HostGlobalAlias {
                        name: alias,
                        capture,
                    },
                );
            }
        }
        let allocated_names = occupied_names
            .difference(&self.occupied_names)
            .cloned()
            .collect();
        let block_required: Vec<(NodeId, SourceSpan)> = self
            .regions
            .iter()
            .filter_map(|region| match (region.root, &region.placement) {
                (
                    Some(CoreRoot::Propagate(node)),
                    RegionPlacement::Host {
                        context, source, ..
                    },
                ) if context.requires_block
                    && context.continuation != HostContinuation::ForInitialize =>
                {
                    Some((node, *source))
                }
                (
                    Some(CoreRoot::Decision(node)),
                    RegionPlacement::Host {
                        context, source, ..
                    },
                ) if context.requires_block => Some((node, *source)),
                (
                    Some(CoreRoot::Propagate(node)),
                    RegionPlacement::Nested {
                        context: Some(context),
                        source: Some(source),
                        ..
                    },
                ) if context.requires_block
                    && context.continuation != HostContinuation::ForInitialize =>
                {
                    Some((node, *source))
                }
                (
                    Some(CoreRoot::Decision(node)),
                    RegionPlacement::Nested {
                        context: Some(context),
                        source: Some(source),
                        ..
                    },
                ) if context.requires_block => Some((node, *source)),
                _ => None,
            })
            .collect();
        let mut block_required_statements: HashSet<NodeId> =
            block_required.iter().map(|(node, _)| *node).collect();
        let lexical_declaration_bodies = core
            .bodies
            .iter()
            .flat_map(|body| &body.statements)
            .filter_map(|statement| match statement {
                Statement::Decision(Decision {
                    extent,
                    kind: DecisionKind::LetElse { binding_mode, .. },
                    ..
                }) => Some((*extent, *binding_mode, BindingStatement::LetElse)),
                Statement::Propagate(Propagate {
                    node,
                    binding: Some(binding),
                    ..
                }) => Some((*node, binding.mode, BindingStatement::Try)),
                _ => None,
            })
            .filter(|(_, mode, _)| *mode != BindingMode::Var)
            .filter_map(|(node, _, statement)| {
                block_required
                    .iter()
                    .find(|(required, _)| *required == node)
                    .map(|(_, source)| LexicalDeclarationBody {
                        source: *source,
                        statement,
                    })
            })
            .collect();
        let mut global_temps = HashMap::new();
        for region in &self.regions {
            let (Some(CoreRoot::Decision(extent)), RegionPlacement::Host { host_owner, .. }) =
                (region.root, &region.placement)
            else {
                continue;
            };
            let Some(global) = self.globals.get(&host_owner.statement()) else {
                continue;
            };
            let Some(decision) = statement_decision(core, extent) else {
                continue;
            };
            if !matches!(decision.kind, DecisionKind::LetElse { .. }) {
                continue;
            }
            match global {
                GlobalStatement::Enclose => {
                    block_required_statements.insert(extent);
                }
                GlobalStatement::Binding(binding) => {
                    for subject in &decision.subjects {
                        global_temps.insert(subject.temporary, binding.clone());
                    }
                }
            }
        }
        Ok(LoweringPlan {
            script: self.script,
            commonjs: self.commonjs,
            global_temps,
            shadowed_globals,
            host_global_aliases,
            directive_prologue_end: self.directive_prologue_end,
            generated_names: Some(crate::generated_names::GeneratedNames::from_occupied(
                occupied_names,
                allocated_names,
            )),
            match_raise_name,
            match_show_name,
            spread_name,
            statement_decision_sources,
            if_tests: self.if_tests.clone(),
            match_subject_names,
            owners: rewrites,
            for_initializer_propagations,
            slot_names,
            capture_dependencies,
            value_slots,
            piped_slots,
            nested_schedules,
            nested_operations,
            nested_values: self
                .regions
                .iter()
                .filter_map(|region| {
                    matches!(region.placement, RegionPlacement::Nested { .. })
                        .then_some(region.root)
                        .flatten()
                })
                .filter_map(|root| match root {
                    CoreRoot::Expr(expr) => Some(expr),
                    CoreRoot::Adt(_) | CoreRoot::Decision(_) | CoreRoot::Propagate(_) => None,
                })
                .collect(),
            structurally_owned_children,
            nested_relocations: self
                .regions
                .iter()
                .filter_map(|region| match &region.placement {
                    RegionPlacement::Nested {
                        source, protocol, ..
                    } => Some((*source, protocol)),
                    RegionPlacement::Host { .. } | RegionPlacement::SourceEdit => None,
                })
                .flat_map(|(source, protocol)| {
                    source
                        .into_iter()
                        .chain(protocol.steps().iter().flat_map(|step| {
                            std::iter::once(step.parent)
                                .chain(step.inputs.iter().map(|input| input.source))
                        }))
                })
                .collect(),
            nested_exits: self
                .regions
                .iter()
                .filter_map(|region| {
                    let Some(CoreRoot::Expr(expr)) = region.root else {
                        return None;
                    };
                    let RegionPlacement::Nested { exits, .. } = &region.placement else {
                        return None;
                    };
                    (!exits.is_empty()).then(|| (expr, exits.clone()))
                })
                .chain(owned_child_exits)
                .collect(),
            expression_boundary_name,
            unsupported_expression_propagations,
            unsupported_matches,
            block_required_statements,
            lexical_declaration_bodies,
            // Every root that hoists a prelude in front of its owner, rather
            // than replacing the owner as a statement-form `try` does.
            block_required_owners: self
                .regions
                .iter()
                .filter_map(|region| match (region.root, &region.placement) {
                    (
                        Some(root),
                        RegionPlacement::Host {
                            context,
                            host_owner,
                            ..
                        },
                    ) if (context.requires_block
                        || self.globals.get(&host_owner.statement())
                            == Some(&GlobalStatement::Enclose))
                        && match root {
                            CoreRoot::Expr(_) => true,
                            CoreRoot::Propagate(_) => {
                                context.continuation == HostContinuation::ForInitialize
                            }
                            CoreRoot::Adt(_) | CoreRoot::Decision(_) => false,
                        } =>
                    {
                        Some(host_owner.statement())
                    }
                    _ => None,
                })
                .collect(),
            ambient_items: self
                .regions
                .iter()
                .filter_map(|region| match (region.root, &region.placement) {
                    (Some(CoreRoot::Adt(node)), RegionPlacement::Host { context, .. })
                        if context.ambient =>
                    {
                        Some(node)
                    }
                    _ => None,
                })
                .collect(),
            owner_model_unavailable: false,
            completion_scopes: Vec::new(),
        })
    }
}

fn globalize_name(
    name: &str,
    binding: &str,
    occupied: &mut HashSet<String>,
) -> Result<String, EvaluationError> {
    occupied.remove(name);
    crate::generated_names::allocate_global(name, binding, occupied)
        .ok_or(EvaluationError::GeneratedNameOverflow)
}

fn statement_decision(core: &CoreFile, extent: NodeId) -> Option<&Decision> {
    core.bodies
        .iter()
        .flat_map(|body| &body.statements)
        .find_map(|statement| match statement {
            Statement::Decision(decision) if decision.extent == extent => Some(decision),
            _ => None,
        })
}
