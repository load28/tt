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

use crate::lexer::{Token, TokenKind};

/// One body's control-flow graph.
#[derive(Debug)]
pub struct FlowBody {
    /// The blocks; [`BlockId`] indexes this.
    pub blocks: Vec<BasicBlock>,
    /// Where execution enters.
    pub entry: BlockId,
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
/// transfer to its handler is an edge the [`Stmt::Try`] lowering draws.
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

/// Lowers one statement stream (a let-else `else` block's tokens) and
/// answers whether it diverges.
pub(crate) fn block_diverges(src: &str, tokens: &[Token]) -> bool {
    diverges(&lower_block(src, tokens))
}

/// Builds the CFG of one token stream treated as a statement sequence.
pub(crate) fn lower_block(src: &str, tokens: &[Token]) -> FlowBody {
    let statements = Scanner { src, tokens }.statements(0, tokens.len());
    let mut builder = Builder { blocks: Vec::new() };
    let end = builder.block(Terminator::End);
    let entry = builder.seq(&statements, end, &mut Vec::new());
    FlowBody {
        blocks: builder.blocks,
        entry,
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
    Break(Option<&'a str>),
    /// `continue [label]`.
    Continue(Option<&'a str>),
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
        let id = BlockId(u32::try_from(self.blocks.len()).expect("block count fits u32"));
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
            Stmt::Break(name) => {
                let terminator =
                    break_target(scopes, *name).map_or(Terminator::Jump, Terminator::Goto);
                self.block(terminator)
            }
            Stmt::Continue(name) => {
                let terminator =
                    continue_target(scopes, *name).map_or(Terminator::Jump, Terminator::Goto);
                self.block(terminator)
            }
            Stmt::Other => self.block(Terminator::Goto(follow)),
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

// ---------------------------------------------------------------------
// Scanning the token stream into the statement model
// ---------------------------------------------------------------------

/// Recursive-descent statement scanning over the token stream. Every
/// statement form the graph models is recognized by its own shape; a
/// shape that does not match is [`Stmt::Other`], whose interior is never
/// entered — which is what keeps an unrecognized construct from ever
/// contributing a divergence it does not have.
struct Scanner<'a> {
    src: &'a str,
    tokens: &'a [Token],
}

impl<'a> Scanner<'a> {
    /// The statements in `tokens[at..end]`.
    fn statements(&self, mut at: usize, end: usize) -> Vec<Stmt<'a>> {
        let mut out = Vec::new();
        while at < end {
            let (stmt, next) = self.statement(at, end);
            out.push(stmt);
            // The scanners always advance, but a malformed stream must
            // not be able to spin here.
            at = if next > at { next } else { at + 1 };
        }
        out
    }

    /// The statement starting at `at`, and the index just past it.
    fn statement(&self, at: usize, end: usize) -> (Stmt<'a>, usize) {
        if at >= end {
            return (Stmt::Other, end);
        }
        if self.is_punct(at, b'{') {
            return match self.braced(at, end) {
                Some((stmts, next)) => (Stmt::Block(stmts), next),
                None => (Stmt::Other, end),
            };
        }
        match self.word(at) {
            Some("return" | "throw") => (Stmt::Return, self.statement_end(at, end)),
            Some(keyword @ ("break" | "continue")) => {
                // The label is a restricted production: a line terminator
                // after the keyword ends the statement.
                let label = self.word(at + 1).filter(|name| {
                    !NON_LABEL_WORDS.contains(name) && !self.line_break_before(at + 1)
                });
                let stmt = if keyword == "break" {
                    Stmt::Break(label)
                } else {
                    Stmt::Continue(label)
                };
                (stmt, self.statement_end(at, end))
            }
            Some("if") => self.if_statement(at, end),
            Some("while") => self.while_statement(at, end),
            Some("do") => self.do_statement(at, end),
            Some("for") => self.for_statement(at, end),
            Some("switch") => self.switch_statement(at, end),
            Some("try") => self.try_statement(at, end),
            Some(label) if !NON_LABEL_WORDS.contains(&label) && self.is_punct(at + 1, b':') => {
                let (body, next) = self.statement(at + 2, end);
                (
                    Stmt::Labeled {
                        label,
                        body: Box::new(body),
                    },
                    next,
                )
            }
            _ => (Stmt::Other, self.statement_end(at, end)),
        }
    }

    /// `if (…) <then> [else <else>]`.
    fn if_statement(&self, at: usize, end: usize) -> (Stmt<'a>, usize) {
        let Some(close) = self.paren(at + 1, end) else {
            return (Stmt::Other, self.statement_end(at, end));
        };
        let (then, after) = self.statement(close + 1, end);
        let (else_, next) = if self.word(after) == Some("else") {
            let (branch, next) = self.statement(after + 1, end);
            (Some(Box::new(branch)), next)
        } else {
            (None, after)
        };
        (
            Stmt::If {
                then: Box::new(then),
                else_,
            },
            next,
        )
    }

    /// `while (…) <body>`.
    fn while_statement(&self, at: usize, end: usize) -> (Stmt<'a>, usize) {
        let Some(close) = self.paren(at + 1, end) else {
            return (Stmt::Other, self.statement_end(at, end));
        };
        let (body, next) = self.statement(close + 1, end);
        (
            Stmt::Loop {
                kind: LoopKind {
                    test_first: true,
                    exits: !self.always_true(at + 2, close),
                },
                body: Box::new(body),
            },
            next,
        )
    }

    /// `do <body> while (…) [;]` — the one loop whose body precedes its
    /// test, and whose `continue` therefore lands on a test that has
    /// already run the body once.
    fn do_statement(&self, at: usize, end: usize) -> (Stmt<'a>, usize) {
        let (body, after) = self.statement(at + 1, end);
        if self.word(after) != Some("while") {
            return (Stmt::Other, self.statement_end(at, end));
        }
        let Some(close) = self.paren(after + 1, end) else {
            return (Stmt::Other, self.statement_end(at, end));
        };
        let next = if self.is_punct(close + 1, b';') {
            close + 2
        } else {
            close + 1
        };
        (
            Stmt::Loop {
                kind: LoopKind {
                    test_first: false,
                    exits: !self.always_true(after + 2, close),
                },
                body: Box::new(body),
            },
            next,
        )
    }

    /// `for [await] (…) <body>` — C-style and `for`-`in`/`of` alike. Only
    /// the C-style head carries a condition that can be absent or
    /// constant; an iteration may always end, or never begin.
    fn for_statement(&self, at: usize, end: usize) -> (Stmt<'a>, usize) {
        let head = if self.word(at + 1) == Some("await") {
            at + 2
        } else {
            at + 1
        };
        let Some(close) = self.paren(head, end) else {
            return (Stmt::Other, self.statement_end(at, end));
        };
        let exits = match self.head_separators(head + 1, close) {
            Some((first, second)) => first + 1 < second && !self.always_true(first + 1, second),
            None => true,
        };
        let (body, next) = self.statement(close + 1, end);
        (
            Stmt::Loop {
                kind: LoopKind {
                    test_first: true,
                    exits,
                },
                body: Box::new(body),
            },
            next,
        )
    }

    /// `switch (…) { (case …: | default:) <statements> … }`.
    fn switch_statement(&self, at: usize, end: usize) -> (Stmt<'a>, usize) {
        let Some(paren) = self.paren(at + 1, end) else {
            return (Stmt::Other, self.statement_end(at, end));
        };
        if !self.is_punct(paren + 1, b'{') {
            return (Stmt::Other, self.statement_end(at, end));
        }
        let Some(close) = self.close(paren + 1, end) else {
            return (Stmt::Other, self.statement_end(at, end));
        };
        match self.clauses(paren + 2, close) {
            Some(clauses) => (Stmt::Switch(clauses), close + 1),
            None => (Stmt::Other, close + 1),
        }
    }

    /// `try { … } [catch [(…)] { … }] [finally { … }]`. A `try` without
    /// either tail is not a try *statement* — in tt it is the error
    /// propagation statement, which this layer leaves opaque.
    fn try_statement(&self, at: usize, end: usize) -> (Stmt<'a>, usize) {
        let fallback = || (Stmt::Other, self.statement_end(at, end));
        let Some((block, after_block)) = self.braced(at + 1, end) else {
            return fallback();
        };
        let mut next = after_block;
        let mut catch = None;
        if self.word(next) == Some("catch") {
            let mut body = next + 1;
            if self.is_punct(body, b'(') {
                let Some(close) = self.close(body, end) else {
                    return fallback();
                };
                body = close + 1;
            }
            let Some((stmts, after)) = self.braced(body, end) else {
                return fallback();
            };
            catch = Some(stmts);
            next = after;
        }
        let mut finally = None;
        if self.word(next) == Some("finally") {
            let Some((stmts, after)) = self.braced(next + 1, end) else {
                return fallback();
            };
            finally = Some(stmts);
            next = after;
        }
        if catch.is_none() && finally.is_none() {
            return fallback();
        }
        (
            Stmt::Try {
                block,
                catch,
                finally,
            },
            next,
        )
    }

    /// The `case`/`default` clauses of a switch body. `None` when the body
    /// does not have the clause shape, which makes the statement opaque.
    fn clauses(&self, from: usize, to: usize) -> Option<Vec<Clause<'a>>> {
        // Clause heads sit at the body's top level; a `case` inside a
        // nested block, object literal, or nested `switch` is bracketed
        // away by the depth count.
        let mut heads: Vec<(bool, usize, usize)> = Vec::new();
        let mut depth = 0usize;
        let mut k = from;
        while k < to {
            match self.tokens[k].kind {
                TokenKind::Punct(b'(' | b'[' | b'{') => depth += 1,
                TokenKind::Punct(b')' | b']' | b'}') => depth = depth.saturating_sub(1),
                TokenKind::Ident if depth == 0 => match self.word(k) {
                    Some("default") if self.is_punct(k + 1, b':') => heads.push((true, k, k + 2)),
                    Some("case") => {
                        let body = self.case_colon(k + 1, to)? + 1;
                        heads.push((false, k, body));
                        k = body - 1;
                    }
                    _ => {}
                },
                _ => {}
            }
            k += 1;
        }
        if heads.is_empty() && from < to {
            return None;
        }
        Some(
            heads
                .iter()
                .enumerate()
                .map(|(index, &(default, _, body))| Clause {
                    default,
                    stmts: self
                        .statements(body, heads.get(index + 1).map_or(to, |&(_, head, _)| head)),
                })
                .collect(),
        )
    }

    /// The `:` ending a `case` label — the first at the label's top level
    /// that no `?` is waiting on, so a conditional in the label does not
    /// close it early.
    fn case_colon(&self, from: usize, to: usize) -> Option<usize> {
        let mut depth = 0usize;
        let mut conditionals = 0usize;
        for k in from..to {
            match self.tokens[k].kind {
                TokenKind::Punct(b'(' | b'[' | b'{') => depth += 1,
                TokenKind::Punct(b')' | b']' | b'}') => depth = depth.saturating_sub(1),
                // `?.` and `??` lex as their own tokens, so a bare `?` at
                // the top level is always a conditional.
                TokenKind::Punct(b'?') if depth == 0 => conditionals += 1,
                TokenKind::Punct(b':') if depth == 0 => {
                    if conditionals == 0 {
                        return Some(k);
                    }
                    conditionals -= 1;
                }
                _ => {}
            }
        }
        None
    }

    /// The index just past the statement starting at `at` when its shape
    /// is not modeled: its `;`, the `}` closing a brace that opens a
    /// statement body, or an automatic-semicolon boundary. Bracket depth
    /// is tracked, so nothing inside parentheses, brackets, an object
    /// literal, or a function body ends the statement.
    fn statement_end(&self, at: usize, end: usize) -> usize {
        let mut depth = 0usize;
        let mut k = at;
        while k < end {
            if depth == 0 && k > at && self.asi_boundary(k) {
                return k;
            }
            match self.tokens[k].kind {
                TokenKind::Punct(b'(' | b'[') => depth += 1,
                TokenKind::Punct(b'{') => {
                    if depth == 0 && brace_opens_statement(self.src, self.tokens, at, k) {
                        return self.close(k, end).map_or(end, |close| close + 1);
                    }
                    depth += 1;
                }
                TokenKind::Punct(b')' | b']' | b'}') => depth = depth.saturating_sub(1),
                TokenKind::Punct(b';') if depth == 0 => return k + 1,
                _ => {}
            }
            k += 1;
        }
        end
    }

    /// Whether a statement boundary sits just before token `k` because a
    /// line terminator separates it from a token that can end an
    /// expression, and `k` starts a statement no expression can continue.
    ///
    /// This is the part of automatic semicolon insertion the graph needs:
    /// without it, semicolon-free source runs a whole block together into
    /// one opaque statement and every divergence in it is lost. Only the
    /// statement forms the graph models are split on, so a boundary the
    /// rule misjudges can add an [`Stmt::Other`] break at worst.
    fn asi_boundary(&self, k: usize) -> bool {
        let Some(word) = self.word(k) else {
            return false;
        };
        STATEMENT_START_WORDS.contains(&word)
            && k.checked_sub(1)
                .and_then(|previous| self.tokens.get(previous))
                .is_some_and(|previous| self.ends_expression(previous))
            && self.line_break_before(k)
    }

    /// Whether `token` can be the last token of an expression — the test
    /// automatic semicolon insertion turns on.
    fn ends_expression(&self, token: &Token) -> bool {
        match token.kind {
            TokenKind::Ident => {
                !NON_VALUE_WORDS.contains(&&self.src[token.span.start..token.span.end])
            }
            TokenKind::Str | TokenKind::Template(_) | TokenKind::Regex | TokenKind::JsxRaw => true,
            TokenKind::Punct(b')' | b']' | b'}') => true,
            // Numeric literals lex as a punctuation run starting at their
            // first digit.
            TokenKind::Punct(byte) => byte.is_ascii_digit(),
            TokenKind::Arrow
            | TokenKind::OrOr
            | TokenKind::OptChain
            | TokenKind::Coalesce
            | TokenKind::PipeOp => false,
        }
    }

    /// Whether a line terminator sits between token `k` and the one
    /// before it.
    fn line_break_before(&self, k: usize) -> bool {
        let (Some(previous), Some(token)) = (
            k.checked_sub(1).and_then(|index| self.tokens.get(index)),
            self.tokens.get(k),
        ) else {
            return false;
        };
        self.src[previous.span.end..token.span.start].contains('\n')
    }

    /// Whether the condition in `tokens[from..to]` is the literal `true`
    /// — the only test a loop is known never to fail. Redundant
    /// parentheses are peeled first; anything else counts as failable,
    /// which can only make the answer "does not diverge".
    fn always_true(&self, mut from: usize, mut to: usize) -> bool {
        while from + 1 < to && self.is_punct(from, b'(') && self.close(from, to) == Some(to - 1) {
            from += 1;
            to -= 1;
        }
        to == from + 1 && self.word(from) == Some("true")
    }

    /// The two `;` separating a C-style `for` head's three clauses, or
    /// `None` for a `for`-`in`/`of` head, which has neither.
    fn head_separators(&self, from: usize, to: usize) -> Option<(usize, usize)> {
        let mut depth = 0usize;
        let mut first = None;
        for k in from..to {
            match self.tokens[k].kind {
                TokenKind::Punct(b'(' | b'[' | b'{') => depth += 1,
                TokenKind::Punct(b')' | b']' | b'}') => depth = depth.saturating_sub(1),
                TokenKind::Punct(b';') if depth == 0 => match first {
                    None => first = Some(k),
                    Some(first) => return Some((first, k)),
                },
                _ => {}
            }
        }
        None
    }

    /// The statements of the `{ … }` at `at`, and the index just past it.
    fn braced(&self, at: usize, end: usize) -> Option<(Vec<Stmt<'a>>, usize)> {
        if !self.is_punct(at, b'{') {
            return None;
        }
        let close = self.close(at, end)?;
        Some((self.statements(at + 1, close), close + 1))
    }

    /// The index closing the `(` at `at`, or `None` when `at` is not one.
    fn paren(&self, at: usize, end: usize) -> Option<usize> {
        self.is_punct(at, b'(')
            .then(|| self.close(at, end))
            .flatten()
    }

    /// The index of the token closing the bracket at `open`, searching no
    /// further than `end`.
    fn close(&self, open: usize, end: usize) -> Option<usize> {
        let mut depth = 0usize;
        for k in open..end {
            match self.tokens[k].kind {
                TokenKind::Punct(b'(' | b'[' | b'{') => depth += 1,
                TokenKind::Punct(b')' | b']' | b'}') => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(k);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn word(&self, at: usize) -> Option<&'a str> {
        match self.tokens.get(at) {
            Some(token) if matches!(token.kind, TokenKind::Ident) => {
                Some(&self.src[token.span.start..token.span.end])
            }
            _ => None,
        }
    }

    fn is_punct(&self, at: usize, byte: u8) -> bool {
        matches!(self.tokens.get(at).map(|token| &token.kind), Some(TokenKind::Punct(found)) if *found == byte)
    }
}

/// Words that start a statement and can never continue an expression —
/// the ones an automatic-semicolon boundary is recognized before. Kept to
/// the forms the graph models: splitting anywhere else would only move a
/// fall-through boundary.
const STATEMENT_START_WORDS: &[&str] = &[
    "return", "throw", "break", "continue", "if", "for", "while", "do", "switch", "try",
];

/// Words that can never be the last token of an expression. Value
/// keywords (`this`, `true`, `false`, `null`, `super`) are deliberately
/// absent — they end one.
const NON_VALUE_WORDS: &[&str] = &[
    "async",
    "await",
    "case",
    "catch",
    "class",
    "const",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "finally",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "let",
    "new",
    "of",
    "return",
    "switch",
    "throw",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "yield",
];

/// Words that cannot be a statement label, so `<word> :` is never a
/// labeled statement. The ECMAScript reserved words, plus the TypeScript
/// declaration keywords that can head a statement.
const NON_LABEL_WORDS: &[&str] = &[
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "declare",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "module",
    "namespace",
    "new",
    "null",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
];

/// Whether a `return` emitted at token index `at` of this statement
/// stream would leave a **user-written function inside the stream** — the
/// placement question `try` asks: its lowering emits a `return`, and that
/// `return` must have a function of the user's to exit. At the top level
/// of a module (or of a tt construct's own statement region, which
/// forms an isolated value region) there is none; inside a `function`, a method, or
/// an arrow body written in the region there is.
///
/// The classification is per opening brace, from its immediate left
/// context: `=> {` is an arrow body; `) {` is a function or method body
/// unless the parenthesis belongs to a control head (`if`/`for`/`while`/
/// `switch`/`catch`/`with`, `for await`) or a `class ... extends call()`
/// heritage clause. Everything else — object literals, class and
/// namespace bodies, control-statement bodies, bare blocks — is
/// transparent or irrelevant: it never *provides* a function to return
/// from, and never blocks an outer one from counting.
pub(crate) fn in_function_body(src: &str, tokens: &[Token], at: usize) -> bool {
    let mut stack: Vec<bool> = Vec::new();
    for (k, t) in tokens.iter().enumerate().take(at) {
        match t.kind {
            TokenKind::Punct(b'{') => stack.push(function_body_brace(src, tokens, k)),
            TokenKind::Punct(b'}') => {
                stack.pop();
            }
            _ => {}
        }
    }
    stack.iter().any(|&is_function| is_function)
}

/// Heads whose parenthesized clause is followed by a *control* body, not a
/// function body.
const CONTROL_PAREN_WORDS: &[&str] = &["if", "for", "while", "switch", "catch", "with"];

/// Statement-position words a return-type walk aborts on: meeting one at
/// the top level means the walk left the annotation and entered the
/// preceding statement (or the brace never had an annotation at all).
const NON_TYPE_WORDS: &[&str] = &[
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "declare",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "finally",
    "for",
    "function",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "module",
    "namespace",
    "new",
    "of",
    "return",
    "switch",
    "throw",
    "try",
    "var",
    "while",
    "with",
    "yield",
];

/// Whether the `{` at token index `k` opens a function body (see
/// [`in_function_body`]).
fn function_body_brace(src: &str, tokens: &[Token], k: usize) -> bool {
    let Some(prev) = k.checked_sub(1) else {
        return false;
    };
    match tokens[prev].kind {
        // `=> {` — an arrow body.
        TokenKind::Arrow => true,
        // `) {` — a parameter list's body, unless the parenthesis is a
        // control head or a heritage-clause call.
        TokenKind::Punct(b')') => paren_heads_function(src, tokens, prev),
        // Anything else may still be `) : <type> {` — a return-type
        // annotation between the parameter list and the body. Walk back
        // over the type (balanced brackets; names, operators and function
        // arrows at its top level); the `:` straight after a `)` is the
        // annotation's colon and the parenthesis decides as above. A token
        // no type contains at its top level ends the walk: the brace is
        // not a body.
        _ => {
            let mut depth = 0usize;
            let mut j = k;
            while j > 0 {
                j -= 1;
                let t = &tokens[j];
                match t.kind {
                    TokenKind::Punct(b'>' | b')' | b']' | b'}') => depth += 1,
                    TokenKind::Punct(b'<' | b'(' | b'[' | b'{') => {
                        if depth == 0 {
                            return false;
                        }
                        depth -= 1;
                    }
                    TokenKind::Punct(b':') if depth == 0 => {
                        return j >= 1
                            && matches!(tokens[j - 1].kind, TokenKind::Punct(b')'))
                            && paren_heads_function(src, tokens, j - 1);
                    }
                    TokenKind::Ident if depth == 0 => {
                        if NON_TYPE_WORDS.contains(&&src[t.span.start..t.span.end]) {
                            return false;
                        }
                    }
                    TokenKind::Str | TokenKind::Arrow => {}
                    TokenKind::Punct(b'.' | b'|' | b'&') => {}
                    TokenKind::Punct(c) if c.is_ascii_digit() => {}
                    _ => {
                        if depth == 0 {
                            return false;
                        }
                    }
                }
            }
            false
        }
    }
}

/// Whether the parameter list closed at token index `close` heads a
/// function body — i.e. it is not a control head (`if (…)`, `for (…)`,
/// `for await (…)`, …) and not a `class … extends call(…)` heritage
/// clause.
fn paren_heads_function(src: &str, tokens: &[Token], close: usize) -> bool {
    let word = |i: usize| match tokens.get(i) {
        Some(t) if matches!(t.kind, TokenKind::Ident) => Some(&src[t.span.start..t.span.end]),
        _ => None,
    };
    let Some(open) = find_open(tokens, close) else {
        return false;
    };
    let Some(before) = open.checked_sub(1) else {
        return false;
    };
    match tokens[before].kind {
        TokenKind::Ident => {
            let name = word(before).unwrap_or_default();
            if CONTROL_PAREN_WORDS.contains(&name) {
                return false;
            }
            // `for await (…) {` — still a control body.
            if name == "await" && word(before.wrapping_sub(1)) == Some("for") {
                return false;
            }
            // `class A extends mixin(B) {` — a class body: walk back over
            // the (possibly dotted) callee to the word before it.
            let mut j = before;
            while j >= 2
                && matches!(tokens[j - 1].kind, TokenKind::Punct(b'.'))
                && matches!(tokens[j - 2].kind, TokenKind::Ident)
            {
                j -= 2;
            }
            !(j >= 1 && word(j - 1) == Some("extends"))
        }
        // `function* (…) {` — an anonymous generator.
        TokenKind::Punct(b'*') => word(before.wrapping_sub(1)) == Some("function"),
        // `f<T>(…) {` — a generic parameter list's body.
        TokenKind::Punct(b'>') => true,
        _ => false,
    }
}

/// The index of the token opening the bracket closed at `close`.
fn find_open(tokens: &[Token], close: usize) -> Option<usize> {
    let mut depth = 0usize;
    for k in (0..=close).rev() {
        match tokens[k].kind {
            TokenKind::Punct(b')' | b']' | b'}') => depth += 1,
            TokenKind::Punct(b'(' | b'[' | b'{') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
    }
    None
}

/// Words that head a statement whose braces are a *body* (their `}` ends
/// the statement).
const BLOCK_STMT_WORDS: &[&str] = &[
    "if",
    "else",
    "for",
    "while",
    "do",
    "try",
    "catch",
    "finally",
    "switch",
    "function",
    "class",
    "async",
    "declare",
    "namespace",
    "module",
    "interface",
    "enum",
    "with",
];

/// Words a `{` may directly follow while still being an *expression*:
/// `return { … }`, `case { … }`, `await { … }`. Without this,
/// `if (c) return { k: 1 };` would read its object literal as the `if`'s
/// block.
const EXPR_BRACE_WORDS: &[&str] = &[
    "return",
    "throw",
    "case",
    "typeof",
    "instanceof",
    "in",
    "of",
    "new",
    "delete",
    "void",
    "await",
    "yield",
];

/// True when the top-level `{` at `k` opens a statement — a bare block or
/// the body of the statement starting at `last` — rather than an
/// expression's braces. Only the first kind ends a statement when it
/// closes: an object literal or an arrow body leaves its statement running
/// until the `;`.
pub(crate) fn brace_opens_statement(src: &str, tokens: &[Token], last: usize, k: usize) -> bool {
    if k == last {
        return true; // the statement *is* a block
    }
    let word = |i: usize| &src[tokens[i].span.start..tokens[i].span.end];
    if !matches!(tokens[last].kind, TokenKind::Ident) || !BLOCK_STMT_WORDS.contains(&word(last)) {
        return false;
    }
    // Inside such a statement the body brace follows its head: `) {` for
    // the parenthesized ones, a name or the keyword itself for the rest.
    match tokens[k - 1].kind {
        TokenKind::Punct(b')') => true,
        TokenKind::Ident => !EXPR_BRACE_WORDS.contains(&word(k - 1)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(body: &str) -> bool {
        let tokens = crate::lexer::lex(body, 0, body.len());
        block_diverges(body, &tokens)
    }

    #[test]
    fn the_four_keywords_diverge_as_before() {
        assert!(check("return 0;"));
        assert!(check("throw new Error(\"x\");"));
        assert!(check("break;"));
        assert!(check("continue;"));
        assert!(check("log(\"x\"); return 0;"));
        assert!(check("return { k: 1 };"));
    }

    #[test]
    fn plain_statements_do_not() {
        assert!(!check("log(\"x\");"));
        assert!(!check(""));
        assert!(!check("const o = { n: 1 };"));
    }

    #[test]
    fn an_if_needs_both_branches_to_diverge() {
        assert!(!check("if (c) { return 1; }"));
        assert!(check("if (c) { return 1; } else { return 2; }"));
        assert!(check("if (c) return 1; else return 2;"));
        assert!(!check("if (c) { return 1; } else { log(\"x\"); }"));
    }

    #[test]
    fn else_if_chains_are_walked() {
        assert!(check(
            "if (a) { return 1; } else if (b) { throw e; } else { return 2; }"
        ));
        assert!(!check("if (a) { return 1; } else if (b) { throw e; }"));
    }

    #[test]
    fn a_bare_block_diverges_when_its_body_does() {
        assert!(check("{ return 1; }"));
        assert!(!check("{ log(\"x\"); }"));
    }

    #[test]
    fn code_after_a_diverging_statement_is_unreachable_not_a_hole() {
        assert!(check("return 0; log(\"never\");"));
    }

    #[test]
    fn a_functions_return_does_not_leave_the_enclosing_block() {
        assert!(!check("function g() { return 1; }"));
        assert!(!check("const g = () => { return 1; };"));
        // …but a diverging statement after one still counts.
        assert!(check("function g() { return 1; } return g();"));
    }

    #[test]
    fn a_loop_diverges_only_when_it_has_no_normal_exit() {
        // An omitted or literal-`true` test is the whole rule: such a loop
        // is left only by `break`, `return`, or `throw`.
        assert!(check("for (;;) { log(\"x\"); }"));
        assert!(check("while (true) { log(\"x\"); }"));
        assert!(check("while ((true)) { log(\"x\"); }"));
        assert!(check("do { log(\"x\"); } while (true);"));
        assert!(check("for (let i = 0;; i += 1) { log(\"x\"); }"));
        // A test that can fail is a normal exit, however the body ends.
        assert!(!check("while (c) { return 1; }"));
        assert!(!check("for (let i = 0; i < n; i += 1) { return 1; }"));
        // An iteration may end, or never begin.
        assert!(!check("for (const x of xs) { return 1; }"));
        assert!(!check("for (const k in o) { return 1; }"));
        assert!(!check("for await (const x of xs) { return 1; }"));
        // `do … while` runs its body before the test, so a body that
        // diverges takes the statement with it.
        assert!(check("do { return 1; } while (c);"));
        assert!(check("do { throw e; } while (c)"));
    }

    #[test]
    fn a_break_leaves_the_loop_it_names_not_the_block() {
        assert!(!check("while (true) { break; }"));
        assert!(!check("for (;;) { if (c) { break; } }"));
        assert!(!check("outer: while (true) { break outer; }"));
        // …but a `break` whose target is outside the analyzed body leaves
        // the body, which is what the four-keyword rule always meant.
        assert!(check("break;"));
        assert!(check("continue;"));
        assert!(check("break outer;"));
        // A `continue` cannot leave the loop it names.
        assert!(check("while (true) { continue; }"));
        assert!(check(
            "outer: while (true) { while (c) { continue outer; } }"
        ));
        // A `break` naming an outer loop leaves the inner one only.
        assert!(!check(
            "outer: while (true) { while (true) { break outer; } }"
        ));
    }

    #[test]
    fn a_labeled_block_is_breakable_but_not_continuable() {
        assert!(!check("outer: { break outer; }"));
        assert!(check("outer: { break outer; } return 0;"));
        assert!(check("outer: { return 1; }"));
        // An unlabeled `break` is not captured by a labeled block, so it
        // still leaves the analyzed body.
        assert!(check("outer: { break; }"));
    }

    #[test]
    fn a_switch_diverges_when_it_has_a_default_and_no_clause_falls_out() {
        assert!(check(
            "switch (k) { case \"a\": return 1; default: throw e; }"
        ));
        // No `default` — an unmatched discriminant walks past the whole
        // statement.
        assert!(!check("switch (k) { case \"a\": return 1; }"));
        // A `break` targets the switch, so the clause reaches the
        // statement's successor.
        assert!(!check(
            "switch (k) { case \"a\": break; default: return 1; }"
        ));
        assert!(!check(
            "switch (k) { case \"a\": return 1; default: break; }"
        ));
        // Clauses fall through to the next one.
        assert!(check(
            "switch (k) { case \"a\": case \"b\": return 1; default: return 2; }"
        ));
        // The last clause falls out of the statement when it completes.
        assert!(!check("switch (k) { default: log(\"x\"); }"));
        // A `continue` is not the switch's to capture — it belongs to the
        // enclosing loop, which is outside the analyzed body here.
        assert!(check("switch (k) { default: continue; }"));
        // A conditional in a `case` label does not close it early.
        assert!(check(
            "switch (k) { case c ? 1 : 2: return 1; default: return 2; }"
        ));
        // A nested `switch`'s clauses belong to it, not to the outer one.
        assert!(check(
            "switch (k) { default: switch (j) { case 1: return 1; default: return 2; } }"
        ));
    }

    #[test]
    fn a_try_diverges_when_every_half_that_can_complete_does_not() {
        assert!(check("try { return 1; } catch (e) { throw e; }"));
        assert!(check("try { return 1; } catch { return 2; }"));
        // The handler can run in place of the guarded block, so one half
        // completing normally is enough to reach the successor.
        assert!(!check("try { return 1; } catch (e) { log(e); }"));
        assert!(!check("try { log(\"x\"); } catch (e) { return 1; }"));
        // Without a handler an exception leaves the function; normal
        // completion is the only edge out.
        assert!(check("try { return 1; } finally { log(\"x\"); }"));
        assert!(!check("try { log(\"x\"); } finally { log(\"x\"); }"));
        // Everything leaving normally runs the `finally` first.
        assert!(check("try { log(\"x\"); } finally { return 1; }"));
        assert!(check(
            "try { return 1; } catch (e) { log(e); } finally { throw e; }"
        ));
        // tt's `try` statement has neither tail, so it stays opaque.
        assert!(!check("try load();"));
    }

    #[test]
    fn statements_read_the_same_without_semicolons() {
        assert!(check("log(\"x\")\nreturn 0"));
        assert!(check(
            "const o = { a: 1 }\nif (o.a) { return 1 } else { return 2 }"
        ));
        assert!(check("const n = 1\nwhile (true) { log(n) }"));
        // A nested function body is still opaque across a line break.
        assert!(!check("const g = () => { return 1 }\nlog(g())"));
        // A line terminator after `break` ends the statement, so the next
        // line is not its label.
        assert!(check("break\nouter;"));
    }

    /// Whether the byte at `needle`'s position sits inside a function body.
    fn inside(src: &str, needle: &str) -> bool {
        let offset = src.find(needle).expect("needle");
        let tokens = crate::lexer::lex(src, 0, src.len());
        let at = tokens
            .iter()
            .position(|t| t.span.start >= offset)
            .unwrap_or(tokens.len());
        in_function_body(src, &tokens, at)
    }

    #[test]
    fn function_method_and_arrow_bodies_are_returnable() {
        assert!(inside("function f() { HERE; }", "HERE"));
        assert!(inside("const f = function () { HERE; };", "HERE"));
        assert!(inside("const f = function* () { HERE; };", "HERE"));
        assert!(inside("const f = () => { HERE; };", "HERE"));
        assert!(inside("const o = { m() { HERE; } };", "HERE"));
        assert!(inside("class A { m() { HERE; } }", "HERE"));
        assert!(inside("class A { constructor() { HERE; } }", "HERE"));
        assert!(inside("class A { get x() { HERE; } }", "HERE"));
        assert!(inside("function f<T>(x: T) { HERE; }", "HERE"));
        // Return-type annotations sit between the parameter list and the
        // body; the walk crosses them.
        assert!(inside("function f(): void { HERE; }", "HERE"));
        assert!(inside("function f(): Promise<void> { HERE; }", "HERE"));
        assert!(inside("function f(): { a: number } { HERE; }", "HERE"));
        assert!(inside("function f(): X[] | null { HERE; }", "HERE"));
        assert!(inside("const o = { m(): number { HERE; } };", "HERE"));
        // Nested in control flow inside the function — the `return` still
        // exits the function.
        assert!(inside(
            "function f() { if (c) { for (;;) { HERE; } } }",
            "HERE"
        ));
    }

    #[test]
    fn module_and_non_function_braces_are_not() {
        assert!(!inside("HERE;", "HERE"));
        assert!(!inside("{ HERE; }", "HERE"));
        assert!(!inside("if (c) { HERE; }", "HERE"));
        assert!(!inside("for (;;) { HERE; }", "HERE"));
        assert!(!inside("for await (const x of y) { HERE; }", "HERE"));
        assert!(!inside("while (c) { HERE; }", "HERE"));
        assert!(!inside("switch (x) { default: HERE; }", "HERE"));
        assert!(!inside("try { HERE; } catch (e) {}", "HERE"));
        assert!(!inside("do { HERE; } while (c);", "HERE"));
        assert!(!inside("namespace N { HERE; }", "HERE"));
        assert!(!inside("class A extends mixin(B) { HERE; }", "HERE"));
        assert!(!inside("class A { static { HERE; } }", "HERE"));
        assert!(!inside("class A<T> { x = HERE; }", "HERE"));
        // A function body *closed before* the position provides nothing.
        assert!(!inside("function f() {} HERE;", "HERE"));
    }
}
