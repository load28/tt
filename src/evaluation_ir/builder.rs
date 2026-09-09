//! Core IR traversal and evaluation-region construction.

use super::*;

pub(super) struct EvaluationBuilder<'a> {
    pub(super) core: &'a CoreFile,
    pub(super) hosts: HashMap<CoreRoot, HostBinding>,
    pub(super) regions: Vec<EvalRegion>,
    pub(super) seen: HashSet<OperationId>,
    pub(super) next_value: u32,
}

impl EvaluationBuilder<'_> {
    pub(super) fn walk_body(
        &mut self,
        body: BodyId,
        parent: Option<RegionId>,
    ) -> Result<(), EvaluationError> {
        for statement in &self.core.bodies[body.index()].statements {
            match statement {
                Statement::Opaque(_) => {}
                Statement::Adt(adt) => {
                    self.add_operation(
                        OperationId::Adt(adt.node),
                        CoreRoot::Adt(adt.node),
                        parent,
                        RegionShape::Linear,
                        false,
                    )?;
                }
                Statement::Import(import) => {
                    self.add_source_edit(OperationId::Import(import.specifier))?;
                }
                Statement::Propagate(propagate) => {
                    let region = self.add_propagate(propagate, parent)?;
                    self.walk_nested_expr(propagate.value, region)?;
                }
                Statement::Decision(decision) => {
                    let region = self.add_decision(
                        decision,
                        CoreRoot::Decision(decision.extent),
                        parent,
                        false,
                    )?;
                    self.walk_decision(decision, region)?;
                }
                Statement::Expr(expr) => self.walk_expr(*expr, parent)?,
            }
        }
        Ok(())
    }

    fn walk_expr(&mut self, expr: ExprId, parent: Option<RegionId>) -> Result<(), EvaluationError> {
        self.walk_expr_with_placement(expr, parent, false)
    }

    fn walk_nested_expr(&mut self, expr: ExprId, parent: RegionId) -> Result<(), EvaluationError> {
        self.walk_expr_with_placement(expr, Some(parent), true)
    }

    fn walk_expr_with_placement(
        &mut self,
        expr: ExprId,
        parent: Option<RegionId>,
        force_nested: bool,
    ) -> Result<(), EvaluationError> {
        match &self.core.exprs[expr.index()] {
            Expr::Opaque(_) => {}
            Expr::Sequence(body) => self.walk_body(*body, parent)?,
            Expr::Decision(decision) => {
                let region = self.add_operation_with_placement(
                    OperationId::Decision(decision.extent),
                    CoreRoot::Expr(expr),
                    parent,
                    RegionShape::Decision {
                        arms: decision.arms.len(),
                    },
                    true,
                    force_nested,
                )?;
                self.walk_decision(decision, region)?;
            }
            Expr::Propagate(propagate) => {
                let region = self.add_operation_with_placement(
                    OperationId::Propagate(propagate.node),
                    CoreRoot::Expr(expr),
                    parent,
                    RegionShape::Propagate,
                    true,
                    force_nested,
                )?;
                self.walk_nested_expr(propagate.value, region)?;
            }
            Expr::Apply(apply) => {
                let region = self.add_operation_with_placement(
                    OperationId::Apply(apply.node),
                    CoreRoot::Expr(expr),
                    parent,
                    RegionShape::Linear,
                    true,
                    force_nested,
                )?;
                if let Some(head) = apply.head {
                    self.walk_expr(head, Some(region))?;
                }
                for step in &apply.steps {
                    self.walk_expr(step.value, Some(region))?;
                }
            }
            Expr::ResultRegion(result) => {
                let region = self.add_operation_with_placement(
                    OperationId::ResultRegion(result.node),
                    CoreRoot::Expr(expr),
                    parent,
                    RegionShape::Linear,
                    true,
                    force_nested,
                )?;
                for item in &result.items {
                    let ResultRegionItem::Statements(body) = item;
                    self.walk_body(*body, Some(region))?;
                }
                if let Some(value) = result.value {
                    self.walk_nested_expr(value, region)?;
                }
            }
            Expr::Template(template) => {
                for part in &template.parts {
                    if let crate::core_ir::TemplatePart::Interpolation(inner) = part {
                        self.walk_expr(*inner, parent)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn walk_decision(
        &mut self,
        decision: &Decision,
        parent: RegionId,
    ) -> Result<(), EvaluationError> {
        for subject in &decision.subjects {
            self.walk_expr(subject.value, Some(parent))?;
        }
        for arm in &decision.arms {
            if let Some(guard) = arm.guard {
                self.walk_expr(guard, Some(parent))?;
            }
            match arm.action {
                ArmAction::Yield { body, .. } | ArmAction::Execute(body) => {
                    self.walk_body(body, Some(parent))?;
                }
                ArmAction::BindThrough(_) => {}
            }
        }
        self.walk_miss(&decision.miss, parent)
    }

    fn walk_miss(&mut self, miss: &MissAction, parent: RegionId) -> Result<(), EvaluationError> {
        match miss {
            MissAction::Execute(body) => self.walk_body(*body, Some(parent)),
            MissAction::Decision(decision) => {
                let region = self.add_decision(
                    decision,
                    CoreRoot::Decision(decision.extent),
                    Some(parent),
                    false,
                )?;
                self.walk_decision(decision, region)
            }
            MissAction::ThrowUnexpected(_) | MissAction::Nothing => Ok(()),
        }
    }

    fn add_decision(
        &mut self,
        decision: &Decision,
        root: CoreRoot,
        parent: Option<RegionId>,
        produces_value: bool,
    ) -> Result<RegionId, EvaluationError> {
        self.add_operation(
            OperationId::Decision(decision.extent),
            root,
            parent,
            RegionShape::Decision {
                arms: decision.arms.len(),
            },
            produces_value,
        )
    }

    fn add_propagate(
        &mut self,
        propagate: &Propagate,
        parent: Option<RegionId>,
    ) -> Result<RegionId, EvaluationError> {
        self.add_operation_with_placement(
            OperationId::Propagate(propagate.node),
            CoreRoot::Propagate(propagate.node),
            parent,
            RegionShape::Propagate,
            false,
            matches!(propagate.exit, ExitTarget::ResultRegion(_)),
        )
    }

    fn add_operation(
        &mut self,
        operation: OperationId,
        root: CoreRoot,
        parent: Option<RegionId>,
        shape: RegionShape,
        produces_value: bool,
    ) -> Result<RegionId, EvaluationError> {
        self.add_operation_with_placement(operation, root, parent, shape, produces_value, false)
    }

    fn add_operation_with_placement(
        &mut self,
        operation: OperationId,
        root: CoreRoot,
        parent: Option<RegionId>,
        shape: RegionShape,
        produces_value: bool,
        force_nested: bool,
    ) -> Result<RegionId, EvaluationError> {
        let placement = if force_nested {
            let parent = parent.ok_or(EvaluationError::MissingHost { root })?;
            let binding = self.hosts.remove(&root);
            let source = binding.as_ref().map(|binding| binding.source);
            let exits = binding
                .as_ref()
                .map_or_else(Vec::new, |binding| binding.exits.clone());
            let protocol =
                binding.map_or_else(HostEvaluationProtocol::default, |binding| binding.protocol);
            RegionPlacement::Nested {
                parent,
                source,
                exits,
                protocol,
            }
        } else {
            self.placement(root, parent)?
        };
        let result = self.result(produces_value)?;
        let blocks = blocks_for(operation, shape, result)?;
        self.push_region(operation, Some(root), placement, blocks, result)
    }

    fn add_source_edit(&mut self, operation: OperationId) -> Result<RegionId, EvaluationError> {
        self.push_region(
            operation,
            None,
            RegionPlacement::SourceEdit,
            blocks_for(operation, RegionShape::Linear, None)?,
            None,
        )
    }

    fn placement(
        &mut self,
        root: CoreRoot,
        parent: Option<RegionId>,
    ) -> Result<RegionPlacement, EvaluationError> {
        let nested_in_owned_exit = parent.is_some_and(|parent| {
            let Some(binding) = self.hosts.get(&root) else {
                return false;
            };
            match &self.regions[parent.0 as usize].placement {
                RegionPlacement::Host { exits, .. } | RegionPlacement::Nested { exits, .. } => {
                    exits.iter().any(|exit| {
                        exit.argument.is_some_and(|argument| {
                            argument.start <= binding.source.start
                                && binding.source.end <= argument.end
                        })
                    })
                }
                RegionPlacement::SourceEdit => false,
            }
        });
        if let Some(parent) = parent
            && let Some(binding) = self.hosts.get(&root)
            && (nested_in_owned_exit
                || (self.region_host_owner(parent) == Some(binding.owner)
                    && (binding.protocol.steps().is_empty()
                        || matches!(root, CoreRoot::Expr(expr)
                            if matches!(self.core.exprs[expr.index()],
                                Expr::ResultRegion(_) | Expr::Propagate(_)))
                        || matches!(root, CoreRoot::Expr(expr)
                        if matches!(self.core.exprs[expr.index()], Expr::Decision(_))
                            && binding.protocol.steps().iter().all(|step| matches!(
                                step.operation,
                                HostEvaluationOperation::Eager(EagerPosition::TemplateInterpolation(_))
                            ))))))
        {
            let binding = self.hosts.remove(&root);
            let source = binding.as_ref().map(|binding| binding.source);
            let exits = binding
                .as_ref()
                .map_or_else(Vec::new, |binding| binding.exits.clone());
            let protocol =
                binding.map_or_else(HostEvaluationProtocol::default, |binding| binding.protocol);
            return Ok(RegionPlacement::Nested {
                parent,
                source,
                exits,
                protocol,
            });
        }
        if let Some(binding) = self.hosts.remove(&root) {
            Ok(RegionPlacement::Host {
                syntax: binding.syntax,
                context: binding.context,
                protocol: binding.protocol,
                source: binding.source,
                host_owner: binding.owner,
                exits: binding.exits,
            })
        } else if let Some(parent) = parent {
            Ok(RegionPlacement::Nested {
                parent,
                source: None,
                exits: Vec::new(),
                protocol: HostEvaluationProtocol::default(),
            })
        } else {
            Err(EvaluationError::MissingHost { root })
        }
    }

    fn region_host_owner(&self, mut region: RegionId) -> Option<HostOwner> {
        loop {
            match &self.regions[region.0 as usize].placement {
                RegionPlacement::Host { host_owner, .. } => return Some(*host_owner),
                RegionPlacement::Nested { parent, .. } => region = *parent,
                RegionPlacement::SourceEdit => return None,
            }
        }
    }

    fn result(&mut self, produces_value: bool) -> Result<Option<ValueId>, EvaluationError> {
        if !produces_value {
            return Ok(None);
        }
        let value = ValueId(self.next_value);
        self.next_value = self
            .next_value
            .checked_add(1)
            .ok_or(EvaluationError::IdOverflow)?;
        Ok(Some(value))
    }

    fn push_region(
        &mut self,
        operation: OperationId,
        root: Option<CoreRoot>,
        placement: RegionPlacement,
        blocks: Vec<EvalBlock>,
        result: Option<ValueId>,
    ) -> Result<RegionId, EvaluationError> {
        if !self.seen.insert(operation) {
            return Err(EvaluationError::DuplicateOperation { operation });
        }
        let id =
            RegionId(u32::try_from(self.regions.len()).map_err(|_| EvaluationError::IdOverflow)?);
        self.regions.push(EvalRegion {
            id,
            root,
            operation,
            placement,
            entry: EvalBlockId(0),
            blocks,
            result,
        });
        Ok(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegionShape {
    Linear,
    Decision { arms: usize },
    Propagate,
}

fn blocks_for(
    operation: OperationId,
    shape: RegionShape,
    result: Option<ValueId>,
) -> Result<Vec<EvalBlock>, EvaluationError> {
    let mut entry_statements = vec![EvalStatement::Core(operation)];
    if shape == RegionShape::Linear
        && let Some(value) = result
    {
        entry_statements.push(EvalStatement::Produce(value));
    }
    let mut blocks = vec![EvalBlock {
        statements: entry_statements,
        terminator: EvalTerminator::Complete,
    }];
    match shape {
        RegionShape::Linear => {}
        RegionShape::Decision { arms: arm_count } => {
            let join = push_block(&mut blocks, EvalTerminator::Complete)?;
            let mut arms = Vec::with_capacity(arm_count);
            for _ in 0..arm_count {
                let arm = push_block(&mut blocks, EvalTerminator::Goto(join))?;
                if let Some(value) = result {
                    blocks[arm.0 as usize]
                        .statements
                        .push(EvalStatement::Produce(value));
                }
                arms.push(arm);
            }
            let fallback = push_block(
                &mut blocks,
                if result.is_some() {
                    EvalTerminator::Exit
                } else {
                    EvalTerminator::Goto(join)
                },
            )?;
            blocks[0].terminator = EvalTerminator::Switch { arms, fallback };
        }
        RegionShape::Propagate => {
            let join = push_block(&mut blocks, EvalTerminator::Complete)?;
            let success = push_block(&mut blocks, EvalTerminator::Goto(join))?;
            if let Some(value) = result {
                blocks[success.0 as usize]
                    .statements
                    .push(EvalStatement::Produce(value));
            }
            let failure = push_block(&mut blocks, EvalTerminator::Exit)?;
            blocks[0].terminator = EvalTerminator::Branch { success, failure };
        }
    }
    Ok(blocks)
}

fn push_block(
    blocks: &mut Vec<EvalBlock>,
    terminator: EvalTerminator,
) -> Result<EvalBlockId, EvaluationError> {
    let id = EvalBlockId(u32::try_from(blocks.len()).map_err(|_| EvaluationError::IdOverflow)?);
    blocks.push(EvalBlock {
        statements: Vec::new(),
        terminator,
    });
    Ok(id)
}
