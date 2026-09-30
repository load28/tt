//! Structural parsing of tt `if let` statements:
//!
//! ```text
//! if let Tag(bindings...) (| Tag[(bindings...)])* = <expr> { ... }
//! if let Tag(bindings...) = <expr> { ... } else { ... }
//! if let Tag(bindings...) = <expr> { ... } else if let ... { ... }
//! ```
//!
//! Contract safety: in valid TypeScript an undotted `if` is always followed
//! by `(` — never by the reserved word `let` — so an `if let` sequence can
//! only be tt syntax. That also means a candidate that *starts* with
//! `if let` but fails to parse cannot be passed through (the output would
//! be invalid TypeScript with no position); the caller records the offset
//! and the semantic phase reports it, like a stray `|>`.
//!
//! The bound expression runs to the top-level `{` opening the then-block.
//! Brace-carrying constructs inside it (`match ( ... ) { ... }`,
//! `result { ... }`) are skipped whole, and a `{`
//! directly after `=>` (an unparenthesized block-bodied arrow) aborts —
//! parenthesize the arrow. The `else` continuation is a block or another
//! `if let`; a plain `else if (...)` is not part of the statement (v1) and
//! fails the parse.

use super::cursor::{Cursor, dotted_at, find_close_at, skip_braced_construct};
use crate::ast::{
    IfLetElse, IfLetStmt, RecoveryKind, RecoveryNode, Span, StrayIfLet, StrayIfLetKind, TagPattern,
};
use crate::lexer::{Token, TokenKind};

/// The token index where an `if let`'s pattern starts, when the token at
/// `k` is an undotted `if` followed by `let` — a sequence TypeScript never
/// writes.
pub(super) fn if_let_pattern(src: &str, tokens: &[Token], k: usize) -> Option<usize> {
    let word = |at: usize| {
        tokens
            .get(at)
            .filter(|token| matches!(token.kind, TokenKind::Ident))
            .map(|token| &src[token.span.start..token.span.end])
    };
    (word(k)? == "if" && word(k + 1)? == "let" && !dotted_at(tokens, 0, k)).then_some(k + 2)
}

pub(super) fn if_let_end(parser: &super::Parser, tokens: &[Token], k: usize) -> Option<usize> {
    if_let_pattern(parser.src, tokens, k)?;
    let keyword = &tokens[k];
    let range_end = tokens
        .last()
        .map_or(keyword.span.end, |token| token.span.end);
    parse_if_let(Cursor::new(parser, tokens, k + 1, range_end), keyword.span)
        .ok()
        .map(|(cur, _, _)| cur.idx)
}

/// `cur` is positioned just past an undotted `if` keyword (`kw_span`) whose
/// next token is `let` (the caller pre-checked). On success returns the
/// advanced cursor, the byte just past the statement, and the parsed
/// statement.
pub(super) fn parse_if_let<'t>(
    cur: Cursor<'t>,
    kw_span: Span,
) -> Result<(Cursor<'t>, usize, IfLetStmt), StrayIfLet> {
    crate::stack::grow(|| parse_if_let_grown(cur, kw_span))
}

fn parse_if_let_grown<'t>(
    cur: Cursor<'t>,
    kw_span: Span,
) -> Result<(Cursor<'t>, usize, IfLetStmt), StrayIfLet> {
    let head = StrayIfLet {
        span: kw_span,
        kind: StrayIfLetKind::Head,
    };
    let (mut cur, mut byte_end, mut stmt, else_at) = parse_if_let_link(cur, kw_span).ok_or(head)?;
    let Some(else_span) = else_at else {
        return Ok((cur, byte_end, stmt));
    };
    match cur.peek() {
        Some(t) if matches!(t.kind, TokenKind::Punct(b'{')) => {
            let open = cur.idx;
            let close = cur.find_close().ok_or(head)?;
            let range = (
                cur.tokens[open].span.start + 1,
                cur.tokens[close].span.start,
            );
            stmt.else_part = Some(IfLetElse::Block(cur.parser.parse_tokens(
                &cur.tokens[open + 1..close],
                range.0,
                range.1,
            )));
            byte_end = cur.tokens[close].span.end;
            cur.idx = close + 1;
        }
        Some(t)
            if matches!(t.kind, TokenKind::Ident)
                && cur.text(t) == "if"
                && matches!(cur.tokens.get(cur.idx + 1), Some(next)
                    if matches!(next.kind, TokenKind::Ident) && cur.text(next) == "let") =>
        {
            let if_span = t.span;
            cur.bump();
            let (next_cur, end, inner) = parse_if_let(cur, if_span)?;
            cur = next_cur;
            byte_end = end;
            stmt.else_part = Some(IfLetElse::IfLet(Box::new(inner)));
        }
        next => {
            return Err(StrayIfLet {
                span: Span {
                    start: else_span.start,
                    end: next.map_or(else_span.end, |token| token.span.end),
                },
                kind: StrayIfLetKind::ElseContinuation,
            });
        }
    }
    stmt.owner_span.end = byte_end;
    Ok((cur, byte_end, stmt))
}

fn parse_if_let_link<'t>(
    cur: Cursor<'t>,
    kw_span: Span,
) -> Option<(Cursor<'t>, usize, IfLetStmt, Option<Span>)> {
    crate::stack::grow(|| parse_if_let_link_grown(cur, kw_span))
}

fn parse_if_let_link_grown<'t>(
    mut cur: Cursor<'t>,
    kw_span: Span,
) -> Option<(Cursor<'t>, usize, IfLetStmt, Option<Span>)> {
    match cur.peek() {
        Some(t) if matches!(t.kind, TokenKind::Ident) && cur.text(t) == "let" => {
            cur.bump();
        }
        _ => return None,
    }

    // pattern: `Tag(bindings...) (| Tag[(bindings...)])*` — the first
    // alternative's parens are mandatory, nested patterns allowed (sema
    // rejects nested combined with or-alternatives, as in a match).
    let (tag, tag_span) = cur.eat_ident()?;
    if super::is_reserved(tag) {
        return None;
    }
    if !cur.at_punct(b'(') {
        return None;
    }
    let open = cur.idx;
    let close = cur.find_close()?;
    let bindings = super::matches::parse_bindings(
        cur.sub(open + 1, close, cur.tokens[close].span.start),
        true,
    )?;
    cur.idx = close + 1;
    let mut alternatives = vec![TagPattern {
        tag: tag.to_string(),
        tag_off: tag_span.start,
        end: cur.tokens[close].span.end,
        bindings: Some(bindings),
        list: Some(crate::ast::Span {
            start: cur.tokens[open].span.start,
            end: cur.tokens[close].span.end,
        }),
    }];
    while cur.at_punct(b'|') {
        cur.bump();
        alternatives.push(super::matches::parse_alternative(&mut cur, true)?);
    }

    // `=` (but not `==` / `=>`; `=>` lexes as a fused Arrow token)
    let eq = cur.eat_punct(b'=')?;
    if matches!(cur.peek(), Some(t) if t.span.start == eq.end
        && matches!(cur.parser.bytes[t.span.start], b'=' | b'>'))
    {
        return None;
    }

    // `<expr> {` — the expression runs to the top-level then-block brace
    let expr_from = cur.idx;
    let expr_start = cur.stop_byte_at(cur.idx);
    let (expr_end, brace_idx) = expr_until_block(&cur)?;
    if cur.parser.src[expr_start..expr_end].trim().is_empty() {
        return None;
    }
    let expr =
        cur.parser
            .parse_expression_tokens(&cur.tokens[expr_from..brace_idx], expr_start, expr_end);
    cur.idx = brace_idx;

    // then-block
    let body_open = cur.idx;
    let body_close = cur.find_close()?;
    let body_range = (
        cur.tokens[body_open].span.start + 1,
        cur.tokens[body_close].span.start,
    );
    let body = cur.parser.parse_tokens(
        &cur.tokens[body_open + 1..body_close],
        body_range.0,
        body_range.1,
    );
    let byte_end = cur.tokens[body_close].span.end;
    cur.idx = body_close + 1;

    // optional `else` continuation: a block or another `if let`, which the
    // caller parses
    let mut else_at = None;
    if let Some(t) = cur.peek()
        && matches!(t.kind, TokenKind::Ident)
        && cur.text(t) == "else"
    {
        else_at = Some(t.span);
        cur.bump();
    }

    Some((
        cur,
        byte_end,
        IfLetStmt {
            owner_span: Span {
                start: kw_span.start,
                end: byte_end,
            },
            keyword_off: kw_span.start,
            head_span: Span {
                start: kw_span.start,
                end: expr_end,
            },
            alternatives,
            expr,
            body,
            else_part: None,
            // Filled by the caller for the outermost statement (a chained
            // `else if let` is never in expression position, so only the
            // outer one's placement is ever judged).
            in_function: false,
            expression_position: false,
        },
        else_at,
    ))
}

/// Scans the bound expression from `cur.idx` until the top-level `{` that
/// opens the then-block, returning `(expression end byte, brace token
/// index)`. Aborts on anything the expression cannot contain at its top
/// level (a closer, `;`, `,`, `=` — `=>` is fused — a `:` without a
/// pending `?`, an undotted statement-only keyword) and on a `{` directly
/// after `=>` (an unparenthesized block-bodied arrow — parenthesize it).
fn expr_until_block(cur: &Cursor) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut ternaries = 0usize;
    let mut expr_end = cur.stop_byte_at(cur.idx);
    let mut k = cur.idx;
    let mut after_construct = false;
    while k < cur.tokens.len() {
        let t = &cur.tokens[k];
        let operand_ended = std::mem::take(&mut after_construct)
            || (k > cur.idx && cur.tokens[k - 1].facts.ends_expression());
        if let TokenKind::Ident = t.kind {
            if depth == 0 && !dotted_at(cur.tokens, cur.idx, k) {
                let word = cur.text(t);
                if crate::lexer::statement_keyword_at(cur.parser.src, cur.tokens, k) {
                    return None;
                }
                // Skip a whole `match ( ... ) { ... }` or `result { ... }`
                // shape so its braces are not mistaken for the then-block.
                if let Some(past) = skip_braced_construct(cur.tokens, word, k) {
                    expr_end = cur.tokens[past - 1].span.end;
                    k = past;
                    after_construct = true;
                    continue;
                }
            }
            expr_end = t.span.end;
            k += 1;
            continue;
        }
        if depth == 0 {
            match t.kind {
                TokenKind::Punct(b'{')
                    if matches!(
                        k.checked_sub(1).map(|p| &cur.tokens[p].kind),
                        Some(TokenKind::Arrow)
                    ) =>
                {
                    return None;
                }
                TokenKind::Punct(b'{') if operand_ended => {
                    return (ternaries == 0).then_some((expr_end, k));
                }
                TokenKind::Punct(b';' | b'}' | b')' | b']' | b',' | b'=') => return None,
                TokenKind::Punct(b'?') => ternaries += 1,
                TokenKind::Punct(b':') => {
                    if ternaries == 0 {
                        return None;
                    }
                    ternaries -= 1;
                }
                _ => {}
            }
        }
        match t.kind {
            _ if t.opens_bracket() => depth += 1,
            _ if t.closes_bracket() => depth = depth.saturating_sub(1),
            _ => {}
        }
        expr_end = t.span.end;
        k += 1;
    }
    None
}

pub(super) fn stray_if_let_recoveries(
    src: &str,
    tokens: &[Token],
    k: usize,
    range_end: usize,
) -> Vec<RecoveryNode> {
    let extent = if_extent(src, tokens, k, range_end);
    let start = tokens[k].span.start;
    let end = extent.end.max(extent.head_end);
    match extent.operand {
        Some(eq) => {
            let mut nodes = vec![RecoveryNode {
                span: Span {
                    start,
                    end: tokens[eq].span.end,
                },
                kind: RecoveryKind::OperandHead,
            }];
            if let Some(tail) = extent.tail {
                nodes.push(RecoveryNode {
                    span: Span { start: tail, end },
                    kind: RecoveryKind::Statement,
                });
            }
            nodes
        }
        None => vec![RecoveryNode {
            span: Span { start, end },
            kind: RecoveryKind::Statement,
        }],
    }
}

struct IfExtent {
    head_end: usize,
    operand: Option<usize>,
    tail: Option<usize>,
    end: usize,
}

fn if_extent(src: &str, tokens: &[Token], k: usize, range_end: usize) -> IfExtent {
    let token = |at: usize| tokens.get(at).filter(|t| t.span.end <= range_end);
    let word = |at: usize| {
        token(at)
            .filter(|t| matches!(t.kind, TokenKind::Ident) && !dotted_at(tokens, 0, at))
            .map(|t| &src[t.span.start..t.span.end])
    };
    let first = if word(k + 1) == Some("let") {
        k + 2
    } else {
        k + 1
    };
    let mut head_end = token(first - 1).map_or(tokens[k].span.end, |t| t.span.end);
    let mut open: Vec<u8> = Vec::new();
    let mut eq = None;
    let mut operand = false;
    let mut at = first;
    let mut block = None;
    while let Some(t) = token(at) {
        if open.is_empty() {
            if matches!(t.kind, TokenKind::Punct(b'{'))
                && !matches!(
                    at.checked_sub(1).map(|p| &tokens[p].kind),
                    Some(TokenKind::Arrow)
                )
            {
                block = Some(at);
                break;
            }
            if matches!(t.kind, TokenKind::Punct(b';')) || at > first && t.facts.boundary_before() {
                break;
            }
            if let Some(past) = word(at).and_then(|w| skip_braced_construct(tokens, w, at)) {
                operand |= eq.is_some();
                head_end = tokens[past - 1].span.end;
                at = past;
                continue;
            }
            if eq.is_none() && is_binding_eq(src, tokens, at) {
                eq = Some(at);
                head_end = t.span.end;
                at += 1;
                continue;
            }
        }
        if t.closes_bracket() {
            if !matches!((open.last(), &t.kind), (Some(&want), TokenKind::Punct(got)) if want == *got)
            {
                break;
            }
            open.pop();
        } else if matches!(open.last(), None | Some(b')' | b']'))
            && crate::lexer::statement_keyword_at(src, tokens, at)
        {
            break;
        } else if t.opens_bracket() {
            open.push(match t.kind {
                TokenKind::Punct(b'(') => b')',
                TokenKind::Punct(b'[') => b']',
                TokenKind::Punct(b'{') => b'}',
                _ => b'>',
            });
        }
        operand |= eq.is_some();
        head_end = t.span.end;
        at += 1;
    }
    let operand = eq.filter(|_| operand);
    let Some(open_at) = block else {
        return IfExtent {
            head_end,
            operand,
            tail: None,
            end: head_end,
        };
    };
    let tail = Some(tokens[open_at].span.start);
    let Some(close) = find_close_at(tokens, open_at).filter(|&c| token(c).is_some()) else {
        return IfExtent {
            head_end,
            operand,
            tail,
            end: range_end,
        };
    };
    let mut end = tokens[close].span.end;
    if word(close + 1) == Some("else") {
        end = token(close + 1).map_or(end, |t| t.span.end);
        if matches!(
            token(close + 2).map(|t| &t.kind),
            Some(TokenKind::Punct(b'{'))
        ) {
            end = find_close_at(tokens, close + 2)
                .and_then(token)
                .map_or(range_end, |t| t.span.end);
        } else if word(close + 2) == Some("if") {
            let next = if_extent(src, tokens, close + 2, range_end);
            end = next.end.max(next.head_end);
        }
    }
    IfExtent {
        head_end,
        operand,
        tail,
        end,
    }
}

fn is_binding_eq(src: &str, tokens: &[Token], k: usize) -> bool {
    let t = &tokens[k];
    if !matches!(t.kind, TokenKind::Punct(b'=')) {
        return false;
    }
    let bytes = src.as_bytes();
    let joined_after = tokens.get(k + 1).is_some_and(|next| {
        next.span.start == t.span.end && matches!(bytes[next.span.start], b'=' | b'>')
    });
    let joined_before = k.checked_sub(1).map(|p| &tokens[p]).is_some_and(|prev| {
        prev.span.end == t.span.start
            && matches!(
                bytes[prev.span.end - 1],
                b'=' | b'!'
                    | b'<'
                    | b'>'
                    | b'+'
                    | b'-'
                    | b'*'
                    | b'/'
                    | b'%'
                    | b'&'
                    | b'|'
                    | b'^'
                    | b'?'
            )
    });
    !joined_after && !joined_before
}
