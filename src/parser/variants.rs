//! Structural parsing of tt `variant` declarations.
//!
//! `variant` is a tt-owned contextual keyword. Every fully parsed declaration
//! is lifted, including unit-only declarations; TypeScript `enum` declarations
//! never enter this parser and pass through verbatim.

use super::Claim;
use super::cursor::Cursor;
use super::is_reserved;
use crate::ast::{Field, RecoveryKind, RecoveryNode, Span, VariantCase, VariantDecl};
use crate::lexer::TokenKind;

/// `cur` is positioned just past the `variant` keyword. On success returns
/// the advanced cursor, the byte just past the closing brace, and the
/// parsed declaration.
pub(super) fn parse_variant<'t>(
    cur: Cursor<'t>,
    exported: bool,
) -> Claim<(Cursor<'t>, usize, VariantDecl)> {
    if let Some(parsed) = parse_variant_complete(cur, exported) {
        return Claim::Parsed(parsed);
    }
    if variant_committed(cur) {
        let keyword = cur
            .idx
            .checked_sub(1)
            .and_then(|idx| cur.tokens.get(idx))
            .map_or(0, |token| token.span.start);
        let start = if exported {
            cur.idx
                .checked_sub(2)
                .and_then(|idx| cur.tokens.get(idx))
                .map_or(keyword, |token| token.span.start)
        } else {
            keyword
        };
        let name = cur
            .tokens
            .get(cur.idx)
            .map(|token| cur.text(token).to_string())
            .unwrap_or_else(|| "$tt_invalid_variant".to_string());
        let end = (cur.idx + 1..cur.tokens.len())
            .find(|&idx| matches!(cur.tokens[idx].kind, TokenKind::Punct(b'{')))
            .and_then(|open| super::cursor::find_close_at(cur.tokens, open))
            .and_then(|close| cur.tokens.get(close))
            .map_or(cur.range_end, |token| token.span.end);
        return Claim::Malformed {
            error: crate::error::TtError::span(
                keyword,
                keyword + "variant".len(),
                "tt `variant` could not be parsed".to_string(),
            )
            .code(crate::DiagnosticCode::MalformedVariant)
            .help("a case is `Case` or `Case(field: Type)`"),
            recovery: RecoveryNode {
                span: Span { start, end },
                kind: RecoveryKind::VariantDecl { name, exported },
            },
        };
    }
    Claim::NotTt
}

fn variant_committed(cur: Cursor<'_>) -> bool {
    let Some(name) = cur.tokens.get(cur.idx) else {
        return false;
    };
    if !matches!(name.kind, TokenKind::Ident) {
        return false;
    }
    matches!(
        cur.tokens.get(cur.idx + 1).map(|t| &t.kind),
        Some(TokenKind::Punct(b'<' | b'{'))
    )
}

fn parse_variant_complete<'t>(
    mut cur: Cursor<'t>,
    exported: bool,
) -> Option<(Cursor<'t>, usize, VariantDecl)> {
    let (name, name_span) = cur.eat_ident()?;
    if is_reserved(name) {
        return None;
    }

    let mut generics = "";
    if cur.at_punct(b'<') {
        let close = cur.find_close()?;
        generics = &cur.parser.src[cur.tokens[cur.idx].span.start..cur.tokens[close].span.end];
        cur.idx = close + 1;
    }

    if !cur.at_punct(b'{') {
        return None;
    }
    let open = cur.idx;
    let close = cur.find_close()?;
    let inner = cur.sub(open + 1, close, cur.tokens[close].span.start);

    let cases = match parse_variant_cases(inner) {
        Some(cases) if !cases.is_empty() => cases,
        _ => return None,
    };

    let byte_end = cur.tokens[close].span.end;
    cur.idx = close + 1;
    Some((
        cur,
        byte_end,
        VariantDecl {
            name: name.to_string(),
            name_off: name_span.start,
            exported,
            generics: generics.to_string(),
            cases,
        },
    ))
}

fn parse_variant_cases(mut cur: Cursor) -> Option<Vec<VariantCase>> {
    let mut cases = Vec::new();
    loop {
        if cur.peek().is_none() {
            break;
        }
        let (tag, tag_span) = cur.eat_ident()?;
        if is_reserved(tag) {
            return None;
        }

        let mut fields = None;
        if cur.at_punct(b'(') {
            let open = cur.idx;
            let close = cur.find_close()?;
            fields = Some(parse_fields(cur.sub(
                open + 1,
                close,
                cur.tokens[close].span.start,
            ))?);
            cur.idx = close + 1;
        }
        cases.push(VariantCase {
            tag: tag.to_string(),
            tag_off: tag_span.start,
            fields,
        });

        if cur.peek().is_none() {
            break;
        }
        cur.eat_punct(b',')?;
    }
    Some(cases)
}

/// Parses `name: Type, name?: Type, ...`. Returns None on failure.
fn parse_fields(mut cur: Cursor) -> Option<Vec<Field>> {
    let mut fields = Vec::new();
    loop {
        if cur.peek().is_none() {
            break;
        }
        let (name, name_span) = cur.eat_ident()?;
        if is_reserved(name) {
            return None;
        }

        let mut optional = false;
        if cur.eat_punct(b'?').is_some() {
            optional = true;
        }
        let colon = cur.eat_punct(b':')?;

        // The annotation text runs from just past the `:` to the stopping
        // token, exactly like the byte scanner — comments inside stay part
        // of the text; only surrounding whitespace is trimmed.
        let ty_start = colon.end;
        let (stop_idx, stop_byte) = type_end(&cur);
        let raw = &cur.parser.src[ty_start..stop_byte];
        let ty = raw.trim();
        if ty.is_empty() {
            return None;
        }
        let ty_off = ty_start + (raw.len() - raw.trim_start().len());
        fields.push(Field {
            name: name.to_string(),
            name_off: name_span.start,
            optional,
            ty: ty.to_string(),
            ty_off,
        });
        cur.idx = stop_idx;

        if cur.peek().is_none() {
            break;
        }
        cur.eat_punct(b',')?;
    }
    Some(fields)
}

/// Scans a type annotation from `cur.idx` until a top-level `,` or closing
/// bracket, returning the stopping token index and the byte where the
/// annotation ends (`range_end` when the tokens run out — the enclosing
/// closer's position).
fn type_end(cur: &Cursor) -> (usize, usize) {
    let mut closers = Vec::new();
    let mut k = cur.idx;
    while k < cur.tokens.len() {
        match cur.tokens[k].kind {
            TokenKind::Punct(b'(') => closers.push(b')'),
            TokenKind::Punct(b'[') => closers.push(b']'),
            TokenKind::Punct(b'{') => closers.push(b'}'),
            TokenKind::Punct(b'<') => closers.push(b'>'),
            TokenKind::Punct(close @ (b')' | b']' | b'}' | b'>'))
                if closers.last() == Some(&close) =>
            {
                closers.pop();
            }
            TokenKind::Punct(b',') if closers.is_empty() => {
                return (k, cur.tokens[k].span.start);
            }
            _ => {}
        }
        k += 1;
    }
    (k, cur.range_end)
}
