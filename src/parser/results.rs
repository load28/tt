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
//! ordinary TypeScript.

use super::cursor::Cursor;
use crate::ast::{IfLetElse, Program, ResultBlock, ResultItem, Segment, Span};

/// What a `result` + `{` candidate turned out to be.
pub(super) enum Attempt<'t> {
    /// A parsed block: the advanced cursor, the byte just past the closing
    /// brace, and the block.
    /// Boxed: every other variant is a word, and the block is the rare
    /// case (one allocation per claimed block).
    Claimed(Cursor<'t>, usize, Box<ResultBlock>),
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
    crate::work::tick("result block attempts");
    let open = cur.idx;
    let Some(close) = cur.find_close() else {
        return (Attempt::Pass, Vec::new()); // unbalanced braces — nothing to claim
    };
    let body_span = Span {
        start: cur.tokens[open].span.end,
        end: cur.tokens[close].span.start,
    };
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
                span: Span {
                    start: kw_span.start,
                    end: byte_end,
                },
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
    let mut tt_owned = std::collections::HashSet::new();
    tt_owned_tokens(program, cur, &mut tt_owned);
    let depth = |offset: usize| function_depth_at(cur.tokens, offset, &tt_owned);
    let Some(baseline) = depth(cur.tokens[open].span.start) else {
        return Vec::new();
    };
    fn collect(
        program: &Program,
        cur: &Cursor<'_>,
        baseline: usize,
        depth: &dyn Fn(usize) -> Option<usize>,
        spans: &mut Vec<Span>,
    ) {
        crate::stack::grow(|| collect_grown(program, cur, baseline, depth, spans));
    }

    fn collect_grown(
        program: &Program,
        cur: &Cursor<'_>,
        baseline: usize,
        depth: &dyn Fn(usize) -> Option<usize>,
        spans: &mut Vec<Span>,
    ) {
        for segment in &program.segments {
            match segment {
                Segment::Try(stmt) => {
                    if depth(stmt.span.start) == Some(baseline) {
                        spans.push(stmt.span);
                    }
                }
                Segment::TryExpr(expr) => {
                    if depth(expr.span.start) == Some(baseline) {
                        spans.push(expr.span);
                    }
                }
                Segment::ResultBlock(_) => {}
                Segment::Match(expr) => {
                    collect(&expr.scrutinee, cur, baseline, depth, spans);
                    for arm in &expr.arms {
                        if let Some(guard) = &arm.guard {
                            collect(&guard.expr, cur, baseline, depth, spans);
                        }
                        collect(&arm.body, cur, baseline, depth, spans);
                    }
                }
                Segment::TupleMatch(expr) => {
                    for (_, value) in &expr.scrutinees {
                        collect(value, cur, baseline, depth, spans);
                    }
                    for arm in &expr.arms {
                        if let Some(guard) = &arm.guard {
                            collect(&guard.expr, cur, baseline, depth, spans);
                        }
                        collect(&arm.body, cur, baseline, depth, spans);
                    }
                }
                Segment::LetElse(stmt) => {
                    collect(&stmt.expr, cur, baseline, depth, spans);
                    collect(&stmt.else_body, cur, baseline, depth, spans);
                }
                Segment::IfLet(stmt) => collect_if_let(stmt, cur, baseline, depth, spans),
                Segment::Pipe(pipe) => {
                    if let Some(head) = &pipe.head {
                        collect(head, cur, baseline, depth, spans);
                    }
                    for step in &pipe.steps {
                        collect(&step.body, cur, baseline, depth, spans);
                    }
                }
                Segment::Template(template) => {
                    for chunk in &template.chunks {
                        if let crate::ast::TemplateChunk::Interp(expr) = chunk {
                            collect(expr, cur, baseline, depth, spans);
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
        depth: &dyn Fn(usize) -> Option<usize>,
        spans: &mut Vec<Span>,
    ) {
        crate::stack::grow(|| collect_if_let_grown(stmt, cur, baseline, depth, spans));
    }

    fn collect_if_let_grown(
        stmt: &crate::ast::IfLetStmt,
        cur: &Cursor<'_>,
        baseline: usize,
        depth: &dyn Fn(usize) -> Option<usize>,
        spans: &mut Vec<Span>,
    ) {
        collect(&stmt.expr, cur, baseline, depth, spans);
        collect(&stmt.body, cur, baseline, depth, spans);
        if let Some(else_part) = &stmt.else_part {
            match else_part {
                IfLetElse::Block(body) => collect(body, cur, baseline, depth, spans),
                IfLetElse::IfLet(next) => collect_if_let(next, cur, baseline, depth, spans),
            }
        }
    }
    let mut spans = Vec::new();
    collect(program, cur, baseline, &depth, &mut spans);
    spans
}

fn tt_owned_tokens(
    program: &Program,
    cur: &Cursor<'_>,
    arrows: &mut std::collections::HashSet<usize>,
) {
    crate::stack::grow(|| tt_owned_tokens_grown(program, cur, arrows));
}

fn tt_owned_tokens_grown(
    program: &Program,
    cur: &Cursor<'_>,
    arrows: &mut std::collections::HashSet<usize>,
) {
    for segment in &program.segments {
        match segment {
            Segment::Match(expr) => {
                owned_brace(expr.body_open, arrows);
                tt_owned_tokens(&expr.scrutinee, cur, arrows);
                for entry in &expr.arms {
                    if let Some(guard) = &entry.guard {
                        tt_owned_tokens(&guard.expr, cur, arrows);
                    }
                    arm_arrow(
                        entry.pattern_span.end,
                        entry.body_span.start,
                        &entry.body,
                        cur,
                        arrows,
                    );
                }
            }
            Segment::TupleMatch(expr) => {
                owned_brace(expr.body_open, arrows);
                for (_, value) in &expr.scrutinees {
                    tt_owned_tokens(value, cur, arrows);
                }
                for entry in &expr.arms {
                    if let Some(guard) = &entry.guard {
                        tt_owned_tokens(&guard.expr, cur, arrows);
                    }
                    arm_arrow(
                        entry.pattern_span.end,
                        entry.body_span.start,
                        &entry.body,
                        cur,
                        arrows,
                    );
                }
            }
            Segment::ResultBlock(block) => {
                for item in &block.items {
                    let crate::ast::ResultItem::Stmts(body) = item;
                    tt_owned_tokens(body, cur, arrows);
                }
            }
            Segment::LetElse(stmt) => {
                tt_owned_tokens(&stmt.expr, cur, arrows);
                tt_owned_tokens(&stmt.else_body, cur, arrows);
            }
            Segment::IfLet(stmt) => tt_owned_if_let(stmt, cur, arrows),
            Segment::Pipe(pipe) => {
                if let Some(head) = &pipe.head {
                    tt_owned_tokens(head, cur, arrows);
                }
                for step in &pipe.steps {
                    tt_owned_tokens(&step.body, cur, arrows);
                }
            }
            Segment::Template(template) => {
                for chunk in &template.chunks {
                    if let crate::ast::TemplateChunk::Interp(expr) = chunk {
                        tt_owned_tokens(expr, cur, arrows);
                    }
                }
            }
            _ => {}
        }
    }
}

fn owned_brace(open: usize, owned: &mut std::collections::HashSet<usize>) {
    owned.insert(open);
}

fn function_depth_at(
    tokens: &[crate::lexer::Token],
    offset: usize,
    owned: &std::collections::HashSet<usize>,
) -> Option<usize> {
    let depth = |index: usize| {
        let indices = tokens
            .iter()
            .enumerate()
            .filter(|(_, token)| owned.contains(&token.span.start))
            .map(|(index, _)| index)
            .collect();
        crate::flow::user_function_depth_at(tokens, index, &indices)
    };
    tokens.iter().enumerate().find_map(|(index, token)| {
        if token.span.start == offset {
            return Some(depth(index));
        }
        let crate::lexer::TokenKind::Template(parts) = &token.kind else {
            return None;
        };
        if !(token.span.start < offset && offset < token.span.end) {
            return None;
        }
        parts.iter().find_map(|part| match part {
            crate::lexer::TplPart::Interp { tokens: inner, .. } => {
                function_depth_at(inner, offset, owned).map(|inner| depth(index) + inner)
            }
            crate::lexer::TplPart::Raw(_) => None,
        })
    })
}

fn tokens_within<'t>(tokens: &'t [crate::lexer::Token], out: &mut Vec<&'t crate::lexer::Token>) {
    crate::stack::grow(|| tokens_within_grown(tokens, out));
}

fn tokens_within_grown<'t>(
    tokens: &'t [crate::lexer::Token],
    out: &mut Vec<&'t crate::lexer::Token>,
) {
    for token in tokens {
        out.push(token);
        if let crate::lexer::TokenKind::Template(parts) = &token.kind {
            for part in parts.iter() {
                if let crate::lexer::TplPart::Interp { tokens: inner, .. } = part {
                    tokens_within(inner, out);
                }
            }
        }
    }
}

fn arm_arrow(
    pattern_end: usize,
    body_start: usize,
    body: &Program,
    cur: &Cursor<'_>,
    arrows: &mut std::collections::HashSet<usize>,
) {
    let mut tokens = Vec::new();
    tokens_within(cur.tokens, &mut tokens);
    if let Some(arrow) = tokens.iter().rev().find(|token| {
        matches!(token.kind, crate::lexer::TokenKind::Arrow)
            && pattern_end <= token.span.start
            && token.span.end <= body_start
    }) {
        arrows.insert(arrow.span.start);
    }
    tt_owned_tokens(body, cur, arrows);
}

fn tt_owned_if_let(
    stmt: &crate::ast::IfLetStmt,
    cur: &Cursor<'_>,
    arrows: &mut std::collections::HashSet<usize>,
) {
    crate::stack::grow(|| tt_owned_if_let_grown(stmt, cur, arrows));
}

fn tt_owned_if_let_grown(
    stmt: &crate::ast::IfLetStmt,
    cur: &Cursor<'_>,
    arrows: &mut std::collections::HashSet<usize>,
) {
    tt_owned_tokens(&stmt.expr, cur, arrows);
    tt_owned_tokens(&stmt.body, cur, arrows);
    if let Some(else_part) = &stmt.else_part {
        match else_part {
            IfLetElse::Block(body) => tt_owned_tokens(body, cur, arrows),
            IfLetElse::IfLet(next) => tt_owned_if_let(next, cur, arrows),
        }
    }
}
