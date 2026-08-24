//! Control-flow IR joining TypeScript host context with Core IR.
//!
//! Every Core operation receives a typed region and explicit block
//! termination and an owner-scoped value target consumed by target lowering.

use std::collections::{HashMap, HashSet};

use crate::core_ir::{
    ArmAction, CoreFile, Decision, Expr, MissAction, Propagate, ResultRegionItem, Statement,
};
use crate::hir::ids::Idx;
use crate::hir::{BodyId, ExprId, NodeId};
use crate::ice::LoweringSubject;
use crate::program_syntax::{
    ConditionalBranch, CoreRoot, EvaluationContext, EvaluationInputMode, EvaluationOwner,
    HostContinuation, HostEvaluationOperation, HostEvaluationProtocol, HostExit, HostOwner,
    OwnerReach, ProgramSyntax, SourceSpan, TtNodeId,
};

/// A failure of one of the lowering validators, already carrying the stage,
/// the named invariant, and the identities it failed on.
pub(crate) type LoweringError = crate::ice::InternalCompilerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RegionId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct EvalBlockId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ValueId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum OperationId {
    Adt(NodeId),
    Import(NodeId),
    Decision(NodeId),
    Propagate(NodeId),
    Apply(NodeId),
    ResultRegion(NodeId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RegionPlacement {
    Host {
        syntax: TtNodeId,
        context: EvaluationContext,
        protocol: HostEvaluationProtocol,
        source: SourceSpan,
        host_owner: HostOwner,
        exits: Vec<HostExit>,
    },
    Nested {
        parent: RegionId,
    },
    SourceEdit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvalStatement {
    Core(OperationId),
    Produce(ValueId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EvalTerminator {
    Goto(EvalBlockId),
    Branch {
        success: EvalBlockId,
        failure: EvalBlockId,
    },
    Switch {
        arms: Vec<EvalBlockId>,
        fallback: EvalBlockId,
    },
    Exit,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EvalBlock {
    statements: Vec<EvalStatement>,
    terminator: EvalTerminator,
}

#[derive(Debug)]
struct EvalRegion {
    id: RegionId,
    root: Option<CoreRoot>,
    operation: OperationId,
    placement: RegionPlacement,
    entry: EvalBlockId,
    blocks: Vec<EvalBlock>,
    result: Option<ValueId>,
}

#[derive(Debug)]
pub(crate) struct EvaluationFile {
    regions: Vec<EvalRegion>,
    occupied_names: HashSet<String>,
    /// Source spans of every tt node in the file. A schedule's source
    /// capture must not overlap one: the capture copies raw source bytes,
    /// and a tt node inside them is lowered elsewhere.
    tt_spans: Vec<SourceSpan>,
}

#[derive(Debug, Default)]
pub(crate) struct LoweringPlan {
    owners: Vec<HostRewrite>,
    slot_names: Vec<String>,
    value_slots: HashMap<ExprId, ValueSlotId>,
    expression_boundary_name: String,
}

#[derive(Debug)]
pub(crate) struct HostRewrite {
    pub(crate) owner: HostOwner,
    pub(crate) values: Vec<PlannedValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ValueSlotId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueTarget {
    Slot(ValueSlotId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedValue {
    pub(crate) expr: ExprId,
    pub(crate) source: SourceSpan,
    pub(crate) target: ValueTarget,
    pub(crate) context: EvaluationContext,
    pub(crate) schedule: EvaluationSchedule,
    pub(crate) exits: Vec<HostExit>,
    /// Whether this value's control flow may become statements in its host
    /// owner. Decided here, from typed facts; target lowering reads it and
    /// chooses only the *shape* of the statements.
    pub(crate) capability: TargetCapability,
}

/// Whether a host value's Core control flow can be lowered to statements in
/// its host owner.
///
/// This is the semantic half of target selection and belongs to this stage:
/// it depends on how often the owner is reached, on the execution regions
/// the schedule opens, and on which source the schedule has to relocate.
/// The remaining half — which statement shape fits the host continuation —
/// belongs to target lowering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetCapability {
    /// Statements in the host owner, delivering the value through its slot.
    StatementRegion,
    /// The named `$tt_expr` intrinsic runs the value's control flow exactly
    /// where the value sits. Not a fallback for a failed analysis: a typed
    /// capability with a recorded reason (`docs/design/program-lowering.md`
    /// §7.4).
    ExpressionBoundary(ExpressionBoundaryReason),
}

/// Why a value cannot be lowered to statements in its host owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpressionBoundaryReason {
    /// A parameter default or class field initializer: standard TypeScript
    /// has no statement position in the owner, and moving the value out of
    /// it would change the parameter scope, `this`, `arguments`, the
    /// function's `length`, or the field initialization order.
    OwnerTakesNoStatements,
    /// The value runs once per iteration but its owner runs once per loop —
    /// it sits in a loop header, so hoisting to the owner would change how
    /// often it runs.
    RepeatedInOwner,
    /// The value's evaluation is conditional relative to its owner through
    /// an edge no protocol step models — a `switch` case test, a
    /// destructuring default, an optional chain's tail — so hoisting to the
    /// owner would evaluate it unconditionally.
    ConditionalInOwner,
    /// A source capture the schedule needs would be declared inside a
    /// generated conditional region while the host occurrence that reads it
    /// stays outside. Lifting the whole conditional operation into the
    /// region is what removes this reason, not a wider `let`.
    CaptureInsideConditionalRegion,
    /// A source capture the schedule needs covers another tt value, or
    /// another capture: the two replacements would overlap in the target.
    CaptureOverlapsValue,
    /// The schedule carries a reference mode statements cannot preserve.
    ReferenceNotPreservable,
    /// The Core value has no statement form ([`CoreFile::has_statement_form`]).
    ValueHasNoStatementForm,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct EvaluationSchedule {
    steps: Vec<PlannedEvaluationStep>,
}

impl EvaluationSchedule {
    pub(crate) fn steps(&self) -> &[PlannedEvaluationStep] {
        &self.steps
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedEvaluationStep {
    pub(crate) parent: SourceSpan,
    pub(crate) operation: HostEvaluationOperation,
    pub(crate) inputs: Vec<PlannedEvaluationInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlannedEvaluationInput {
    Source {
        source: SourceSpan,
        mode: EvaluationInputMode,
        target: ValueSlotId,
        receiver: Option<(SourceSpan, ValueSlotId)>,
    },
    Slot {
        slot: ValueSlotId,
        mode: EvaluationInputMode,
    },
}

struct PendingPlannedValue {
    expr: ExprId,
    source: SourceSpan,
    context: EvaluationContext,
    protocol: HostEvaluationProtocol,
    exits: Vec<HostExit>,
}

#[derive(Debug, Clone)]
struct HostBinding {
    syntax: TtNodeId,
    context: EvaluationContext,
    protocol: HostEvaluationProtocol,
    source: SourceSpan,
    owner: HostOwner,
    exits: Vec<HostExit>,
}

#[derive(Debug, Clone, Copy)]
struct PlannedSourceSlot {
    target: ValueSlotId,
    receiver: Option<(SourceSpan, ValueSlotId)>,
}

impl LoweringPlan {
    pub(crate) fn owners(&self) -> impl Iterator<Item = &HostRewrite> {
        self.owners.iter()
    }

    pub(crate) fn slot_name(&self, slot: ValueSlotId) -> &str {
        &self.slot_names[slot.0 as usize]
    }

    pub(crate) fn value_slot_names(&self) -> impl Iterator<Item = (ExprId, &str)> {
        self.value_slots
            .iter()
            .map(|(expr, slot)| (*expr, self.slot_name(*slot)))
    }

    pub(crate) fn slots(&self) -> impl Iterator<Item = (ValueSlotId, &str)> {
        (0u32..)
            .zip(&self.slot_names)
            .map(|(index, name)| (ValueSlotId(index), name.as_str()))
    }

    pub(crate) fn expression_boundary_name(&self) -> &str {
        &self.expression_boundary_name
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EvaluationError {
    DuplicateHost {
        root: CoreRoot,
    },
    MissingHost {
        root: CoreRoot,
    },
    OrphanHost {
        root: CoreRoot,
    },
    DuplicateOperation {
        operation: OperationId,
    },
    IdOverflow,
    GeneratedNameOverflow,
    InvalidEntry {
        region: RegionId,
    },
    InvalidTarget {
        region: RegionId,
        target: EvalBlockId,
    },
    UnreachableBlock {
        region: RegionId,
        block: EvalBlockId,
    },
    CoreOperationMismatch {
        region: RegionId,
    },
    MissingResultDefinition {
        region: RegionId,
        block: EvalBlockId,
    },
    UnexpectedResultDefinition {
        region: RegionId,
        value: ValueId,
    },
    InvalidHostOwner {
        root: CoreRoot,
        owner: HostOwner,
        value: SourceSpan,
    },
}

impl EvaluationFile {
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
            rewrites.push(HostRewrite { owner, values });
        }
        for region in &self.regions {
            let Some(CoreRoot::Expr(expr)) = region.root else {
                continue;
            };
            if region.result.is_none() || value_slots.contains_key(&expr) {
                continue;
            }
            let slot = allocate_value_slot(&mut next_slot, &mut slot_names, &mut occupied_names)?;
            value_slots.insert(expr, slot);
        }
        let expression_boundary_name = allocate_generated_name("$tt_expr", &mut occupied_names)?;
        Ok(LoweringPlan {
            owners: rewrites,
            slot_names,
            value_slots,
            expression_boundary_name,
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
                        return Err(InternalCompilerError::new(
                            stage,
                            Invariant::RepetitionRegionLeft,
                            subject,
                        )
                        .at(value.source));
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
                            return Err(InternalCompilerError::new(
                                stage,
                                Invariant::ConditionalRegionLeft,
                                subject.with_slot(*target),
                            )
                            .at(*source));
                        }
                        for span in &self.tt_spans {
                            if overlaps(*source, *span) {
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
                                if optional_argument {
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
                            | PlannedEvaluationInput::Slot { .. } => {}
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), EvaluationError> {
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

fn resolve_schedule(
    protocol: HostEvaluationProtocol,
    slots: &HashMap<SourceSpan, ValueSlotId>,
    source_slots: &mut HashMap<SourceSpan, PlannedSourceSlot>,
    next_slot: &mut u32,
    slot_names: &mut Vec<String>,
    occupied_names: &mut HashSet<String>,
) -> Result<EvaluationSchedule, EvaluationError> {
    let steps = protocol
        .steps()
        .iter()
        .map(|step| {
            Ok(PlannedEvaluationStep {
                parent: step.parent,
                operation: step.operation,
                inputs: step
                    .inputs
                    .iter()
                    .map(|input| {
                        slots.get(&input.source).map_or_else(
                            || {
                                if let Some(slot) = source_slots.get(&input.source) {
                                    return Ok(PlannedEvaluationInput::Source {
                                        source: input.source,
                                        mode: input.mode,
                                        target: slot.target,
                                        receiver: slot.receiver,
                                    });
                                }
                                let target =
                                    allocate_value_slot(next_slot, slot_names, occupied_names)?;
                                let receiver = input
                                    .receiver
                                    .map(|receiver| {
                                        Ok((
                                            receiver,
                                            allocate_value_slot(
                                                next_slot,
                                                slot_names,
                                                occupied_names,
                                            )?,
                                        ))
                                    })
                                    .transpose()?;
                                source_slots
                                    .insert(input.source, PlannedSourceSlot { target, receiver });
                                Ok(PlannedEvaluationInput::Source {
                                    source: input.source,
                                    mode: input.mode,
                                    target,
                                    receiver,
                                })
                            },
                            |slot| {
                                Ok(PlannedEvaluationInput::Slot {
                                    slot: *slot,
                                    mode: input.mode,
                                })
                            },
                        )
                    })
                    .collect::<Result<Vec<_>, EvaluationError>>()?,
            })
        })
        .collect::<Result<Vec<_>, EvaluationError>>()?;
    Ok(EvaluationSchedule { steps })
}

fn overlaps(left: SourceSpan, right: SourceSpan) -> bool {
    left.start < right.end && right.start < left.end
}

/// Decides whether a host value's Core control flow may become statements
/// in its host owner, from typed facts alone: what kind of owner it has,
/// how often the owner is reached, whether the Core value has a statement
/// form, and what the schedule would have to capture and preserve.
///
/// Every refusal names its reason. Target lowering consumes the decision;
/// [`EvaluationFile::validate_order`] and
/// [`EvaluationFile::validate_reference`] re-check the resulting plan
/// independently.
fn target_capability(
    core: &CoreFile,
    tt_spans: &[SourceSpan],
    expr: ExprId,
    context: &EvaluationContext,
    schedule: &EvaluationSchedule,
) -> TargetCapability {
    use ExpressionBoundaryReason as Reason;
    if matches!(
        context.owner,
        EvaluationOwner::ParameterInitializer | EvaluationOwner::ClassInitializer
    ) {
        return TargetCapability::ExpressionBoundary(Reason::OwnerTakesNoStatements);
    }
    if !core.has_statement_form(expr) {
        return TargetCapability::ExpressionBoundary(Reason::ValueHasNoStatementForm);
    }
    match context.owner_reach {
        OwnerReach::Same => {}
        OwnerReach::Repeated => {
            return TargetCapability::ExpressionBoundary(Reason::RepeatedInOwner);
        }
        OwnerReach::UnmodeledConditional => {
            return TargetCapability::ExpressionBoundary(Reason::ConditionalInOwner);
        }
    }
    let steps = schedule.steps();
    for step in steps {
        let optional_argument = matches!(
            step.operation,
            HostEvaluationOperation::Conditional(ConditionalBranch::OptionalCallArgument(_))
        );
        for input in &step.inputs {
            match input {
                PlannedEvaluationInput::Source {
                    mode: EvaluationInputMode::MemberReference,
                    receiver,
                    ..
                } => {
                    if receiver.is_none() || optional_argument {
                        return TargetCapability::ExpressionBoundary(
                            Reason::ReferenceNotPreservable,
                        );
                    }
                }
                PlannedEvaluationInput::Slot {
                    mode: EvaluationInputMode::MemberReference,
                    ..
                } => {
                    return TargetCapability::ExpressionBoundary(Reason::ReferenceNotPreservable);
                }
                PlannedEvaluationInput::Source { .. } | PlannedEvaluationInput::Slot { .. } => {}
            }
        }
    }
    let mut captured: Vec<SourceSpan> = Vec::new();
    for (index, step) in steps.iter().enumerate() {
        let conditional_after = steps[index + 1..]
            .iter()
            .any(|later| matches!(later.operation, HostEvaluationOperation::Conditional(_)));
        for input in &step.inputs {
            let PlannedEvaluationInput::Source { source, .. } = input else {
                continue;
            };
            if captured.contains(source) {
                continue;
            }
            // The capture would be declared inside the generated conditional
            // region while the host occurrence stays outside it.
            if conditional_after {
                return TargetCapability::ExpressionBoundary(
                    Reason::CaptureInsideConditionalRegion,
                );
            }
            // The capture copies raw source bytes; a tt node or another
            // capture inside them is lowered or relocated elsewhere.
            if tt_spans.iter().any(|span| overlaps(*source, *span))
                || captured.iter().any(|span| overlaps(*source, *span))
            {
                return TargetCapability::ExpressionBoundary(Reason::CaptureOverlapsValue);
            }
            captured.push(*source);
        }
    }
    TargetCapability::StatementRegion
}

fn allocate_value_slot(
    next_slot: &mut u32,
    slot_names: &mut Vec<String>,
    occupied: &mut HashSet<String>,
) -> Result<ValueSlotId, EvaluationError> {
    let slot = ValueSlotId(*next_slot);
    *next_slot = next_slot
        .checked_add(1)
        .ok_or(EvaluationError::IdOverflow)?;
    slot_names.push(allocate_slot_name(slot, occupied)?);
    Ok(slot)
}

fn allocate_slot_name(
    slot: ValueSlotId,
    occupied: &mut HashSet<String>,
) -> Result<String, EvaluationError> {
    let base = format!("$tt_v{}", slot.0);
    if occupied.insert(base.clone()) {
        return Ok(base);
    }
    let mut suffix = 1u32;
    loop {
        let candidate = format!("{base}_{suffix}");
        if occupied.insert(candidate.clone()) {
            return Ok(candidate);
        }
        suffix = suffix
            .checked_add(1)
            .ok_or(EvaluationError::GeneratedNameOverflow)?;
    }
}

fn allocate_generated_name(
    base: &str,
    occupied: &mut HashSet<String>,
) -> Result<String, EvaluationError> {
    if occupied.insert(base.to_owned()) {
        return Ok(base.to_owned());
    }
    let mut suffix = 1u32;
    loop {
        let candidate = format!("{base}_{suffix}");
        if occupied.insert(candidate.clone()) {
            return Ok(candidate);
        }
        suffix = suffix
            .checked_add(1)
            .ok_or(EvaluationError::GeneratedNameOverflow)?;
    }
}

struct EvaluationBuilder<'a> {
    core: &'a CoreFile,
    hosts: HashMap<CoreRoot, HostBinding>,
    regions: Vec<EvalRegion>,
    seen: HashSet<OperationId>,
    next_value: u32,
}

impl EvaluationBuilder<'_> {
    fn walk_body(&mut self, body: BodyId, parent: Option<RegionId>) -> Result<(), EvaluationError> {
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
                    self.walk_nested_expr(head, region)?;
                }
                for step in &apply.steps {
                    self.walk_nested_expr(step.value, region)?;
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
                    match item {
                        ResultRegionItem::Statements(body) => {
                            self.walk_body(*body, Some(region))?;
                        }
                        ResultRegionItem::Propagate(propagate) => {
                            let child = self.add_propagate(propagate, Some(region))?;
                            self.walk_nested_expr(propagate.value, child)?;
                        }
                    }
                }
                self.walk_nested_expr(result.value, region)?;
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
        self.add_operation(
            OperationId::Propagate(propagate.node),
            CoreRoot::Propagate(propagate.node),
            parent,
            RegionShape::Propagate,
            false,
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
            self.hosts.remove(&root);
            RegionPlacement::Nested { parent }
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
        if let Some(parent) = parent
            && let Some(binding) = self.hosts.get(&root)
            && self.region_host_owner(parent) == Some(binding.owner)
        {
            self.hosts.remove(&root);
            return Ok(RegionPlacement::Nested { parent });
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
            Ok(RegionPlacement::Nested { parent })
        } else {
            Err(EvaluationError::MissingHost { root })
        }
    }

    fn region_host_owner(&self, mut region: RegionId) -> Option<HostOwner> {
        loop {
            match &self.regions[region.0 as usize].placement {
                RegionPlacement::Host { host_owner, .. } => return Some(*host_owner),
                RegionPlacement::Nested { parent } => region = *parent,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn evaluation(source: &str) -> (EvaluationFile, CoreFile) {
        let program = crate::parser::parse(source);
        let semantic = crate::analysis::coverage_semantics(&program, &[]);
        let core = crate::core_ir::lower_semantic(&semantic, source);
        let syntax = ProgramSyntax::build(&semantic, &core, source, crate::SourceKind::TypeScript)
            .expect("program syntax");
        let file = EvaluationFile::build(&syntax, &core).expect("evaluation ir");
        (file, core)
    }

    fn plan(file: &EvaluationFile, core: &CoreFile) -> LoweringPlan {
        let plan = file.lowering_plan(core).expect("lowering plan");
        file.validate_order(&plan).expect("validate_order");
        file.validate_reference(&plan).expect("validate_reference");
        plan
    }

    #[test]
    fn every_core_primitive_gets_one_region() {
        let (file, _core) = evaluation(
            "enum E { A(value: number), B }\n\
             import { load } from \"./load.tt\";\n\
             function f(e: E) {\n\
               try load();\n\
               return result { const x <- load(); match (e) { A(value) => x + value, B => 0 } |> done };\n\
             }\n",
        );
        let operations: Vec<_> = file.regions.iter().map(|region| region.operation).collect();
        assert!(
            operations
                .iter()
                .any(|operation| matches!(operation, OperationId::Adt(_))),
            "{operations:?}"
        );
        assert!(
            operations
                .iter()
                .any(|operation| matches!(operation, OperationId::Import(_))),
            "{operations:?}"
        );
        assert!(
            operations
                .iter()
                .any(|operation| matches!(operation, OperationId::Decision(_))),
            "{operations:?}"
        );
        assert!(
            operations
                .iter()
                .any(|operation| matches!(operation, OperationId::Propagate(_))),
            "{operations:?}"
        );
        assert!(
            operations
                .iter()
                .any(|operation| matches!(operation, OperationId::Apply(_))),
            "{operations:?}"
        );
        assert!(
            operations
                .iter()
                .any(|operation| matches!(operation, OperationId::ResultRegion(_))),
            "{operations:?}"
        );
    }

    #[test]
    fn a_decision_region_has_switch_arms_and_a_join() {
        let (file, core) = evaluation(
            "enum E { A, B }\nfunction f(e: E) { return match (e) { A => 1, B => 2 }; }\n",
        );
        let region = file
            .regions
            .iter()
            .find(|region| matches!(region.operation, OperationId::Decision(_)))
            .expect("decision region");
        assert!(matches!(
            &region.blocks[0].terminator,
            EvalTerminator::Switch { arms, .. } if arms.len() == 2
        ));
        assert!(region.result.is_some());
        let plan = plan(&file, &core);
        let owner = plan.owners().next().expect("host rewrite");
        assert_eq!(owner.values.len(), 1);
        assert!(matches!(owner.values[0].target, ValueTarget::Slot(_)));
    }

    #[test]
    fn nested_decisions_share_the_outer_host_plan() {
        let (file, core) = evaluation(
            "enum E { A, B }\nconst value = match (outer) { A => match (inner) { A => 1, B => 2 }, B => 0 };\n",
        );
        let plan = plan(&file, &core);
        let values = plan
            .owners()
            .flat_map(|owner| &owner.values)
            .collect::<Vec<_>>();
        assert_eq!(values.len(), 1, "{values:#?}");
        assert!(
            values.iter().any(|value| {
                value.context.continuation == HostContinuation::Initialize
                    && value.schedule.steps().is_empty()
            }),
            "{values:#?}"
        );
    }

    #[test]
    fn a_call_argument_is_composed_by_its_host_owner() {
        let (file, core) =
            evaluation("enum E { A, B }\nconst out = render(match (e) { A => 1, B => 2 });\n");
        let placement = &file
            .regions
            .iter()
            .find(|region| matches!(region.operation, OperationId::Decision(_)))
            .expect("decision region")
            .placement;
        assert!(matches!(
            placement,
            RegionPlacement::Host {
                context: EvaluationContext {
                    continuation: HostContinuation::Compose,
                    ..
                },
                ..
            }
        ));
        let plan = plan(&file, &core);
        let value = &plan.owners().next().expect("host rewrite").values[0];
        assert!(value.schedule.steps().iter().any(|step| matches!(
            step.operation,
            crate::program_syntax::HostEvaluationOperation::Eager(
                crate::program_syntax::EagerPosition::CallArgument(0)
            )
        )));
    }

    #[test]
    fn a_result_binding_is_nested_under_the_result_region() {
        let (file, _core) = evaluation("const out = result { const x <- load(); x };\n");
        let result = file
            .regions
            .iter()
            .find(|region| matches!(region.operation, OperationId::ResultRegion(_)))
            .expect("result region");
        let propagation = file
            .regions
            .iter()
            .find(|region| matches!(region.operation, OperationId::Propagate(_)))
            .expect("propagation region");
        assert_eq!(
            propagation.placement,
            RegionPlacement::Nested { parent: result.id }
        );
    }

    #[test]
    fn values_in_one_host_statement_form_one_rewrite() {
        let (file, core) = evaluation(
            "const out = [match (left) { A => 1, _ => 0 }, match (right) { B => 2, _ => 0 }];\n",
        );
        let plan = plan(&file, &core);
        let owners: Vec<_> = plan.owners().collect();
        assert_eq!(owners.len(), 1);
        assert_eq!(owners[0].values.len(), 2);
        let [ValueTarget::Slot(left), ValueTarget::Slot(right)] =
            [owners[0].values[0].target, owners[0].values[1].target];
        assert_ne!(left, right);
    }

    #[test]
    fn a_later_tt_value_depends_on_the_prior_value_slot() {
        let (file, core) = evaluation(
            "consume(match (left) { A => 1, _ => 0 }, match (right) { B => 2, _ => 0 });\n",
        );
        let plan = plan(&file, &core);
        let owner = plan.owners().next().expect("host rewrite");
        let ValueTarget::Slot(first) = owner.values[0].target;
        assert!(
            owner.values[1]
                .schedule
                .steps()
                .iter()
                .flat_map(|step| &step.inputs)
                .any(|input| matches!(
                    input,
                    PlannedEvaluationInput::Slot { slot, .. } if *slot == first
                ))
        );
    }

    #[test]
    fn generated_slot_names_do_not_collide_with_typescript_identifiers() {
        let (file, core) =
            evaluation("const $tt_v0 = 1;\nconst out = match (value) { A => $tt_v0, _ => 0 };\n");
        let plan = plan(&file, &core);
        let ValueTarget::Slot(slot) = plan.owners().next().expect("host rewrite").values[0].target;
        assert_eq!(plan.slot_name(slot), "$tt_v0_1");
    }

    #[test]
    fn validation_rejects_a_normal_path_without_its_result() {
        let (mut file, _core) = evaluation(
            "enum E { A, B }\nfunction f(e: E) { return match (e) { A => 1, B => 2 }; }\n",
        );
        let region = file
            .regions
            .iter_mut()
            .find(|region| matches!(region.operation, OperationId::Decision(_)))
            .expect("decision region");
        region.blocks[2].statements.clear();
        assert!(matches!(
            file.validate(),
            Err(EvaluationError::MissingResultDefinition { .. })
        ));
    }

    #[test]
    fn validation_rejects_an_out_of_region_target() {
        let (mut file, _core) = evaluation("enum E { A(value: number), B }\n");
        file.regions[0].blocks[0].terminator = EvalTerminator::Goto(EvalBlockId(u32::MAX));
        assert!(matches!(
            file.validate(),
            Err(EvaluationError::InvalidTarget { .. })
        ));
    }

    fn sole_value(plan: &LoweringPlan) -> &PlannedValue {
        let values: Vec<_> = plan.owners().flat_map(|owner| &owner.values).collect();
        assert_eq!(values.len(), 1, "{values:#?}");
        values[0]
    }

    #[test]
    fn a_loop_header_value_may_not_become_owner_statements() {
        let (file, core) = evaluation(
            "declare function id(v: number): number;\nlet n = 0;\nwhile (id(match (n) { 0 => 1, _ => 0 })) { n = n + 1; }\n",
        );
        let plan = plan(&file, &core);
        assert_eq!(
            sole_value(&plan).capability,
            TargetCapability::ExpressionBoundary(ExpressionBoundaryReason::RepeatedInOwner),
        );
    }

    #[test]
    fn a_loop_body_value_still_becomes_owner_statements() {
        let (file, core) = evaluation(
            "let n = 0;\nwhile (n < 3) { const v = match (n) { 0 => 1, _ => 0 }; n = n + v; }\n",
        );
        let plan = plan(&file, &core);
        assert_eq!(
            sole_value(&plan).capability,
            TargetCapability::StatementRegion
        );
    }

    #[test]
    fn a_switch_case_test_value_may_not_become_owner_statements() {
        let (file, core) = evaluation(
            "declare const n: number;\nswitch (n) { case match (n) { 1 => 1, _ => 0 }: break; }\n",
        );
        let plan = plan(&file, &core);
        assert_eq!(
            sole_value(&plan).capability,
            TargetCapability::ExpressionBoundary(ExpressionBoundaryReason::ConditionalInOwner),
        );
    }

    #[test]
    fn a_destructuring_default_value_may_not_become_owner_statements() {
        let (file, core) = evaluation(
            "declare const source: { value?: number };\nconst { value = match (1) { 1 => 1, _ => 0 } } = source;\n",
        );
        let plan = plan(&file, &core);
        assert_eq!(
            sole_value(&plan).capability,
            TargetCapability::ExpressionBoundary(ExpressionBoundaryReason::ConditionalInOwner),
        );
    }

    #[test]
    fn a_capture_may_not_move_into_a_generated_conditional_region() {
        // The callee capture `id` would be declared inside the generated
        // `if (flag)` region while the host expression reads it outside.
        let (file, core) = evaluation(
            "declare const flag: boolean;\ndeclare function id(v: number): number;\nexport const short = flag && id(match (flag) { true => 1, _ => 0 });\n",
        );
        let plan = plan(&file, &core);
        assert_eq!(
            sole_value(&plan).capability,
            TargetCapability::ExpressionBoundary(
                ExpressionBoundaryReason::CaptureInsideConditionalRegion
            ),
        );
    }

    #[test]
    fn a_capture_may_not_copy_a_tt_value_it_overlaps() {
        // The second value's prior-argument input contains the first tt
        // value; capturing it would copy tt source into generated code.
        let (file, core) = evaluation(
            "declare function g(x: unknown, y: unknown): void;\ndeclare const a: boolean;\ng(a && match (a) { true => 1, _ => 0 }, match (a) { true => 2, _ => 3 });\n",
        );
        let plan = plan(&file, &core);
        let values: Vec<_> = plan.owners().flat_map(|owner| &owner.values).collect();
        assert_eq!(values.len(), 2, "{values:#?}");
        assert_eq!(
            values[1].capability,
            TargetCapability::ExpressionBoundary(ExpressionBoundaryReason::CaptureOverlapsValue),
        );
    }

    #[test]
    fn a_parameter_default_value_uses_the_expression_boundary_capability() {
        let (file, core) = evaluation(
            "function f(x: number = match (1) { 1 => 1, _ => 0 }): number { return x; }\n",
        );
        let plan = plan(&file, &core);
        assert_eq!(
            sole_value(&plan).capability,
            TargetCapability::ExpressionBoundary(ExpressionBoundaryReason::OwnerTakesNoStatements),
        );
    }

    #[test]
    fn a_direct_optional_call_argument_still_becomes_owner_statements() {
        let (file, core) = evaluation(
            "declare const f: ((v: number) => number) | undefined;\nf?.(match (1) { 1 => 1, _ => 0 });\n",
        );
        let plan = plan(&file, &core);
        assert_eq!(
            sole_value(&plan).capability,
            TargetCapability::StatementRegion
        );
    }

    #[test]
    fn a_member_optional_call_argument_keeps_the_expression_boundary() {
        let (file, core) = evaluation(
            "declare const host: { f?: (v: number) => number };\nhost.f?.(match (1) { 1 => 1, _ => 0 });\n",
        );
        let plan = plan(&file, &core);
        assert_eq!(
            sole_value(&plan).capability,
            TargetCapability::ExpressionBoundary(ExpressionBoundaryReason::ReferenceNotPreservable),
        );
    }

    #[test]
    fn validate_order_rejects_a_repeated_value_planned_into_its_owner() {
        let (file, core) = evaluation(
            "declare function id(v: number): number;\nlet n = 0;\nwhile (id(match (n) { 0 => 1, _ => 0 })) { n = n + 1; }\n",
        );
        let mut plan = file.lowering_plan(&core).expect("lowering plan");
        // Break the decision the way a capability bug would: claim the loop
        // header's value may become statements in the loop's own owner.
        plan.owners[0].values[0].capability = TargetCapability::StatementRegion;
        let error = file.validate_order(&plan).expect_err("must be rejected");
        assert_eq!(error.invariant, crate::ice::Invariant::RepetitionRegionLeft);
        assert_eq!(error.stage, crate::ice::LoweringStage::EvaluationOrder);
    }

    #[test]
    fn validate_order_rejects_a_conditional_region_capture() {
        let (file, core) = evaluation(
            "declare const flag: boolean;\ndeclare function id(v: number): number;\nexport const short = flag && id(match (flag) { true => 1, _ => 0 });\n",
        );
        let mut plan = file.lowering_plan(&core).expect("lowering plan");
        plan.owners[0].values[0].capability = TargetCapability::StatementRegion;
        let error = file.validate_order(&plan).expect_err("must be rejected");
        assert_eq!(
            error.invariant,
            crate::ice::Invariant::ConditionalRegionLeft
        );
    }

    #[test]
    fn validate_order_rejects_a_capture_overlapping_a_tt_value() {
        let (file, core) = evaluation(
            "declare function g(x: unknown, y: unknown): void;\ndeclare const a: boolean;\ng(a && match (a) { true => 1, _ => 0 }, match (a) { true => 2, _ => 3 });\n",
        );
        let mut plan = file.lowering_plan(&core).expect("lowering plan");
        for value in &mut plan.owners[0].values {
            value.capability = TargetCapability::StatementRegion;
        }
        let error = file.validate_order(&plan).expect_err("must be rejected");
        assert_eq!(
            error.invariant,
            crate::ice::Invariant::EvaluationCountChanged
        );
    }

    #[test]
    fn validate_order_rejects_values_out_of_source_order() {
        let (file, core) = evaluation(
            "consume(match (left) { A => 1, _ => 0 }, match (right) { B => 2, _ => 0 });\n",
        );
        let mut plan = file.lowering_plan(&core).expect("lowering plan");
        plan.owners[0].values.swap(0, 1);
        let error = file.validate_order(&plan).expect_err("must be rejected");
        assert!(
            matches!(
                error.invariant,
                crate::ice::Invariant::EvaluationOrderChanged
                    | crate::ice::Invariant::ValueReadBeforeItIsProduced
            ),
            "{error}"
        );
    }

    #[test]
    fn validate_order_rejects_a_slot_read_before_it_is_produced() {
        let (file, core) = evaluation(
            "consume(match (left) { A => 1, _ => 0 }, match (right) { B => 2, _ => 0 });\n",
        );
        let mut plan = file.lowering_plan(&core).expect("lowering plan");
        // Point the second value's dependency at its own not-yet-produced slot.
        let ValueTarget::Slot(own) = plan.owners[0].values[1].target;
        for step in &mut plan.owners[0].values[1].schedule.steps {
            for input in &mut step.inputs {
                if let PlannedEvaluationInput::Slot { slot, .. } = input {
                    *slot = own;
                }
            }
        }
        let error = file.validate_order(&plan).expect_err("must be rejected");
        assert_eq!(
            error.invariant,
            crate::ice::Invariant::ValueReadBeforeItIsProduced
        );
    }

    #[test]
    fn validate_reference_rejects_a_receiverless_member_reference() {
        let (file, core) = evaluation(
            "declare const host: { f: (v: number) => number };\nhost.f(match (1) { 1 => 1, _ => 0 });\n",
        );
        let mut plan = file.lowering_plan(&core).expect("lowering plan");
        assert_eq!(
            plan.owners[0].values[0].capability,
            TargetCapability::StatementRegion
        );
        for step in &mut plan.owners[0].values[0].schedule.steps {
            for input in &mut step.inputs {
                if let PlannedEvaluationInput::Source {
                    mode: EvaluationInputMode::MemberReference,
                    receiver,
                    ..
                } = input
                {
                    *receiver = None;
                }
            }
        }
        let error = file
            .validate_reference(&plan)
            .expect_err("must be rejected");
        assert_eq!(error.invariant, crate::ice::Invariant::ReceiverLost);
        assert_eq!(error.stage, crate::ice::LoweringStage::EvaluationReference);
    }

    #[test]
    fn validate_reference_rejects_a_reference_demoted_to_a_value_slot() {
        let (file, core) = evaluation(
            "consume(match (left) { A => 1, _ => 0 }, match (right) { B => 2, _ => 0 });\n",
        );
        let mut plan = file.lowering_plan(&core).expect("lowering plan");
        for step in &mut plan.owners[0].values[1].schedule.steps {
            for input in &mut step.inputs {
                if let PlannedEvaluationInput::Slot { mode, .. } = input {
                    *mode = EvaluationInputMode::MemberReference;
                }
            }
        }
        let error = file
            .validate_reference(&plan)
            .expect_err("must be rejected");
        assert_eq!(error.invariant, crate::ice::Invariant::ReferenceDemoted);
    }
}
