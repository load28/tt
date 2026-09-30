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

use swc_common::Spanned;
use swc_ecma_ast::{
    ArrayLit, ArrowExpr, AssignExpr, AssignOp, AwaitExpr, BinExpr, BinaryOp, BlockStmt, CallExpr,
    CondExpr, Constructor, Function, Ident, JSXAttrOrSpread, JSXAttrValue, JSXElement,
    JSXElementChild, JSXExpr, JSXFragment, MemberExpr, MemberProp, Module, ModuleItem, NewExpr,
    ObjectLit, OptCall, Pat, Prop, PropName, PropOrSpread, ReturnStmt, SeqExpr, Stmt, TaggedTpl,
    Tpl, TsType, TsTypeAnn, UnaryExpr, VarDeclarator, YieldExpr,
};
use swc_ecma_visit::{AstNodePath, AstParentKind, VisitAstPath, VisitWithAstPath, fields};

use crate::analysis::SemanticFile;
use crate::core_ir::{
    Adt, Apply, CoreFile, Decision, Expr, Import, Propagate, ResultRegion, Statement, Template,
    TemplatePart,
};
use crate::hir::ids::Idx;
use crate::hir::{self, BodyId, ExprId, NodeId};
use crate::host_input::{HostInput, HostOrigin};

use collector::*;
#[cfg(test)]
use projection::ProjectionBuilder;
pub(crate) use projection::{HostOwnerSyntax, ProgramSyntax, ProgramSyntaxError};
use projection::{
    OverlayMarker, PendingOverlay, ProjectionSegmentKind, ProjectionSegments,
    ProjectionSourceSegment,
};
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
    /// AST value beneath parentheses and TypeScript expression wrappers.
    pub(crate) value_argument: Option<SourceSpan>,
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
/// expression with a source-backed callee whose final non-spread argument
/// contains the whole TT value. Containment — rather than equality — is
/// what licenses dispatch arms to perform the call themselves; target
/// planning decides separately whether the authored text between the
/// argument and the value may be re-emitted inside the arms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CallCompletionFacts {
    /// The whole call expression.
    pub(crate) call: SourceSpan,
    /// The final argument. Equal to the value's own span when the value is
    /// the whole argument; wider when the value sits inside a literal the
    /// argument builds.
    pub(crate) argument: SourceSpan,
    /// Whether the path from the argument down to the value runs only
    /// through whole object- and array-literal positions, so the authored
    /// text around the value can be re-emitted around an arm's value and
    /// still mean the same thing. Trivially true when the value *is* the
    /// argument.
    pub(crate) literal_positions: bool,
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
    /// What an optional call's arguments are conditional on. `None` for
    /// other operations.
    pub(crate) optional_test: Option<OptionalCallTest>,
}

/// The link of its optional chain an optional call is skipped at. A chain
/// short-circuits at the first `?.` whose base is `undefined` or `null`
/// (ECMA-262 §13.3.9.1), and everything after that link, the call and its
/// arguments included, is skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptionalCallTest {
    /// `callee?.(...)`: the call's own link. The call is skipped when the
    /// callee is nullish, which is also the case when a `?.` inside the
    /// callee short-circuits it (`o?.m?.(...)`).
    Callee,
    /// `receiver?.name(...)` or `receiver?.[key](...)`: the link of the
    /// callee's member access. The call is skipped when the receiver is
    /// nullish; otherwise the callee is called, and a callee that is not
    /// callable throws.
    Receiver,
    /// A `?.` further inside the callee's receiver (`a?.b.m(...)`,
    /// `a?.b.m?.(...)`), or a call the callee chains from (`f?.()(...)`).
    /// Whether the chain short-circuits is known only by evaluating that
    /// link's base, which no captured input of the call holds.
    Inner,
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
    /// A member reference's receiver (the member's object).
    pub(crate) receiver: Option<HostReferencePart>,
    /// A member reference's computed key.
    pub(crate) key: Option<HostReferencePart>,
    /// What evaluating this input may do — an optimization fact only.
    pub(crate) effects: Effects,
}

/// One part of a member reference that is evaluated before a call's
/// arguments: the member's object or its computed key. The member itself is
/// read where the call is made, so the call stays a member call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HostReferencePart {
    pub(crate) source: SourceSpan,
    /// An optimization fact only, as [`HostEvaluationInput::effects`].
    pub(crate) effects: Effects,
    /// An authored identifier or `this`, read again where the call is made
    /// instead of captured, as TypeScript's own down-level transforms read a
    /// simple-copiable operand (`isSimpleCopiableExpression`). The call then
    /// keeps the reference TypeScript narrows.
    pub(crate) read_at_call: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvaluationInputMode {
    Value,
    JsxChildValue,
    DirectReference,
    MemberReference,
    /// The value a compound assignment's target holds before its right
    /// operand runs (ECMA-262 §13.15.2: `GetValue(lref)` precedes the right
    /// operand). The source is the whole target; the capture is an
    /// accumulator the assignment then applies its operator to, so
    /// `t += v` becomes `t = accumulator += v`.
    CompoundAssignmentTarget {
        operator: &'static str,
    },
    /// An operand of a comma expression before the value's operand. The
    /// comma operator evaluates it and discards its value (ECMA-262
    /// §13.16.1: `GetValue` of the left operand, whose result is not used),
    /// so the lowering evaluates it as an expression statement in order and
    /// removes it, with its comma, where it was written.
    Discarded,
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
    let input = HostInput::new(text);
    let mut parser = input.parser(source_kind);
    let expression = match parser.parse_expr() {
        Ok(expression) if parser.take_errors().is_empty() => expression,
        Ok(_) | Err(_) => return Effects::ANY,
    };
    expression_effects(&expression)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MemberCallee {
    pub(crate) receiver: Option<SourceSpan>,
    /// A computed key the step evaluates before the member is read. A
    /// simple-copiable key is none: it stays where it was written.
    pub(crate) key: Option<SourceSpan>,
    pub(crate) grouped: bool,
    pub(crate) optional: bool,
}

/// A key TypeScript's own down-level transforms copy instead of capturing
/// (`isSimpleCopiableExpression`): a string, template, or numeric literal,
/// a keyword such as `this`, or an identifier. Reading it where the member
/// is read, right after the receiver, is the order `receiver[key]` reads it
/// in, and it keeps the type TypeScript gives it there: a literal key
/// captured as a parameter would widen (`"m"` to `string`, `0` to
/// `number`) and no longer name its member.
fn simple_copiable(key: &swc_ecma_ast::Expr) -> bool {
    use swc_ecma_ast::{Expr as SwcExpr, Lit};
    match key {
        SwcExpr::Lit(Lit::Str(_) | Lit::Num(_) | Lit::Bool(_) | Lit::Null(_)) => true,
        SwcExpr::Tpl(template) => template.exprs.is_empty(),
        SwcExpr::Ident(_) | SwcExpr::This(_) => true,
        _ => false,
    }
}

pub(crate) fn source_member_callee(
    source: &str,
    span: crate::hir::Span,
    source_kind: crate::SourceKind,
) -> Option<MemberCallee> {
    use swc_ecma_ast::{Expr as SwcExpr, MemberProp, OptChainBase, SuperProp};

    let text = source.get(span.start..span.end)?;
    if crate::lexer::host_syntax_error(text, source_kind).is_some() {
        return None;
    }
    let input = HostInput::new(text);
    let parse = |context: swc_ecma_parser::Context| {
        let mut parser = input.parser(source_kind);
        parser.set_ctx(parser.ctx() | context);
        match parser.parse_expr() {
            Ok(expression) if parser.take_errors().is_empty() => Some(expression),
            Ok(_) | Err(_) => None,
        }
    };
    let expression = parse(swc_ecma_parser::Context::empty()).or_else(|| {
        parse(
            swc_ecma_parser::Context::Module
                | swc_ecma_parser::Context::CanBeModule
                | swc_ecma_parser::Context::InAsync
                | swc_ecma_parser::Context::InGenerator,
        )
    })?;
    let at = |node: swc_common::Span| SourceSpan {
        start: span.start + input.byte(node.lo),
        end: span.start + input.byte(node.hi),
    };
    let grouped = matches!(
        &*expression,
        SwcExpr::TsAs(_) | SwcExpr::TsSatisfies(_) | SwcExpr::TsTypeAssertion(_)
    );
    let mut callee = &*expression;
    loop {
        callee = match callee {
            SwcExpr::Paren(inner) => &inner.expr,
            SwcExpr::TsNonNull(inner) => &inner.expr,
            SwcExpr::TsAs(inner) => &inner.expr,
            SwcExpr::TsSatisfies(inner) => &inner.expr,
            SwcExpr::TsTypeAssertion(inner) => &inner.expr,
            _ => break,
        };
    }
    match callee {
        SwcExpr::Member(member) => Some(MemberCallee {
            receiver: Some(at(member.obj.span())),
            key: match &member.prop {
                MemberProp::Computed(computed) if !simple_copiable(&computed.expr) => {
                    Some(at(computed.expr.span()))
                }
                MemberProp::Computed(_) | MemberProp::Ident(_) | MemberProp::PrivateName(_) => None,
            },
            grouped,
            optional: false,
        }),
        SwcExpr::SuperProp(member) => Some(MemberCallee {
            receiver: None,
            key: match &member.prop {
                SuperProp::Computed(computed) if !simple_copiable(&computed.expr) => {
                    Some(at(computed.expr.span()))
                }
                SuperProp::Computed(_) | SuperProp::Ident(_) => None,
            },
            grouped,
            optional: false,
        }),
        SwcExpr::OptChain(chain) if matches!(&*chain.base, OptChainBase::Member(_)) => {
            let mut root = None;
            let mut node = callee;
            loop {
                node = match node {
                    SwcExpr::OptChain(link) => {
                        let object = match &*link.base {
                            OptChainBase::Member(member) => &member.obj,
                            OptChainBase::Call(call) => &call.callee,
                        };
                        if link.optional {
                            root = Some(&**object);
                        }
                        object
                    }
                    SwcExpr::Member(member) => &member.obj,
                    SwcExpr::Call(call) => match &call.callee {
                        swc_ecma_ast::Callee::Expr(callee) => callee,
                        swc_ecma_ast::Callee::Super(_) | swc_ecma_ast::Callee::Import(_) => break,
                    },
                    _ => break,
                };
            }
            Some(MemberCallee {
                receiver: Some(at(root?.span())),
                key: None,
                grouped,
                optional: true,
            })
        }
        _ => None,
    }
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
    /// Where the statement a prelude hoisted to this owner is written
    /// before begins; it ends where the owner does. See [`HostOwner::anchor`].
    anchor_start: usize,
    statement: SourceSpan,
    pub(crate) split: Option<DeclaratorSplit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct DeclaratorSplit {
    pub(crate) previous_end: usize,
    pub(crate) kind: DeclarationKind,
    pub(crate) exported: bool,
    pub(crate) declared: bool,
}

impl DeclaratorSplit {
    pub(crate) fn head(&self) -> String {
        let keyword = match self.kind {
            DeclarationKind::Var => "var",
            DeclarationKind::Let => "let",
            DeclarationKind::Const => "const",
            DeclarationKind::Using => "using",
            DeclarationKind::AwaitUsing => "await using",
        };
        format!(
            "{}{}{keyword} ",
            if self.exported { "export " } else { "" },
            if self.declared { "declare " } else { "" },
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DeclarationKind {
    Var,
    Let,
    Const,
    Using,
    AwaitUsing,
}

impl HostOwner {
    /// The statement a prelude hoisted to this owner is written before: the
    /// owner itself, or the outermost label of the labels an iteration
    /// statement owner stands under. A `continue` can name a label only
    /// when the label applies directly to its loop (ECMA-262 §14.13.1,
    /// ContainsUndefinedContinueTarget), so the prelude — and any block
    /// the owner needs — must enclose the labels rather than separate them
    /// from the loop.
    pub(crate) fn anchor(&self) -> SourceSpan {
        SourceSpan {
            start: self.anchor_start,
            end: self.span.end,
        }
    }

    pub(crate) fn statement(&self) -> SourceSpan {
        self.statement
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GlobalStatement {
    Enclose,
    Binding(String),
}

fn is_script(module: &Module) -> bool {
    use swc_ecma_ast::{ForOfStmt, MetaPropExpr, MetaPropKind, ModuleDecl, TsModuleRef, UsingDecl};
    use swc_ecma_visit::{Visit, VisitWith};

    #[derive(Default)]
    struct ModuleOnlySyntax {
        found: bool,
        function_depth: usize,
    }
    impl Visit for ModuleOnlySyntax {
        fn visit_meta_prop_expr(&mut self, node: &MetaPropExpr) {
            self.found |= node.kind == MetaPropKind::ImportMeta;
        }
        fn visit_await_expr(&mut self, node: &AwaitExpr) {
            self.found |= self.function_depth == 0;
            node.visit_children_with(self);
        }
        fn visit_for_of_stmt(&mut self, node: &ForOfStmt) {
            self.found |= node.is_await && self.function_depth == 0;
            node.visit_children_with(self);
        }
        fn visit_using_decl(&mut self, node: &UsingDecl) {
            self.found |= node.is_await && self.function_depth == 0;
            node.visit_children_with(self);
        }
        fn visit_function(&mut self, node: &Function) {
            self.function_depth += 1;
            node.visit_children_with(self);
            self.function_depth -= 1;
        }
        fn visit_arrow_expr(&mut self, node: &ArrowExpr) {
            self.function_depth += 1;
            node.visit_children_with(self);
            self.function_depth -= 1;
        }
        fn visit_class(&mut self, node: &swc_ecma_ast::Class) {
            self.function_depth += 1;
            node.visit_children_with(self);
            self.function_depth -= 1;
        }
    }

    let indicator = module.body.iter().any(|item| match item {
        ModuleItem::ModuleDecl(ModuleDecl::TsImportEquals(import)) => {
            import.is_export || matches!(import.module_ref, TsModuleRef::TsExternalModuleRef(_))
        }
        ModuleItem::ModuleDecl(_) => true,
        ModuleItem::Stmt(_) => false,
    });
    if indicator {
        return false;
    }
    let mut syntax = ModuleOnlySyntax::default();
    module.visit_with(&mut syntax);
    !syntax.found
}

fn global_statements(
    module: &Module,
    source_start: HostOrigin,
) -> HashMap<ProjectedSpan, GlobalStatement> {
    module
        .body
        .iter()
        .filter_map(|item| match item {
            ModuleItem::Stmt(statement) => Some((
                projected_span(statement.span(), source_start),
                global_statement(statement)?,
            )),
            ModuleItem::ModuleDecl(_) => None,
        })
        .collect()
}

fn global_statement(statement: &Stmt) -> Option<GlobalStatement> {
    use swc_ecma_ast::{Decl, VarDeclKind};

    let binding = match statement {
        Stmt::Decl(Decl::Var(var)) if var.kind == VarDeclKind::Var => None,
        Stmt::Decl(Decl::Var(var)) => var
            .decls
            .iter()
            .find_map(|declarator| first_binding(&declarator.name)),
        Stmt::Decl(Decl::Using(using)) => using
            .decls
            .iter()
            .find_map(|declarator| first_binding(&declarator.name)),
        Stmt::Decl(Decl::Class(class)) => Some(class.ident.sym.to_string()),
        Stmt::Decl(Decl::Fn(function)) => Some(function.ident.sym.to_string()),
        Stmt::Decl(
            Decl::TsEnum(_) | Decl::TsModule(_) | Decl::TsInterface(_) | Decl::TsTypeAlias(_),
        ) => return None,
        _ => None,
    };
    Some(binding.map_or(GlobalStatement::Enclose, GlobalStatement::Binding))
}

fn first_binding(pattern: &Pat) -> Option<String> {
    use swc_ecma_ast::ObjectPatProp;

    match pattern {
        Pat::Ident(ident) => Some(ident.id.sym.to_string()),
        Pat::Array(array) => array.elems.iter().flatten().find_map(first_binding),
        Pat::Object(object) => object.props.iter().find_map(|property| match property {
            ObjectPatProp::KeyValue(property) => first_binding(&property.value),
            ObjectPatProp::Assign(property) => Some(property.key.id.sym.to_string()),
            ObjectPatProp::Rest(rest) => first_binding(&rest.arg),
        }),
        Pat::Rest(rest) => first_binding(&rest.arg),
        Pat::Assign(assign) => first_binding(&assign.left),
        Pat::Invalid(_) | Pat::Expr(_) => None,
    }
}

fn let_else_global_binding(
    semantic: &SemanticFile,
    core: &CoreFile,
    source: &str,
    extent: NodeId,
) -> Option<GlobalStatement> {
    use crate::core_ir::{DecisionKind, PatternPlan};

    fn binds(pattern: &PatternPlan, out: &mut Vec<NodeId>) {
        match pattern {
            PatternPlan::Bind(bind) => out.push(bind.binding),
            PatternPlan::AllOf(parts) | PatternPlan::AnyOf(parts) => {
                for part in parts {
                    binds(part, out);
                }
            }
            PatternPlan::Any | PatternPlan::Test(_) => {}
        }
    }

    let decision = core
        .bodies
        .iter()
        .flat_map(|body| &body.statements)
        .find_map(|statement| match statement {
            Statement::Decision(decision) if decision.extent == extent => Some(decision),
            _ => None,
        })?;
    let DecisionKind::LetElse { binding_mode, .. } = decision.kind else {
        return None;
    };
    if binding_mode == hir::BindingMode::Var {
        return Some(GlobalStatement::Enclose);
    }
    let mut nodes = Vec::new();
    for arm in &decision.arms {
        binds(&arm.pattern, &mut nodes);
    }
    Some(
        nodes
            .into_iter()
            .filter_map(|node| semantic.hir.source_map.node_span(node))
            .min_by_key(|span| span.start)
            .map_or(GlobalStatement::Enclose, |span| {
                GlobalStatement::Binding(source[span.start..span.end].to_owned())
            }),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum HostOwnerKind {
    Statement,
    ModuleItem,
    /// The expression body of a concise arrow function. Lowering rewrites
    /// this expression to a block when a nested tt value needs statements.
    ArrowExpression,
    Declarator,
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
    ClassDefinition,
    StaticBlock,
    EnumInitializer,
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
    pub(crate) contextual_type_asserted: bool,
    /// The owning statement is the unbraced body of an `if`, loop, label, or
    /// `with`, so a statement lowering has to open its own block there.
    pub(crate) requires_block: bool,
    /// The construct sits inside an ambient (`declare`) module, where only
    /// declarations without initializers are TypeScript.
    pub(crate) ambient: bool,
    pub(crate) loop_head_declarator: bool,
}

pub(crate) struct OverlayFacts {
    pub(crate) function_target: Option<EvaluationOwner>,
    pub(crate) contextual_type: Option<SourceSpan>,
    pub(crate) function_return_type: Option<SourceSpan>,
    pub(crate) function_return_awaited: bool,
    pub(crate) assertion: Option<Option<SourceSpan>>,
    pub(crate) ambient: bool,
    pub(crate) decorated_classes: Vec<usize>,
    pub(crate) value_is_owner: bool,
}

impl EvaluationContext {
    /// `host_owner_edge` is where the chosen [`HostOwner`]'s own child edges
    /// begin in `parents`; everything from there down sits between the owner
    /// and the value.
    fn from_path(
        category: SyntaxCategory,
        parents: &[AstParentKind],
        host_owner_edge: usize,
        requires_block: bool,
        facts: OverlayFacts,
    ) -> Self {
        let OverlayFacts {
            function_target,
            contextual_type,
            function_return_type,
            function_return_awaited,
            assertion,
            ambient,
            decorated_classes,
            value_is_owner,
        } = facts;
        let (mut owner, owner_edge) = evaluation_owner(parents, &decorated_classes);
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
                contextual_type_asserted: false,
                requires_block,
                ambient,
                loop_head_declarator: false,
            };
        }

        let local_path = &parents[owner_edge..];
        let value_role = value_role(local_path);
        let frequency = frequency_within_owner(parents, owner_edge);
        let continuation = match host_continuation(local_path) {
            HostContinuation::ArrowReturn if !value_is_owner => HostContinuation::Compose,
            continuation => continuation,
        };
        let uses_function_return = matches!(
            continuation,
            HostContinuation::Return | HostContinuation::ArrowReturn
        );
        let asserted = local_path
            .iter()
            .rev()
            .take_while(|parent| is_transparent_expression_edge(parent))
            .any(is_assertion_edge);
        let (contextual_type, contextual_type_awaited) = match assertion {
            Some(assertion) if asserted => (assertion, false),
            _ if uses_function_return => (
                function_return_type,
                function_return_type.is_some() && function_return_awaited,
            ),
            _ => (contextual_type, false),
        };
        Self {
            frequency,
            owner_reach,
            owner,
            value_role,
            continuation,
            contextual_type,
            contextual_type_awaited,
            contextual_type_asserted: asserted,
            requires_block,
            ambient,
            loop_head_declarator: loop_head_declarator(local_path),
        }
    }
}

fn loop_head_declarator(parents: &[AstParentKind]) -> bool {
    parents
        .iter()
        .position(|parent| matches!(parent, AstParentKind::ForStmt(fields::ForStmtField::Init)))
        .and_then(|head| {
            parents[head..].iter().find_map(|parent| match parent {
                AstParentKind::VarDecl(fields::VarDeclField::Decls(index))
                | AstParentKind::UsingDecl(fields::UsingDeclField::Decls(index)) => Some(*index),
                _ => None,
            })
        })
        .is_some_and(|index| index > 0)
}

/// Whether the statement the path `above` leads into is the unbraced body
/// of an `if`, loop, label, or `with`: the one position where replacing that
/// statement with several leaves only the first under the parent.
fn is_unbraced_body(above: &[AstParentKind]) -> bool {
    above.last().is_some_and(|parent| {
        matches!(
            parent,
            AstParentKind::IfStmt(fields::IfStmtField::Cons | fields::IfStmtField::Alt)
                | AstParentKind::ForStmt(fields::ForStmtField::Body)
                | AstParentKind::ForInStmt(fields::ForInStmtField::Body)
                | AstParentKind::ForOfStmt(fields::ForOfStmtField::Body)
                | AstParentKind::WhileStmt(fields::WhileStmtField::Body)
                | AstParentKind::DoWhileStmt(fields::DoWhileStmtField::Body)
                | AstParentKind::LabeledStmt(fields::LabeledStmtField::Body)
                | AstParentKind::WithStmt(fields::WithStmtField::Body)
        )
    })
}

/// The owner a prelude for the innermost of `owners` is written before.
/// `owners` runs from the outermost enclosing host owner to the chosen one;
/// only an iteration statement looks through the labels naming it.
fn prelude_anchor<'o>(
    owners: &'o [ProjectedHostOwner],
    parents: &[AstParentKind],
) -> &'o ProjectedHostOwner {
    let mut index = owners.len() - 1;
    let owner = &owners[index];
    let iteration = owner.kind == HostOwnerKind::Statement
        && matches!(
            parents.get(owner.edge),
            Some(AstParentKind::Stmt(
                fields::StmtField::For
                    | fields::StmtField::ForIn
                    | fields::StmtField::ForOf
                    | fields::StmtField::While
                    | fields::StmtField::DoWhile
            ))
        );
    while iteration
        && index > 0
        && owners[index].edge > 0
        && matches!(
            parents.get(owners[index].edge - 1),
            Some(AstParentKind::LabeledStmt(fields::LabeledStmtField::Body))
        )
    {
        index -= 1;
    }
    &owners[index]
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

fn evaluation_owner(
    parents: &[AstParentKind],
    decorated_classes: &[usize],
) -> (EvaluationOwner, usize) {
    for (index, parent) in parents.iter().enumerate().rev() {
        match parent {
            AstParentKind::Class(
                fields::ClassField::Decorators(_) | fields::ClassField::Body(_),
            ) => {
                return (EvaluationOwner::ClassDefinition, index + 1);
            }
            AstParentKind::Class(fields::ClassField::SuperClass)
                if decorated_classes.contains(&index) =>
            {
                return (EvaluationOwner::ClassDefinition, index + 1);
            }
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
            AstParentKind::TsEnumMember(fields::TsEnumMemberField::Init) => {
                return (EvaluationOwner::EnumInitializer, index + 1);
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

fn is_assertion_edge(parent: &AstParentKind) -> bool {
    matches!(
        parent,
        AstParentKind::TsAsExpr(fields::TsAsExprField::Expr)
            | AstParentKind::TsSatisfiesExpr(fields::TsSatisfiesExprField::Expr)
            | AstParentKind::TsTypeAssertion(fields::TsTypeAssertionField::Expr)
    )
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
