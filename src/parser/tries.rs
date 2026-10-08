//! Structural parsing of tt `try` statements (Rust-style error propagation).
//!
//! Two forms, both terminated by a top-level `;`:
//!
//! - `try <expr>;` — propagate, discarding the `Ok` value
//! - `const|let|var <binding> = try <expr>;` — propagate or bind the value
//!
//! Contract safety: `try` is a reserved word, so in valid TypeScript it can
//! only start a `try { ... } catch` statement or appear as a member name
//! (`obj.try(...)`, `{ try: 1 }`, `class A { try = 5 }`, interface method
//! signatures). An expression-position `try` is never valid TypeScript, so
//! lifting `= try <expr>;` can never mis-transform a valid file, and the bare
//! form structurally excludes every valid member shape: dotted `try` is
//! rejected by the caller, `try {` is rejected here, and the expression
//! scanner aborts on anything an expression cannot start with or contain at
//! top level (`:` without a ternary `?`, `=`, a bare `{`, closers, `,`).
//! Anything rejected passes through verbatim, as always.

use super::Claim;
use super::cursor::{Cursor, dotted_at, skip_braced_construct};
use crate::ast::{Span, TryExpr, TryStmt, UnclaimedTtCandidate, UnclaimedTtKind};
use crate::lexer::{Token, TokenKind};

/// `cur` is positioned just past the `try` keyword (`kw_span`) of a bare
/// `try <expr>;` statement. On success returns the advanced cursor, the
/// byte just past the `;`, and the parsed statement.
pub(super) fn parse_try_stmt<'t>(
    cur: Cursor<'t>,
    kw_span: Span,
) -> Claim<(Cursor<'t>, usize, TryStmt)> {
    let Some(first) = cur.peek() else {
        return Claim::NotTt;
    };
    let parenthesized = matches!(first.kind, TokenKind::Punct(b'('));
    if !is_expr_start(first) && !parenthesized {
        return Claim::NotTt;
    }
    let Some((cur, byte_end, expr_span, expr_tokens)) = parse_try_tail(cur, parenthesized) else {
        return Claim::Unclaimed(UnclaimedTtCandidate {
            kind: UnclaimedTtKind::Try,
            keyword: kw_span,
            extent: unclaimed_try_extent(&cur, kw_span),
        });
    };
    let expr = cur
        .parser
        .parse_expression_tokens(expr_tokens, expr_span.start, expr_span.end);
    // `try(x);` is a valid member signature in classes and interfaces, so
    // a merely parenthesized tail cannot establish tt ownership. A fully
    // recognized tt construct inside the parentheses does: valid
    // TypeScript cannot contain that construct, and the outer propagation
    // can therefore own the complete operand structurally.
    if parenthesized
        && expr
            .segments
            .iter()
            .all(|segment| matches!(segment, crate::ast::Segment::Verbatim(_)))
    {
        return Claim::NotTt;
    }
    Claim::Parsed((
        cur,
        byte_end,
        TryStmt {
            keyword_off: kw_span.start,
            owner_span: Span {
                start: kw_span.start,
                end: byte_end,
            },
            span: Span {
                start: kw_span.start,
                end: expr_span.end,
            },
            expr_span,
            decl: None,
            expr,
            // Filled by the caller, which knows the statement's token
            // index in the parse region.
            in_function: false,
        },
    ))
}

/// Parses a value-producing `try <primary>` inside another expression.
/// `try` binds like a prefix operator to the following primary expression,
/// including its calls and member/index postfixes. Parentheses deliberately
/// widen the operand to an arbitrary expression.
pub(super) fn parse_try_expr(cur: Cursor<'_>, kw_span: Span) -> Option<(usize, TryExpr)> {
    let first = cur.peek()?;
    if !is_expr_start(first) && !matches!(first.kind, TokenKind::Punct(b'(')) {
        return None;
    }

    let (k, end) = scan_primary_operand(&cur)?;
    let expr_span = Span {
        start: first.span.start,
        end,
    };
    let expr =
        cur.parser
            .parse_expression_tokens(&cur.tokens[cur.idx..k], expr_span.start, expr_span.end);
    Some((
        k,
        TryExpr {
            span: Span {
                start: kw_span.start,
                end,
            },
            expr_span,
            expr,
        },
    ))
}

fn scan_primary_operand(cur: &Cursor) -> Option<(usize, usize)> {
    let first = cur.peek()?;
    let mut k = cur.idx;
    if matches!(first.kind, TokenKind::Ident) && cur.text(first) == "await" {
        k += 1;
    }
    let head = cur.tokens.get(k)?;
    if matches!(head.kind, TokenKind::Ident)
        && !dotted_at(cur.tokens, cur.idx, k)
        && crate::lexer::statement_only_keyword(cur.text(head))
    {
        return None;
    }
    let operand_start = head.span.start;
    let mut operand_end = None;
    let mut operand_token_end = cur.idx;
    if matches!(head.kind, TokenKind::Ident)
        && let Some(past) = skip_braced_construct(cur.tokens, cur.text(head), k)
    {
        operand_end = Some(cur.tokens[past - 1].span.end);
        operand_token_end = past;
        k = past;
    }
    // The closer each open bracket waits for, innermost last.
    let mut open: Vec<u8> = Vec::new();
    let mut piped = false;
    while let Some(token) = cur.tokens.get(k) {
        if open.is_empty()
            && matches!(
                token.kind,
                TokenKind::Punct(b')' | b']' | b'}' | b',' | b';')
            )
        {
            break;
        }
        // A closer of another bracket, or a statement keyword directly in
        // a parenthesis or an index, belongs to the enclosing syntax: it is
        // where TypeScript ends a list the operand left open. `try` is the
        // one such keyword tt reads as a value there.
        if token.closes_bracket()
            && !matches!((open.last(), &token.kind), (Some(&want), TokenKind::Punct(got)) if want == *got)
        {
            break;
        }
        if matches!(open.last(), None | Some(b')' | b']'))
            && !open.contains(&b'}')
            && k > cur.idx
            && !dotted_at(cur.tokens, cur.idx, k)
            && crate::lexer::statement_keyword_at(cur.parser.src, cur.tokens, k)
            && (open.is_empty() || cur.text(token) != "try")
        {
            break;
        }
        // TypeScript ends an argument list at `;` (`isListTerminator`), and
        // an array list returns `;` to the enclosing statement list. Inside a
        // block (`() => { for (;;) {} }`) `;` belongs to that block's statements.
        if matches!(open.last(), Some(b')' | b']'))
            && !open.contains(&b'}')
            && matches!(token.kind, TokenKind::Punct(b';'))
        {
            break;
        }
        if open.is_empty() && matches!(token.kind, TokenKind::PipeOp) {
            if operand_token_end != k {
                break;
            }
            piped = true;
        }

        match token.kind {
            TokenKind::Punct(byte) if token.opens_bracket() => open.push(match byte {
                b'(' => b')',
                b'[' => b']',
                b'{' => b'}',
                _ => b'>',
            }),
            _ if token.closes_bracket() => {
                open.pop();
            }
            _ => {}
        }
        k += 1;
        if piped {
            if open.is_empty() && !matches!(token.kind, TokenKind::PipeOp) {
                operand_end = Some(token.span.end);
                operand_token_end = k;
            }
        } else if open.is_empty()
            && crate::lexer::is_primary_expression(
                cur.parser.src,
                operand_start,
                token.span.end,
                cur.parser.source_kind,
            )
        {
            operand_end = Some(token.span.end);
            operand_token_end = k;
        }
    }
    // A bracket still open is a list being written: TypeScript reads it as
    // the operand's, with the rest of its last line, where the next argument
    // is typed. A line break separates the enclosing syntax that resumes
    // after it and stays that syntax's.
    if !open.is_empty() {
        operand_end = Some(
            match k.checked_sub(1).and_then(|last| cur.tokens.get(last)) {
                Some(last) if k > cur.idx => crate::lexer::line_trivia_end(
                    cur.parser.src,
                    last.span.end,
                    cur.stop_byte_at(k),
                ),
                _ => cur.stop_byte_at(k),
            },
        );
        operand_token_end = k;
    }
    if operand_end.is_some()
        && let Some(dot) = cur.tokens.get(operand_token_end)
        && matches!(dot.kind, TokenKind::Punct(b'.') | TokenKind::OptChain)
        && !cur
            .tokens
            .get(operand_token_end + 1)
            .is_some_and(|name| match name.kind {
                TokenKind::Ident | TokenKind::Punct(b'#') => true,
                TokenKind::Punct(b'(' | b'[') => matches!(dot.kind, TokenKind::OptChain),
                _ => false,
            })
    {
        operand_end = Some(dot.span.end);
        operand_token_end += 1;
    }
    operand_end.map(|end| (operand_token_end, end))
}

/// Bounds a `try <expr>` candidate that rolled back before its required
/// semicolon. This uses the same token/depth rules as the claim itself and
/// synchronizes before the next statement; no later phase has to infer the
/// region from source text.
fn unclaimed_try_extent(cur: &Cursor, kw_span: Span) -> Span {
    let mut end = kw_span.end;
    let mut depth = 0usize;
    let mut k = cur.idx;
    while let Some(token) = cur.tokens.get(k) {
        if depth == 0 {
            if matches!(token.kind, TokenKind::Punct(b';')) {
                end = token.span.end;
                break;
            }
            if matches!(
                token.kind,
                TokenKind::Punct(b'{' | b'}' | b')' | b']' | b',' | b'=')
            ) {
                break;
            }
            if k > cur.idx
                && !dotted_at(cur.tokens, cur.idx, k)
                && crate::lexer::statement_keyword_at(cur.parser.src, cur.tokens, k)
            {
                break;
            }
        }
        end = token.span.end;
        match token.kind {
            TokenKind::Punct(b'(' | b'[') => depth += 1,
            TokenKind::Punct(b')' | b']') => depth = depth.saturating_sub(1),
            _ => {}
        }
        k += 1;
    }
    Span {
        start: kw_span.start,
        end,
    }
}

/// `cur` is positioned just past a `const`/`let`/`var` keyword
/// (`kw_span`). Parses `<binding> = try <expr>;`; on success returns the
/// advanced cursor, the byte just past the `;`, and the parsed statement.
pub(super) fn parse_try_decl<'t>(
    mut cur: Cursor<'t>,
    kw_span: Span,
) -> Option<(Cursor<'t>, usize, TryStmt)> {
    let scan_start = cur.stop_byte_at(cur.idx);
    let (eq_idx, eq_byte) = binding_end(&cur)?;
    // The span of the binding itself, trivia on either side dropped:
    // codegen copies these bytes so the emitted declaration maps back to
    // the name the user wrote, and writes the trivia before `=` itself.
    let binding_start = scan_start;
    let binding_end = cur.tokens[..eq_idx]
        .last()
        .map_or(binding_start, |token| token.span.end)
        .max(binding_start);
    if binding_end <= binding_start || binding_end > eq_byte {
        return None;
    }
    let binding_span = Span {
        start: binding_start,
        end: binding_end,
    };
    // A real binding starts with the variable name or a destructuring
    // pattern. A leading reserved word means the scan ran across some other
    // construct (e.g. `const enum E { ... }` with a `= try` further down).
    match cur.tokens[cur.idx].kind {
        TokenKind::Ident => {
            if super::is_reserved(cur.text(&cur.tokens[cur.idx])) {
                return None;
            }
        }
        TokenKind::Punct(b'{' | b'[') => {}
        _ => return None,
    }
    cur.idx = eq_idx + 1;

    let try_off = match cur.peek() {
        Some(t) if matches!(t.kind, TokenKind::Ident) && cur.text(t) == "try" => {
            let at = t.span.start;
            cur.bump();
            at
        }
        _ => return None,
    };

    let (cur, byte_end, expr_span, expr_tokens) = parse_try_tail(cur, true)?;
    let expr = cur
        .parser
        .parse_expression_tokens(expr_tokens, expr_span.start, expr_span.end);
    Some((
        cur,
        byte_end,
        TryStmt {
            keyword_off: kw_span.start,
            owner_span: Span {
                start: kw_span.start,
                end: byte_end,
            },
            // The propagation, not the declaration: a diagnostic about the
            // `Err` belongs on `try <expr>`, not on `const x = `.
            span: Span {
                start: try_off,
                end: expr_span.end,
            },
            expr_span,
            decl: Some((
                cur.parser.src[kw_span.start..kw_span.end].to_string(),
                binding_span,
            )),
            expr,
            in_function: false,
        },
    ))
}

/// Parses `<expr>;` with the cursor just past a `try` keyword. Returns the
/// advanced cursor, the byte just past the `;`, and the expression's span
/// and tokens.
fn parse_try_tail<'t>(
    mut cur: Cursor<'t>,
    allow_parenthesized: bool,
) -> Option<(Cursor<'t>, usize, Span, &'t [Token])> {
    let first = cur.peek()?;
    if !is_expr_start(first)
        && !(allow_parenthesized && matches!(first.kind, TokenKind::Punct(b'(')))
    {
        return None; // includes `try {` blocks and member-signature shapes
    }
    let (semi_idx, semi_byte) = stmt_expr_end(&cur)?;
    let span = Span {
        start: first.span.start,
        end: semi_byte,
    };
    if cur.parser.src[span.start..span.end].trim().is_empty() {
        return None;
    }
    let (operand_token_end, operand_end) = scan_primary_operand(&cur)?;
    if operand_token_end != semi_idx || !cur.parser.src[operand_end..semi_byte].trim().is_empty() {
        return None;
    }
    let expr_tokens = &cur.tokens[cur.idx..semi_idx];
    cur.idx = semi_idx + 1;
    Some((cur, semi_byte + 1, span, expr_tokens))
}

/// True for a token that can start the expression of a `try`. Deliberately
/// excludes `(` and `<`: a member named `try` in valid TypeScript can
/// continue with a call/generic signature (`try(x);`, `try<T>(x: T);` in an
/// interface), which would be indistinguishable from a parenthesized
/// expression — so `try f(x);` works but `try (expr);` does not (documented).
/// Everything else a member declaration can continue with (`?`, `:`, `=`,
/// `{`, ...) is excluded too.
fn is_expr_start(t: &Token) -> bool {
    match t.kind {
        TokenKind::Ident
        | TokenKind::Str
        | TokenKind::Template(_)
        | TokenKind::Regex
        | TokenKind::JsxRaw => true,
        TokenKind::Punct(c) => {
            c.is_ascii_digit() || matches!(c, b'[' | b'!' | b'~' | b'+' | b'-' | b'/')
        }
        TokenKind::Arrow
        | TokenKind::OrOr
        | TokenKind::OptChain
        | TokenKind::Coalesce
        | TokenKind::PipeOp => false,
    }
}

/// Scans a statement expression from `cur.idx` until a top-level `;`,
/// returning its token index and byte offset. Aborts (None) on anything
/// that cannot appear at the top level of an expression: a bare `{`, a
/// closer, `,`, `=` (`=>` is a fused token and passes), `:` without a
/// pending ternary `?`, or an undotted statement-only keyword — these are
/// member-declaration or next-statement shapes, not expressions.
fn stmt_expr_end(cur: &Cursor) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut ternaries = 0usize;
    let mut k = cur.idx;
    while k < cur.tokens.len() {
        let t = &cur.tokens[k];
        if let TokenKind::Ident = t.kind {
            if depth == 0 && !dotted_at(cur.tokens, cur.idx, k) {
                let word = cur.text(t);
                if crate::lexer::statement_keyword_at(cur.parser.src, cur.tokens, k) {
                    return None;
                }
                // A match expression and a `result` block carry their own
                // top-level braces; skip the whole shape so the bare-`{`
                // abort below doesn't reject it (the recursive parse of the
                // expression decides whether it really is a tt construct).
                if let Some(past) = skip_braced_construct(cur.tokens, word, k) {
                    k = past;
                    continue;
                }
            }
            k += 1;
            continue;
        }
        if depth == 0 {
            match t.kind {
                TokenKind::Punct(b';') => {
                    return if ternaries == 0 {
                        Some((k, t.span.start))
                    } else {
                        None
                    };
                }
                TokenKind::Punct(b'{' | b'}' | b')' | b']' | b',' | b'=') => return None,
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
        k += 1;
    }
    None
}

/// Scans a declaration binding (identifier or destructuring pattern, with an
/// optional type annotation) from `cur.idx` until the top-level `=`,
/// returning its token index and byte offset. Aborts on a top-level `;`,
/// `,`, or closer before any `=` is found.
fn binding_end(cur: &Cursor) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut k = cur.idx;
    while k < cur.tokens.len() {
        let t = &cur.tokens[k];
        match t.kind {
            _ if t.opens_bracket() => depth += 1,
            _ if t.closes_bracket() => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
            }
            TokenKind::Punct(b'=') if depth == 0 => return Some((k, t.span.start)),
            TokenKind::Punct(b';' | b',') if depth == 0 => return None,
            _ => {}
        }
        k += 1;
    }
    None
}
