//! The flow IR — tt's control-flow analysis.
//!
//! This is the compiler core's flow layer (`docs/design/compiler-core.md`
//! §9): **not** a TypeScript MIR, but a real control-flow graph over the
//! statement forms TypeScript actually has, built to answer the
//! control-flow questions tt's own constructs pose. The first consumer is
//! let-else — Rust's rule is "the `else` block must diverge", and the
//! graph answers it structurally, for every statement form, instead of by
//! the shape of the last statement.
//!
//! What the lowering models is the whole statement grammar that carries
//! control flow: the four diverging statements (`return`/`throw`/
//! `break`/`continue`, the latter two with their labels resolved to the
//! construct they leave), `if`/`else` (chains included), bare blocks,
//! labeled statements, every iteration statement (`while`, `do`-`while`,
//! C-style `for`, `for`-`in`/`of`, `for await`), `switch` (clause
//! fall-through, `default`, and `break`), and `try`/`catch`/`finally`.
//! Statement boundaries follow `;`, a statement body's closing brace, and
//! a restricted automatic-semicolon rule, so semicolon-free source reads
//! the same as semicolon-terminated source.
//!
//! Two things stay deliberately outside the graph, and both can only make
//! the answer "does not diverge", never a false "diverges":
//!
//! - **Function bodies written inside the stream are opaque.** Their
//!   `return` leaves *them*, not the enclosing function.
//! - **tt's own constructs (`match`, `if let`, `try`, `result`) are
//!   fall-through.** This layer runs on the token stream, before those
//!   constructs are parsed; deciding their divergence belongs to the HIR
//!   flow pass, which owns their lowered bodies.
//!
//! The block/expression brace distinction (an object literal's `}` ends no
//! statement) lives here too, moved from the let-else parser — one
//! implementation, shared by statement splitting wherever flow looks.

mod scanner;
mod syntax;

#[cfg(test)]
mod tests;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use crate::ast::{IfLetElse, IfLetStmt, Program, Segment};
use crate::lexer::{Token, TokenKind};

use scanner::*;
use syntax::*;
pub(crate) use syntax::{
    FunctionTarget, brace_opens_statement, concise_arrow_boundary_before, function_depth_at,
    function_target_at, in_function_body, in_static_block,
};

/// One body's control-flow graph.
#[derive(Debug)]
pub struct FlowBody {
    /// The blocks; [`BlockId`] indexes this.
    pub blocks: Vec<BasicBlock>,
    /// Where execution enters.
    pub entry: BlockId,
}

/// Memoized `flow_body` queries for one parse. The key is the complete body
/// text: flow lowering is translation-invariant, so structurally identical
/// bodies at different byte offsets share one answer.
#[derive(Debug, Default)]
pub(crate) struct FlowBodyQueries {
    cache: RefCell<HashMap<String, bool>>,
    hits: Cell<usize>,
}

impl FlowBodyQueries {
    pub(crate) fn diverges(
        &self,
        src: &str,
        span: crate::ast::Span,
        tokens: &[Token],
        program: &Program,
    ) -> bool {
        let text = &src[span.start..span.end];
        if let Some(answer) = self.cache.borrow().get(text).copied() {
            self.hits.set(self.hits.get() + 1);
            return answer;
        }
        let answer = program_diverges(src, tokens, program);
        self.cache.borrow_mut().insert(text.to_owned(), answer);
        answer
    }

    #[cfg(test)]
    pub(crate) fn hits(&self) -> usize {
        self.hits.get()
    }
}

/// Index into [`FlowBody::blocks`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockId(pub u32);

/// One basic block. The lowering keeps blocks minimal — the terminator is
/// the analysis; statements that neither branch nor diverge are a `Goto`.
#[derive(Debug)]
pub struct BasicBlock {
    /// How the block leaves.
    pub terminator: Terminator,
}

/// How control leaves a block.
///
/// The design's fuller shape (`Branch { condition: ExprId, .. }`) arrives
/// when bodies are lowered from the HIR; today's consumer asks about token
/// streams, so conditions and discriminants are not modeled — every split
/// is an unconditional two-way [`Terminator::Branch`], which is what makes
/// both successors reachable and the analysis conservative.
///
/// A `switch` needs no terminator of its own: testing a discriminant
/// against each `case` in source order *is* a chain of two-way branches,
/// so the graph states the dispatch exactly. `throw` needs none either —
/// it leaves the enclosing function like `return`, and a guarded block's
/// transfer to its handler is an edge the `try` lowering draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminator {
    /// Fall through to another block.
    Goto(BlockId),
    /// A two-way split: an `if`, a loop's exit test, a `case` test, or a
    /// guarded block's transfer to its handler.
    Branch {
        /// Where the then-branch enters.
        then_bb: BlockId,
        /// Where the else-branch enters — the join block when no `else`
        /// was written.
        else_bb: BlockId,
    },
    /// Leaves the enclosing function (`return`, `throw`).
    Return,
    /// A `break`/`continue` whose target lies *outside* the analyzed body
    /// — it leaves the body without reaching its end.
    Jump,
    /// Falls off the end of the analyzed body.
    End,
}

/// Whether every path through the body leaves it by `Return`/`Jump` —
/// i.e. no path reaches [`Terminator::End`]. This is Rust's "the else
/// block must diverge", answered on the graph instead of by the shape of
/// the last statement.
pub fn diverges(body: &FlowBody) -> bool {
    let mut seen = vec![false; body.blocks.len()];
    let mut stack = vec![body.entry];
    while let Some(BlockId(at)) = stack.pop() {
        let at = at as usize;
        if seen[at] {
            continue;
        }
        seen[at] = true;
        match body.blocks[at].terminator {
            Terminator::End => return false,
            Terminator::Return | Terminator::Jump => {}
            Terminator::Goto(next) => stack.push(next),
            Terminator::Branch { then_bb, else_bb } => {
                stack.push(then_bb);
                stack.push(else_bb);
            }
        }
    }
    true
}

/// Lowers a *parsed* statement stream — a let-else `else` block, whose tt
/// constructs the parser has already claimed — and answers whether it
/// diverges.
///
/// Of tt's constructs only `if let` can carry a region's divergence, and
/// that is a fact about placement rather than a limit of this layer: an
/// `if let` body and a let-else `else` block are **inline**, so an exit
/// written there leaves the enclosing function, while a match arm, a
/// `result` block and every other value region are isolated — an exit
/// written in one belongs to the construct's value and can never leave
/// the region it sits in (`crate::sema`'s `Place`). Treating them as
/// fall-through is therefore the exact answer, not an approximation.
pub(crate) fn program_diverges(src: &str, tokens: &[Token], program: &Program) -> bool {
    let mut heads = IfLetHeads::new();
    collect_if_let_heads(program, &mut heads);
    diverges(&lower_region(src, tokens, &heads))
}

/// The same query for one lexically delimited statement stream inside a
/// larger file.  The parser keeps source coordinates absolute, so callers
/// retain the original buffer and select only the tokens owned by the body.
pub(crate) fn program_diverges_in_span(
    src: &str,
    program: &Program,
    span: crate::ast::Span,
) -> bool {
    // The enclosing token stream may represent a template literal or JSX
    // host as one opaque token even though the parser recursively exposed a
    // Result body inside it. Lex the owned body span so its `return` and
    // control-flow statements are visible to the same structural analysis.
    let body_tokens = crate::lexer::lex(src, span.start, span.end);
    program_diverges(src, &body_tokens, program)
}

/// An abrupt completion that would leave a Result body instead of a
/// user-written loop or switch inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutwardControl {
    Break {
        span: crate::ast::Span,
        labeled: bool,
    },
    Continue {
        span: crate::ast::Span,
        labeled: bool,
    },
    Yield(crate::ast::Span),
}

/// Finds control transfers that a ResultRegion cannot own. The statement
/// scanner already resolves lexical loop, switch, and label scopes for the
/// CFG, so this query uses the same model instead of a second token walk.
pub(crate) fn outward_controls_in_span(
    src: &str,
    tokens: &[Token],
    program: &Program,
    span: crate::ast::Span,
) -> Vec<OutwardControl> {
    let start = tokens.partition_point(|token| token.span.start < span.start);
    let end = start + tokens[start..].partition_point(|token| token.span.end <= span.end);
    let mut heads = IfLetHeads::new();
    collect_if_let_heads(program, &mut heads);
    let function_depth = function_depth_at(src, tokens, start);
    let statements = Scanner {
        src,
        tokens: &tokens[start..end],
        if_lets: &heads,
    }
    .statements(0, end - start);
    let mut controls = Vec::new();
    collect_outward_controls(&statements, &mut Vec::new(), None, &mut controls);
    // `yield` can be nested in an expression (`const sent = yield value`),
    // where the statement model intentionally treats the declaration as
    // opaque. It still crosses the ResultRegion, so record the lexical
    // keyword while respecting nested user-written function bodies.
    for (index, token) in tokens[start..end].iter().enumerate() {
        let absolute = start + index;
        if !matches!(token.kind, TokenKind::Ident)
            || &src[token.span.start..token.span.end] != "yield"
            || function_depth_at(src, tokens, absolute) != function_depth
            || absolute
                .checked_sub(1)
                .and_then(|previous| tokens.get(previous))
                .is_some_and(|previous| matches!(previous.kind, TokenKind::Punct(b'.')))
        {
            continue;
        }
        let control = OutwardControl::Yield(token.span);
        if !controls.contains(&control) {
            controls.push(control);
        }
    }
    controls
}

/// Builds the CFG of one token stream treated as a statement sequence.
fn lower_region(src: &str, tokens: &[Token], if_lets: &IfLetHeads) -> FlowBody {
    let statements = Scanner {
        src,
        tokens,
        if_lets,
    }
    .statements(0, tokens.len());
    let mut builder = Builder { blocks: Vec::new() };
    let end = builder.block(Terminator::End);
    let entry = builder.seq(&statements, end, &mut Vec::new());
    FlowBody {
        blocks: builder.blocks,
        entry,
    }
}

/// Where each tt `if let` statement's head ends, keyed by the byte offset
/// of its `if`.
///
/// The scanner recognizes every TypeScript statement form from its own
/// shape, but `if let` is tt syntax: where its pattern ends, where the
/// scrutinee ends, and which `{` opens the then-block are decisions
/// [`crate::parser::iflets`] already made. Re-deciding them here would be
/// a second implementation of one rule, free to drift from the first — so
/// the parser's answer is handed in and the scanner asks it only "does a
/// tt statement start at this token, and where does its head end".
type IfLetHeads = HashMap<usize, usize>;

/// Every `if let` the scanner can reach: the region's own, and those
/// nested in one's then-block or `else` continuation. Constructs the
/// scanner never enters (a match arm, a `result` block) are not walked —
/// missing one could only cost a divergence, never invent one.
fn collect_if_let_heads(program: &Program, out: &mut IfLetHeads) {
    for segment in &program.segments {
        if let Segment::IfLet(stmt) = segment {
            collect_if_let(stmt, out);
        }
    }
}

fn collect_if_let(stmt: &IfLetStmt, out: &mut IfLetHeads) {
    out.insert(stmt.keyword_off, stmt.head_span.end);
    collect_if_let_heads(&stmt.body, out);
    match &stmt.else_part {
        Some(IfLetElse::Block(program)) => collect_if_let_heads(program, out),
        Some(IfLetElse::IfLet(chained)) => collect_if_let(chained, out),
        None => {}
    }
}

// ---------------------------------------------------------------------
// The statement model
// ---------------------------------------------------------------------

/// One statement, as far as the flow lowering cares. Labels borrow the
/// source: resolving `break label` is a comparison against the enclosing
/// [`Scope`]s, not a lookup, so nothing needs to own them.
#[derive(Debug)]
enum Stmt<'a> {
    /// `return`/`throw` — leaves the function.
    Return,
    /// `break [label]`.
    Break {
        label: Option<&'a str>,
        span: crate::ast::Span,
    },
    /// `continue [label]`.
    Continue {
        label: Option<&'a str>,
        span: crate::ast::Span,
    },
    /// `yield` and `yield*` leave a generator frame, which a ResultRegion
    /// must not capture or re-route.
    Yield(crate::ast::Span),
    /// `if (…) <then> [else <else>]`.
    If {
        then: Box<Stmt<'a>>,
        else_: Option<Box<Stmt<'a>>>,
    },
    /// A bare `{ … }` statement.
    Block(Vec<Stmt<'a>>),
    /// `label: <body>`.
    Labeled { label: &'a str, body: Box<Stmt<'a>> },
    /// Any iteration statement — they differ only in where the exit test
    /// sits and whether it can fail.
    Loop { kind: LoopKind, body: Box<Stmt<'a>> },
    /// `switch (…) { … }`, in clause order.
    Switch(Vec<Clause<'a>>),
    /// `try { … } [catch [(…)] { … }] [finally { … }]`.
    Try {
        block: Vec<Stmt<'a>>,
        catch: Option<Vec<Stmt<'a>>>,
        finally: Option<Vec<Stmt<'a>>>,
    },
    /// Anything else — falls through, and its interior is opaque.
    Other,
}

/// What distinguishes one iteration statement from another, as far as
/// control flow is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LoopKind {
    /// Whether the exit test runs before the first iteration. `false`
    /// only for `do … while (…)`, whose body always runs once.
    test_first: bool,
    /// Whether the test can fail, i.e. whether the loop has a normal
    /// exit at all. `false` for an omitted or literal-`true` condition;
    /// such a loop is left only by `break`, `return`, or `throw`.
    exits: bool,
}

/// One `case …:`/`default:` clause of a `switch`.
#[derive(Debug)]
struct Clause<'a> {
    /// `default:` rather than `case …:`.
    default: bool,
    /// The clause's statements, up to the next clause head.
    stmts: Vec<Stmt<'a>>,
}

// ---------------------------------------------------------------------
// Lowering the statement model to the graph
// ---------------------------------------------------------------------

/// One enclosing construct a `break`/`continue` can name.
#[derive(Debug, Clone, Copy)]
struct Scope<'a> {
    /// The label written in front of the construct, if any.
    label: Option<&'a str>,
    /// What an *unlabeled* `break`/`continue` may target here.
    kind: ScopeKind,
    /// Where a `break` leaving this construct lands.
    break_to: BlockId,
    /// Where a `continue` re-entering this construct lands — the exit
    /// test, which is the only construct that has one.
    continue_to: Option<BlockId>,
}

/// Which unlabeled jumps a [`Scope`] captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    /// A loop: captures unlabeled `break` and `continue`.
    Iteration,
    /// A `switch`: captures unlabeled `break` only.
    Switch,
    /// A labeled non-loop statement: captures nothing unlabeled.
    Labeled,
}

#[derive(Clone, Copy)]
struct ControlScope<'a> {
    label: Option<&'a str>,
    kind: ScopeKind,
}

fn control_break_target(scopes: &[ControlScope<'_>], label: Option<&str>) -> bool {
    scopes.iter().rev().any(|scope| match label {
        Some(name) => scope.label == Some(name),
        None => matches!(scope.kind, ScopeKind::Iteration | ScopeKind::Switch),
    })
}

fn control_continue_target(scopes: &[ControlScope<'_>], label: Option<&str>) -> bool {
    scopes.iter().rev().any(|scope| {
        scope.kind == ScopeKind::Iteration && label.is_none_or(|name| scope.label == Some(name))
    })
}

fn collect_outward_controls<'a>(
    statements: &[Stmt<'a>],
    scopes: &mut Vec<ControlScope<'a>>,
    label: Option<&'a str>,
    controls: &mut Vec<OutwardControl>,
) {
    for statement in statements {
        collect_outward_control(statement, scopes, label, controls);
    }
}

fn collect_outward_control<'a>(
    statement: &Stmt<'a>,
    scopes: &mut Vec<ControlScope<'a>>,
    label: Option<&'a str>,
    controls: &mut Vec<OutwardControl>,
) {
    match statement {
        Stmt::Break { label, span } => {
            if !control_break_target(scopes, *label) {
                controls.push(OutwardControl::Break {
                    span: *span,
                    labeled: label.is_some(),
                });
            }
        }
        Stmt::Continue { label, span } => {
            if !control_continue_target(scopes, *label) {
                controls.push(OutwardControl::Continue {
                    span: *span,
                    labeled: label.is_some(),
                });
            }
        }
        Stmt::Yield(span) => controls.push(OutwardControl::Yield(*span)),
        Stmt::If { then, else_ } => {
            collect_outward_control(then, scopes, None, controls);
            if let Some(else_) = else_ {
                collect_outward_control(else_, scopes, None, controls);
            }
        }
        Stmt::Block(body) => collect_outward_controls(body, scopes, None, controls),
        Stmt::Labeled {
            label: nested,
            body,
        } => {
            scopes.push(ControlScope {
                label: Some(nested),
                kind: ScopeKind::Labeled,
            });
            collect_outward_control(body, scopes, Some(nested), controls);
            scopes.pop();
        }
        Stmt::Loop { body, .. } => {
            scopes.push(ControlScope {
                label,
                kind: ScopeKind::Iteration,
            });
            collect_outward_control(body, scopes, None, controls);
            scopes.pop();
        }
        Stmt::Switch(clauses) => {
            scopes.push(ControlScope {
                label,
                kind: ScopeKind::Switch,
            });
            for clause in clauses {
                collect_outward_controls(&clause.stmts, scopes, None, controls);
            }
            scopes.pop();
        }
        Stmt::Try {
            block,
            catch,
            finally,
        } => {
            collect_outward_controls(block, scopes, None, controls);
            if let Some(catch) = catch {
                collect_outward_controls(catch, scopes, None, controls);
            }
            if let Some(finally) = finally {
                collect_outward_controls(finally, scopes, None, controls);
            }
        }
        Stmt::Return | Stmt::Other => {}
    }
}

/// Where `break [label]` lands, or `None` when its target lies outside
/// the analyzed body.
fn break_target(scopes: &[Scope<'_>], label: Option<&str>) -> Option<BlockId> {
    scopes
        .iter()
        .rev()
        .find(|scope| match label {
            Some(name) => scope.label == Some(name),
            None => matches!(scope.kind, ScopeKind::Iteration | ScopeKind::Switch),
        })
        .map(|scope| scope.break_to)
}

/// Where `continue [label]` lands, or `None` when its target lies outside
/// the analyzed body. Only a loop is continuable, labeled or not.
fn continue_target(scopes: &[Scope<'_>], label: Option<&str>) -> Option<BlockId> {
    scopes
        .iter()
        .rev()
        .find(|scope| {
            scope.kind == ScopeKind::Iteration && label.is_none_or(|name| scope.label == Some(name))
        })
        .and_then(|scope| scope.continue_to)
}

struct Builder {
    blocks: Vec<BasicBlock>,
}

impl Builder {
    fn block(&mut self, terminator: Terminator) -> BlockId {
        // Blocks come from statements in one function body; a file with
        // 4 billion of them cannot be read into memory in the first place.
        let id = BlockId(
            u32::try_from(self.blocks.len()).expect("a body has fewer than u32::MAX blocks"),
        );
        self.blocks.push(BasicBlock { terminator });
        id
    }

    /// Fills in a block reserved before its successors existed — a loop's
    /// exit test, which its body's back edge already targets.
    fn set(&mut self, block: BlockId, terminator: Terminator) {
        self.blocks[block.0 as usize].terminator = terminator;
    }

    /// The entry block of `stmts` followed by `follow`. Lowered back to
    /// front, so each statement's successor is already built; scopes nest
    /// lexically, never sequentially, so the order is free.
    fn seq<'a>(
        &mut self,
        stmts: &[Stmt<'a>],
        follow: BlockId,
        scopes: &mut Vec<Scope<'a>>,
    ) -> BlockId {
        let mut next = follow;
        for stmt in stmts.iter().rev() {
            next = self.stmt(stmt, next, scopes);
        }
        next
    }

    fn stmt<'a>(
        &mut self,
        stmt: &Stmt<'a>,
        follow: BlockId,
        scopes: &mut Vec<Scope<'a>>,
    ) -> BlockId {
        self.labeled(stmt, follow, scopes, None)
    }

    /// Lowers `stmt`; `label` is the label written in front of it, which
    /// a loop or `switch` claims so `break`/`continue` can name it.
    fn labeled<'a>(
        &mut self,
        stmt: &Stmt<'a>,
        follow: BlockId,
        scopes: &mut Vec<Scope<'a>>,
        label: Option<&'a str>,
    ) -> BlockId {
        match stmt {
            Stmt::Return => self.block(Terminator::Return),
            Stmt::Break { label: name, .. } => {
                let terminator =
                    break_target(scopes, *name).map_or(Terminator::Jump, Terminator::Goto);
                self.block(terminator)
            }
            Stmt::Continue { label: name, .. } => {
                let terminator =
                    continue_target(scopes, *name).map_or(Terminator::Jump, Terminator::Goto);
                self.block(terminator)
            }
            Stmt::Yield(_) | Stmt::Other => self.block(Terminator::Goto(follow)),
            Stmt::Block(inner) => self.seq(inner, follow, scopes),
            Stmt::If { then, else_ } => {
                let then_bb = self.stmt(then, follow, scopes);
                let else_bb = match else_ {
                    Some(branch) => self.stmt(branch, follow, scopes),
                    None => follow,
                };
                self.block(Terminator::Branch { then_bb, else_bb })
            }
            Stmt::Labeled { label: name, body } => {
                // The label is breakable whatever it names; when it names
                // a loop, that loop pushes the same label again with a
                // `continue` target, and being innermost it wins.
                scopes.push(Scope {
                    label: Some(name),
                    kind: ScopeKind::Labeled,
                    break_to: follow,
                    continue_to: None,
                });
                let entry = self.labeled(body, follow, scopes, Some(name));
                scopes.pop();
                entry
            }
            Stmt::Loop { kind, body } => {
                // The exit test is reserved first: the body's back edge
                // targets it, so it must exist before the body is built.
                let test = self.block(Terminator::End);
                scopes.push(Scope {
                    label,
                    kind: ScopeKind::Iteration,
                    break_to: follow,
                    continue_to: Some(test),
                });
                let body_entry = self.stmt(body, test, scopes);
                scopes.pop();
                self.set(
                    test,
                    if kind.exits {
                        Terminator::Branch {
                            then_bb: body_entry,
                            else_bb: follow,
                        }
                    } else {
                        Terminator::Goto(body_entry)
                    },
                );
                if kind.test_first { test } else { body_entry }
            }
            Stmt::Switch(clauses) => self.switch(clauses, follow, scopes, label),
            Stmt::Try {
                block,
                catch,
                finally,
            } => {
                // Everything that leaves the guarded block or its handler
                // normally runs the `finally` first, so a `finally` that
                // diverges makes the whole statement diverge. An abrupt
                // exit (`return`/`break`/`continue`) from inside is *not*
                // routed through this copy: it already diverges, and a
                // diverging `finally` could only make it more so — the
                // omission can never claim a divergence that is not there.
                let join = match finally {
                    Some(stmts) => self.seq(stmts, follow, scopes),
                    None => follow,
                };
                let try_entry = self.seq(block, join, scopes);
                match catch {
                    // An exception can be raised anywhere in the guarded
                    // block, so the handler is reachable in place of any
                    // prefix of it. The statement then reaches `join`
                    // when either half does — exactly "the statement
                    // diverges when both halves do".
                    Some(stmts) => {
                        let else_bb = self.seq(stmts, join, scopes);
                        self.block(Terminator::Branch {
                            then_bb: try_entry,
                            else_bb,
                        })
                    }
                    // With no handler an exception leaves the function;
                    // normal completion is the only edge out.
                    None => try_entry,
                }
            }
        }
    }

    fn switch<'a>(
        &mut self,
        clauses: &[Clause<'a>],
        follow: BlockId,
        scopes: &mut Vec<Scope<'a>>,
        label: Option<&'a str>,
    ) -> BlockId {
        scopes.push(Scope {
            label,
            kind: ScopeKind::Switch,
            break_to: follow,
            continue_to: None,
        });
        // Clause bodies chain in source order: one that completes normally
        // falls into the next, and the last into the statement's
        // successor. Building back to front gives each its successor.
        let mut entries = vec![follow; clauses.len()];
        let mut next = follow;
        for (index, clause) in clauses.iter().enumerate().rev() {
            next = self.seq(&clause.stmts, next, scopes);
            entries[index] = next;
        }
        scopes.pop();
        // Dispatch: the discriminant is tested against each `case` in
        // source order, and what no test matches goes to `default` — or
        // straight past the statement when there is none.
        let mut dispatch = clauses
            .iter()
            .position(|clause| clause.default)
            .map_or(follow, |index| entries[index]);
        for (index, clause) in clauses.iter().enumerate().rev() {
            if !clause.default {
                dispatch = self.block(Terminator::Branch {
                    then_bb: entries[index],
                    else_bb: dispatch,
                });
            }
        }
        dispatch
    }
}
