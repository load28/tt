//! Recursive-descent token scanning into the flow statement model.

use super::*;

/// Recursive-descent statement scanning over the token stream. Every
/// statement form the graph models is recognized by its own shape; a
/// shape that does not match is [`Stmt::Other`], whose interior is never
/// entered — which is what keeps an unrecognized construct from ever
/// contributing a divergence it does not have.
pub(super) struct Scanner<'a> {
    pub(super) src: &'a str,
    pub(super) tokens: &'a [Token],
    /// The tt parser's answer for the `if let` statements in this region
    /// ([`IfLetHeads`]). Empty when the region has not been parsed, which
    /// leaves every `if let` an opaque [`Stmt::Other`].
    pub(super) if_lets: &'a IfLetHeads,
}

impl<'a> Scanner<'a> {
    /// The statements in `tokens[at..end]`.
    pub(super) fn statements(&self, mut at: usize, end: usize) -> Vec<Stmt<'a>> {
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
                    Stmt::Break {
                        label,
                        span: self.tokens[at].span,
                    }
                } else {
                    Stmt::Continue {
                        label,
                        span: self.tokens[at].span,
                    }
                };
                (stmt, self.statement_end(at, end))
            }
            Some("yield") => (
                Stmt::Yield(self.tokens[at].span),
                self.statement_end(at, end),
            ),
            Some("if") => match self.if_let_head(at) {
                Some(head_end) => self.if_let_statement(at, end, head_end),
                None => self.if_statement(at, end),
            },
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

    /// The end of the `if let` head starting at token `at`, when the tt
    /// parser claimed one there.
    fn if_let_head(&self, at: usize) -> Option<usize> {
        self.if_lets.get(&self.tokens.get(at)?.span.start).copied()
    }

    /// `if let <pattern> = <scrutinee> { … } [else <embedded>]`, whose
    /// head ends at `head_end`. From the block on, the shape *and* the
    /// control flow are an `if`'s: the binding either matches and the
    /// then-block runs, or it does not and the `else` continuation does
    /// (a chained `else if let` among them). Both halves are inline, so
    /// an exit written in either leaves the enclosing function — which is
    /// what lets the statement carry a region's divergence at all.
    fn if_let_statement(&self, at: usize, end: usize, head_end: usize) -> (Stmt<'a>, usize) {
        let block =
            (at..end).find(|&k| self.tokens[k].span.start >= head_end && self.is_punct(k, b'{'));
        let Some((then, after)) = block.and_then(|block| self.braced(block, end)) else {
            return (Stmt::Other, self.statement_end(at, end));
        };
        let (else_, next) = if self.word(after) == Some("else") {
            let (branch, next) = self.statement(after + 1, end);
            (Some(Box::new(branch)), next)
        } else {
            (None, after)
        };
        (
            Stmt::If {
                then: Box::new(Stmt::Block(then)),
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
        asi_boundary_at(self.src, self.tokens, k)
    }

    /// Whether a line terminator sits between token `k` and the one
    /// before it.
    fn line_break_before(&self, k: usize) -> bool {
        line_break_before_tokens(self.src, self.tokens, k)
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
