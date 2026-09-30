//! Structural parsing of tt let-else statements (Rust-style refutable
//! binding):
//!
//! ```text
//! const|let|var Tag(bindings...) (| Tag[(bindings...)])* = <expr> else { ... };
//! ```
//!
//! Contract safety: in valid TypeScript a `const`/`let`/`var` keyword is
//! always followed by a binding identifier or a destructuring pattern —
//! never by `<ident>(`. The pattern's mandatory parens therefore decide
//! construct-hood immediately after the keyword, before anything else is
//! scanned. Reserved tags reject TypeScript-only forms (`const enum`), and
//! a method *named* `const`/`let`/`var` (`{ const(x) { ... } }`) is
//! followed by `(`, not an identifier, so it never gets here. Anything
//! that deviates passes through verbatim, as always.
//!
//! The "else block must diverge" rule is *computed* by the flow layer
//! ([`crate::flow::program_diverges`] — a real CFG answer over the whole
//! statement grammar, tt's own `if let` included) as a bool on the AST
//! node, and *enforced* by [`crate::sema`] — the parser stays infallible.

use super::cursor::{Cursor, brace_begins_expression, dotted_at, skip_braced_construct};
use crate::ast::{LetElseStmt, Span};
use crate::lexer::{Token, TokenKind};

/// The token index where a let-else's pattern starts, when the token at
/// `k` is an undotted `const`, `let`, or `var` followed by `Tag(`: a
/// declaration keyword is never followed by `<ident>(` in valid
/// TypeScript, and a reserved tag (`const enum`) is TypeScript's own.
pub(super) fn let_else_pattern(src: &str, tokens: &[Token], k: usize) -> Option<usize> {
    let word = |at: usize| {
        tokens
            .get(at)
            .filter(|token| matches!(token.kind, TokenKind::Ident))
            .map(|token| &src[token.span.start..token.span.end])
    };
    let tag = word(k + 1)?;
    (matches!(word(k)?, "const" | "let" | "var")
        && !dotted_at(tokens, 0, k)
        && tag.is_ascii()
        && !super::is_reserved(tag)
        && matches!(tokens.get(k + 2)?.kind, TokenKind::Punct(b'(')))
    .then_some(k + 1)
}

/// `cur` is positioned just past a `const`/`let`/`var` keyword
/// (`kw_span`) whose [`let_else_pattern`] the caller found. Parses
/// `Tag(bindings...) = <expr> else { ... };`; on success returns the
/// advanced cursor, the byte just past the `;`, and the parsed statement.
pub(super) fn parse_let_else<'t>(
    mut cur: Cursor<'t>,
    kw_span: crate::ast::Span,
) -> Option<(Cursor<'t>, usize, LetElseStmt)> {
    let (tag, tag_span) = cur.eat_ident()?;
    let open = cur.idx;
    let close = cur.find_close()?;
    let bindings = super::matches::parse_bindings(
        cur.sub(open + 1, close, cur.tokens[close].span.start),
        false, // let-else bindings stay alias-only (no nested patterns)
    )?;
    cur.idx = close + 1;
    let mut alternatives = vec![crate::ast::TagPattern {
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
        alternatives.push(super::matches::parse_alternative(&mut cur, false)?);
    }

    // `=` (but not `==` / `=>`; `=>` lexes as a fused Arrow token)
    let eq = cur.eat_punct(b'=')?;
    if matches!(cur.peek(), Some(t) if t.span.start == eq.end
        && matches!(cur.parser.bytes[t.span.start], b'=' | b'>'))
    {
        return None;
    }

    // `<expr> else`
    let expr_from = cur.idx;
    let expr_start = cur.stop_byte_at(cur.idx);
    let (expr_end, else_idx) = expr_until_else(&cur)?;
    if cur.parser.src[expr_start..expr_end].trim().is_empty() {
        return None;
    }
    let else_off = cur.tokens[else_idx].span.start;
    cur.idx = else_idx + 1;

    // `{ ... };`
    if !cur.at_punct(b'{') {
        return None;
    }
    let body_open = cur.idx;
    let body_close = cur.find_close()?;
    cur.idx = body_close + 1;
    let semi = cur.eat_punct(b';')?;

    let body_range = (
        cur.tokens[body_open].span.start + 1,
        cur.tokens[body_close].span.start,
    );
    let body_tokens = &cur.tokens[body_open + 1..body_close];
    // The block is parsed first so the flow layer sees its tt constructs:
    // an `if let` written here is inline, so its exits are the block's
    // (`crate::flow::program_diverges`).
    let else_body = cur
        .parser
        .parse_tokens(body_tokens, body_range.0, body_range.1);
    let diverges = cur.parser.body_diverges(
        Span {
            start: body_range.0,
            end: body_range.1,
        },
        body_tokens,
        &else_body,
    );
    Some((
        cur,
        semi.end,
        LetElseStmt {
            owner_span: Span {
                start: kw_span.start,
                end: semi.end,
            },
            keyword_off: kw_span.start,
            head_span: Span {
                start: kw_span.start,
                end: expr_end,
            },
            kw: cur.parser.src[kw_span.start..kw_span.end].to_string(),
            alternatives,
            expr: cur.parser.parse_expression_tokens(
                &cur.tokens[expr_from..else_idx],
                expr_start,
                expr_end,
            ),
            else_body,
            else_off,
            diverges,
            // Filled by the caller, which knows the statement's token
            // index in the parse region.
            in_function: false,
        },
    ))
}

/// Scans the bound expression from `cur.idx` until a top-level undotted
/// `else`, returning `(expression end byte, else token index)`. Anything
/// that cannot appear at the top level of an expression (a `{` after a
/// token that ends an expression, a closer, `,`, `=` except the fused
/// `=>`, `:` without a pending `?`, `;`, an undotted statement-only
/// keyword) fails the parse so the text passes through. A `{` that begins
/// an expression is an object literal and is stepped over as a group.
fn expr_until_else(cur: &Cursor) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut ternaries = 0usize;
    let mut expr_end = cur.stop_byte_at(cur.idx);
    let mut k = cur.idx;
    while k < cur.tokens.len() {
        let t = &cur.tokens[k];
        if let TokenKind::Ident = t.kind {
            if depth == 0 && !dotted_at(cur.tokens, cur.idx, k) {
                let word = cur.text(t);
                if word == "else" {
                    return if ternaries == 0 {
                        Some((expr_end, k))
                    } else {
                        None
                    };
                }
                if crate::lexer::statement_keyword_at(cur.parser.src, cur.tokens, k) {
                    return None;
                }
                // Skip a whole `match ( ... ) { ... }` or `result { ... }`
                // shape so the block-`{` abort below doesn't reject it (the
                // recursive parse decides whether it really is tt syntax).
                if let Some(past) = skip_braced_construct(cur.tokens, word, k) {
                    expr_end = cur.tokens[past - 1].span.end;
                    k = past;
                    continue;
                }
            }
            expr_end = t.span.end;
            k += 1;
            continue;
        }
        if depth == 0 {
            match t.kind {
                TokenKind::Punct(b'{') if !brace_begins_expression(cur.tokens, cur.idx, k) => {
                    return None;
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
