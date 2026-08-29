//! Structural parsing of tt `result { ... }` computation blocks.
//!
//! ```text
//! result {
//!   const user = try getUser(id);  // Err completes the block
//!   const name = user.name.trim(); // ordinary TypeScript
//!   return { user, name };          // the block's success value
//! }
//! ```
//!
//! Contract safety: `result` is an ordinary identifier in TypeScript, and
//! an expression statement naming it *can* be followed by a block statement
//! (`result` + newline + `{ ... }`), so the keyword alone cannot decide.
//! A block is claimed only when its speculative body contains a `try` whose
//! nearest lexical Result scope is that block. Otherwise the source stays
//! ordinary TypeScript. The removed `<-` spelling is recognized only to emit
//! its located migration diagnostic; `a < -b` and `let x: Foo<-1>;` remain
//! untouched TypeScript.

use super::cursor::Cursor;
use crate::ast::{IfLetElse, Program, ResultBlock, ResultItem, Segment, Span};
use crate::lexer::TokenKind;

/// What a `result` + `{` candidate turned out to be.
pub(super) enum Attempt<'t> {
    /// A parsed block: the advanced cursor, the byte just past the closing
    /// brace, and the block.
    /// Boxed: every other variant is a word, and the block is the rare
    /// case (one allocation per claimed block).
    Claimed(Cursor<'t>, usize, Box<ResultBlock>),
    /// Removed legacy binding syntax, retained only for its migration
    /// diagnostic and replacement edit.
    LegacyBinding {
        span: Span,
        arrow: Span,
        recovery: Span,
    },
    /// Not a tt construct: an ordinary `result` identifier and a block.
    Pass,
}

/// `cur` is positioned at the `{` token following an undotted `result`
/// identifier (`kw_span`, used only by the caller for error reporting).
/// Besides the attempt, returns an empty compatibility vector retained for
/// the parser caller while the old nested-bind recovery is removed.
pub(super) fn parse_result_block<'t>(
    mut cur: Cursor<'t>,
    kw_span: Span,
) -> (Attempt<'t>, Vec<Span>) {
    let open = cur.idx;
    let Some(close) = cur.find_close() else {
        return (Attempt::Pass, Vec::new()); // unbalanced braces — nothing to claim
    };
    let body_span = Span {
        start: cur.tokens[open].span.end,
        end: cur.tokens[close].span.start,
    };
    let recovery = Span {
        start: kw_span.start,
        end: cur.tokens[close].span.end,
    };

    // Legacy `<-` was the old claim marker. It is no longer syntax, but a
    // declaration followed by it cannot be TypeScript, so diagnose it at
    // the complete binding statement instead of silently passing it on.
    let mut run_start = open + 1;
    let mut depth = 0usize;

    for k in open + 1..close {
        let t = &cur.tokens[k];
        let boundary = match t.kind {
            TokenKind::Punct(b'(' | b'[' | b'{') => {
                depth += 1;
                false
            }
            TokenKind::Punct(b')' | b']') => {
                depth = depth.saturating_sub(1);
                false
            }
            TokenKind::Punct(b'}') => {
                depth = depth.saturating_sub(1);
                // a `}` that ended a block statement, not an object literal
                // or a function-expression body, ends the statement too
                depth == 0 && !cur.parser.brace_ends_expression(cur.tokens, k)
            }
            TokenKind::Punct(b';') => depth == 0,
            _ => false,
        };
        if !boundary {
            continue;
        }
        if let Some(arrow) = scan_bind(&cur, run_start, k) {
            return (
                Attempt::LegacyBinding {
                    span: Span {
                        start: cur.tokens[run_start].span.start,
                        end: t.span.end,
                    },
                    arrow,
                    recovery,
                },
                Vec::new(),
            );
        }
        run_start = k + 1;
    }
    let body =
        cur.parser
            .parse_tokens(&cur.tokens[open + 1..close], body_span.start, body_span.end);
    let direct_try_spans = nearest_result_try_spans(&body, &cur, open);
    if direct_try_spans.is_empty() {
        return (Attempt::Pass, Vec::new());
    }

    let byte_end = cur.tokens[close].span.end;
    cur.idx = close + 1;
    (
        Attempt::Claimed(
            cur,
            byte_end,
            Box::new(ResultBlock {
                keyword_off: kw_span.start,
                body_span,
                direct_try_spans,
                items: vec![ResultItem::Stmts(body)],
                value: None,
            }),
        ),
        Vec::new(),
    )
}

/// Whether a lossless speculative parse found a tt `try` owned by this
/// candidate Result region. Nested Result blocks and nested functions own
/// their own scopes, so neither may claim their inner propagation here.
fn nearest_result_try_spans(program: &Program, cur: &Cursor<'_>, open: usize) -> Vec<Span> {
    let baseline = crate::flow::function_depth_at(cur.parser.src, cur.tokens, open);
    fn collect(program: &Program, cur: &Cursor<'_>, baseline: usize, spans: &mut Vec<Span>) {
        for segment in &program.segments {
            match segment {
                Segment::Try(stmt) => {
                    if token_at(cur, stmt.span.start).is_some_and(|at| {
                        crate::flow::function_depth_at(cur.parser.src, cur.tokens, at) == baseline
                    }) {
                        spans.push(stmt.span);
                    }
                }
                Segment::TryExpr(expr) => {
                    if token_at(cur, expr.span.start).is_some_and(|at| {
                        crate::flow::function_depth_at(cur.parser.src, cur.tokens, at) == baseline
                    }) {
                        spans.push(expr.span);
                    }
                }
                Segment::ResultBlock(_) => {}
                Segment::Match(expr) => {
                    collect(&expr.scrutinee, cur, baseline, spans);
                    for arm in &expr.arms {
                        if let Some(guard) = &arm.guard {
                            collect(&guard.expr, cur, baseline, spans);
                        }
                        collect(&arm.body, cur, baseline, spans);
                    }
                }
                Segment::TupleMatch(expr) => {
                    for (_, value) in &expr.scrutinees {
                        collect(value, cur, baseline, spans);
                    }
                    for arm in &expr.arms {
                        if let Some(guard) = &arm.guard {
                            collect(&guard.expr, cur, baseline, spans);
                        }
                        collect(&arm.body, cur, baseline, spans);
                    }
                }
                Segment::LetElse(stmt) => {
                    collect(&stmt.expr, cur, baseline, spans);
                    collect(&stmt.else_body, cur, baseline, spans);
                }
                Segment::IfLet(stmt) => collect_if_let(stmt, cur, baseline, spans),
                Segment::Pipe(pipe) => {
                    if let Some(head) = &pipe.head {
                        collect(head, cur, baseline, spans);
                    }
                    for step in &pipe.steps {
                        collect(&step.body, cur, baseline, spans);
                    }
                }
                Segment::Template(template) => {
                    for chunk in &template.chunks {
                        if let crate::ast::TemplateChunk::Interp(expr) = chunk {
                            collect(expr, cur, baseline, spans);
                        }
                    }
                }
                Segment::Verbatim(_)
                | Segment::Variant(_)
                | Segment::TtImport(_)
                | Segment::ValModifier(_) => {}
            }
        }
    }
    fn collect_if_let(
        stmt: &crate::ast::IfLetStmt,
        cur: &Cursor<'_>,
        baseline: usize,
        spans: &mut Vec<Span>,
    ) {
        collect(&stmt.expr, cur, baseline, spans);
        collect(&stmt.body, cur, baseline, spans);
        if let Some(else_part) = &stmt.else_part {
            match else_part {
                IfLetElse::Block(body) => collect(body, cur, baseline, spans),
                IfLetElse::IfLet(next) => collect_if_let(next, cur, baseline, spans),
            }
        }
    }
    let mut spans = Vec::new();
    collect(program, cur, baseline, &mut spans);
    spans
}

fn token_at(cur: &Cursor<'_>, start: usize) -> Option<usize> {
    cur.tokens
        .iter()
        .position(|token| token.span.start == start)
}

/// A declaration followed by adjacent `<-` is the removed Result binding
/// spelling. An initializer before the marker keeps ordinary TypeScript
/// comparisons such as `const value = a < -b;` untouched.
fn scan_bind(cur: &Cursor<'_>, from: usize, boundary: usize) -> Option<Span> {
    let keyword = cur.tokens.get(from)?;
    if !matches!(keyword.kind, TokenKind::Ident)
        || !matches!(cur.text(keyword), "const" | "let" | "var")
        || !matches!(
            cur.tokens.get(boundary).map(|token| &token.kind),
            Some(TokenKind::Punct(b';'))
        )
    {
        return None;
    }
    let mut depth = 0usize;
    for index in from + 1..boundary {
        match cur.tokens[index].kind {
            TokenKind::Punct(b'(' | b'[' | b'{') => depth += 1,
            TokenKind::Punct(b')' | b']' | b'}') => depth = depth.saturating_sub(1),
            TokenKind::Punct(b'=') if depth == 0 => return None,
            TokenKind::Punct(b'<') if depth == 0 => {
                let left = cur.tokens[index].span;
                if let Some(next) = cur.tokens.get(index + 1)
                    && matches!(next.kind, TokenKind::Punct(b'-'))
                    && next.span.start == left.end
                {
                    let mut tail_depth = 0usize;
                    let mut opened_generic = false;
                    for tail in index + 2..boundary {
                        match cur.tokens[tail].kind {
                            TokenKind::Punct(b'(' | b'[' | b'{') => tail_depth += 1,
                            TokenKind::Punct(b')' | b']' | b'}') => {
                                tail_depth = tail_depth.saturating_sub(1)
                            }
                            TokenKind::Punct(b'<') if tail_depth == 0 => opened_generic = true,
                            TokenKind::Punct(b'>') if tail_depth == 0 && !opened_generic => {
                                return None;
                            }
                            _ => {}
                        }
                    }
                    return Some(Span {
                        start: left.start,
                        end: next.span.end,
                    });
                }
            }
            _ => {}
        }
    }
    None
}
