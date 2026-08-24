//! Whole-program TypeScript syntax for tt-aware target lowering.
//!
//! The lossless tt parser remains authoritative for claiming tt syntax.
//! This module projects Core IR primitives to category-preserving TypeScript
//! placeholders, parses the complete projection with SWC, and joins every
//! placeholder to its exact SWC parent path and stable minimum host owner.
//!
//! SWC is the compiler's in-process TypeScript **syntax substrate**, not a
//! substitute for TypeScript's type checker. Its whole-program AST supplies
//! the parent/owner/evaluation structure that lowering must retain while it
//! rewrites a tt value. Sending that work to the TypeScript 7 backend would
//! turn a local compiler invariant into an external semantic-service call and
//! would duplicate the source-preserving target model maintained here.
//!
//! TypeScript 7 has a deliberately narrower boundary: the compiler asks it
//! only for facts that syntax cannot prove, such as inferred types, narrowing,
//! and symbol identity (`crate::typescript`). Requiring that backend as part
//! of the toolchain does not transfer syntax ownership away from this SWC AST.

use std::collections::{HashMap, HashSet};

use swc_common::input::StringInput;
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap, Spanned};
use swc_ecma_ast::{
    ArrayLit, ArrowExpr, AssignExpr, AwaitExpr, BinExpr, BinaryOp, CallExpr, CondExpr, Constructor,
    Function, Ident, JSXAttrOrSpread, JSXAttrValue, JSXElement, JSXElementChild, JSXExpr,
    JSXFragment, MemberExpr, MemberProp, Module, ModuleItem, NewExpr, ObjectLit, OptCall, Prop,
    PropName, PropOrSpread, ReturnStmt, SeqExpr, Stmt, TaggedTpl, Tpl, UnaryExpr, YieldExpr,
};
use swc_ecma_parser::lexer::Lexer;
use swc_ecma_parser::{Parser, Syntax, TsSyntax};
use swc_ecma_visit::{AstNodePath, AstParentKind, VisitAstPath, VisitWithAstPath, fields};

use crate::analysis::SemanticFile;
use crate::core_ir::{
    Adt, Apply, CoreFile, Decision, Expr, Import, Propagate, ResultRegion, Statement, Template,
    TemplatePart,
};
use crate::hir::ids::Idx;
use crate::hir::{self, BodyId, ExprId, NodeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Stable identity assigned to an TT node in the projected syntax overlay.
pub(crate) struct TtNodeId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Byte coordinate in the original source buffer.
pub(crate) struct SourceByte(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ProjectedByte(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SourceSpan {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl From<hir::Span> for SourceSpan {
    fn from(span: hir::Span) -> Self {
        Self {
            start: span.start,
            end: span.end,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ProjectedSpan {
    start: ProjectedByte,
    end: ProjectedByte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SyntaxCategory {
    Expression,
    Statement,
    Item,
}

#[derive(Debug)]
struct OverlayEntry {
    id: TtNodeId,
    category: SyntaxCategory,
    source: SourceSpan,
    projected: ProjectedSpan,
    parents: Vec<AstParentKind>,
    context: EvaluationContext,
    protocol: HostEvaluationProtocol,
    core_root: CoreRoot,
    host_owner: HostOwner,
    exits: Vec<HostExit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HostExit {
    pub(crate) statement: SourceSpan,
    pub(crate) argument: Option<SourceSpan>,
    /// Whether the exit sits inside a statement that consumes an unlabeled
    /// `break` — a loop or a `switch` written in the arm body. The rewrite
    /// turns the `return` into a `break`, so such an exit is the only
    /// reason a value region needs a label: everywhere else the region's
    /// own dispatch is already the nearest `break` target.
    pub(crate) captured_break: bool,
}

/// Ordered JavaScript evaluation obligations between one TT value and its
/// minimum source-backed owner. Target lowering must consume every step.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct HostEvaluationProtocol {
    steps: Vec<HostEvaluationStep>,
}

impl HostEvaluationProtocol {
    pub(crate) fn steps(&self) -> &[HostEvaluationStep] {
        &self.steps
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostEvaluationStep {
    pub(crate) parent: SourceSpan,
    pub(crate) operation: HostEvaluationOperation,
    pub(crate) inputs: Vec<HostEvaluationInput>,
    /// The structure of the conditional operation this step belongs to,
    /// when [`HostEvaluationStep::operation`] is a
    /// [`HostEvaluationOperation::Conditional`] — everything lowering needs
    /// to restructure the *whole* operation rather than just its inputs.
    pub(crate) conditional: Option<ConditionalFacts>,
}

/// The complete syntactic structure of one conditional operation, read off
/// the SWC AST: the branch the tt value sits in, the branch the operation
/// skips, and (for an optional call) the full argument list in evaluation
/// order. This is what lets target lowering own the operation as one
/// region instead of promoting an argument out of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConditionalFacts {
    /// The span of the conditionally-evaluated branch containing the value.
    pub(crate) branch: SourceSpan,
    /// The other branch of a ternary — evaluated exactly when the value's
    /// branch is not. `None` for logical operators (the operation's result
    /// is then the condition's own value) and for optional calls (the
    /// result is then `undefined`).
    pub(crate) skipped: Option<SourceSpan>,
    /// An optional call's arguments, in order. Empty for other operations.
    pub(crate) operands: Vec<ConditionalOperand>,
    /// An optional call's explicit type arguments, verbatim.
    pub(crate) type_args: Option<SourceSpan>,
}

/// One argument of an optional call: the argument expression, and whether
/// the call spreads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConditionalOperand {
    pub(crate) span: SourceSpan,
    pub(crate) spread: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HostEvaluationInput {
    pub(crate) source: SourceSpan,
    pub(crate) mode: EvaluationInputMode,
    pub(crate) receiver: Option<SourceSpan>,
    /// What evaluating this input may do — an optimization fact only.
    pub(crate) effects: Effects,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvaluationInputMode {
    Value,
    DirectReference,
    MemberReference,
}

/// What evaluating one host expression may observably do
/// (`docs/design/program-lowering.md` §9). Owned by this layer because it
/// is a fact about TypeScript syntax; consumed **only** by optimization
/// decisions (a capture that may be skipped) — correctness never branches
/// on it, and an unknown expression is every effect at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Effects {
    pub(crate) may_read_mutable: bool,
    pub(crate) may_write: bool,
    pub(crate) may_call: bool,
    pub(crate) may_throw: bool,
    pub(crate) may_suspend: bool,
    pub(crate) may_allocate: bool,
    pub(crate) requires_reference: bool,
}

impl Effects {
    /// The conservative default: anything might happen.
    pub(crate) const ANY: Effects = Effects {
        may_read_mutable: true,
        may_write: true,
        may_call: true,
        may_throw: true,
        may_suspend: true,
        may_allocate: true,
        requires_reference: false,
    };

    /// Nothing observable happens and re-evaluation yields the same value.
    pub(crate) const NONE: Effects = Effects {
        may_read_mutable: false,
        may_write: false,
        may_call: false,
        may_throw: false,
        may_suspend: false,
        may_allocate: false,
        requires_reference: false,
    };

    /// Whether evaluating the expression is observable at all — the proof a
    /// capture of it may be elided: with no reads, writes, calls, throws,
    /// suspensions, or allocation identity, moving its evaluation to the
    /// host occurrence changes no trace, no count, and no value.
    pub(crate) fn is_inert(&self) -> bool {
        *self == Effects::NONE
    }
}

/// The effects one host expression may have, judged from syntax alone.
///
/// Only shapes whose evaluation is provably unobservable answer
/// [`Effects::NONE`]: plain literals (a regex literal allocates a fresh
/// object per evaluation, so it is not one), possibly under TypeScript's
/// transparent expression wrappers. Identifiers may read mutable bindings
/// and may throw (TDZ); user types never prove runtime purity; everything
/// unknown is [`Effects::ANY`].
fn expression_effects(expression: &swc_ecma_ast::Expr) -> Effects {
    use swc_ecma_ast::{Expr as SwcExpr, Lit};
    match expression {
        SwcExpr::Lit(Lit::Str(_) | Lit::Bool(_) | Lit::Null(_) | Lit::Num(_) | Lit::BigInt(_)) => {
            Effects::NONE
        }
        SwcExpr::Paren(inner) => expression_effects(&inner.expr),
        SwcExpr::TsAs(inner) => expression_effects(&inner.expr),
        SwcExpr::TsSatisfies(inner) => expression_effects(&inner.expr),
        SwcExpr::TsNonNull(inner) => expression_effects(&inner.expr),
        SwcExpr::TsTypeAssertion(inner) => expression_effects(&inner.expr),
        SwcExpr::TsInstantiation(inner) => expression_effects(&inner.expr),
        _ => Effects::ANY,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostEvaluationOperation {
    Eager(EagerPosition),
    Conditional(ConditionalBranch),
    Reference(ReferencePosition),
    Suspend(SuspensionKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EagerPosition {
    BinaryLeft,
    BinaryRight,
    ArrayElement(u32),
    ObjectEvaluation(u32),
    AssignmentRight,
    SequenceElement(u32),
    UnaryOperand,
    CallArgument(u32),
    ConstructArgument(u32),
    TemplateInterpolation(u32),
    JsxExpression(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConditionalBranch {
    LogicalAndRight,
    LogicalOrRight,
    NullishRight,
    Consequent,
    Alternate,
    OptionalCallArgument(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReferencePosition {
    CallCallee,
    OptionalCallCallee,
    MemberObject,
    MemberProperty,
    ConstructorCallee,
    TaggedTemplateTag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SuspensionKind {
    Await,
    Yield,
    YieldDelegate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct HostOwnerId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct HostOwner {
    pub(crate) id: HostOwnerId,
    pub(crate) kind: HostOwnerKind,
    pub(crate) span: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum HostOwnerKind {
    Statement,
    ModuleItem,
    /// The expression body of a concise arrow function. Lowering rewrites
    /// this expression to a block when a nested tt value needs statements.
    ArrowExpression,
}

/// The Core IR node that owns one TypeScript host placeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CoreRoot {
    Adt(NodeId),
    Propagate(NodeId),
    Decision(NodeId),
    Expr(ExprId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvaluationFrequency {
    Once,
    Conditional,
    Repeated,
    Indeterminate,
}

/// Whether reaching a value's [`HostOwner`] happens exactly as often as
/// reaching the value.
///
/// Statement lowering inserts a value's control flow immediately before its
/// host owner, so it is sound only when the two are reached equally often.
/// That is a different question from [`EvaluationContext::frequency`], which
/// is measured against the enclosing *function*: a value in the body of a
/// `while` runs once per iteration relative to the function **and** relative
/// to its owner (the body statement), but a value in the `while` **test**
/// has the `while` statement itself as its owner, so hoisting to that owner
/// would run it once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnerReach {
    /// The owner is reached exactly when the value is. Edges the evaluation
    /// protocol models as [`ConditionalBranch`] steps count as `Same`: the
    /// schedule reproduces their conditionality in the target.
    Same,
    /// A loop header sits between them: the owner is reached once per loop,
    /// the value once per iteration.
    Repeated,
    /// An edge between them makes the value's evaluation conditional in a
    /// way no protocol step models — a `switch` case test, a destructuring
    /// default, the tail of an optional chain. Statements hoisted to the
    /// owner would run unconditionally.
    UnmodeledConditional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvaluationOwner {
    Module,
    FunctionBody,
    ParameterInitializer,
    ClassInitializer,
    StaticBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueRole {
    None,
    Value,
    AssignmentTarget,
    Pattern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostContinuation {
    Return,
    ArrowReturn,
    Initialize,
    Discard,
    Compose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EvaluationContext {
    /// How often the value runs inside its [`EvaluationOwner`].
    pub(crate) frequency: EvaluationFrequency,
    /// How often the value runs relative to its [`HostOwner`] — the fact
    /// that decides whether statements may be hoisted to that owner.
    pub(crate) owner_reach: OwnerReach,
    pub(crate) owner: EvaluationOwner,
    pub(crate) value_role: ValueRole,
    pub(crate) continuation: HostContinuation,
}

impl EvaluationContext {
    /// `host_owner_edge` is where the chosen [`HostOwner`]'s own child edges
    /// begin in `parents`; everything from there down sits between the owner
    /// and the value.
    fn from_path(
        category: SyntaxCategory,
        parents: &[AstParentKind],
        host_owner_edge: usize,
    ) -> Self {
        let (owner, owner_edge) = evaluation_owner(parents);
        let owner_reach = owner_reach(&parents[host_owner_edge.min(parents.len())..]);
        if category != SyntaxCategory::Expression {
            return Self {
                frequency: frequency_within_owner(parents, owner_edge),
                owner_reach,
                owner,
                value_role: ValueRole::None,
                continuation: HostContinuation::Discard,
            };
        }

        let local_path = &parents[owner_edge..];
        let value_role = value_role(local_path);
        let frequency = frequency_within_owner(parents, owner_edge);
        let continuation = host_continuation(local_path);
        Self {
            frequency,
            owner_reach,
            owner,
            value_role,
            continuation,
        }
    }
}

/// The evaluation regions between a value's host owner and the value.
///
/// Only the *header* positions of a loop can make the reach `Repeated`: a
/// loop body is a statement, and a statement is itself a host owner, so a
/// value in a body never sees the loop edge from its own owner.
///
/// Conditional edges split by who reproduces them. The ternary, logical
/// right-hand sides, and optional call arguments become
/// [`ConditionalBranch`] steps whose target regenerates the condition, so
/// they leave the reach `Same`. A `switch` case test, a destructuring
/// default, and the tail of an optional chain have no protocol step — a
/// value behind one of them cannot be hoisted to its owner at all.
fn owner_reach(local_path: &[AstParentKind]) -> OwnerReach {
    let mut reach = OwnerReach::Same;
    for (index, parent) in local_path.iter().enumerate() {
        match parent {
            AstParentKind::ForStmt(
                fields::ForStmtField::Test
                | fields::ForStmtField::Update
                | fields::ForStmtField::Body,
            )
            | AstParentKind::ForInStmt(fields::ForInStmtField::Body)
            | AstParentKind::ForOfStmt(fields::ForOfStmtField::Body)
            | AstParentKind::WhileStmt(fields::WhileStmtField::Test | fields::WhileStmtField::Body)
            | AstParentKind::DoWhileStmt(
                fields::DoWhileStmtField::Test | fields::DoWhileStmtField::Body,
            ) => return OwnerReach::Repeated,
            // Evaluated only when no earlier case matched — and always after
            // the discriminant, which hoisting would also reorder.
            AstParentKind::SwitchCase(fields::SwitchCaseField::Test)
            // A destructuring default: evaluated only when the matched
            // property or element is undefined.
            | AstParentKind::AssignPat(fields::AssignPatField::Right)
            | AstParentKind::AssignPatProp(fields::AssignPatPropField::Value) => {
                reach = OwnerReach::UnmodeledConditional;
            }
            // Inside an optional chain, everything but the base object, the
            // callee, and the arguments of the chain's own optional call is
            // skipped when the chain short-circuits. The arguments are the
            // one position a protocol step models
            // ([`ConditionalBranch::OptionalCallArgument`]).
            AstParentKind::OptChainExpr(fields::OptChainExprField::Base) => {
                let modeled = match local_path.get(index + 1) {
                    Some(AstParentKind::OptChainBase(fields::OptChainBaseField::Call)) => {
                        matches!(
                            local_path.get(index + 2),
                            Some(AstParentKind::OptCall(
                                fields::OptCallField::Args(_) | fields::OptCallField::Callee,
                            ))
                        )
                    }
                    Some(AstParentKind::OptChainBase(fields::OptChainBaseField::Member)) => {
                        matches!(
                            local_path.get(index + 2),
                            Some(AstParentKind::MemberExpr(fields::MemberExprField::Obj))
                        )
                    }
                    _ => false,
                };
                if !modeled {
                    reach = OwnerReach::UnmodeledConditional;
                }
            }
            _ => {}
        }
    }
    reach
}

fn evaluation_owner(parents: &[AstParentKind]) -> (EvaluationOwner, usize) {
    for (index, parent) in parents.iter().enumerate().rev() {
        match parent {
            AstParentKind::Function(fields::FunctionField::Params(_))
            | AstParentKind::ArrowExpr(fields::ArrowExprField::Params(_))
            | AstParentKind::Constructor(fields::ConstructorField::Params(_)) => {
                return (EvaluationOwner::ParameterInitializer, index + 1);
            }
            AstParentKind::Function(fields::FunctionField::Body)
            | AstParentKind::ArrowExpr(fields::ArrowExprField::Body)
            | AstParentKind::Constructor(fields::ConstructorField::Body) => {
                return (EvaluationOwner::FunctionBody, index + 1);
            }
            AstParentKind::ClassProp(fields::ClassPropField::Value)
            | AstParentKind::PrivateProp(fields::PrivatePropField::Value)
            | AstParentKind::AutoAccessor(fields::AutoAccessorField::Value) => {
                return (EvaluationOwner::ClassInitializer, index + 1);
            }
            AstParentKind::StaticBlock(fields::StaticBlockField::Body) => {
                return (EvaluationOwner::StaticBlock, index + 1);
            }
            _ => {}
        }
    }
    (EvaluationOwner::Module, 0)
}

fn frequency_within_owner(parents: &[AstParentKind], owner_edge: usize) -> EvaluationFrequency {
    let mut frequency = EvaluationFrequency::Once;
    for parent in &parents[owner_edge..] {
        if matches!(
            parent,
            AstParentKind::ForStmt(
                fields::ForStmtField::Test
                    | fields::ForStmtField::Update
                    | fields::ForStmtField::Body
            ) | AstParentKind::ForInStmt(fields::ForInStmtField::Body)
                | AstParentKind::ForOfStmt(fields::ForOfStmtField::Body)
                | AstParentKind::WhileStmt(
                    fields::WhileStmtField::Test | fields::WhileStmtField::Body
                )
                | AstParentKind::DoWhileStmt(
                    fields::DoWhileStmtField::Test | fields::DoWhileStmtField::Body
                )
        ) {
            return EvaluationFrequency::Repeated;
        }
        if matches!(parent, AstParentKind::BinExpr(fields::BinExprField::Right)) {
            frequency = EvaluationFrequency::Indeterminate;
        }
        if matches!(
            parent,
            AstParentKind::CondExpr(fields::CondExprField::Cons | fields::CondExprField::Alt)
                | AstParentKind::IfStmt(fields::IfStmtField::Cons | fields::IfStmtField::Alt)
                | AstParentKind::SwitchCase(fields::SwitchCaseField::Cons(_))
        ) {
            frequency = EvaluationFrequency::Conditional;
        }
    }
    frequency
}

fn value_role(parents: &[AstParentKind]) -> ValueRole {
    if parents.iter().rev().any(|parent| {
        matches!(
            parent,
            AstParentKind::AssignExpr(fields::AssignExprField::Left)
                | AstParentKind::AssignTarget(_)
                | AstParentKind::SimpleAssignTarget(_)
        )
    }) {
        ValueRole::AssignmentTarget
    } else if parents.iter().rev().any(|parent| {
        matches!(
            parent,
            AstParentKind::Pat(_)
                | AstParentKind::ArrayPat(_)
                | AstParentKind::ObjectPat(_)
                | AstParentKind::AssignPat(fields::AssignPatField::Left)
        )
    }) {
        ValueRole::Pattern
    } else {
        ValueRole::Value
    }
}

fn host_continuation(parents: &[AstParentKind]) -> HostContinuation {
    let significant = parents
        .iter()
        .rev()
        .find(|parent| !is_transparent_expression_edge(parent));
    match significant {
        Some(AstParentKind::ReturnStmt(fields::ReturnStmtField::Arg)) => HostContinuation::Return,
        Some(AstParentKind::ArrowFunctionBody(fields::ArrowFunctionBodyField::Expr))
        | Some(AstParentKind::ArrowExpr(fields::ArrowExprField::Body)) => {
            HostContinuation::ArrowReturn
        }
        Some(AstParentKind::VarDeclarator(fields::VarDeclaratorField::Init)) => {
            HostContinuation::Initialize
        }
        Some(AstParentKind::ExprStmt(fields::ExprStmtField::Expr)) => HostContinuation::Discard,
        _ => HostContinuation::Compose,
    }
}

fn is_transparent_expression_edge(parent: &AstParentKind) -> bool {
    matches!(
        parent,
        AstParentKind::Expr(_)
            | AstParentKind::ExprOrSpread(fields::ExprOrSpreadField::Expr)
            | AstParentKind::ParenExpr(fields::ParenExprField::Expr)
            | AstParentKind::TsAsExpr(fields::TsAsExprField::Expr)
            | AstParentKind::TsSatisfiesExpr(fields::TsSatisfiesExprField::Expr)
            | AstParentKind::TsNonNullExpr(fields::TsNonNullExprField::Expr)
            | AstParentKind::TsTypeAssertion(fields::TsTypeAssertionField::Expr)
            | AstParentKind::TsInstantiation(fields::TsInstantiationField::Expr)
    )
}

/// A complete SWC view of one tt-containing TypeScript module.
#[derive(Debug)]
pub(crate) struct ProgramSyntax {
    source_len: usize,
    projection: String,
    module: Module,
    overlay: Vec<OverlayEntry>,
    owners: Vec<HostOwnerSyntax>,
    occupied_names: HashSet<String>,
}

#[derive(Debug)]
pub(crate) struct HostOwnerSyntax {
    pub(crate) owner: HostOwner,
    pub(crate) roots: Vec<TtNodeId>,
}

/// Why a shadow program model could not be built.
///
/// Every variant but [`ProgramSyntaxError::SourceNotTypeScript`] is a broken
/// compiler invariant — a validator failure in the sense of
/// `docs/design/program-lowering.md` §11, and therefore an internal compiler
/// error. The projection's *parse*, though, is not a validator: the
/// projection is TypeScript the user wrote with tt values replaced by
/// placeholders, so it can also fail because that TypeScript is not
/// TypeScript. That cause is a fact about the input and carries the source
/// byte to report it at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProgramSyntaxError {
    MissingSourceSpan {
        node: NodeId,
    },
    InvalidSourceSpan {
        start: SourceByte,
        end: SourceByte,
    },
    NodeCountOverflow,
    /// The projection stopped parsing at a byte copied verbatim from the
    /// source: the user's own TypeScript does not parse.
    SourceNotTypeScript {
        message: String,
        source: usize,
    },
    /// The projection stopped parsing at a byte this compiler generated.
    Parse {
        message: String,
        projection: String,
    },
    MissingOverlay {
        id: TtNodeId,
    },
    DuplicateOverlay {
        id: TtNodeId,
    },
    UnmappedEvaluationSpan {
        start: usize,
        end: usize,
    },
}

impl ProgramSyntax {
    /// Builds and validates the shadow program model.
    pub(crate) fn build(
        semantic: &SemanticFile,
        core: &CoreFile,
        source: &str,
        source_kind: crate::SourceKind,
    ) -> Result<Self, ProgramSyntaxError> {
        let projection = ProjectionBuilder::new(semantic, core, source).build()?;
        let parsed = parse_module(&projection.code, &projection.source_segments, source_kind)?;
        let mut collector = ParentCollector::new(
            parsed.start,
            &projection.pending,
            &projection.source_segments,
        );
        let mut path = AstNodePath::default();
        parsed.module.visit_with_ast_path(&mut collector, &mut path);
        let collected = collector.finish(projection.pending)?;
        let syntax = Self {
            source_len: source.len(),
            projection: projection.code,
            module: parsed.module,
            overlay: collected.overlay,
            owners: collected.owners,
            occupied_names: collected.occupied_names,
        };
        syntax.validate()?;
        Ok(syntax)
    }

    /// Returns the Core roots joined to their TypeScript host contexts.
    pub(crate) fn core_contexts(
        &self,
    ) -> impl Iterator<
        Item = (
            CoreRoot,
            TtNodeId,
            EvaluationContext,
            HostEvaluationProtocol,
            SourceSpan,
            HostOwner,
            Vec<HostExit>,
        ),
    > + '_ {
        self.overlay.iter().map(|entry| {
            (
                entry.core_root,
                entry.id,
                entry.context,
                entry.protocol.clone(),
                entry.source,
                entry.host_owner,
                entry.exits.clone(),
            )
        })
    }

    pub(crate) fn owners(&self) -> impl Iterator<Item = &HostOwnerSyntax> {
        self.owners.iter()
    }

    pub(crate) fn occupied_names(&self) -> impl Iterator<Item = &str> {
        self.occupied_names.iter().map(String::as_str)
    }

    fn validate(&self) -> Result<(), ProgramSyntaxError> {
        let _module_span = self.module.span;
        let projection_len = self.projection.len();
        for entry in &self.overlay {
            let start = entry.projected.start.0;
            let end = entry.projected.end.0;
            if start >= end || end > projection_len {
                return Err(ProgramSyntaxError::InvalidSourceSpan {
                    start: SourceByte(entry.source.start),
                    end: SourceByte(entry.source.end),
                });
            }
            if entry.parents.is_empty() {
                return Err(ProgramSyntaxError::MissingOverlay { id: entry.id });
            }
            match entry.category {
                SyntaxCategory::Expression | SyntaxCategory::Statement | SyntaxCategory::Item => {}
            }
            match entry.context.continuation {
                HostContinuation::Return
                | HostContinuation::ArrowReturn
                | HostContinuation::Initialize
                | HostContinuation::Discard
                | HostContinuation::Compose => {}
            }
            for step in entry.protocol.steps() {
                if step.parent.start >= step.parent.end || step.parent.end > self.source_len {
                    return Err(ProgramSyntaxError::InvalidSourceSpan {
                        start: SourceByte(step.parent.start),
                        end: SourceByte(step.parent.end),
                    });
                }
            }
        }
        for (index, owner) in self.owners.iter().enumerate() {
            if owner.owner.id.0 as usize != index || owner.roots.is_empty() {
                return Err(ProgramSyntaxError::NodeCountOverflow);
            }
            if owner.owner.span.start >= owner.owner.span.end
                || owner.owner.span.end > self.source_len
            {
                return Err(ProgramSyntaxError::InvalidSourceSpan {
                    start: SourceByte(owner.owner.span.start),
                    end: SourceByte(owner.owner.span.end),
                });
            }
        }
        Ok(())
    }
}

struct Projection {
    code: String,
    pending: Vec<PendingOverlay>,
    source_segments: Vec<ProjectionSourceSegment>,
}

#[derive(Debug, Clone, Copy)]
struct ProjectionSourceSegment {
    projected: ProjectedSpan,
    source: SourceSpan,
    kind: ProjectionSegmentKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectionSegmentKind {
    Copied,
    Placeholder,
}

#[derive(Debug)]
struct PendingOverlay {
    id: TtNodeId,
    category: SyntaxCategory,
    source: SourceSpan,
    projected: ProjectedSpan,
    core_root: CoreRoot,
    marker: OverlayMarker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayMarker {
    Identifier,
    CallExpression,
    DecisionCallExpression,
}

struct ProjectionBuilder<'a> {
    semantic: &'a SemanticFile,
    core: &'a CoreFile,
    source: &'a str,
    code: String,
    pending: Vec<PendingOverlay>,
    source_segments: Vec<ProjectionSourceSegment>,
}

impl<'a> ProjectionBuilder<'a> {
    fn new(semantic: &'a SemanticFile, core: &'a CoreFile, source: &'a str) -> Self {
        Self {
            semantic,
            core,
            source,
            code: String::with_capacity(source.len()),
            pending: Vec::new(),
            source_segments: Vec::new(),
        }
    }

    fn build(mut self) -> Result<Projection, ProgramSyntaxError> {
        self.emit_body(self.core.root)?;
        Ok(Projection {
            code: self.code,
            pending: self.pending,
            source_segments: self.source_segments,
        })
    }

    fn source_span(&self, node: NodeId) -> Result<SourceSpan, ProgramSyntaxError> {
        self.semantic
            .hir
            .source_map
            .node_span(node)
            .map(SourceSpan::from)
            .ok_or(ProgramSyntaxError::MissingSourceSpan { node })
    }

    fn push_source(&mut self, node: NodeId) -> Result<(), ProgramSyntaxError> {
        let span = self.source_span(node)?;
        let text =
            self.source
                .get(span.start..span.end)
                .ok_or(ProgramSyntaxError::InvalidSourceSpan {
                    start: SourceByte(span.start),
                    end: SourceByte(span.end),
                })?;
        let start = ProjectedByte(self.code.len());
        self.code.push_str(text);
        let end = ProjectedByte(self.code.len());
        self.source_segments.push(ProjectionSourceSegment {
            projected: ProjectedSpan { start, end },
            source: span,
            kind: ProjectionSegmentKind::Copied,
        });
        Ok(())
    }

    fn push_placeholder(
        &mut self,
        category: SyntaxCategory,
        source: SourceSpan,
        core_root: CoreRoot,
    ) -> Result<(), ProgramSyntaxError> {
        let ordinal =
            u32::try_from(self.pending.len()).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
        let id = TtNodeId(ordinal);
        let owner_start = ProjectedByte(self.code.len());
        match category {
            SyntaxCategory::Expression => self.code.push('('),
            SyntaxCategory::Statement => self.code.push('{'),
            SyntaxCategory::Item => self.code.push_str("const "),
        }
        let start = ProjectedByte(self.code.len());
        let prefix = match category {
            SyntaxCategory::Expression => "$tt_syntax_expr_",
            SyntaxCategory::Statement => "$tt_syntax_stmt_",
            SyntaxCategory::Item => "$tt_syntax_item_",
        };
        self.code.push_str(prefix);
        self.code.push_str(&ordinal.to_string());
        let end = ProjectedByte(self.code.len());
        match category {
            SyntaxCategory::Expression => self.code.push(')'),
            SyntaxCategory::Statement => self.code.push_str(";}"),
            SyntaxCategory::Item => self.code.push_str(" = 0;"),
        }
        let owner_end = ProjectedByte(self.code.len());
        self.source_segments.push(ProjectionSourceSegment {
            projected: ProjectedSpan {
                start: owner_start,
                end: owner_end,
            },
            source,
            kind: ProjectionSegmentKind::Placeholder,
        });
        self.pending.push(PendingOverlay {
            id,
            category,
            source,
            projected: ProjectedSpan { start, end },
            core_root,
            marker: OverlayMarker::Identifier,
        });
        Ok(())
    }

    fn emit_body(&mut self, body: BodyId) -> Result<(), ProgramSyntaxError> {
        for statement in &self.core.bodies[body.index()].statements {
            match statement {
                Statement::Opaque(node) => self.push_source(*node)?,
                Statement::Adt(adt) => self.emit_adt(adt)?,
                Statement::Import(import) => self.emit_import(import)?,
                Statement::Propagate(propagate) => self.emit_propagate(propagate)?,
                Statement::Decision(decision) => self.emit_statement_decision(decision)?,
                Statement::Expr(expr) => self.emit_expr(*expr)?,
            }
        }
        Ok(())
    }

    fn emit_adt(&mut self, adt: &Adt) -> Result<(), ProgramSyntaxError> {
        self.push_placeholder(
            SyntaxCategory::Item,
            self.source_span(adt.node)?,
            CoreRoot::Adt(adt.node),
        )
    }

    fn emit_import(&mut self, import: &Import) -> Result<(), ProgramSyntaxError> {
        self.push_source(import.specifier)
    }

    fn emit_propagate(&mut self, propagate: &Propagate) -> Result<(), ProgramSyntaxError> {
        self.push_placeholder(
            SyntaxCategory::Statement,
            self.source_span(propagate.owner)?,
            CoreRoot::Propagate(propagate.node),
        )
    }

    fn emit_statement_decision(&mut self, decision: &Decision) -> Result<(), ProgramSyntaxError> {
        self.push_placeholder(
            SyntaxCategory::Statement,
            self.source_span(decision.extent)?,
            CoreRoot::Decision(decision.extent),
        )
    }

    fn emit_expr(&mut self, expr: ExprId) -> Result<(), ProgramSyntaxError> {
        match &self.core.exprs[expr.index()] {
            Expr::Opaque(node) => self.push_source(*node),
            Expr::Sequence(body) => self.emit_body(*body),
            Expr::Decision(decision) => self.emit_decision_region(expr, decision),
            Expr::Apply(apply) => self.emit_apply(expr, apply),
            Expr::ResultRegion(region) => self.emit_result_region(expr, region),
            Expr::Template(template) => self.emit_template(template),
        }
    }

    fn emit_apply(&mut self, expr: ExprId, apply: &Apply) -> Result<(), ProgramSyntaxError> {
        let mut source = self.source_span(apply.node)?;
        for step in &apply.steps {
            let step_span = self.source_span(step.node)?;
            source.start = source.start.min(step_span.start);
            source.end = source.end.max(step_span.end);
        }
        self.push_placeholder(SyntaxCategory::Expression, source, CoreRoot::Expr(expr))
    }

    fn emit_result_region(
        &mut self,
        expr: ExprId,
        region: &ResultRegion,
    ) -> Result<(), ProgramSyntaxError> {
        let source = self.source_span(region.node)?;
        let ordinal =
            u32::try_from(self.pending.len()).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
        let id = TtNodeId(ordinal);
        let start = ProjectedByte(self.code.len());
        let pending_index = self.pending.len();
        self.pending.push(PendingOverlay {
            id,
            category: SyntaxCategory::Expression,
            source,
            projected: ProjectedSpan { start, end: start },
            core_root: CoreRoot::Expr(expr),
            marker: OverlayMarker::CallExpression,
        });
        self.code.push('(');
        if region.is_async {
            self.code.push_str("async ");
        }
        self.code.push_str("() => {");
        for item in &region.items {
            match item {
                crate::core_ir::ResultRegionItem::Statements(body) => self.emit_body(*body)?,
                crate::core_ir::ResultRegionItem::Propagate(_) => self.code.push(';'),
            }
        }
        self.code.push_str("0;})()");
        let end = ProjectedByte(self.code.len());
        let projected = ProjectedSpan { start, end };
        self.source_segments.insert(
            0,
            ProjectionSourceSegment {
                projected,
                source,
                kind: ProjectionSegmentKind::Placeholder,
            },
        );
        self.pending[pending_index].projected = projected;
        Ok(())
    }

    fn emit_decision_region(
        &mut self,
        expr: ExprId,
        decision: &Decision,
    ) -> Result<(), ProgramSyntaxError> {
        let source = self.source_span(decision.extent)?;
        let ordinal =
            u32::try_from(self.pending.len()).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
        let id = TtNodeId(ordinal);
        let start = ProjectedByte(self.code.len());
        let pending_index = self.pending.len();
        self.pending.push(PendingOverlay {
            id,
            category: SyntaxCategory::Expression,
            source,
            projected: ProjectedSpan { start, end: start },
            core_root: CoreRoot::Expr(expr),
            marker: OverlayMarker::DecisionCallExpression,
        });
        self.code.push('(');
        if decision.is_async {
            self.code.push_str("async ");
        }
        self.code.push_str("() => {");
        for subject in &decision.subjects {
            self.code.push('(');
            self.emit_expr(subject.value)?;
            self.code.push_str(");");
        }
        for arm in &decision.arms {
            if let Some(guard) = arm.guard {
                self.code.push('(');
                self.emit_expr(guard)?;
                self.code.push_str(");");
            }
            let crate::core_ir::ArmAction::Yield { body, kind } = arm.action else {
                continue;
            };
            match kind {
                hir::ArmBodyKind::Expression => {
                    self.code.push('(');
                    self.emit_body(body)?;
                    self.code.push_str(");");
                }
                hir::ArmBodyKind::Block { .. } => {
                    self.code.push('{');
                    self.emit_body(body)?;
                    self.code.push('}');
                }
            }
        }
        self.code.push_str("0;})()");
        let end = ProjectedByte(self.code.len());
        let projected = ProjectedSpan { start, end };
        self.source_segments.insert(
            0,
            ProjectionSourceSegment {
                projected,
                source,
                kind: ProjectionSegmentKind::Placeholder,
            },
        );
        self.pending[pending_index].projected = projected;
        Ok(())
    }

    fn emit_template(&mut self, template: &Template) -> Result<(), ProgramSyntaxError> {
        for part in &template.parts {
            match part {
                TemplatePart::Raw(node) => self.push_source(*node)?,
                TemplatePart::Interpolation(expr) => {
                    self.code.push_str("${");
                    self.emit_expr(*expr)?;
                    self.code.push('}');
                }
            }
        }
        Ok(())
    }
}

struct ParsedModule {
    module: Module,
    start: u32,
}

fn parse_module(
    code: &str,
    segments: &[ProjectionSourceSegment],
    source_kind: crate::SourceKind,
) -> Result<ParsedModule, ProgramSyntaxError> {
    let source_map: Lrc<SourceMap> = Default::default();
    let file = source_map.new_source_file(Lrc::new(FileName::Anon), code.to_string());
    let start = file.start_pos.0;
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: source_kind.is_tsx(),
            decorators: true,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*file),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let result = parser.parse_module();
    let mut errors = parser.take_errors();
    let module = match result {
        Ok(module) => module,
        Err(error) => {
            errors.push(error);
            return Err(parse_failure(code, segments, start, &errors[0]));
        }
    };
    if let Some(error) = errors.into_iter().next() {
        return Err(parse_failure(code, segments, start, &error));
    }
    Ok(ParsedModule { module, start })
}

/// Classifies a projection parse failure by the byte it stopped at.
///
/// The projection is a sequence of two kinds of bytes: text copied from the
/// source, and placeholders this compiler wrote. Which kind the parser
/// stopped on *is* the cause, so the classification is a lookup in the
/// projection's own segment table rather than a guess about the message.
fn parse_failure(
    code: &str,
    segments: &[ProjectionSourceSegment],
    start: u32,
    error: &swc_ecma_parser::error::Error,
) -> ProgramSyntaxError {
    let message = error.kind().msg().to_string();
    // A parser can stop one byte past the end (`<eof>` expectations); that
    // byte belongs to the segment it ends.
    let at = usize::try_from(error.span().lo().0.saturating_sub(start)).unwrap_or(0);
    let at = ProjectedByte(at.min(code.len().saturating_sub(1)));
    match source_byte_for_projection(segments, at) {
        Some(source) => ProgramSyntaxError::SourceNotTypeScript { message, source },
        None => ProgramSyntaxError::Parse {
            message,
            projection: code.to_owned(),
        },
    }
}

struct ParentCollector {
    source_start: u32,
    expected_identifiers: HashMap<ProjectedSpan, TtNodeId>,
    expected_calls: HashMap<ProjectedSpan, TtNodeId>,
    expected_exit_calls: HashSet<TtNodeId>,
    found: HashMap<TtNodeId, FoundOverlay>,
    duplicates: Vec<TtNodeId>,
    source_segments: Vec<ProjectionSourceSegment>,
    host_owners: Vec<ProjectedHostOwner>,
    protocol_frames: Vec<ProjectedProtocolFrame>,
    occupied_names: HashSet<String>,
    function_depth: usize,
    /// How many enclosing statements consume an unlabeled `break`
    /// (loops and `switch`).
    break_capture_depth: usize,
    /// The exit-collecting regions in scope, with the function depth an
    /// exit must sit at and the break-capture depth the region opened at.
    exit_regions: Vec<(TtNodeId, usize, usize)>,
}

struct CollectedProgramSyntax {
    overlay: Vec<OverlayEntry>,
    owners: Vec<HostOwnerSyntax>,
    occupied_names: HashSet<String>,
}

struct FoundOverlay {
    parents: Vec<AstParentKind>,
    host_owners: Vec<ProjectedHostOwner>,
    protocol_frames: Vec<ProjectedProtocolFrame>,
    exits: Vec<ProjectedHostExit>,
}

#[derive(Debug, Clone, Copy)]
struct ProjectedHostExit {
    statement: ProjectedSpan,
    argument: Option<ProjectedSpan>,
    captured_break: bool,
}

#[derive(Debug, Clone)]
enum ProjectedProtocolFrame {
    Ordered {
        parent: ProjectedSpan,
        positions: Vec<(ProjectedSpan, Effects)>,
        kind: OrderedEvaluationKind,
    },
    Binary {
        parent: ProjectedSpan,
        operator: BinaryOp,
        left: (ProjectedSpan, Effects),
        right: ProjectedSpan,
    },
    Conditional {
        parent: ProjectedSpan,
        test: (ProjectedSpan, Effects),
        consequent: ProjectedSpan,
        alternate: ProjectedSpan,
    },
    Call {
        parent: ProjectedSpan,
        callee: Option<ProjectedSpan>,
        callee_mode: EvaluationInputMode,
        callee_receiver: Option<ProjectedSpan>,
        arguments: Vec<(ProjectedSpan, bool, Effects)>,
        type_args: Option<ProjectedSpan>,
        optional: bool,
    },
    Member {
        parent: ProjectedSpan,
        object: (ProjectedSpan, Effects),
        property: Option<ProjectedSpan>,
    },
    Construct {
        parent: ProjectedSpan,
        callee: ProjectedSpan,
        arguments: Vec<(ProjectedSpan, bool, Effects)>,
    },
    TaggedTemplate {
        parent: ProjectedSpan,
        tag: ProjectedSpan,
        tag_mode: EvaluationInputMode,
        tag_receiver: Option<ProjectedSpan>,
        expressions: Vec<(ProjectedSpan, Effects)>,
    },
    Template {
        parent: ProjectedSpan,
        expressions: Vec<(ProjectedSpan, Effects)>,
    },
    Jsx {
        parent: ProjectedSpan,
        expressions: Vec<(ProjectedSpan, Effects)>,
    },
    Suspend {
        parent: ProjectedSpan,
        kind: SuspensionKind,
        value: Option<ProjectedSpan>,
    },
}

impl ProjectedProtocolFrame {
    /// The projected span of the node that owns this evaluation frame.
    fn parent(&self) -> ProjectedSpan {
        match self {
            ProjectedProtocolFrame::Ordered { parent, .. }
            | ProjectedProtocolFrame::Binary { parent, .. }
            | ProjectedProtocolFrame::Conditional { parent, .. }
            | ProjectedProtocolFrame::Call { parent, .. }
            | ProjectedProtocolFrame::Member { parent, .. }
            | ProjectedProtocolFrame::Construct { parent, .. }
            | ProjectedProtocolFrame::TaggedTemplate { parent, .. }
            | ProjectedProtocolFrame::Template { parent, .. }
            | ProjectedProtocolFrame::Jsx { parent, .. }
            | ProjectedProtocolFrame::Suspend { parent, .. } => *parent,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum OrderedEvaluationKind {
    Array,
    Object,
    Assignment,
    Sequence,
    Unary,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ProjectedHostOwner {
    kind: HostOwnerKind,
    span: ProjectedSpan,
    /// How many parent edges precede this owner's own child edges. Slicing
    /// a value's parent path here leaves exactly the edges between the owner
    /// and the value ([`owner_reach`]).
    edge: usize,
}

fn object_evaluation_positions(
    node: &ObjectLit,
    source_start: u32,
) -> Vec<(ProjectedSpan, Effects)> {
    let mut positions = Vec::new();
    for property in &node.props {
        match property {
            PropOrSpread::Spread(spread) => {
                positions.push((
                    projected_span(spread.expr.span(), source_start),
                    expression_effects(&spread.expr),
                ));
            }
            PropOrSpread::Prop(property) => match &**property {
                Prop::Shorthand(identifier) => {
                    positions.push((projected_span(identifier.span, source_start), Effects::ANY));
                }
                Prop::KeyValue(property) => {
                    push_computed_property(&mut positions, &property.key, source_start);
                    positions.push((
                        projected_span(property.value.span(), source_start),
                        expression_effects(&property.value),
                    ));
                }
                Prop::Assign(property) => {
                    positions.push((
                        projected_span(property.value.span(), source_start),
                        expression_effects(&property.value),
                    ));
                }
                Prop::Getter(property) => {
                    push_computed_property(&mut positions, &property.key, source_start);
                }
                Prop::Setter(property) => {
                    push_computed_property(&mut positions, &property.key, source_start);
                }
                Prop::Method(property) => {
                    push_computed_property(&mut positions, &property.key, source_start);
                }
            },
        }
    }
    positions
}

fn push_computed_property(
    positions: &mut Vec<(ProjectedSpan, Effects)>,
    name: &PropName,
    source_start: u32,
) {
    if let PropName::Computed(computed) = name {
        positions.push((
            projected_span(computed.expr.span(), source_start),
            expression_effects(&computed.expr),
        ));
    }
}

/// The evaluation positions of a call's arguments: the argument
/// *expression* spans (a spread's `...` stays with the call, so a capture
/// of the position captures a value, not spread syntax), plus whether the
/// call spreads each.
fn argument_positions(
    arguments: &[swc_ecma_ast::ExprOrSpread],
    source_start: u32,
) -> Vec<(ProjectedSpan, bool, Effects)> {
    arguments
        .iter()
        .map(|argument| {
            (
                projected_span(argument.expr.span(), source_start),
                argument.spread.is_some(),
                expression_effects(&argument.expr),
            )
        })
        .collect()
}

fn jsx_expression_span(expression: &JSXExpr, source_start: u32) -> Option<ProjectedSpan> {
    match expression {
        JSXExpr::Expr(expression) => Some(projected_span(expression.span(), source_start)),
        JSXExpr::JSXEmptyExpr(_) => None,
    }
}

fn jsx_evaluation_positions(node: &JSXElement, source_start: u32) -> Vec<ProjectedSpan> {
    let attributes = node
        .opening
        .attrs
        .iter()
        .filter_map(|attribute| match attribute {
            JSXAttrOrSpread::SpreadElement(spread) => {
                Some(projected_span(spread.expr.span(), source_start))
            }
            JSXAttrOrSpread::JSXAttr(attribute) => match attribute.value.as_ref()? {
                JSXAttrValue::JSXExprContainer(container) => {
                    jsx_expression_span(&container.expr, source_start)
                }
                JSXAttrValue::JSXElement(element) => {
                    Some(projected_span(element.span, source_start))
                }
                JSXAttrValue::JSXFragment(fragment) => {
                    Some(projected_span(fragment.span, source_start))
                }
                JSXAttrValue::Str(_) => None,
            },
        });
    let children = node.children.iter().filter_map(|child| match child {
        JSXElementChild::JSXExprContainer(container) => {
            jsx_expression_span(&container.expr, source_start)
        }
        JSXElementChild::JSXSpreadChild(spread) => {
            Some(projected_span(spread.expr.span(), source_start))
        }
        JSXElementChild::JSXElement(element) => Some(projected_span(element.span, source_start)),
        JSXElementChild::JSXFragment(fragment) => Some(projected_span(fragment.span, source_start)),
        JSXElementChild::JSXText(_) => None,
    });
    attributes.chain(children).collect()
}

fn jsx_fragment_positions(node: &JSXFragment, source_start: u32) -> Vec<ProjectedSpan> {
    node.children
        .iter()
        .filter_map(|child| match child {
            JSXElementChild::JSXExprContainer(container) => {
                jsx_expression_span(&container.expr, source_start)
            }
            JSXElementChild::JSXSpreadChild(spread) => {
                Some(projected_span(spread.expr.span(), source_start))
            }
            JSXElementChild::JSXElement(element) => {
                Some(projected_span(element.span, source_start))
            }
            JSXElementChild::JSXFragment(fragment) => {
                Some(projected_span(fragment.span, source_start))
            }
            JSXElementChild::JSXText(_) => None,
        })
        .collect()
}

impl ParentCollector {
    fn new(
        source_start: u32,
        pending: &[PendingOverlay],
        source_segments: &[ProjectionSourceSegment],
    ) -> Self {
        let expected_identifiers = pending
            .iter()
            .filter(|entry| entry.marker == OverlayMarker::Identifier)
            .map(|entry| (entry.projected, entry.id))
            .collect();
        let expected_calls = pending
            .iter()
            .filter(|entry| {
                matches!(
                    entry.marker,
                    OverlayMarker::CallExpression | OverlayMarker::DecisionCallExpression
                )
            })
            .map(|entry| (entry.projected, entry.id))
            .collect();
        let expected_exit_calls = pending
            .iter()
            .filter(|entry| entry.marker == OverlayMarker::DecisionCallExpression)
            .map(|entry| entry.id)
            .collect();
        Self {
            source_start,
            expected_identifiers,
            expected_calls,
            expected_exit_calls,
            found: HashMap::new(),
            duplicates: Vec::new(),
            source_segments: source_segments.to_vec(),
            host_owners: Vec::new(),
            protocol_frames: Vec::new(),
            occupied_names: HashSet::new(),
            function_depth: 0,
            break_capture_depth: 0,
            exit_regions: Vec::new(),
        }
    }

    fn record_overlay(&mut self, id: TtNodeId, path: &AstNodePath<'_>) {
        if self
            .found
            .insert(
                id,
                FoundOverlay {
                    parents: path.kinds().to_vec(),
                    host_owners: self.host_owners.clone(),
                    protocol_frames: self.protocol_frames.clone(),
                    exits: Vec::new(),
                },
            )
            .is_some()
        {
            self.duplicates.push(id);
        }
    }

    fn finish(
        mut self,
        pending: Vec<PendingOverlay>,
    ) -> Result<CollectedProgramSyntax, ProgramSyntaxError> {
        if let Some(id) = self.duplicates.into_iter().next() {
            return Err(ProgramSyntaxError::DuplicateOverlay { id });
        }
        let mut owner_ids: HashMap<ProjectedHostOwner, HostOwnerId> = HashMap::new();
        let mut owners: Vec<HostOwnerSyntax> = Vec::new();
        let mut overlay: Vec<OverlayEntry> = Vec::with_capacity(pending.len());
        for entry in pending {
            let found = self
                .found
                .remove(&entry.id)
                .ok_or(ProgramSyntaxError::MissingOverlay { id: entry.id })?;
            let (projected_owner, kind, span) = found
                .host_owners
                .iter()
                .rev()
                .filter(|owner| {
                    owner.span.start <= entry.projected.start
                        && entry.projected.end <= owner.span.end
                })
                .find_map(|owner| {
                    source_span_for_projection(&self.source_segments, owner.span)
                        .map(|span| (*owner, owner.kind, span))
                })
                .ok_or(ProgramSyntaxError::MissingOverlay { id: entry.id })?;
            let owner_id = if let Some(owner_id) = owner_ids.get(&projected_owner).copied() {
                owner_id
            } else {
                let owner_id = HostOwnerId(
                    u32::try_from(owners.len())
                        .map_err(|_| ProgramSyntaxError::NodeCountOverflow)?,
                );
                owner_ids.insert(projected_owner, owner_id);
                owners.push(HostOwnerSyntax {
                    owner: HostOwner {
                        id: owner_id,
                        kind,
                        span,
                    },
                    roots: Vec::new(),
                });
                owner_id
            };
            owners[owner_id.0 as usize].roots.push(entry.id);
            overlay.push(OverlayEntry {
                id: entry.id,
                category: entry.category,
                source: entry.source,
                projected: entry.projected,
                context: EvaluationContext::from_path(
                    entry.category,
                    &found.parents,
                    projected_owner.edge,
                ),
                // A frame outside the host owner is not this owner's
                // evaluation obligation: a statement (or a concise arrow
                // body) can sit inside an outer expression only across a
                // function boundary, and the rewrite happens where the owner
                // executes, not where the enclosing expression does.
                protocol: evaluation_protocol(
                    &self.source_segments,
                    entry.projected,
                    &found
                        .protocol_frames
                        .iter()
                        .filter(|frame| projected_contains(projected_owner.span, frame.parent()))
                        .cloned()
                        .collect::<Vec<_>>(),
                )?,
                core_root: entry.core_root,
                parents: found.parents,
                host_owner: owners[owner_id.0 as usize].owner,
                exits: found
                    .exits
                    .into_iter()
                    .map(|exit| {
                        Ok(HostExit {
                            statement: map_evaluation_span(&self.source_segments, exit.statement)?,
                            argument: exit
                                .argument
                                .map(|argument| {
                                    map_evaluation_span(&self.source_segments, argument)
                                })
                                .transpose()?,
                            captured_break: exit.captured_break,
                        })
                    })
                    .collect::<Result<Vec<_>, ProgramSyntaxError>>()?,
            });
        }
        Ok(CollectedProgramSyntax {
            overlay,
            owners,
            occupied_names: self.occupied_names,
        })
    }
}

impl VisitAstPath for ParentCollector {
    fn visit_module_item<'ast: 'r, 'r>(
        &mut self,
        item: &'ast ModuleItem,
        path: &mut AstNodePath<'r>,
    ) {
        self.host_owners.push(ProjectedHostOwner {
            kind: HostOwnerKind::ModuleItem,
            span: projected_span(item.span(), self.source_start),
            edge: path.kinds().len(),
        });
        <ModuleItem as VisitWithAstPath<Self>>::visit_children_with_ast_path(item, self, path);
        self.host_owners.pop();
    }

    fn visit_stmt<'ast: 'r, 'r>(&mut self, statement: &'ast Stmt, path: &mut AstNodePath<'r>) {
        self.host_owners.push(ProjectedHostOwner {
            kind: HostOwnerKind::Statement,
            span: projected_span(statement.span(), self.source_start),
            edge: path.kinds().len(),
        });
        <Stmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(statement, self, path);
        self.host_owners.pop();
    }

    fn visit_array_lit<'ast: 'r, 'r>(&mut self, node: &'ast ArrayLit, path: &mut AstNodePath<'r>) {
        self.protocol_frames.push(ProjectedProtocolFrame::Ordered {
            parent: projected_span(node.span, self.source_start),
            positions: node
                .elems
                .iter()
                .flatten()
                .map(|element| {
                    (
                        projected_span(element.expr.span(), self.source_start),
                        expression_effects(&element.expr),
                    )
                })
                .collect(),
            kind: OrderedEvaluationKind::Array,
        });
        <ArrayLit as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_object_lit<'ast: 'r, 'r>(
        &mut self,
        node: &'ast ObjectLit,
        path: &mut AstNodePath<'r>,
    ) {
        self.protocol_frames.push(ProjectedProtocolFrame::Ordered {
            parent: projected_span(node.span, self.source_start),
            positions: object_evaluation_positions(node, self.source_start),
            kind: OrderedEvaluationKind::Object,
        });
        <ObjectLit as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_assign_expr<'ast: 'r, 'r>(
        &mut self,
        node: &'ast AssignExpr,
        path: &mut AstNodePath<'r>,
    ) {
        self.protocol_frames.push(ProjectedProtocolFrame::Ordered {
            parent: projected_span(node.span, self.source_start),
            positions: vec![(
                projected_span(node.right.span(), self.source_start),
                expression_effects(&node.right),
            )],
            kind: OrderedEvaluationKind::Assignment,
        });
        <AssignExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_seq_expr<'ast: 'r, 'r>(&mut self, node: &'ast SeqExpr, path: &mut AstNodePath<'r>) {
        self.protocol_frames.push(ProjectedProtocolFrame::Ordered {
            parent: projected_span(node.span, self.source_start),
            positions: node
                .exprs
                .iter()
                .map(|expression| {
                    (
                        projected_span(expression.span(), self.source_start),
                        expression_effects(expression),
                    )
                })
                .collect(),
            kind: OrderedEvaluationKind::Sequence,
        });
        <SeqExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_unary_expr<'ast: 'r, 'r>(
        &mut self,
        node: &'ast UnaryExpr,
        path: &mut AstNodePath<'r>,
    ) {
        self.protocol_frames.push(ProjectedProtocolFrame::Ordered {
            parent: projected_span(node.span, self.source_start),
            positions: vec![(
                projected_span(node.arg.span(), self.source_start),
                expression_effects(&node.arg),
            )],
            kind: OrderedEvaluationKind::Unary,
        });
        <UnaryExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_bin_expr<'ast: 'r, 'r>(&mut self, node: &'ast BinExpr, path: &mut AstNodePath<'r>) {
        self.protocol_frames.push(ProjectedProtocolFrame::Binary {
            parent: projected_span(node.span, self.source_start),
            operator: node.op,
            left: (
                projected_span(node.left.span(), self.source_start),
                expression_effects(&node.left),
            ),
            right: projected_span(node.right.span(), self.source_start),
        });
        <BinExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_cond_expr<'ast: 'r, 'r>(&mut self, node: &'ast CondExpr, path: &mut AstNodePath<'r>) {
        self.protocol_frames
            .push(ProjectedProtocolFrame::Conditional {
                parent: projected_span(node.span, self.source_start),
                test: (
                    projected_span(node.test.span(), self.source_start),
                    expression_effects(&node.test),
                ),
                consequent: projected_span(node.cons.span(), self.source_start),
                alternate: projected_span(node.alt.span(), self.source_start),
            });
        <CondExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_call_expr<'ast: 'r, 'r>(&mut self, node: &'ast CallExpr, path: &mut AstNodePath<'r>) {
        let span = projected_span(node.span, self.source_start);
        if let Some(id) = self.expected_calls.get(&span).copied() {
            self.record_overlay(id, path);
            let collects_exits = self.expected_exit_calls.contains(&id);
            if collects_exits {
                self.exit_regions
                    .push((id, self.function_depth + 1, self.break_capture_depth));
            }
            <CallExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
            if collects_exits {
                self.exit_regions.pop();
            }
            return;
        }
        let (callee_mode, callee_receiver) = match &node.callee {
            swc_ecma_ast::Callee::Expr(expression) => call_callee_mode(expression),
            swc_ecma_ast::Callee::Super(_) | swc_ecma_ast::Callee::Import(_) => {
                (EvaluationInputMode::MemberReference, None)
            }
        };
        self.protocol_frames.push(ProjectedProtocolFrame::Call {
            parent: span,
            callee: Some(projected_span(
                match &node.callee {
                    swc_ecma_ast::Callee::Expr(expression) => reference_value_span(expression),
                    swc_ecma_ast::Callee::Super(_) | swc_ecma_ast::Callee::Import(_) => {
                        node.callee.span()
                    }
                },
                self.source_start,
            )),
            callee_mode,
            callee_receiver: callee_receiver.map(|span| projected_span(span, self.source_start)),
            arguments: argument_positions(&node.args, self.source_start),
            type_args: node
                .type_args
                .as_ref()
                .map(|args| projected_span(args.span(), self.source_start)),
            optional: false,
        });
        <CallExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_for_stmt<'ast: 'r, 'r>(
        &mut self,
        node: &'ast swc_ecma_ast::ForStmt,
        path: &mut AstNodePath<'r>,
    ) {
        self.break_capture_depth += 1;
        <swc_ecma_ast::ForStmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(
            node, self, path,
        );
        self.break_capture_depth -= 1;
    }

    fn visit_for_in_stmt<'ast: 'r, 'r>(
        &mut self,
        node: &'ast swc_ecma_ast::ForInStmt,
        path: &mut AstNodePath<'r>,
    ) {
        self.break_capture_depth += 1;
        <swc_ecma_ast::ForInStmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(
            node, self, path,
        );
        self.break_capture_depth -= 1;
    }

    fn visit_for_of_stmt<'ast: 'r, 'r>(
        &mut self,
        node: &'ast swc_ecma_ast::ForOfStmt,
        path: &mut AstNodePath<'r>,
    ) {
        self.break_capture_depth += 1;
        <swc_ecma_ast::ForOfStmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(
            node, self, path,
        );
        self.break_capture_depth -= 1;
    }

    fn visit_while_stmt<'ast: 'r, 'r>(
        &mut self,
        node: &'ast swc_ecma_ast::WhileStmt,
        path: &mut AstNodePath<'r>,
    ) {
        self.break_capture_depth += 1;
        <swc_ecma_ast::WhileStmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(
            node, self, path,
        );
        self.break_capture_depth -= 1;
    }

    fn visit_do_while_stmt<'ast: 'r, 'r>(
        &mut self,
        node: &'ast swc_ecma_ast::DoWhileStmt,
        path: &mut AstNodePath<'r>,
    ) {
        self.break_capture_depth += 1;
        <swc_ecma_ast::DoWhileStmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(
            node, self, path,
        );
        self.break_capture_depth -= 1;
    }

    fn visit_switch_stmt<'ast: 'r, 'r>(
        &mut self,
        node: &'ast swc_ecma_ast::SwitchStmt,
        path: &mut AstNodePath<'r>,
    ) {
        self.break_capture_depth += 1;
        <swc_ecma_ast::SwitchStmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(
            node, self, path,
        );
        self.break_capture_depth -= 1;
    }

    fn visit_arrow_expr<'ast: 'r, 'r>(
        &mut self,
        node: &'ast ArrowExpr,
        path: &mut AstNodePath<'r>,
    ) {
        self.function_depth += 1;
        self.host_owners.push(ProjectedHostOwner {
            kind: HostOwnerKind::ArrowExpression,
            span: projected_span(node.body.span(), self.source_start),
            edge: path.kinds().len(),
        });
        <ArrowExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.host_owners.pop();
        self.function_depth -= 1;
    }

    fn visit_function<'ast: 'r, 'r>(&mut self, node: &'ast Function, path: &mut AstNodePath<'r>) {
        self.function_depth += 1;
        <Function as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.function_depth -= 1;
    }

    fn visit_constructor<'ast: 'r, 'r>(
        &mut self,
        node: &'ast Constructor,
        path: &mut AstNodePath<'r>,
    ) {
        self.function_depth += 1;
        <Constructor as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.function_depth -= 1;
    }

    fn visit_return_stmt<'ast: 'r, 'r>(
        &mut self,
        node: &'ast ReturnStmt,
        path: &mut AstNodePath<'r>,
    ) {
        if let Some((id, target_depth, region_break_depth)) = self.exit_regions.last().copied()
            && target_depth == self.function_depth
            && let Some(found) = self.found.get_mut(&id)
        {
            found.exits.push(ProjectedHostExit {
                statement: projected_span(node.span, self.source_start),
                argument: node
                    .arg
                    .as_ref()
                    .map(|argument| projected_span(argument.span(), self.source_start)),
                captured_break: self.break_capture_depth > region_break_depth,
            });
        }
        <ReturnStmt as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
    }

    fn visit_opt_call<'ast: 'r, 'r>(&mut self, node: &'ast OptCall, path: &mut AstNodePath<'r>) {
        let (callee_mode, callee_receiver) = call_callee_mode(&node.callee);
        self.protocol_frames.push(ProjectedProtocolFrame::Call {
            parent: projected_span(node.span, self.source_start),
            callee: Some(projected_span(
                reference_value_span(&node.callee),
                self.source_start,
            )),
            callee_mode,
            callee_receiver: callee_receiver.map(|span| projected_span(span, self.source_start)),
            arguments: argument_positions(&node.args, self.source_start),
            type_args: node
                .type_args
                .as_ref()
                .map(|args| projected_span(args.span(), self.source_start)),
            optional: true,
        });
        <OptCall as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_member_expr<'ast: 'r, 'r>(
        &mut self,
        node: &'ast MemberExpr,
        path: &mut AstNodePath<'r>,
    ) {
        let property = match &node.prop {
            MemberProp::Computed(property) => {
                Some(projected_span(property.expr.span(), self.source_start))
            }
            MemberProp::Ident(_) | MemberProp::PrivateName(_) => None,
        };
        self.protocol_frames.push(ProjectedProtocolFrame::Member {
            parent: projected_span(node.span, self.source_start),
            object: (
                projected_span(node.obj.span(), self.source_start),
                expression_effects(&node.obj),
            ),
            property,
        });
        <MemberExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_new_expr<'ast: 'r, 'r>(&mut self, node: &'ast NewExpr, path: &mut AstNodePath<'r>) {
        self.protocol_frames
            .push(ProjectedProtocolFrame::Construct {
                parent: projected_span(node.span, self.source_start),
                callee: projected_span(reference_value_span(&node.callee), self.source_start),
                arguments: node
                    .args
                    .as_deref()
                    .map(|args| argument_positions(args, self.source_start))
                    .unwrap_or_default(),
            });
        <NewExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_tagged_tpl<'ast: 'r, 'r>(
        &mut self,
        node: &'ast TaggedTpl,
        path: &mut AstNodePath<'r>,
    ) {
        let (tag_mode, tag_receiver) = call_callee_mode(&node.tag);
        self.protocol_frames
            .push(ProjectedProtocolFrame::TaggedTemplate {
                parent: projected_span(node.span, self.source_start),
                tag: projected_span(reference_value_span(&node.tag), self.source_start),
                tag_mode,
                tag_receiver: tag_receiver.map(|span| projected_span(span, self.source_start)),
                expressions: node
                    .tpl
                    .exprs
                    .iter()
                    .map(|expression| {
                        (
                            projected_span(expression.span(), self.source_start),
                            expression_effects(expression),
                        )
                    })
                    .collect(),
            });
        <TaggedTpl as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_tpl<'ast: 'r, 'r>(&mut self, node: &'ast Tpl, path: &mut AstNodePath<'r>) {
        self.protocol_frames.push(ProjectedProtocolFrame::Template {
            parent: projected_span(node.span, self.source_start),
            expressions: node
                .exprs
                .iter()
                .map(|expression| {
                    (
                        projected_span(expression.span(), self.source_start),
                        expression_effects(expression),
                    )
                })
                .collect(),
        });
        <Tpl as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_jsx_element<'ast: 'r, 'r>(
        &mut self,
        node: &'ast JSXElement,
        path: &mut AstNodePath<'r>,
    ) {
        self.protocol_frames.push(ProjectedProtocolFrame::Jsx {
            parent: projected_span(node.span, self.source_start),
            expressions: jsx_evaluation_positions(node, self.source_start)
                .into_iter()
                .map(|span| (span, Effects::ANY))
                .collect(),
        });
        <JSXElement as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_jsx_fragment<'ast: 'r, 'r>(
        &mut self,
        node: &'ast JSXFragment,
        path: &mut AstNodePath<'r>,
    ) {
        self.protocol_frames.push(ProjectedProtocolFrame::Jsx {
            parent: projected_span(node.span, self.source_start),
            expressions: jsx_fragment_positions(node, self.source_start)
                .into_iter()
                .map(|span| (span, Effects::ANY))
                .collect(),
        });
        <JSXFragment as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_await_expr<'ast: 'r, 'r>(
        &mut self,
        node: &'ast AwaitExpr,
        path: &mut AstNodePath<'r>,
    ) {
        self.protocol_frames.push(ProjectedProtocolFrame::Suspend {
            parent: projected_span(node.span, self.source_start),
            kind: SuspensionKind::Await,
            value: Some(projected_span(node.arg.span(), self.source_start)),
        });
        <AwaitExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_yield_expr<'ast: 'r, 'r>(
        &mut self,
        node: &'ast YieldExpr,
        path: &mut AstNodePath<'r>,
    ) {
        self.protocol_frames.push(ProjectedProtocolFrame::Suspend {
            parent: projected_span(node.span, self.source_start),
            kind: if node.delegate {
                SuspensionKind::YieldDelegate
            } else {
                SuspensionKind::Yield
            },
            value: node
                .arg
                .as_ref()
                .map(|value| projected_span(value.span(), self.source_start)),
        });
        <YieldExpr as VisitWithAstPath<Self>>::visit_children_with_ast_path(node, self, path);
        self.protocol_frames.pop();
    }

    fn visit_ident<'ast: 'r, 'r>(&mut self, ident: &'ast Ident, path: &mut AstNodePath<'r>) {
        self.occupied_names.insert(ident.sym.to_string());
        let start = ident.span.lo.0.saturating_sub(self.source_start) as usize;
        let end = ident.span.hi.0.saturating_sub(self.source_start) as usize;
        let projected = ProjectedSpan {
            start: ProjectedByte(start),
            end: ProjectedByte(end),
        };
        let Some(id) = self.expected_identifiers.get(&projected).copied() else {
            return;
        };
        self.record_overlay(id, path);
    }
}

fn evaluation_protocol(
    segments: &[ProjectionSourceSegment],
    value: ProjectedSpan,
    frames: &[ProjectedProtocolFrame],
) -> Result<HostEvaluationProtocol, ProgramSyntaxError> {
    let steps = frames
        .iter()
        .rev()
        .map(|frame| protocol_step(segments, value, frame))
        .collect::<Result<Vec<_>, ProgramSyntaxError>>()?
        .into_iter()
        .flatten()
        .collect();
    Ok(HostEvaluationProtocol { steps })
}

/// The projected shape of one conditional operation, before span mapping.
struct ProjectedConditionalFacts {
    branch: ProjectedSpan,
    skipped: Option<ProjectedSpan>,
    operands: Vec<(ProjectedSpan, bool)>,
    type_args: Option<ProjectedSpan>,
}

fn protocol_step(
    segments: &[ProjectionSourceSegment],
    value: ProjectedSpan,
    frame: &ProjectedProtocolFrame,
) -> Result<Option<HostEvaluationStep>, ProgramSyntaxError> {
    let mut conditional: Option<ProjectedConditionalFacts> = None;
    let (parent, operation, inputs) = match frame {
        ProjectedProtocolFrame::Ordered {
            parent,
            positions,
            kind,
        } => {
            let Some(position) = positions
                .iter()
                .position(|(span, _)| projected_contains(*span, value))
            else {
                return Ok(None);
            };
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let operation = HostEvaluationOperation::Eager(match kind {
                OrderedEvaluationKind::Array => EagerPosition::ArrayElement(index),
                OrderedEvaluationKind::Object => EagerPosition::ObjectEvaluation(index),
                OrderedEvaluationKind::Assignment => EagerPosition::AssignmentRight,
                OrderedEvaluationKind::Sequence => EagerPosition::SequenceElement(index),
                OrderedEvaluationKind::Unary => EagerPosition::UnaryOperand,
            });
            let inputs = positions[..position]
                .iter()
                .copied()
                .map(|(span, effects)| (span, EvaluationInputMode::Value, None, effects))
                .collect();
            (*parent, operation, inputs)
        }
        ProjectedProtocolFrame::Binary {
            parent,
            left: (left, _),
            ..
        } if projected_contains(*left, value) => (
            *parent,
            HostEvaluationOperation::Eager(EagerPosition::BinaryLeft),
            Vec::new(),
        ),
        ProjectedProtocolFrame::Binary {
            parent,
            operator,
            left,
            right,
            ..
        } if projected_contains(*right, value) => {
            let operation = match operator {
                BinaryOp::LogicalAnd => {
                    HostEvaluationOperation::Conditional(ConditionalBranch::LogicalAndRight)
                }
                BinaryOp::LogicalOr => {
                    HostEvaluationOperation::Conditional(ConditionalBranch::LogicalOrRight)
                }
                BinaryOp::NullishCoalescing => {
                    HostEvaluationOperation::Conditional(ConditionalBranch::NullishRight)
                }
                _ => HostEvaluationOperation::Eager(EagerPosition::BinaryRight),
            };
            if matches!(operation, HostEvaluationOperation::Conditional(_)) {
                conditional = Some(ProjectedConditionalFacts {
                    branch: *right,
                    skipped: None,
                    operands: Vec::new(),
                    type_args: None,
                });
            }
            let (left, effects) = *left;
            (
                *parent,
                operation,
                vec![(left, EvaluationInputMode::Value, None, effects)],
            )
        }
        ProjectedProtocolFrame::Conditional {
            parent,
            test,
            consequent,
            alternate,
        } if projected_contains(*consequent, value) => {
            conditional = Some(ProjectedConditionalFacts {
                branch: *consequent,
                skipped: Some(*alternate),
                operands: Vec::new(),
                type_args: None,
            });
            let (test, effects) = *test;
            (
                *parent,
                HostEvaluationOperation::Conditional(ConditionalBranch::Consequent),
                vec![(test, EvaluationInputMode::Value, None, effects)],
            )
        }
        ProjectedProtocolFrame::Conditional {
            parent,
            test,
            consequent,
            alternate,
        } if projected_contains(*alternate, value) => {
            conditional = Some(ProjectedConditionalFacts {
                branch: *alternate,
                skipped: Some(*consequent),
                operands: Vec::new(),
                type_args: None,
            });
            let (test, effects) = *test;
            (
                *parent,
                HostEvaluationOperation::Conditional(ConditionalBranch::Alternate),
                vec![(test, EvaluationInputMode::Value, None, effects)],
            )
        }
        ProjectedProtocolFrame::Call {
            parent,
            callee: Some(callee),
            optional,
            ..
        } if projected_contains(*callee, value) => (
            *parent,
            HostEvaluationOperation::Reference(if *optional {
                ReferencePosition::OptionalCallCallee
            } else {
                ReferencePosition::CallCallee
            }),
            Vec::new(),
        ),
        ProjectedProtocolFrame::Call {
            parent,
            callee,
            callee_mode,
            callee_receiver,
            arguments,
            type_args,
            optional,
        } => {
            let Some(position) = arguments
                .iter()
                .position(|(argument, _, _)| projected_contains(*argument, value))
            else {
                return Ok(None);
            };
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let operation = if *optional {
                conditional = Some(ProjectedConditionalFacts {
                    branch: arguments[position].0,
                    skipped: None,
                    operands: arguments
                        .iter()
                        .map(|(span, spread, _)| (*span, *spread))
                        .collect(),
                    type_args: *type_args,
                });
                HostEvaluationOperation::Conditional(ConditionalBranch::OptionalCallArgument(index))
            } else {
                HostEvaluationOperation::Eager(EagerPosition::CallArgument(index))
            };
            let inputs = callee
                .iter()
                .copied()
                .map(|callee| (callee, *callee_mode, *callee_receiver, Effects::ANY))
                .chain(arguments[..position].iter().map(|(argument, _, effects)| {
                    (*argument, EvaluationInputMode::Value, None, *effects)
                }))
                .collect();
            (*parent, operation, inputs)
        }
        ProjectedProtocolFrame::Member {
            parent,
            object: (object, _),
            ..
        } if projected_contains(*object, value) => (
            *parent,
            HostEvaluationOperation::Reference(ReferencePosition::MemberObject),
            Vec::new(),
        ),
        ProjectedProtocolFrame::Member {
            parent,
            object: (object, effects),
            property: Some(property),
        } if projected_contains(*property, value) => (
            *parent,
            HostEvaluationOperation::Reference(ReferencePosition::MemberProperty),
            vec![(*object, EvaluationInputMode::Value, None, *effects)],
        ),
        ProjectedProtocolFrame::Construct { parent, callee, .. }
            if projected_contains(*callee, value) =>
        {
            (
                *parent,
                HostEvaluationOperation::Reference(ReferencePosition::ConstructorCallee),
                Vec::new(),
            )
        }
        ProjectedProtocolFrame::Construct {
            parent,
            callee,
            arguments,
        } => {
            let Some(position) = arguments
                .iter()
                .position(|(argument, _, _)| projected_contains(*argument, value))
            else {
                return Ok(None);
            };
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let inputs = std::iter::once((*callee, EvaluationInputMode::Value, None, Effects::ANY))
                .chain(arguments[..position].iter().map(|(argument, _, effects)| {
                    (*argument, EvaluationInputMode::Value, None, *effects)
                }))
                .collect();
            (
                *parent,
                HostEvaluationOperation::Eager(EagerPosition::ConstructArgument(index)),
                inputs,
            )
        }
        ProjectedProtocolFrame::TaggedTemplate { parent, tag, .. }
            if projected_contains(*tag, value) =>
        {
            (
                *parent,
                HostEvaluationOperation::Reference(ReferencePosition::TaggedTemplateTag),
                Vec::new(),
            )
        }
        ProjectedProtocolFrame::TaggedTemplate {
            parent,
            tag,
            tag_mode,
            tag_receiver,
            expressions,
        } => {
            let Some(position) = expressions
                .iter()
                .position(|(span, _)| projected_contains(*span, value))
            else {
                return Ok(None);
            };
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let inputs = std::iter::once((*tag, *tag_mode, *tag_receiver, Effects::ANY))
                .chain(
                    expressions[..position]
                        .iter()
                        .copied()
                        .map(|(expression, effects)| {
                            (expression, EvaluationInputMode::Value, None, effects)
                        }),
                )
                .collect();
            (
                *parent,
                HostEvaluationOperation::Eager(EagerPosition::TemplateInterpolation(index)),
                inputs,
            )
        }
        ProjectedProtocolFrame::Template {
            parent,
            expressions,
        } => {
            let Some(position) = expressions
                .iter()
                .position(|(span, _)| projected_contains(*span, value))
            else {
                return Ok(None);
            };
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let inputs = expressions[..position]
                .iter()
                .copied()
                .map(|(expression, effects)| {
                    (expression, EvaluationInputMode::Value, None, effects)
                })
                .collect();
            (
                *parent,
                HostEvaluationOperation::Eager(EagerPosition::TemplateInterpolation(index)),
                inputs,
            )
        }
        ProjectedProtocolFrame::Jsx {
            parent,
            expressions,
        } => {
            let Some(position) = expressions
                .iter()
                .position(|(span, _)| projected_contains(*span, value))
            else {
                return Ok(None);
            };
            let index =
                u32::try_from(position).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
            let inputs = expressions[..position]
                .iter()
                .copied()
                .map(|(expression, effects)| {
                    (expression, EvaluationInputMode::Value, None, effects)
                })
                .collect();
            (
                *parent,
                HostEvaluationOperation::Eager(EagerPosition::JsxExpression(index)),
                inputs,
            )
        }
        ProjectedProtocolFrame::Suspend {
            parent,
            kind,
            value: Some(argument),
        } if projected_contains(*argument, value) => {
            (*parent, HostEvaluationOperation::Suspend(*kind), Vec::new())
        }
        _ => return Ok(None),
    };
    let parent = map_evaluation_span(segments, parent)?;
    let inputs = inputs
        .into_iter()
        .map(|(source, mode, receiver, effects)| {
            Ok(HostEvaluationInput {
                source: map_evaluation_span(segments, source)?,
                mode,
                receiver: receiver
                    .map(|receiver| map_evaluation_span(segments, receiver))
                    .transpose()?,
                effects: Effects {
                    requires_reference: mode != EvaluationInputMode::Value,
                    ..effects
                },
            })
        })
        .collect::<Result<Vec<_>, ProgramSyntaxError>>()?;
    let conditional = conditional
        .map(|facts| {
            Ok(ConditionalFacts {
                // The branch holds the tt value, so it has no contiguous
                // source mapping; its bounds map endpoint-by-endpoint.
                branch: map_evaluation_span(segments, facts.branch)?,
                skipped: facts
                    .skipped
                    .map(|span| map_evaluation_span(segments, span))
                    .transpose()?,
                operands: facts
                    .operands
                    .into_iter()
                    .map(|(span, spread)| {
                        Ok(ConditionalOperand {
                            span: map_evaluation_span(segments, span)?,
                            spread,
                        })
                    })
                    .collect::<Result<Vec<_>, ProgramSyntaxError>>()?,
                type_args: facts
                    .type_args
                    .map(|span| map_evaluation_span(segments, span))
                    .transpose()?,
            })
        })
        .transpose()?;
    Ok(Some(HostEvaluationStep {
        parent,
        operation,
        inputs,
        conditional,
    }))
}

fn map_evaluation_span(
    segments: &[ProjectionSourceSegment],
    projected: ProjectedSpan,
) -> Result<SourceSpan, ProgramSyntaxError> {
    source_span_for_projection(segments, projected).ok_or(
        ProgramSyntaxError::UnmappedEvaluationSpan {
            start: projected.start.0,
            end: projected.end.0,
        },
    )
}

fn projected_contains(container: ProjectedSpan, value: ProjectedSpan) -> bool {
    container.start <= value.start && value.end <= container.end
}

fn call_callee_mode(
    expression: &swc_ecma_ast::Expr,
) -> (EvaluationInputMode, Option<swc_common::Span>) {
    use swc_ecma_ast::{Expr as SwcExpr, OptChainBase};

    match expression {
        SwcExpr::Member(member) => (
            EvaluationInputMode::MemberReference,
            Some(member.obj.span()),
        ),
        SwcExpr::SuperProp(_) => (EvaluationInputMode::MemberReference, None),
        SwcExpr::OptChain(chain) => match &*chain.base {
            OptChainBase::Member(member) => (
                EvaluationInputMode::MemberReference,
                Some(member.obj.span()),
            ),
            OptChainBase::Call(_) => (EvaluationInputMode::DirectReference, None),
        },
        SwcExpr::Paren(paren) => call_callee_mode(&paren.expr),
        SwcExpr::TsAs(expression) => call_callee_mode(&expression.expr),
        SwcExpr::TsTypeAssertion(expression) => call_callee_mode(&expression.expr),
        SwcExpr::TsNonNull(expression) => call_callee_mode(&expression.expr),
        SwcExpr::TsInstantiation(expression) => call_callee_mode(&expression.expr),
        SwcExpr::TsSatisfies(expression) => call_callee_mode(&expression.expr),
        _ => (EvaluationInputMode::DirectReference, None),
    }
}

fn reference_value_span(expression: &swc_ecma_ast::Expr) -> swc_common::Span {
    use swc_ecma_ast::Expr as SwcExpr;

    match expression {
        SwcExpr::Paren(expression) => reference_value_span(&expression.expr),
        SwcExpr::TsAs(expression) => reference_value_span(&expression.expr),
        SwcExpr::TsTypeAssertion(expression) => reference_value_span(&expression.expr),
        SwcExpr::TsNonNull(expression) => reference_value_span(&expression.expr),
        SwcExpr::TsInstantiation(expression) => reference_value_span(&expression.expr),
        SwcExpr::TsSatisfies(expression) => reference_value_span(&expression.expr),
        _ => expression.span(),
    }
}

fn projected_span(span: swc_common::Span, source_start: u32) -> ProjectedSpan {
    ProjectedSpan {
        start: ProjectedByte(span.lo.0.saturating_sub(source_start) as usize),
        end: ProjectedByte(span.hi.0.saturating_sub(source_start) as usize),
    }
}

/// The source byte a projected byte was copied from, or `None` when no
/// copied segment owns it — a byte of a placeholder this compiler wrote.
fn source_byte_for_projection(
    segments: &[ProjectionSourceSegment],
    projected: ProjectedByte,
) -> Option<usize> {
    segments.iter().find_map(|segment| {
        (segment.kind == ProjectionSegmentKind::Copied
            && segment.projected.start <= projected
            && projected < segment.projected.end)
            .then(|| segment.source.start + projected.0 - segment.projected.start.0)
    })
}

fn source_span_for_projection(
    segments: &[ProjectionSourceSegment],
    projected: ProjectedSpan,
) -> Option<SourceSpan> {
    let start = segments.iter().find_map(|segment| {
        if projected.start == segment.projected.start {
            Some(segment.source.start)
        } else if segment.projected.start < projected.start
            && projected.start < segment.projected.end
            && segment.kind == ProjectionSegmentKind::Copied
        {
            Some(segment.source.start + projected.start.0 - segment.projected.start.0)
        } else {
            None
        }
    })?;
    let end = segments.iter().find_map(|segment| {
        if projected.end == segment.projected.end {
            Some(segment.source.end)
        } else if segment.projected.start < projected.end
            && projected.end < segment.projected.end
            && segment.kind == ProjectionSegmentKind::Copied
        {
            Some(segment.source.start + projected.end.0 - segment.projected.start.0)
        } else {
            None
        }
    })?;
    Some(SourceSpan { start, end })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn syntax(source: &str) -> ProgramSyntax {
        let program = crate::parser::parse(source);
        let semantic = crate::analysis::coverage_semantics(&program, &[]);
        let core = crate::core_ir::lower_semantic(&semantic, source);
        ProgramSyntax::build(&semantic, &core, source, crate::SourceKind::TypeScript)
            .expect("projection should parse")
    }

    fn build_error(source: &str) -> ProgramSyntaxError {
        let program = crate::parser::parse(source);
        let semantic = crate::analysis::coverage_semantics(&program, &[]);
        let core = crate::core_ir::lower_semantic(&semantic, source);
        ProgramSyntax::build(&semantic, &core, source, crate::SourceKind::TypeScript)
            .expect_err("projection should not parse")
    }

    #[test]
    fn a_parse_failure_in_copied_text_is_the_source_not_an_internal_error() {
        // The byte the parse stopped on was copied from the source, so the
        // cause is the user's TypeScript — carried with the source byte it
        // is reported at.
        let source = "const x = match (s) { A(v) => { const q = ; return q; }, _ => 0 };\n";
        let error = build_error(source);
        let ProgramSyntaxError::SourceNotTypeScript { source: at, .. } = error else {
            panic!("expected a source-caused failure, got {error:?}");
        };
        assert_eq!(&source[at..at + 1], ";");
    }

    #[test]
    fn a_projected_byte_maps_only_through_copied_segments() {
        // A placeholder's bytes are the compiler's own text and belong to no
        // source byte: that is what separates an input fact from a broken
        // invariant.
        let source = "const value = match (s) { A(v) => v, _ => 0 };\n";
        let program = crate::parser::parse(source);
        let semantic = crate::analysis::coverage_semantics(&program, &[]);
        let core = crate::core_ir::lower_semantic(&semantic, source);
        let projection = ProjectionBuilder::new(&semantic, &core, source)
            .build()
            .expect("projection");
        let segments = &projection.source_segments;
        let placeholder = segments
            .iter()
            .find(|segment| segment.kind == ProjectionSegmentKind::Placeholder)
            .expect("a placeholder segment");
        let inside = ProjectedByte(placeholder.projected.start.0 + 1);
        assert_eq!(source_byte_for_projection(segments, inside), None);

        let copied = segments
            .iter()
            .find(|segment| segment.kind == ProjectionSegmentKind::Copied)
            .expect("a copied segment");
        assert_eq!(
            source_byte_for_projection(segments, copied.projected.start),
            Some(copied.source.start)
        );
    }

    #[test]
    fn plain_typescript_projection_is_byte_identical() {
        let source = "// 주석\nconst value: { readonly x: number } = { x: 1 };\n";
        let syntax = syntax(source);
        assert_eq!(syntax.projection, source);
        assert!(syntax.overlay.is_empty());
    }

    #[test]
    fn a_match_argument_keeps_its_call_parent_path() {
        let syntax = syntax(
            "enum E { A, B(x: number) }\nconst out = render(1, match (e) { A => 0, B(x) => x });\n",
        );
        let entry = syntax
            .overlay
            .iter()
            .find(|entry| entry.category == SyntaxCategory::Expression)
            .expect("match overlay");
        assert!(
            entry
                .parents
                .iter()
                .any(|parent| matches!(parent, AstParentKind::CallExpr(_))),
            "{:?}",
            entry.parents
        );
        assert_eq!(entry.context.continuation, HostContinuation::Compose);
    }

    #[test]
    fn a_direct_return_is_a_function_statement_continuation() {
        let source = "enum E { A, B }\nfunction f(e: E) { return match (e) { A => 1, B => 2 }; }\n";
        let syntax = syntax(source);
        let context = syntax
            .overlay
            .iter()
            .find(|entry| entry.category == SyntaxCategory::Expression)
            .expect("match overlay")
            .context;
        assert_eq!(context.owner, EvaluationOwner::FunctionBody);
        assert_eq!(context.frequency, EvaluationFrequency::Once);
        assert_eq!(context.value_role, ValueRole::Value);
        assert_eq!(context.continuation, HostContinuation::Return);
        let entry = syntax
            .overlay
            .iter()
            .find(|entry| entry.category == SyntaxCategory::Expression)
            .expect("match overlay");
        let owner = entry.host_owner.span;
        assert_eq!(&source[owner.start..owner.start + 6], "return");
        assert_eq!(&source[owner.end - 1..owner.end], ";");
    }

    #[test]
    fn an_expression_bodied_arrow_has_an_arrow_return_continuation() {
        let syntax = syntax("enum E { A, B }\nconst f = (e: E) => match (e) { A => 1, B => 2 };\n");
        let entry = syntax
            .overlay
            .iter()
            .find(|entry| entry.category == SyntaxCategory::Expression)
            .expect("match overlay");
        let context = entry.context;
        assert_eq!(context.owner, EvaluationOwner::FunctionBody);
        assert_eq!(
            context.continuation,
            HostContinuation::ArrowReturn,
            "{:?}",
            entry.parents
        );
        assert!(entry.protocol.steps().is_empty(), "{:?}", entry.protocol);
    }

    #[test]
    fn a_nested_decision_keeps_the_outer_initializer_owner() {
        let syntax = syntax(
            "enum E { A, B }\nconst value = match (outer) { A => match (inner) { A => 1, B => 2 }, B => 0 };\n",
        );
        let entries = syntax
            .overlay
            .iter()
            .filter(|entry| matches!(entry.core_root, CoreRoot::Expr(_)))
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 2, "{:#?}", syntax.overlay);
        assert_eq!(
            entries[0].context.continuation,
            HostContinuation::Initialize
        );
        assert_eq!(entries[1].context.continuation, HostContinuation::Discard);
    }

    #[test]
    fn a_conditional_branch_stays_conditional() {
        let syntax =
            syntax("enum E { A, B }\nconst out = flag ? match (e) { A => 1, B => 2 } : 0;\n");
        let context = syntax
            .overlay
            .iter()
            .find(|entry| entry.category == SyntaxCategory::Expression)
            .expect("match overlay")
            .context;
        assert_eq!(context.frequency, EvaluationFrequency::Conditional);
        assert_eq!(context.continuation, HostContinuation::Compose);
    }

    #[test]
    fn an_eager_binary_rhs_has_an_explicit_evaluation_step() {
        let syntax = syntax("enum E { A, B }\nconst out = left + match (e) { A => 1, B => 2 };\n");
        let entry = syntax
            .overlay
            .iter()
            .find(|entry| entry.category == SyntaxCategory::Expression)
            .expect("match overlay");
        assert_eq!(entry.context.frequency, EvaluationFrequency::Indeterminate);
        assert_eq!(entry.context.continuation, HostContinuation::Compose);
        assert!(entry.protocol.steps().iter().any(|step| {
            step.operation == HostEvaluationOperation::Eager(EagerPosition::BinaryRight)
        }));
    }

    #[test]
    fn a_whole_variable_initializer_has_an_initialize_continuation() {
        let syntax = syntax("enum E { A, B }\nconst out = match (e) { A => 1, B => 2 };\n");
        let entry = syntax
            .overlay
            .iter()
            .find(|entry| entry.category == SyntaxCategory::Expression)
            .expect("match overlay");
        assert_eq!(entry.context.continuation, HostContinuation::Initialize);
        assert!(entry.protocol.steps().is_empty());
    }

    #[test]
    fn a_short_circuit_rhs_is_a_conditional_protocol_branch() {
        let syntax =
            syntax("enum E { A, B }\nconst out = ready && match (e) { A => 1, B => 2 };\n");
        let entry = syntax
            .overlay
            .iter()
            .find(|entry| entry.category == SyntaxCategory::Expression)
            .expect("match overlay");
        assert!(entry.protocol.steps().iter().any(|step| {
            step.operation
                == HostEvaluationOperation::Conditional(ConditionalBranch::LogicalAndRight)
        }));
    }

    #[test]
    fn a_member_call_preserves_the_reference_chain() {
        let syntax =
            syntax("enum E { A, B }\nconst out = (match (e) { A => left, B => right }).run();\n");
        let entry = syntax
            .overlay
            .iter()
            .find(|entry| entry.category == SyntaxCategory::Expression)
            .expect("match overlay");
        let references: Vec<_> = entry
            .protocol
            .steps()
            .iter()
            .filter_map(|step| match step.operation {
                HostEvaluationOperation::Reference(reference) => Some(reference),
                _ => None,
            })
            .collect();
        assert_eq!(
            references,
            vec![
                ReferencePosition::MemberObject,
                ReferencePosition::CallCallee
            ]
        );
    }

    #[test]
    fn a_call_argument_keeps_left_to_right_argument_position() {
        let syntax =
            syntax("enum E { A, B }\nconst out = render(first(), match (e) { A => 1, B => 2 });\n");
        let entry = syntax
            .overlay
            .iter()
            .find(|entry| entry.category == SyntaxCategory::Expression)
            .expect("match overlay");
        assert!(entry.protocol.steps().iter().any(|step| {
            step.operation == HostEvaluationOperation::Eager(EagerPosition::CallArgument(1))
        }));
    }

    #[test]
    fn an_await_operand_records_the_suspension_point() {
        let syntax = syntax(
            "enum E { A, B }\nasync function f(e: E) { return await match (e) { A => 1, B => 2 }; }\n",
        );
        let entry = syntax
            .overlay
            .iter()
            .find(|entry| entry.category == SyntaxCategory::Expression)
            .expect("match overlay");
        assert!(entry.protocol.steps().iter().any(|step| {
            step.operation == HostEvaluationOperation::Suspend(SuspensionKind::Await)
        }));
    }

    #[test]
    fn a_parameter_initializer_is_a_composed_owner_value() {
        let syntax = syntax(
            "enum E { A, B }\nfunction f(e: E, x = match (e) { A => 1, B => 2 }) { return x; }\n",
        );
        let context = syntax
            .overlay
            .iter()
            .find(|entry| entry.category == SyntaxCategory::Expression)
            .expect("match overlay")
            .context;
        assert_eq!(context.owner, EvaluationOwner::ParameterInitializer);
        assert_eq!(context.continuation, HostContinuation::Compose);
    }

    #[test]
    fn an_expression_in_a_loop_keeps_repeated_frequency() {
        let syntax =
            syntax("enum E { A, B }\nwhile (ready()) { consume(match (e) { A => 1, B => 2 }); }\n");
        let context = syntax
            .overlay
            .iter()
            .find(|entry| entry.category == SyntaxCategory::Expression)
            .expect("match overlay")
            .context;
        assert_eq!(context.frequency, EvaluationFrequency::Repeated);
    }

    #[test]
    fn every_core_surface_gets_a_typed_overlay() {
        let syntax = syntax(
            "enum E { A(x: number), B }\n\
             function f(e: E) {\n\
               if let A(x) = e { use(x); }\n\
               const A(y) = e else { return 0; };\n\
               const r = result { const z <- load(); z };\n\
               return match (e) { A(x) => x, B => r } |> done;\n\
             }\n",
        );
        assert!(
            syntax
                .overlay
                .iter()
                .any(|entry| entry.category == SyntaxCategory::Item)
        );
        assert!(
            syntax
                .overlay
                .iter()
                .any(|entry| entry.category == SyntaxCategory::Statement)
        );
        assert!(
            syntax
                .overlay
                .iter()
                .any(|entry| entry.category == SyntaxCategory::Expression)
        );
    }

    #[test]
    fn a_try_declaration_gets_a_whole_statement_overlay() {
        let syntax = syntax(
            "function run(r: Result<number, string>) {\n  const n = try r;\n  return n;\n}\n",
        );
        assert!(syntax.overlay.iter().any(|entry| {
            entry.category == SyntaxCategory::Statement
                && matches!(entry.core_root, CoreRoot::Propagate(_))
        }));
    }

    #[test]
    fn expressions_in_one_statement_share_one_stable_owner() {
        let syntax = syntax(
            "const out = [match (left) { A => 1, _ => 0 }, match (right) { B => 2, _ => 0 }];\n",
        );
        let owners: Vec<_> = syntax
            .overlay
            .iter()
            .filter(|entry| entry.category == SyntaxCategory::Expression)
            .map(|entry| entry.host_owner.id)
            .collect();
        assert_eq!(owners.len(), 2);
        assert_eq!(owners[0], owners[1]);
        let owner = syntax.owners().next().expect("statement owner");
        assert_eq!(owner.roots.len(), 2);
    }

    #[test]
    fn a_nested_statement_is_a_distinct_owner() {
        let syntax = syntax(
            "const out = [match (left) { A => 1, _ => 0 }, () => { return match (right) { B => 2, _ => 0 }; }];\n",
        );
        let owners: Vec<_> = syntax
            .overlay
            .iter()
            .filter(|entry| entry.category == SyntaxCategory::Expression)
            .map(|entry| entry.host_owner.id)
            .collect();
        assert_eq!(owners.len(), 2);
        assert_ne!(owners[0], owners[1]);
    }

    #[test]
    fn a_result_statement_island_exposes_its_nested_initializer_owner() {
        let syntax = syntax(
            "const outer = result {\n  const x <- f();\n  const inner = result { const y <- g(); y };\n  inner\n};\n",
        );
        let mut entries: Vec<_> = syntax
            .overlay
            .iter()
            .filter(|entry| matches!(entry.core_root, CoreRoot::Expr(_)))
            .collect();
        entries.sort_unstable_by_key(|entry| entry.source.start);
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].context.continuation,
            HostContinuation::Initialize
        );
        assert_eq!(
            entries[1].context.continuation,
            HostContinuation::Initialize
        );
        assert_ne!(entries[0].host_owner.id, entries[1].host_owner.id);
    }

    #[test]
    fn multibyte_source_and_projection_coordinates_do_not_mix() {
        let source = "const 한글 = match (e) { A => \"안녕\", _ => \"끝\" };\n";
        let syntax = syntax(source);
        let entry = syntax
            .overlay
            .iter()
            .find(|entry| entry.category == SyntaxCategory::Expression)
            .expect("match overlay");
        assert_eq!(entry.source.start, source.find("match").expect("match"));
        let projected = &syntax.projection[entry.projected.start.0..entry.projected.end.0];
        assert!(projected.starts_with("(() => {"), "{projected}");
        assert!(projected.contains("\"안녕\""), "{projected}");
        assert!(projected.contains("\"끝\""), "{projected}");
    }
}
