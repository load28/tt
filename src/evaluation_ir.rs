//! Control-flow IR joining TypeScript host context with Core IR.
//!
//! Every Core operation receives a typed region and explicit block
//! termination and an owner-scoped value target consumed by target lowering.

use std::collections::{HashMap, HashSet};

use crate::core_ir::{
    ArmAction, CoreFile, Decision, ExitTarget, Expr, MissAction, Propagate, ResultRegionItem,
    Statement,
};
use crate::hir::ids::Idx;
use crate::hir::{BodyId, ExprId, NodeId};
use crate::ice::LoweringSubject;
use crate::program_syntax::{
    ConditionalBranch, ConditionalFacts, CoreRoot, EvaluationContext, EvaluationInputMode,
    EvaluationOwner, HostContinuation, HostEvaluationOperation, HostEvaluationProtocol, HostExit,
    HostOwner, OwnerReach, ProgramSyntax, SourceSpan, TtNodeId,
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
        source: Option<SourceSpan>,
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
    for_initializer_propagations: Vec<ForInitializerPropagation>,
    slot_names: Vec<String>,
    value_slots: HashMap<ExprId, ValueSlotId>,
    expression_boundary_name: String,
    result_return_name: String,
    unsupported_expression_propagations: Vec<UnsupportedExpressionPropagation>,
    unsupported_matches: Vec<UnsupportedMatch>,
}

/// A propagation declaration in a C-style `for` initializer. Its evaluation
/// and error exit are emitted before the loop; the header keeps its payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ForInitializerPropagation {
    pub(crate) node: NodeId,
    pub(crate) owner: HostOwner,
    pub(crate) source: SourceSpan,
}

/// A value-form propagation whose host cannot preserve its function return.
/// Keep the owner beside the capability reason so public diagnostics can name
/// the actual TypeScript boundary instead of flattening every case into one
/// generic expression error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnsupportedExpressionPropagation {
    pub(crate) expr: ExprId,
    pub(crate) source: SourceSpan,
    pub(crate) owner: EvaluationOwner,
    pub(crate) reason: ExpressionBoundaryReason,
}

/// A match whose host cannot carry a statement region without changing
/// JavaScript evaluation semantics. Match never falls back to an expression
/// closure; the public boundary turns this typed refusal into a placement
/// diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnsupportedMatch {
    pub(crate) expr: ExprId,
    pub(crate) source: SourceSpan,
    pub(crate) owner: EvaluationOwner,
    pub(crate) reason: ExpressionBoundaryReason,
}

#[derive(Debug)]
pub(crate) struct HostRewrite {
    pub(crate) owner: HostOwner,
    pub(crate) values: Vec<PlannedValue>,
    /// Conditional operations lowered as whole regions. A value consumed by
    /// one delivers into the operation's structure; the host occurrence of
    /// the *operation* is what gets replaced (결정 17).
    pub(crate) operations: Vec<PlannedConditionalOperation>,
}

/// One conditional TypeScript operation the target lowers as a whole:
/// evaluate the condition (or callee reference), branch, run the active
/// branch's evaluations in source order — tt regions included — and join
/// every path into one result slot. Never "promote the value and keep the
/// original conditional syntax": that loses the definite-assignment
/// correlation TypeScript needs (TASK-160 결정 17).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedConditionalOperation {
    /// The whole operation's source span — replaced by the result slot.
    pub(crate) parent: SourceSpan,
    /// Where every path of the operation writes its result.
    pub(crate) result: ValueSlotId,
    pub(crate) kind: PlannedConditionalKind,
    /// The condition (logical/ternary) or callee (optional call), with its
    /// capture slot and — for a member callee — its receiver.
    pub(crate) condition: PlannedEvaluationInput,
    /// The tt values the operation consumes, in source order.
    pub(crate) values: Vec<ExprId>,
    /// For a logical operation, the complete active branch and the
    /// evaluation steps between each consumed value and that branch. This
    /// lets the target rebuild `condition && wrapper(match ...)` as one
    /// region instead of requiring the match to be the entire branch.
    pub(crate) active_branch: Option<SourceSpan>,
    pub(crate) active_steps: Vec<PlannedEvaluationStep>,
    /// The evaluation steps outside this operation (its own host context),
    /// shared by every consumed value.
    pub(crate) outer: Vec<PlannedEvaluationStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlannedConditionalKind {
    /// `cond && value` — the inactive path's result is the condition value.
    LogicalAnd,
    /// `cond || value`.
    LogicalOr,
    /// `cond ?? value`.
    Nullish,
    /// `cond ? a : b` — each side is a tt region or relocated source.
    Ternary {
        consequent: PlannedBranch,
        alternate: PlannedBranch,
    },
    /// `callee?.(args)` — the arguments evaluate only past the nullish
    /// check, and a member callee calls through its receiver.
    OptionalCall {
        arguments: Vec<PlannedOperand>,
        type_args: Option<SourceSpan>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlannedBranch {
    /// A tt value delivering straight into the result slot.
    Value(ExprId),
    /// Original source, relocated into the branch.
    Source(SourceSpan),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlannedOperand {
    /// A tt value delivering into its own slot before the call.
    Value(ExprId),
    /// Original argument source. Arguments before the last tt value are
    /// captured (in order) before the values run; arguments after it are
    /// inlined into the rebuilt call, where they evaluate in place.
    Source {
        span: SourceSpan,
        spread: bool,
        /// The capture slot, when this argument evaluates before a value.
        capture: Option<ValueSlotId>,
    },
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
    /// The value sits under a conditional operation the lowering cannot own
    /// whole — nested conditionals, an evaluation between the operation and
    /// the value whose captures would escape the region, or a shape the
    /// rebuilt operation cannot reproduce (a member callee with explicit
    /// type arguments). Owning the whole operation is what removes this
    /// reason (결정 17) — never promoting the value under the original
    /// conditional syntax.
    ConditionalOperationNotStructurable,
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
    /// The whole-operation structure, carried from the protocol when the
    /// step is conditional ([`crate::program_syntax::ConditionalFacts`]).
    pub(crate) conditional: Option<ConditionalFacts>,
    pub(crate) loop_test: Option<crate::program_syntax::LoopTestFacts>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlannedEvaluationInput {
    Source {
        source: SourceSpan,
        mode: EvaluationInputMode,
        target: ValueSlotId,
        receiver: Option<PlannedReceiver>,
    },
    Slot {
        slot: ValueSlotId,
        mode: EvaluationInputMode,
    },
    /// An input whose evaluation is provably unobservable and stable
    /// ([`crate::program_syntax::Effects::is_inert`]) — a plain literal. It
    /// needs no capture: evaluating it at its original host position after
    /// the region changes no trace, no count, and no value. This is the
    /// proof-based capture elision of `docs/design/program-lowering.md` §9,
    /// decided here and only here — never re-derived by the target.
    Stable { source: SourceSpan },
}

/// How a member reference preserves its `this` receiver. A provably inert
/// receiver can be re-read when the captured callee is invoked; every other
/// receiver is evaluated once into its own slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlannedReceiver {
    Captured {
        source: SourceSpan,
        slot: ValueSlotId,
    },
    Stable {
        source: SourceSpan,
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
    receiver: Option<PlannedReceiver>,
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

    pub(crate) fn result_return_name(&self) -> &str {
        &self.result_return_name
    }

    pub(crate) fn for_initializer_propagations(
        &self,
    ) -> impl Iterator<Item = ForInitializerPropagation> + '_ {
        self.for_initializer_propagations.iter().copied()
    }

    /// Expression propagations whose host cannot carry the emitted early
    /// return. The source semantic layer consumes this typed placement fact;
    /// target emission never attempts an expression-boundary fallback for it.
    pub(crate) fn unsupported_expression_propagations(
        &self,
    ) -> Vec<UnsupportedExpressionPropagation> {
        self.unsupported_expression_propagations.clone()
    }

    pub(crate) fn unsupported_matches(&self) -> Vec<UnsupportedMatch> {
        self.unsupported_matches.clone()
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
    /// A Result expression used as a statement cannot preserve its failure
    /// completion. Reject it before target source-preservation would see an
    /// incomplete rewrite.
    DiscardedResult {
        source: SourceSpan,
    },
    /// A statement propagation in a loop header would be re-evaluated on
    /// each iteration if hoisted to its statement owner.
    RepeatedPropagation {
        source: SourceSpan,
    },
    /// Only a declaration initializer can retain its successful payload in a
    /// C-style `for` header. An assignment has no statement-safe rewrite.
    UnsupportedForInitializer {
        source: SourceSpan,
    },
}

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
            // A pipeline remains an expression when a nested value has
            // crossed into a separate source-backed owner, such as a
            // concise arrow step. Rewriting the outer Apply as statements
            // would move that value's propagation target out of the arrow.
            if matches!(core.exprs[expr.index()], Expr::Apply(_))
                && self.has_separately_hosted_descendant(core, expr)
            {
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
            ) {
                continue;
            }
            if region.result.is_none() || value_slots.contains_key(&expr) {
                continue;
            }
            let slot = allocate_value_slot(&mut next_slot, &mut slot_names, &mut occupied_names)?;
            value_slots.insert(expr, slot);
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
            while let RegionPlacement::Nested { parent, .. } = host_region.placement {
                host_region = &self.regions[parent.0 as usize];
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
            let reason = match (context.owner, capability) {
                (EvaluationOwner::FunctionBody, TargetCapability::StatementRegion) => continue,
                (_, TargetCapability::ExpressionBoundary(reason)) => reason,
                (_, TargetCapability::StatementRegion) => {
                    ExpressionBoundaryReason::OwnerTakesNoStatements
                }
            };
            let source = match &region.placement {
                RegionPlacement::Host { source, .. } => Some(*source),
                RegionPlacement::Nested { source, .. } => *source,
                RegionPlacement::SourceEdit => None,
            }
            .ok_or(EvaluationError::MissingHost {
                root: CoreRoot::Expr(expr),
            })?;
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
        let unsupported_matches = rewrites
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
        let expression_boundary_name = allocate_generated_name("$tt_expr", &mut occupied_names)?;
        let result_return_name = allocate_generated_name("$tt_result", &mut occupied_names)?;
        Ok(LoweringPlan {
            owners: rewrites,
            for_initializer_propagations,
            slot_names,
            value_slots,
            expression_boundary_name,
            result_return_name,
            unsupported_expression_propagations,
            unsupported_matches,
        })
    }

    fn has_separately_hosted_descendant(&self, core: &CoreFile, expr: ExprId) -> bool {
        self.expr_has_hosted_descendant(core, expr)
    }

    fn expr_has_hosted_descendant(&self, core: &CoreFile, expr: ExprId) -> bool {
        let nested = |child| {
            self.regions.iter().any(|region| {
                region.root == Some(CoreRoot::Expr(child))
                    && matches!(region.placement, RegionPlacement::Host { .. })
            }) || self.expr_has_hosted_descendant(core, child)
        };
        match &core.exprs[expr.index()] {
            Expr::Opaque(_) | Expr::Propagate(_) => false,
            Expr::Sequence(body) => self.body_has_hosted_descendant(core, *body),
            Expr::Decision(decision) => {
                decision
                    .subjects
                    .iter()
                    .any(|subject| nested(subject.value))
                    || decision.arms.iter().any(|arm| {
                        arm.guard.is_some_and(nested)
                            || match arm.action {
                                ArmAction::Yield { body, .. } | ArmAction::Execute(body) => {
                                    self.body_has_hosted_descendant(core, body)
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
                    self.body_has_hosted_descendant(core, *body)
                }) || region.value.is_some_and(nested)
            }
            Expr::Template(template) => template.parts.iter().any(|part| match part {
                crate::core_ir::TemplatePart::Raw(_) => false,
                crate::core_ir::TemplatePart::Interpolation(expr) => nested(*expr),
            }),
        }
    }

    fn body_has_hosted_descendant(&self, core: &CoreFile, body: BodyId) -> bool {
        core.bodies[body.index()]
            .statements
            .iter()
            .any(|statement| match statement {
                Statement::Expr(expr) => {
                    self.regions.iter().any(|region| {
                        region.root == Some(CoreRoot::Expr(*expr))
                            && matches!(region.placement, RegionPlacement::Host { .. })
                    }) || self.expr_has_hosted_descendant(core, *expr)
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
                conditional: step.conditional.clone(),
                loop_test: step.loop_test,
                inputs: step
                    .inputs
                    .iter()
                    .map(|input| {
                        slots.get(&input.source).map_or_else(
                            || {
                                // §9 capture elision: an inert value input
                                // is left in place — its only role here was
                                // order preservation, and evaluating it is
                                // unobservable.
                                if input.mode == EvaluationInputMode::Value
                                    && input.effects.is_inert()
                                {
                                    return Ok(PlannedEvaluationInput::Stable {
                                        source: input.source,
                                    });
                                }
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
                                    .map(|(source, effects)| {
                                        if effects.is_inert() {
                                            Ok(PlannedReceiver::Stable { source })
                                        } else {
                                            Ok(PlannedReceiver::Captured {
                                                source,
                                                slot: allocate_value_slot(
                                                    next_slot,
                                                    slot_names,
                                                    occupied_names,
                                                )?,
                                            })
                                        }
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

/// The sole conditional step of a value's schedule. Eager steps before it
/// belong to the active branch; steps after it belong to the operation's
/// outer host context.
fn whole_operation_step(schedule: &EvaluationSchedule) -> Option<(usize, &PlannedEvaluationStep)> {
    let steps = schedule.steps();
    let mut conditional = steps
        .iter()
        .enumerate()
        .filter(|(_, step)| matches!(step.operation, HostEvaluationOperation::Conditional(_)));
    let found = conditional.next()?;
    conditional.next().is_none().then_some(found)
}

/// Groups the owner's conditional-candidate values into whole conditional
/// operations (결정 17). A group that cannot form a complete operation is
/// downgraded to the expression boundary — never half-lowered.
fn plan_conditional_operations(
    values: &mut [PlannedValue],
    tt_spans: &[SourceSpan],
    next_slot: &mut u32,
    slot_names: &mut Vec<String>,
    occupied_names: &mut HashSet<String>,
) -> Result<Vec<PlannedConditionalOperation>, EvaluationError> {
    let mut order: Vec<SourceSpan> = Vec::new();
    let mut groups: HashMap<SourceSpan, Vec<usize>> = HashMap::new();
    for (index, value) in values.iter().enumerate() {
        if value.capability != TargetCapability::StatementRegion {
            continue;
        }
        let Some((_, step)) = whole_operation_step(&value.schedule) else {
            continue;
        };
        if !groups.contains_key(&step.parent) {
            order.push(step.parent);
        }
        groups.entry(step.parent).or_default().push(index);
    }
    let mut operations = Vec::with_capacity(order.len());
    for parent in order {
        let members = &groups[&parent];
        match plan_one_operation(
            values,
            members,
            parent,
            tt_spans,
            next_slot,
            slot_names,
            occupied_names,
        )? {
            Some(operation) => operations.push(operation),
            None => {
                for member in members {
                    values[*member].capability = TargetCapability::ExpressionBoundary(
                        ExpressionBoundaryReason::ConditionalOperationNotStructurable,
                    );
                }
            }
        }
    }
    Ok(operations)
}

fn plan_one_operation(
    values: &[PlannedValue],
    members: &[usize],
    parent: SourceSpan,
    tt_spans: &[SourceSpan],
    next_slot: &mut u32,
    slot_names: &mut Vec<String>,
    occupied_names: &mut HashSet<String>,
) -> Result<Option<PlannedConditionalOperation>, EvaluationError> {
    let first = &values[members[0]];
    let first_steps = first.schedule.steps();
    let Some((conditional_index, step)) = whole_operation_step(&first.schedule) else {
        return Ok(None);
    };
    // Every member shares the operation, so it must share the operation's
    // host context; a mismatch means the projection joined two different
    // operations to one span, and the group cannot be owned whole.
    if members.iter().any(|member| {
        let Some((member_index, _)) = whole_operation_step(&values[*member].schedule) else {
            return true;
        };
        member_index != conditional_index
            || values[*member].schedule.steps().len() != first_steps.len()
            || values[*member].schedule.steps()[conditional_index + 1..]
                != first_steps[conditional_index + 1..]
    }) {
        return Ok(None);
    }
    let Some(condition) = step.inputs.first().copied() else {
        return Ok(None);
    };
    let Some(facts) = step.conditional.clone() else {
        return Ok(None);
    };
    let overlaps_tt = |span: SourceSpan| tt_spans.iter().any(|tt| overlaps(span, *tt));
    let kind = match step.operation {
        HostEvaluationOperation::Conditional(ConditionalBranch::LogicalAndRight)
        | HostEvaluationOperation::Conditional(ConditionalBranch::LogicalOrRight)
        | HostEvaluationOperation::Conditional(ConditionalBranch::NullishRight) => {
            if members.len() != 1 {
                return Ok(None);
            }
            match step.operation {
                HostEvaluationOperation::Conditional(ConditionalBranch::LogicalAndRight) => {
                    PlannedConditionalKind::LogicalAnd
                }
                HostEvaluationOperation::Conditional(ConditionalBranch::LogicalOrRight) => {
                    PlannedConditionalKind::LogicalOr
                }
                _ => PlannedConditionalKind::Nullish,
            }
        }
        HostEvaluationOperation::Conditional(
            ConditionalBranch::Consequent | ConditionalBranch::Alternate,
        ) => {
            if conditional_index != 0 {
                return Ok(None);
            }
            let mut consequent = None;
            let mut alternate = None;
            for member in members {
                let value = &values[*member];
                let Some(member_facts) = &value.schedule.steps()[0].conditional else {
                    return Ok(None);
                };
                let side = match value.schedule.steps()[0].operation {
                    HostEvaluationOperation::Conditional(ConditionalBranch::Consequent) => {
                        &mut consequent
                    }
                    HostEvaluationOperation::Conditional(ConditionalBranch::Alternate) => {
                        &mut alternate
                    }
                    _ => return Ok(None),
                };
                if side.is_some() {
                    return Ok(None);
                }
                *side = Some((PlannedBranch::Value(value.expr), member_facts.skipped));
            }
            let fill = |taken: Option<(PlannedBranch, Option<SourceSpan>)>,
                        other: &Option<(PlannedBranch, Option<SourceSpan>)>|
             -> Option<PlannedBranch> {
                match taken {
                    Some((branch, _)) => Some(branch),
                    // The side with no tt value is the other member's
                    // skipped span — original source relocated into the
                    // branch, which must not contain tt of its own.
                    None => {
                        let (_, skipped) = other.as_ref()?;
                        let span = (*skipped)?;
                        (!overlaps_tt(span)).then_some(PlannedBranch::Source(span))
                    }
                }
            };
            let Some(consequent_branch) = fill(consequent, &alternate) else {
                return Ok(None);
            };
            let Some(alternate_branch) = fill(alternate, &consequent) else {
                return Ok(None);
            };
            PlannedConditionalKind::Ternary {
                consequent: consequent_branch,
                alternate: alternate_branch,
            }
        }
        HostEvaluationOperation::Conditional(ConditionalBranch::OptionalCallArgument(_)) => {
            if conditional_index != 0 {
                return Ok(None);
            }
            // A member callee calls through its captured receiver
            // (`callee.call(receiver, ...)`), which cannot carry explicit
            // type arguments.
            let member_callee = matches!(
                condition,
                PlannedEvaluationInput::Source {
                    mode: EvaluationInputMode::MemberReference,
                    ..
                }
            );
            if member_callee && facts.type_args.is_some() {
                return Ok(None);
            }
            let mut value_indices: HashMap<u32, ExprId> = HashMap::new();
            for member in members {
                let value = &values[*member];
                let HostEvaluationOperation::Conditional(ConditionalBranch::OptionalCallArgument(
                    index,
                )) = value.schedule.steps()[0].operation
                else {
                    return Ok(None);
                };
                if value_indices.insert(index, value.expr).is_some() {
                    return Ok(None);
                }
            }
            let last_value = *value_indices.keys().max().unwrap_or(&0);
            // Argument capture slots come from the members' planned inputs:
            // the schedule already assigned one to every argument that
            // evaluates before a value.
            // A captured argument answers with its slot; an inert one with
            // `None` — the rebuilt call inlines it, which is unobservable.
            let capture_of = |span: SourceSpan| {
                members.iter().find_map(|member| {
                    values[*member].schedule.steps()[0].inputs.iter().find_map(
                        |input| match input {
                            PlannedEvaluationInput::Source { source, target, .. }
                                if *source == span =>
                            {
                                Some(Some(*target))
                            }
                            PlannedEvaluationInput::Stable { source } if *source == span => {
                                Some(None)
                            }
                            _ => None,
                        },
                    )
                })
            };
            let mut arguments = Vec::with_capacity(facts.operands.len());
            for (index, operand) in facts.operands.iter().enumerate() {
                let index = u32::try_from(index).map_err(|_| EvaluationError::IdOverflow)?;
                match value_indices.get(&index) {
                    Some(expr) => arguments.push(PlannedOperand::Value(*expr)),
                    None => {
                        if overlaps_tt(operand.span) {
                            return Ok(None);
                        }
                        let capture = if index < last_value {
                            match capture_of(operand.span) {
                                Some(capture) => capture,
                                None => return Ok(None),
                            }
                        } else {
                            None
                        };
                        arguments.push(PlannedOperand::Source {
                            span: operand.span,
                            spread: operand.spread,
                            capture,
                        });
                    }
                }
            }
            PlannedConditionalKind::OptionalCall {
                arguments,
                type_args: facts.type_args,
            }
        }
        _ => return Ok(None),
    };
    let result = allocate_value_slot(next_slot, slot_names, occupied_names)?;
    let active_branch = matches!(
        &kind,
        PlannedConditionalKind::LogicalAnd
            | PlannedConditionalKind::LogicalOr
            | PlannedConditionalKind::Nullish
    )
    .then_some(facts.branch);
    Ok(Some(PlannedConditionalOperation {
        parent,
        result,
        kind,
        condition,
        values: members.iter().map(|member| values[*member].expr).collect(),
        active_branch,
        active_steps: first_steps[..conditional_index].to_vec(),
        outer: first_steps[conditional_index + 1..].to_vec(),
    }))
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
    source: SourceSpan,
    context: &EvaluationContext,
    schedule: &EvaluationSchedule,
) -> TargetCapability {
    use ExpressionBoundaryReason as Reason;
    if matches!(
        context.owner,
        EvaluationOwner::ParameterInitializer
            | EvaluationOwner::ClassInitializer
            | EvaluationOwner::StaticBlock
            | EvaluationOwner::Constructor
            | EvaluationOwner::Generator
    ) {
        return TargetCapability::ExpressionBoundary(Reason::OwnerTakesNoStatements);
    }
    if !core.has_statement_form(expr) {
        return TargetCapability::ExpressionBoundary(Reason::ValueHasNoStatementForm);
    }
    match context.owner_reach {
        OwnerReach::Same => {}
        OwnerReach::Repeated => {
            if !matches!(core.exprs[expr.index()], Expr::Decision(_)) {
                return TargetCapability::ExpressionBoundary(Reason::RepeatedInOwner);
            }
            let loop_steps = schedule
                .steps()
                .iter()
                .filter(|step| step.operation == HostEvaluationOperation::LoopTest)
                .count();
            if loop_steps != 1
                || schedule.steps().last().is_none_or(|step| {
                    step.operation != HostEvaluationOperation::LoopTest || step.loop_test.is_none()
                })
            {
                return TargetCapability::ExpressionBoundary(Reason::RepeatedInOwner);
            }
        }
        OwnerReach::UnmodeledConditional => {
            return TargetCapability::ExpressionBoundary(Reason::ConditionalInOwner);
        }
    }
    let steps = schedule.steps();
    for step in steps {
        for input in &step.inputs {
            match input {
                PlannedEvaluationInput::Source {
                    mode: EvaluationInputMode::MemberReference,
                    receiver: None,
                    ..
                }
                | PlannedEvaluationInput::Slot {
                    mode: EvaluationInputMode::MemberReference,
                    ..
                } => {
                    return TargetCapability::ExpressionBoundary(Reason::ReferenceNotPreservable);
                }
                PlannedEvaluationInput::Source { .. }
                | PlannedEvaluationInput::Slot { .. }
                | PlannedEvaluationInput::Stable { .. } => {}
            }
        }
    }
    // A conditional step is lowerable only when the whole operation can be
    // owned as one region. Exactly one conditional boundary is allowed;
    // eager steps inside its active branch are kept there by the operation
    // plan. Anything else takes the boundary — never a promoted value under
    // the original syntax.
    let conditional_steps = steps
        .iter()
        .filter(|step| matches!(step.operation, HostEvaluationOperation::Conditional(_)))
        .count();
    if conditional_steps > 0 {
        let Some((_, step)) = whole_operation_step(schedule) else {
            return TargetCapability::ExpressionBoundary(
                Reason::ConditionalOperationNotStructurable,
            );
        };
        let structurable = step.conditional.as_ref().is_some_and(|facts| {
            facts.branch.start <= source.start && source.end <= facts.branch.end
        });
        if !structurable {
            return TargetCapability::ExpressionBoundary(
                Reason::ConditionalOperationNotStructurable,
            );
        }
    }
    let mut captured: Vec<SourceSpan> = Vec::new();
    for step in steps {
        for input in &step.inputs {
            let PlannedEvaluationInput::Source {
                source: capture, ..
            } = input
            else {
                continue;
            };
            if captured.contains(capture) {
                continue;
            }
            // The capture copies raw source bytes; a tt node or another
            // capture inside them is lowered or relocated elsewhere.
            if tt_spans.iter().any(|span| overlaps(*capture, *span))
                || captured.iter().any(|span| overlaps(*capture, *span))
            {
                return TargetCapability::ExpressionBoundary(Reason::CaptureOverlapsValue);
            }
            captured.push(*capture);
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
            let source = self.hosts.remove(&root).map(|binding| binding.source);
            RegionPlacement::Nested { parent, source }
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
            let source = self.hosts.remove(&root).map(|binding| binding.source);
            return Ok(RegionPlacement::Nested { parent, source });
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
            "variant E { A(value: number), B }\n\
             import { load } from \"./load.tt\";\n\
             function f(e: E) {\n\
               try load();\n\
               return result { const x = try load(); return match (e) { A(value) => x + value, B => 0 } |> done; };\n\
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
            "variant E { A, B }\nfunction f(e: E) { return match (e) { A => 1, B => 2 }; }\n",
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
            "variant E { A, B }\nconst value = match (outer) { A => match (inner) { A => 1, B => 2 }, B => 0 };\n",
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
            evaluation("variant E { A, B }\nconst out = render(match (e) { A => 1, B => 2 });\n");
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
        let (file, _core) = evaluation("const out = result { const x = try load(); return x; };\n");
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
            RegionPlacement::Nested {
                parent: result.id,
                source: Some(SourceSpan { start: 21, end: 42 }),
            }
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
            "variant E { A, B }\nfunction f(e: E) { return match (e) { A => 1, B => 2 }; }\n",
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
        let (mut file, _core) = evaluation("variant E { A(value: number), B }\n");
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
    fn a_while_test_value_becomes_a_repeated_owner_region() {
        let (file, core) = evaluation(
            "declare function id(v: number): number;\nlet n = 0;\nwhile (id(match (n) { 0 => 1, _ => 0 })) { n = n + 1; }\n",
        );
        let plan = plan(&file, &core);
        assert_eq!(
            sole_value(&plan).capability,
            TargetCapability::StatementRegion,
            "{:#?}",
            sole_value(&plan).schedule,
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
    fn a_conditional_operation_owns_its_complete_active_branch() {
        // The `id` call sits between the `&&` and the value. The operation
        // owns that complete active branch so its captures stay inside the
        // conditional region.
        let (file, core) = evaluation(
            "declare const flag: boolean;\ndeclare function id(v: number): number;\nexport const short = flag && id(match (flag) { true => 1, _ => 0 });\n",
        );
        let plan = plan(&file, &core);
        assert_eq!(
            sole_value(&plan).capability,
            TargetCapability::StatementRegion,
        );
        let operation = &plan.owners[0].operations[0];
        assert!(operation.active_branch.is_some());
        assert_eq!(operation.active_steps.len(), 1);
    }

    #[test]
    fn a_direct_conditional_branch_becomes_a_whole_operation() {
        let (file, core) = evaluation(
            "declare const flag: boolean;\nexport const short = flag && match (flag) { true => 1, _ => 0 };\n",
        );
        let plan = plan(&file, &core);
        assert_eq!(
            sole_value(&plan).capability,
            TargetCapability::StatementRegion
        );
        let operations: Vec<_> = plan.owners().flat_map(|owner| &owner.operations).collect();
        assert_eq!(operations.len(), 1, "{operations:#?}");
        assert_eq!(operations[0].kind, PlannedConditionalKind::LogicalAnd);
    }

    #[test]
    fn both_ternary_branches_join_one_operation() {
        let (file, core) = evaluation(
            "declare const flag: boolean;\nexport const pick = flag ? match (1) { 1 => 1, _ => 0 } : match (2) { 2 => 2, _ => 0 };\n",
        );
        let plan = plan(&file, &core);
        let operations: Vec<_> = plan.owners().flat_map(|owner| &owner.operations).collect();
        assert_eq!(operations.len(), 1, "{operations:#?}");
        assert!(matches!(
            operations[0].kind,
            PlannedConditionalKind::Ternary {
                consequent: PlannedBranch::Value(_),
                alternate: PlannedBranch::Value(_),
            }
        ));
        assert_eq!(operations[0].values.len(), 2);
    }

    #[test]
    fn a_later_value_depends_on_the_prior_conditional_operation_slot() {
        // The second value's prior argument contains the first tt value.
        // The complete conditional operation produces a slot, so the later
        // schedule never copies tt source.
        let (file, core) = evaluation(
            "declare function g(x: unknown, y: unknown): void;\ndeclare const a: boolean;\ng(a && match (a) { true => 1, _ => 0 }, match (a) { true => 2, _ => 3 });\n",
        );
        let plan = plan(&file, &core);
        let values: Vec<_> = plan.owners().flat_map(|owner| &owner.values).collect();
        assert_eq!(values.len(), 2, "{values:#?}");
        assert_eq!(values[1].capability, TargetCapability::StatementRegion,);
        let operation = &plan.owners[0].operations[0];
        assert!(values[1].schedule.steps().iter().any(|step| {
            step.inputs.iter().any(|input| {
                matches!(input, PlannedEvaluationInput::Slot { slot, .. } if *slot == operation.result)
            })
        }));
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
    fn a_member_optional_call_becomes_a_whole_operation_through_its_receiver() {
        let (file, core) = evaluation(
            "declare const host: { f?: (v: number) => number };\nhost.f?.(match (1) { 1 => 1, _ => 0 });\n",
        );
        let plan = plan(&file, &core);
        assert_eq!(
            sole_value(&plan).capability,
            TargetCapability::StatementRegion
        );
        let operations: Vec<_> = plan.owners().flat_map(|owner| &owner.operations).collect();
        assert_eq!(operations.len(), 1, "{operations:#?}");
        assert!(matches!(
            operations[0].condition,
            PlannedEvaluationInput::Source {
                mode: EvaluationInputMode::MemberReference,
                receiver: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn an_inert_input_needs_no_capture_but_an_effectful_one_does() {
        // §9 capture elision: a literal's evaluation is unobservable, so it
        // stays in place; a call may do anything, so it is captured to
        // preserve its order against the hoisted region.
        let (file, core) = evaluation(
            "declare function g(a: number, b: number): void;\ndeclare function eff(): number;\ng(1, match (1) { 1 => 1, _ => 0 });\ng(eff(), match (1) { 1 => 2, _ => 0 });\n",
        );
        let plan = plan(&file, &core);
        let inputs: Vec<_> = plan
            .owners()
            .flat_map(|owner| &owner.values)
            .flat_map(|value| value.schedule.steps())
            .flat_map(|step| &step.inputs)
            .collect();
        assert!(
            inputs
                .iter()
                .any(|input| matches!(input, PlannedEvaluationInput::Stable { .. })),
            "{inputs:#?}"
        );
        assert!(
            inputs.iter().any(|input| matches!(
                input,
                PlannedEvaluationInput::Source {
                    mode: EvaluationInputMode::Value,
                    ..
                }
            )),
            "{inputs:#?}"
        );
    }

    #[test]
    fn validate_order_rejects_a_repeated_value_planned_into_its_owner() {
        let (file, core) = evaluation(
            "declare function id(v: number): number;\nlet n = 0;\nwhile (id(match (n) { 0 => 1, _ => 0 })) { n = n + 1; }\n",
        );
        let mut plan = file.lowering_plan(&core).expect("lowering plan");
        // Break the plan the way a protocol bug would: keep the statement
        // capability but drop the loop operation that preserves repetition.
        plan.owners[0].values[0]
            .schedule
            .steps
            .retain(|step| step.operation != HostEvaluationOperation::LoopTest);
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
        // Break the plan by removing the operation that owns the active
        // branch while leaving its value statement-capable.
        plan.owners[0].operations.clear();
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
        // Break the dependency edge by turning the prior operation's slot
        // back into a raw source capture containing tt syntax.
        let operation = plan.owners[0].operations[0].clone();
        for step in &mut plan.owners[0].values[1].schedule.steps {
            for input in &mut step.inputs {
                let PlannedEvaluationInput::Slot { slot, mode } = *input else {
                    continue;
                };
                if slot == operation.result {
                    *input = PlannedEvaluationInput::Source {
                        source: operation.parent,
                        mode,
                        target: slot,
                        receiver: None,
                    };
                }
            }
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
        // Isolate the ordinal contract: drop the slot dependency so only
        // the source-order inversion remains for the validator to see.
        for value in &mut plan.owners[0].values {
            for step in &mut value.schedule.steps {
                step.inputs
                    .retain(|input| !matches!(input, PlannedEvaluationInput::Slot { .. }));
            }
        }
        let error = file.validate_order(&plan).expect_err("must be rejected");
        assert_eq!(
            error.invariant,
            crate::ice::Invariant::EvaluationOrderChanged,
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
