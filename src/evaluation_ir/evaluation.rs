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
        };
        builder.walk_body(core.root, None)?;
        if let Some(root) = builder.hosts.keys().copied().next() {
            return Err(EvaluationError::OrphanHost { root });
        }
        let file = Self {
            regions: builder.regions,
            occupied_names: syntax.occupied_names().map(str::to_owned).collect(),
            tt_spans: syntax
                .core_contexts()
                .map(|(_, _, _, _, source, _, _)| source)
                .collect(),
        };
        file.validate()?;
        Ok(file)
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
            // A propagation that terminates a Result region is emitted by
            // that region's structured body printer. Scheduling it again at
            // its TypeScript host would both duplicate the exit and make an
            // enclosing source capture overlap a nested function boundary.
            if matches!(
                &core.exprs[expr.index()],
                Expr::Propagate(Propagate {
                    exit: ExitTarget::ResultRegion(_),
                    ..
                })
            ) {
                continue;
            }
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
        let mut next_slot = 0u32;
        let mut occupied_names = self.occupied_names.clone();
        let mut slot_names = Vec::new();
        let mut value_slots = HashMap::new();
        let mut rewrites = Vec::with_capacity(owners.len());
        let mut structurally_owned_children = HashSet::new();
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
            let mut values = values;
            // A statement-capable outer Core value owns same-host tt values
            // lexically nested inside it. Its structural emitter evaluates
            // those children at their exact position and writes their
            // already allocated slots; planning the children again as
            // sibling owner actions would either run them twice or consume
            // authored syntax that still contains the outer construct.
            let owned_children: HashSet<_> = values
                .iter()
                .filter(|child| {
                    values.iter().any(|outer| {
                        outer.expr != child.expr
                            && outer.capability == TargetCapability::StatementRegion
                            && outer.source.start <= child.source.start
                            && child.source.end <= outer.source.end
                            && (outer.source.start < child.source.start
                                || child.source.end < outer.source.end)
                    })
                })
                .map(|child| child.expr)
                .collect();
            structurally_owned_children.extend(owned_children.iter().copied());
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
            if matches!(
                &core.exprs[expr.index()],
                Expr::Propagate(Propagate {
                    exit: ExitTarget::ResultRegion(_),
                    ..
                })
            ) && matches!(&region.placement, RegionPlacement::Nested { protocol, .. } if protocol.steps().is_empty())
            {
                continue;
            }
            if region.result.is_none() || value_slots.contains_key(&expr) {
                continue;
            }
            let slot = allocate_value_slot(&mut next_slot, &mut slot_names, &mut occupied_names)?;
            value_slots.insert(expr, slot);
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
        let mut nested_schedules = HashMap::new();
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
                parent, protocol, ..
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
                let RegionPlacement::Nested { parent, .. } = ancestor.placement else {
                    break None;
                };
                ancestor = &self.regions[parent.0 as usize];
            };
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
                &nested_sources,
                &mut nested_source_slots,
                &mut next_slot,
                &mut slot_names,
                &mut occupied_names,
            )?;
            nested_schedules.insert(expr, schedule);
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
            let Expr::Propagate(propagate) = &core.exprs[expr.index()] else {
                continue;
            };
            if matches!(propagate.exit, ExitTarget::ResultRegion(_)) {
                continue;
            }
            let RegionPlacement::Host {
                context, source, ..
            } = &region.placement
            else {
                continue;
            };
            if context.continuation == HostContinuation::ForInitialize {
                return Err(EvaluationError::UnsupportedForInitializer { source: *source });
            }
        }
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
        // Core arm bodies are flat arena entries, so build their ownership
        // index once. Propagation placement then answers by ExprId instead of
        // rescanning every decision and arm for every value region.
        let isolated_arm_values: HashSet<_> = core
            .exprs
            .iter()
            .filter_map(|candidate| {
                let Expr::Decision(decision) = candidate else {
                    return None;
                };
                Some(decision)
            })
            .flat_map(|decision| &decision.arms)
            .filter_map(|arm| match arm.action {
                ArmAction::Yield {
                    body,
                    kind: ArmBodyKind::Block { .. },
                }
                | ArmAction::Execute(body) => Some(body),
                ArmAction::Yield {
                    kind: ArmBodyKind::Expression,
                    ..
                }
                | ArmAction::BindThrough(_) => None,
            })
            .flat_map(|body| &core.bodies[body.index()].statements)
            .filter_map(|statement| match statement {
                Statement::Expr(value) => Some(*value),
                _ => None,
            })
            .collect();
        for region in &self.regions {
            let Some(CoreRoot::Expr(expr)) = region.root else {
                continue;
            };
            let Expr::Propagate(propagate) = &core.exprs[expr.index()] else {
                continue;
            };
            if matches!(propagate.exit, ExitTarget::ResultRegion(_)) {
                continue;
            }
            let mut host_region = region;
            let mut covered_by_parent_propagation = false;
            // A value-form propagation emitted directly by a decision arm
            // cannot use the enclosing function's failure edge: the arm is
            // an isolated value region even when SWC identifies the nested
            // `return` statement as its own host owner.
            let crossed_value_region = isolated_arm_values.contains(&expr);
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
            let reason = match (crossed_value_region, context.owner, capability) {
                (true, _, _) => ExpressionBoundaryReason::OwnerTakesNoStatements,
                (false, EvaluationOwner::FunctionBody, TargetCapability::StatementRegion) => {
                    continue;
                }
                (false, _, TargetCapability::ExpressionBoundary(reason)) => reason,
                (false, _, TargetCapability::StatementRegion) => {
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
        let for_initializer_propagations = self
            .regions
            .iter()
            .filter_map(|region| {
                let CoreRoot::Propagate(node) = region.root? else {
                    return None;
                };
                let RegionPlacement::Host {
                    context,
                    host_owner,
                    source,
                    ..
                } = &region.placement
                else {
                    return None;
                };
                (context.continuation == HostContinuation::ForInitialize).then_some(
                    ForInitializerPropagation {
                        node,
                        owner: *host_owner,
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
                    EvaluationOwner::ParameterInitializer | EvaluationOwner::ClassInitializer => {
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
        let expression_boundary_name = allocate_generated_name("$tt_expr", &mut occupied_names)?;
        Ok(LoweringPlan {
            owners: rewrites,
            for_initializer_propagations,
            slot_names,
            value_slots,
            nested_schedules,
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
                .collect(),
            expression_boundary_name,
            unsupported_expression_propagations,
            unsupported_matches,
        })
    }
}
