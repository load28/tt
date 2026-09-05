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

mod collector;
mod projection;
mod protocol;
mod visit;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use swc_common::input::StringInput;
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap, Spanned};
use swc_ecma_ast::{
    ArrayLit, ArrowExpr, AssignExpr, AwaitExpr, BinExpr, BinaryOp, BlockStmt, CallExpr, CondExpr,
    Constructor, Function, Ident, JSXAttrOrSpread, JSXAttrValue, JSXElement, JSXElementChild,
    JSXExpr, JSXFragment, MemberExpr, MemberProp, Module, ModuleItem, NewExpr, ObjectLit, OptCall,
    Pat, Prop, PropName, PropOrSpread, ReturnStmt, SeqExpr, Stmt, TaggedTpl, Tpl, UnaryExpr,
    VarDeclarator, YieldExpr,
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
use crate::lexer::Token;

use collector::*;
#[cfg(test)]
use projection::ProjectionBuilder;
pub(crate) use projection::{HostOwnerSyntax, ProgramSyntax, ProgramSyntaxError};
use projection::{OverlayMarker, PendingOverlay, ProjectionSegmentKind, ProjectionSourceSegment};
use protocol::*;

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
    /// A `try` statement projected as an expression plus its terminator.
    /// This stays valid in both ordinary statement streams and C-style
    /// `for` initializer headers.
    Propagation,
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
    /// The arm block this exit leaves, when the return sits directly in a
    /// projected arm body at the arm's own function depth.
    pub(crate) body: Option<BodyId>,
    /// Whether this exit's arm block is free of cleanup boundaries — no
    /// `try`, `with`, or `using` anywhere in the block outside nested
    /// functions — so a consuming call carried on the rewritten return
    /// cannot land inside a handler or run before a finalizer or disposal.
    pub(crate) call_safe: bool,
    /// The complete match arm body is exactly this value-returning AST
    /// statement. This identity is established by visiting the projected
    /// arm's BlockStmt, not inferred from source text during emission.
    pub(crate) single_return_body: Option<BodyId>,
    pub(crate) statement: SourceSpan,
    pub(crate) argument: Option<SourceSpan>,
    /// Whether the exit sits inside a statement that consumes an unlabeled
    /// `break` — a loop or a `switch` written in the arm body. The rewrite
    /// turns the `return` into a `break`, so such an exit is the only
    /// reason a value region needs a label: everywhere else the region's
    /// own dispatch is already the nearest `break` target.
    pub(crate) captured_break: bool,
    /// Whether replacing this one statement with assignment-plus-exit
    /// statements requires a block to remain one statement for its parent
    /// (`if (cond) return value`, loop bodies, and labeled statements).
    pub(crate) requires_block: bool,
}

/// Ordered JavaScript evaluation obligations between one TT value and its
/// minimum source-backed owner. Target lowering must consume every step.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct HostEvaluationProtocol {
    /// AST-proven call whose single non-spread argument is exactly the TT
    /// value. Target planning must still tie these facts to the value's
    /// innermost evaluation step before consuming the call.
    pub(crate) call_completion: Option<CallCompletionFacts>,
    steps: Vec<HostEvaluationStep>,
}

/// The syntactic facts of one completable call: a non-optional call
/// expression with a source-backed callee and exactly one non-spread
/// argument whose span is the whole TT value. The argument-span equality is
/// what licenses dispatch arms to perform the call themselves — a larger
/// argument expression (a cast, an operator) must keep its authored frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CallCompletionFacts {
    /// The whole call expression.
    pub(crate) call: SourceSpan,
    /// Whether the call's result flows onward. A call in expression-statement
    /// position is discarded; everywhere else the completed call must still
    /// deliver its result to the authored position.
    pub(crate) consumed: bool,
    /// The call's explicit type arguments, verbatim.
    pub(crate) type_args: Option<SourceSpan>,
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
    /// The complete loop boundary when this step owns a repeated condition.
    /// Target lowering uses these spans to move the condition's statement
    /// region inside the loop without changing its evaluation count.
    pub(crate) loop_test: Option<LoopTestFacts>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LoopTestFacts {
    pub(crate) kind: LoopTestKind,
    pub(crate) test: SourceSpan,
    pub(crate) body: SourceSpan,
    pub(crate) update: Option<SourceSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopTestKind {
    While,
    For,
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
    /// A member reference's receiver and its independent effect proof.
    pub(crate) receiver: Option<(SourceSpan, Effects)>,
    /// What evaluating this input may do — an optimization fact only.
    pub(crate) effects: Effects,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvaluationInputMode {
    Value,
    JsxChildValue,
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
/// [`Effects::NONE`]: plain literals (a regex literal runs its own
/// construction, so it is not one), object and array literals built from
/// them, and function creation — possibly under TypeScript's transparent
/// expression wrappers. Identifiers may read mutable bindings and may throw
/// (TDZ); user types never prove runtime purity; everything unknown is
/// [`Effects::ANY`].
///
/// A fresh object or array is allocated per evaluation, and so is a
/// closure. That allocation is not observable here because eliding a
/// capture does not change how often the expression is evaluated — only
/// where — and nothing else holds the value to compare it against
/// ([`Effects::is_inert`]).
fn expression_effects(expression: &swc_ecma_ast::Expr) -> Effects {
    use swc_ecma_ast::{Expr as SwcExpr, Lit};
    match expression {
        SwcExpr::Lit(Lit::Str(_) | Lit::Bool(_) | Lit::Null(_) | Lit::Num(_) | Lit::BigInt(_)) => {
            Effects::NONE
        }
        // Creating a function does not execute its body or parameter
        // initializers. Keeping it in its host also preserves contextual
        // parameter inference; each authored function is still evaluated once.
        SwcExpr::Arrow(_) | SwcExpr::Fn(_) => Effects::NONE,
        SwcExpr::Object(object) => object_literal_effects(object),
        SwcExpr::Array(array) => array_literal_effects(array),
        SwcExpr::Paren(inner) => expression_effects(&inner.expr),
        SwcExpr::TsAs(inner) => expression_effects(&inner.expr),
        SwcExpr::TsSatisfies(inner) => expression_effects(&inner.expr),
        SwcExpr::TsNonNull(inner) => expression_effects(&inner.expr),
        SwcExpr::TsTypeAssertion(inner) => expression_effects(&inner.expr),
        SwcExpr::TsInstantiation(inner) => expression_effects(&inner.expr),
        _ => Effects::ANY,
    }
}

/// Defining a property does not call a setter, and defining an accessor or
/// method does not run its body, so an object literal is as observable as
/// the expressions it evaluates: its computed keys and its property values.
/// A spread reads its operand and may run getters; shorthand reads a
/// binding.
fn object_literal_effects(node: &ObjectLit) -> Effects {
    for property in &node.props {
        let inert = match property {
            PropOrSpread::Spread(_) => false,
            PropOrSpread::Prop(property) => match &**property {
                Prop::Shorthand(_) => false,
                Prop::KeyValue(property) => {
                    prop_name_is_inert(&property.key)
                        && expression_effects(&property.value).is_inert()
                }
                // `{ key = value }` only parses inside a destructuring
                // pattern, where this classification is never consulted.
                Prop::Assign(_) => false,
                Prop::Getter(property) => prop_name_is_inert(&property.key),
                Prop::Setter(property) => prop_name_is_inert(&property.key),
                Prop::Method(property) => prop_name_is_inert(&property.key),
            },
        };
        if !inert {
            return Effects::ANY;
        }
    }
    Effects::NONE
}

fn array_literal_effects(node: &ArrayLit) -> Effects {
    for element in node.elems.iter().flatten() {
        // A spread iterates its operand, which runs user code.
        if element.spread.is_some() || !expression_effects(&element.expr).is_inert() {
            return Effects::ANY;
        }
    }
    Effects::NONE
}

fn prop_name_is_inert(name: &PropName) -> bool {
    match name {
        PropName::Ident(_) | PropName::Str(_) | PropName::Num(_) | PropName::BigInt(_) => true,
        PropName::Computed(computed) => expression_effects(&computed.expr).is_inert(),
    }
}

/// Computes the conservative effect fact for one source-backed expression.
/// The span comes from HIR; SWC owns the expression classification, so
/// target optimization never guesses purity from source text.
pub(crate) fn source_expression_effects(
    source: &str,
    span: crate::hir::Span,
    source_kind: crate::SourceKind,
) -> Effects {
    let Some(text) = source.get(span.start..span.end) else {
        return Effects::ANY;
    };
    if crate::lexer::host_syntax_error(text, source_kind).is_some() {
        return Effects::ANY;
    }
    let source_map: Lrc<SourceMap> = Default::default();
    let file = source_map.new_source_file(Lrc::new(FileName::Anon), text.to_owned());
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
    let expression = match parser.parse_expr() {
        Ok(expression) if parser.take_errors().is_empty() => expression,
        Ok(_) | Err(_) => return Effects::ANY,
    };
    expression_effects(&expression)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostEvaluationOperation {
    Eager(EagerPosition),
    Conditional(ConditionalBranch),
    Reference(ReferencePosition),
    Suspend(SuspensionKind),
    LoopTest,
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
    Constructor,
    Generator,
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
    /// A declaration initializer in a C-style `for` header. Its propagation
    /// prelude runs before the loop, while the header retains the payload
    /// declaration.
    ForInitialize,
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
    /// Authored TypeScript annotation that contextually types this value,
    /// including its leading colon.
    pub(crate) contextual_type: Option<SourceSpan>,
    /// Async functions contextually type their returned expression with the
    /// awaited form of the authored Promise return type.
    pub(crate) contextual_type_awaited: bool,
}

impl EvaluationContext {
    /// `host_owner_edge` is where the chosen [`HostOwner`]'s own child edges
    /// begin in `parents`; everything from there down sits between the owner
    /// and the value.
    fn from_path(
        category: SyntaxCategory,
        parents: &[AstParentKind],
        host_owner_edge: usize,
        function_target: Option<EvaluationOwner>,
        contextual_type: Option<SourceSpan>,
        function_return_type: Option<SourceSpan>,
        function_return_awaited: bool,
    ) -> Self {
        let (mut owner, owner_edge) = evaluation_owner(parents);
        // The AST path owns local positions such as parameters and class
        // initializers. Function-target metadata only refines a function
        // body into the return contracts that differ from an ordinary
        // function; an ordinary nested function still acts as a lexical
        // barrier against an outer generator or constructor.
        if owner == EvaluationOwner::FunctionBody
            && let Some(function_target) = function_target
        {
            owner = function_target;
        }
        let owner_reach = owner_reach(&parents[host_owner_edge.min(parents.len())..]);
        if !matches!(
            category,
            SyntaxCategory::Expression | SyntaxCategory::Propagation
        ) {
            return Self {
                frequency: frequency_within_owner(parents, owner_edge),
                owner_reach,
                owner,
                value_role: ValueRole::None,
                continuation: HostContinuation::Discard,
                contextual_type,
                contextual_type_awaited: false,
            };
        }

        let local_path = &parents[owner_edge..];
        let value_role = value_role(local_path);
        let frequency = frequency_within_owner(parents, owner_edge);
        let continuation = host_continuation(local_path);
        let uses_function_return = matches!(
            continuation,
            HostContinuation::Return | HostContinuation::ArrowReturn
        );
        let contextual_type = if uses_function_return {
            function_return_type
        } else {
            contextual_type
        };
        Self {
            frequency,
            owner_reach,
            owner,
            value_role,
            continuation,
            contextual_type,
            contextual_type_awaited: uses_function_return
                && function_return_type.is_some()
                && function_return_awaited,
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
    if parents
        .iter()
        .any(|parent| matches!(parent, AstParentKind::ForStmt(fields::ForStmtField::Init)))
    {
        return HostContinuation::ForInitialize;
    }
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
