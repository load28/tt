//! Control-flow IR joining TypeScript host context with Core IR.
//!
//! Every Core operation receives a typed region and explicit block
//! termination and an owner-scoped value target consumed by target lowering.

mod builder;
mod evaluation;
mod planning;
mod validation;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use crate::core_ir::{
    ArmAction, CoreFile, Decision, DecisionKind, ExitTarget, Expr, MissAction, Propagate,
    ResultRegionItem, Statement,
};
use crate::hir::ids::Idx;
use crate::hir::{BindingMode, BodyId, ExprId, NodeId};
use crate::ice::LoweringSubject;
use crate::program_syntax::{
    ConditionalBranch, ConditionalFacts, CoreRoot, EagerPosition, EvaluationContext,
    EvaluationInputMode, EvaluationOwner, GlobalStatement, HostContinuation,
    HostEvaluationOperation, HostEvaluationProtocol, HostExit, HostOwner, OptionalCallTest,
    OwnerReach, ProgramSyntax, SourceSpan, TtNodeId,
};

use builder::*;
use planning::*;

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
        exits: Vec<HostExit>,
        protocol: HostEvaluationProtocol,
        context: Option<EvaluationContext>,
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
    declared_names: HashSet<String>,
    module_declared_names: HashSet<String>,
    directive_prologue_end: Option<usize>,
    /// Source spans of every tt node in the file. A schedule's source
    /// capture must not overlap one: the capture copies raw source bytes,
    /// and a tt node inside them is lowered elsewhere.
    tt_spans: Vec<SourceSpan>,
    script: bool,
    commonjs: bool,
    globals: HashMap<SourceSpan, GlobalStatement>,
}

#[derive(Debug, Default)]
pub(crate) struct LoweringPlan {
    owners: Vec<HostRewrite>,
    for_initializer_propagations: Vec<ForInitializerPropagation>,
    slot_names: Vec<String>,
    /// Earlier materializations substituted when an enclosing source is captured.
    capture_dependencies: HashMap<ValueSlotId, Vec<(SourceSpan, ValueSlotId)>>,
    value_slots: HashMap<ExprId, ValueSlotId>,
    piped_slots: HashMap<ExprId, Vec<ValueSlotId>>,
    nested_exits: HashMap<ExprId, Vec<HostExit>>,
    nested_schedules: HashMap<ExprId, EvaluationSchedule>,
    nested_operations: Vec<PlannedConditionalOperation>,
    nested_values: HashSet<ExprId>,
    structurally_owned_children: HashSet<ExprId>,
    nested_relocations: Vec<SourceSpan>,
    expression_boundary_name: String,
    match_raise_name: String,
    match_show_name: String,
    generated_names: Option<crate::generated_names::GeneratedNames>,
    shadowed_globals: HashSet<String>,
    host_global_aliases: HashMap<String, HostGlobalAlias>,
    directive_prologue_end: Option<usize>,
    match_subject_names: HashMap<ExprId, Vec<String>>,
    unsupported_expression_propagations: Vec<UnsupportedExpressionPropagation>,
    unsupported_matches: Vec<UnsupportedMatch>,
    block_required_statements: HashSet<NodeId>,
    block_required_owners: HashSet<SourceSpan>,
    lexical_declaration_bodies: Vec<LexicalDeclarationBody>,
    ambient_items: HashSet<NodeId>,
    script: bool,
    commonjs: bool,
    global_temps: HashMap<crate::core_ir::TempId, String>,
    owner_model_unavailable: bool,
    /// What TypeScript's completion rules say at each construct's place,
    /// for the editor ([`crate::program_syntax::CompletionScope`]).
    pub(crate) completion_scopes: Vec<crate::program_syntax::CompletionScope>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostGlobalAlias {
    pub(crate) name: String,
    pub(crate) capture: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LexicalDeclarationBody {
    pub(crate) source: SourceSpan,
    pub(crate) statement: BindingStatement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BindingStatement {
    LetElse,
    Try,
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
    /// For each consumed value that is not a whole branch or argument by
    /// itself, the branch or argument holding it and the evaluation steps
    /// between the value and that branch. This lets the target rebuild
    /// `condition && wrapper(match ...)`, `c ? (try a) + (try b) : d`, and
    /// `f?.(try r * 2)` as one region instead of requiring each value to be
    /// the entire branch or argument.
    pub(crate) active: Vec<PlannedActiveBranch>,
    /// The evaluation steps outside this operation (its own host context),
    /// shared by every consumed value.
    pub(crate) outer: Vec<PlannedEvaluationStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedActiveBranch {
    pub(crate) value: ExprId,
    pub(crate) branch: SourceSpan,
    pub(crate) steps: Vec<PlannedEvaluationStep>,
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
    /// `callee?.(args)` or `receiver?.name(args)` — the arguments evaluate
    /// only past the nullish check of the input `test` names, and a member
    /// callee calls through its receiver.
    OptionalCall {
        arguments: Vec<PlannedOperand>,
        type_args: Option<SourceSpan>,
        test: OptionalCallTest,
    },
    LogicalAssignment {
        operator: crate::program_syntax::LogicalAssignment,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlannedBranch {
    /// The tt values of the branch, in source order: one delivering straight
    /// into the result slot, or several (or one inside a larger branch)
    /// evaluated in order before the branch is rebuilt from their slots.
    Values(Vec<ExprId>),
    /// Original source, relocated into the branch.
    Source(SourceSpan),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlannedOperand {
    /// A tt value delivering into its own slot before the call.
    Value(ExprId),
    /// An argument holding tt values inside a larger expression: the values
    /// evaluate, in source order, into their slots before the call, and the
    /// argument is rebuilt from its source around them.
    Composed {
        span: SourceSpan,
        spread: bool,
        values: Vec<ExprId>,
    },
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
    /// A parameter default, class field initializer, class definition
    /// position (a decorator, a computed member name, or a decorated class's
    /// heritage), or enum member initializer: standard TypeScript has no
    /// statement position in the owner, and moving the value out of it would
    /// change the parameter scope, `this`, `arguments`, the function's
    /// `length`, the field or class definition evaluation order, or which
    /// binding an enum member's name denotes and when the member is
    /// evaluated.
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
    LoopHeadDeclarator,
    LoopHeadBinding,
    LogicalAssignmentValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct EvaluationSchedule {
    /// Optional host-call completion carried from the syntax proof.
    pub(crate) call_completion: Option<PlannedCallCompletion>,
    steps: Vec<PlannedEvaluationStep>,
}

/// A syntax-proven completable call with its generated-name reservations
/// ([`crate::program_syntax::CallCompletionFacts`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlannedCallCompletion {
    pub(crate) facts: crate::program_syntax::CallCompletionFacts,
    /// The slot that holds the captured callee instantiated with the
    /// authored type arguments, when the call carries them. Instantiating
    /// once keeps one source-mapped copy of the type arguments while every
    /// dispatch arm calls through the instantiated binding.
    pub(crate) instantiated: Option<ValueSlotId>,
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
        /// A member reference's computed key, evaluated after its receiver.
        key: Option<PlannedReceiver>,
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
    ///
    /// `reserved` holds a generated name for the one lowering that cannot
    /// leave the input in place: a completed call re-emits itself inside the
    /// match's dispatch, where the authored position no longer exists. That
    /// lowering captures the input under this name instead of copying its
    /// source into every arm; every other lowering ignores the reservation
    /// and emits nothing for it.
    Stable {
        source: SourceSpan,
        reserved: Option<ValueSlotId>,
    },
}

/// How a part of a member reference — its receiver or its computed key — is
/// evaluated before the call's arguments. The member itself is read where
/// the call is made, through the receiver, so the call keeps `this`, a
/// generic method's inference, and the receiver's narrowing. A provably
/// inert part, or an authored identifier or `this`
/// ([`crate::program_syntax::HostReferencePart::read_at_call`]), is read
/// again at the call; every other part is evaluated once into its own slot.
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
    key: Option<PlannedReceiver>,
}

impl LoweringPlan {
    /// The plan of a file whose TypeScript gives no owner model. The helpers
    /// a lowering without owners still calls are named against the names
    /// the source already uses, as a built plan names them.
    pub(crate) fn without_owner_model(source: &str, source_kind: crate::SourceKind) -> Self {
        let written = crate::generated_names::source_names(source, source_kind);
        let mut occupied = written.clone();
        let mut name = |base: &str| {
            crate::generated_names::allocate(base, &mut occupied)
                .unwrap_or_else(|| crate::ice::bug!("no free generated name remains for {base}"))
        };
        let expression_boundary_name = name("$tt_expr");
        let match_raise_name = name("$tt_raise");
        let match_show_name = name("$tt_show");
        let allocated = occupied.difference(&written).cloned().collect();
        Self {
            owner_model_unavailable: true,
            expression_boundary_name,
            match_raise_name,
            match_show_name,
            generated_names: Some(crate::generated_names::GeneratedNames::from_occupied(
                occupied, allocated,
            )),
            ..Self::default()
        }
    }

    pub(crate) fn has_owner_model(&self) -> bool {
        !self.owner_model_unavailable
    }

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

    pub(crate) fn piped_slot_names(&self) -> impl Iterator<Item = (ExprId, Vec<String>)> + '_ {
        self.piped_slots.iter().map(|(expr, slots)| {
            (
                *expr,
                slots
                    .iter()
                    .map(|slot| self.slot_name(*slot).to_owned())
                    .collect(),
            )
        })
    }

    pub(crate) fn nested_value_exits(&self) -> impl Iterator<Item = (ExprId, &[HostExit])> {
        self.nested_exits
            .iter()
            .map(|(expr, exits)| (*expr, exits.as_slice()))
    }

    pub(crate) fn nested_value_schedules(
        &self,
    ) -> impl Iterator<Item = (ExprId, &EvaluationSchedule)> {
        self.nested_schedules
            .iter()
            .map(|(expr, schedule)| (*expr, schedule))
    }

    pub(crate) fn nested_values(&self) -> impl Iterator<Item = ExprId> + '_ {
        self.nested_values.iter().copied()
    }

    pub(crate) fn nested_operations(&self) -> &[PlannedConditionalOperation] {
        &self.nested_operations
    }

    pub(crate) fn structurally_owned_children(&self) -> impl Iterator<Item = ExprId> + '_ {
        self.structurally_owned_children.iter().copied()
    }

    pub(crate) fn nested_relocations(&self) -> impl Iterator<Item = SourceSpan> + '_ {
        self.nested_relocations.iter().copied()
    }

    pub(crate) fn slots(&self) -> impl Iterator<Item = (ValueSlotId, &str)> {
        (0u32..)
            .zip(&self.slot_names)
            .map(|(index, name)| (ValueSlotId(index), name.as_str()))
    }

    pub(crate) fn expression_boundary_name(&self) -> &str {
        &self.expression_boundary_name
    }

    pub(crate) fn match_raise_name(&self) -> &str {
        &self.match_raise_name
    }

    pub(crate) fn match_show_name(&self) -> &str {
        &self.match_show_name
    }

    pub(crate) fn host_global(&self, name: &str) -> String {
        if let Some(alias) = self.host_global_aliases.get(name) {
            alias.name.clone()
        } else if self.shadowed_globals.contains(name) {
            format!("globalThis.{name}")
        } else {
            name.to_owned()
        }
    }

    pub(crate) fn host_global_alias(&self, name: &str) -> Option<&HostGlobalAlias> {
        self.host_global_aliases.get(name)
    }

    pub(crate) fn directive_prologue_end(&self) -> Option<usize> {
        self.directive_prologue_end
    }

    pub(crate) fn generated_names(&self) -> Option<&crate::generated_names::GeneratedNames> {
        self.generated_names.as_ref()
    }

    pub(crate) fn match_subject_names(&self, expr: ExprId) -> &[String] {
        self.match_subject_names
            .get(&expr)
            .map(Vec::as_slice)
            .unwrap_or_default()
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

    pub(crate) fn block_required_statements(&self) -> &HashSet<NodeId> {
        &self.block_required_statements
    }

    /// Host owners, by their anchor, that are the unbraced body of an `if`,
    /// loop, label, or `with`: a prelude hoisted in front of one has to open
    /// a block there so the owner stays a single statement under its parent.
    pub(crate) fn block_required_owners(&self) -> &HashSet<SourceSpan> {
        &self.block_required_owners
    }

    pub(crate) fn ambient_items(&self) -> &HashSet<NodeId> {
        &self.ambient_items
    }

    pub(crate) fn unsupported_matches(&self) -> Vec<UnsupportedMatch> {
        self.unsupported_matches.clone()
    }

    pub(crate) fn lexical_declaration_bodies(&self) -> &[LexicalDeclarationBody] {
        &self.lexical_declaration_bodies
    }

    pub(crate) fn is_script(&self) -> bool {
        self.script
    }

    pub(crate) fn uses_commonjs_syntax(&self) -> bool {
        self.commonjs
    }

    pub(crate) fn global_temps(&self) -> &HashMap<crate::core_ir::TempId, String> {
        &self.global_temps
    }
}

impl std::fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reason = match self {
            EvaluationError::DuplicateHost { .. } => "the construct has two TypeScript hosts",
            EvaluationError::MissingHost { .. } => "the construct has no TypeScript host",
            EvaluationError::OrphanHost { .. } => "a TypeScript host has no construct",
            EvaluationError::DuplicateOperation { .. } => "the construct is planned twice",
            EvaluationError::IdOverflow => "too many evaluation regions in one file",
            EvaluationError::GeneratedNameOverflow => "no free generated name remains",
            EvaluationError::InvalidEntry { .. } => "an evaluation region has no entry",
            EvaluationError::InvalidTarget { .. } => "an evaluation edge has no target",
            EvaluationError::UnreachableBlock { .. } => "an evaluation block is unreachable",
            EvaluationError::CoreOperationMismatch { .. } => {
                "an evaluation region disagrees with its construct"
            }
            EvaluationError::MissingResultDefinition { .. } => {
                "an evaluation path delivers no value"
            }
            EvaluationError::UnexpectedResultDefinition { .. } => {
                "an evaluation path delivers a value it does not own"
            }
            EvaluationError::InvalidHostOwner { .. } => "the construct's host owner is unknown",
            EvaluationError::DiscardedResult { .. } => "a Result value is discarded",
            EvaluationError::RepeatedPropagation { .. } => {
                "a propagation would repeat in its loop header"
            }
        };
        f.write_str(reason)
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
}

impl EvaluationError {
    pub(crate) fn source(&self) -> Option<SourceSpan> {
        match self {
            EvaluationError::DiscardedResult { source }
            | EvaluationError::RepeatedPropagation { source } => Some(*source),
            EvaluationError::InvalidHostOwner { value, .. } => Some(*value),
            _ => None,
        }
    }
}
