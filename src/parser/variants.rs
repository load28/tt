//! Structural parsing of tt `variant` declarations.
//!
//! `variant` is a tt-owned contextual keyword. Every fully parsed declaration
//! is lifted, including unit-only declarations; TypeScript `enum` declarations
//! never enter this parser and pass through verbatim.

use super::Claim;
use super::cursor::Cursor;
use super::is_reserved;
use crate::ast::{
    Comment, Comments, Field, RecoveryKind, RecoveryNode, Span, VariantCase, VariantDecl,
};
use crate::lexer::TokenKind;

/// `cur` is positioned just past the `variant` keyword. On success returns
/// the advanced cursor, the byte just past the closing brace, and the
/// parsed declaration.
pub(super) fn parse_variant<'t>(
    cur: Cursor<'t>,
    exported: bool,
    declared: bool,
) -> Claim<(Cursor<'t>, usize, VariantDecl)> {
    if cur.line_break_before() {
        return Claim::NotTt;
    }
    if let Some(parsed) = parse_variant_complete(cur, exported, declared) {
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

/// `export default variant Name { ... }`, with `cur` just past `variant` and
/// `export_start` the byte of `export`. A variant declares a type and a
/// value, and a TypeScript module has one default export that no
/// declaration form can give both meanings, so the declaration is never
/// claimed. A complete declaration reports `variant-default-export` at
/// `default`; an incomplete one reports what `variant` alone would.
pub(super) fn parse_default_variant(
    cur: Cursor<'_>,
    export_start: usize,
) -> Claim<std::convert::Infallible> {
    let Some(default) = cur
        .idx
        .checked_sub(2)
        .and_then(|idx| cur.tokens.get(idx))
        .map(|token| token.span)
    else {
        return Claim::NotTt;
    };
    let keyword = cur.tokens[cur.idx - 1].span.start;
    match parse_variant(cur, false, false) {
        Claim::Parsed((_, end, decl)) => Claim::Malformed {
            error: crate::error::TtError::span(
                default.start,
                default.end,
                format!("variant `{}` cannot be a default export", decl.name),
            )
            .code(crate::DiagnosticCode::VariantDefaultExport)
            .suggest(
                format!("export it by name and import it as `{{ {} }}`", decl.name),
                default.start,
                keyword,
                "",
            ),
            recovery: RecoveryNode {
                span: Span {
                    start: export_start,
                    end,
                },
                kind: RecoveryKind::VariantDecl {
                    name: decl.name,
                    exported: true,
                },
            },
        },
        Claim::Malformed { error, recovery } => Claim::Malformed {
            error,
            recovery: RecoveryNode {
                span: Span {
                    start: export_start,
                    end: recovery.span.end,
                },
                kind: match recovery.kind {
                    RecoveryKind::VariantDecl { name, .. } => RecoveryKind::VariantDecl {
                        name,
                        exported: true,
                    },
                    kind => kind,
                },
            },
        },
        Claim::Unclaimed(candidate) => Claim::Unclaimed(candidate),
        Claim::NotTt => Claim::NotTt,
    }
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
    declared: bool,
) -> Option<(Cursor<'t>, usize, VariantDecl)> {
    let keyword_index = cur.idx.checked_sub(1)?;
    let owner_start = keyword_index.checked_sub(usize::from(exported) + usize::from(declared))?;
    let owner_start = cur.tokens.get(owner_start)?.span.start;
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

    let cases = match parse_variant_cases(inner, cur.tokens[open].span.end) {
        Some(cases) if !cases.is_empty() => cases,
        _ => return None,
    };

    let byte_end = cur.tokens[close].span.end;
    cur.idx = close + 1;
    Some((
        cur,
        byte_end,
        VariantDecl {
            span: Span {
                start: owner_start,
                end: byte_end,
            },
            name: name.to_string(),
            name_off: name_span.start,
            exported,
            declared,
            generics: generics.to_string(),
            cases,
        },
    ))
}

fn parse_variant_cases(mut cur: Cursor, body_start: usize) -> Option<Vec<VariantCase>> {
    let src = cur.parser.src;
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
        let mut end = tag_span.end;
        let mut comments = Comments::default();
        if cur.at_punct(b'(') {
            let open = cur.idx;
            let close = cur.find_close()?;
            let open_span = cur.tokens[open].span;
            let close_start = cur.tokens[close].span.start;
            comments.trailing.extend(
                gap_comments(src, tag_span.end, open_span.start)
                    .into_iter()
                    .map(|(comment, _)| comment),
            );
            let parsed = parse_fields(cur.sub(open + 1, close, close_start), open_span.end)?;
            if parsed.is_empty() {
                comments.trailing.extend(
                    gap_comments(src, open_span.end, close_start)
                        .into_iter()
                        .map(|(comment, _)| comment),
                );
            }
            fields = Some(parsed);
            end = cur.tokens[close].span.end;
            cur.idx = close + 1;
        }
        cases.push(VariantCase {
            span: Span {
                start: tag_span.start,
                end,
            },
            tag: tag.to_string(),
            tag_off: tag_span.start,
            fields,
            comments,
        });

        if cur.peek().is_none() {
            break;
        }
        cur.eat_punct(b',')?;
    }
    attach_comments(
        src,
        body_start,
        cur.range_end,
        cases
            .iter_mut()
            .map(|case| (case.span, &mut case.comments))
            .collect(),
    );
    Some(cases)
}

/// Parses `name: Type, name?: Type, ...`. Returns None on failure.
fn parse_fields(mut cur: Cursor, list_start: usize) -> Option<Vec<Field>> {
    let src = cur.parser.src;
    let mut fields = Vec::new();
    let mut extents = Vec::new();
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

        // The annotation text runs from just past the `:` to its last
        // token, so comments between its tokens stay part of the text and
        // comments after it belong to the field.
        let ty_start = colon.end;
        let (stop_idx, _) = type_end(&cur);
        if stop_idx == cur.idx {
            return None;
        }
        let ty_end = cur.tokens[stop_idx - 1].span.end;
        let raw = &src[ty_start..ty_end];
        let ty = raw.trim();
        if ty.is_empty() {
            return None;
        }
        let ty_off = ty_start + (raw.len() - raw.trim_start().len());
        let comments = Comments {
            leading: Vec::new(),
            trailing: gap_comments(src, name_span.end, colon.start)
                .into_iter()
                .map(|(comment, _)| comment)
                .collect(),
        };
        fields.push(Field {
            name: name.to_string(),
            name_off: name_span.start,
            optional,
            ty: ty.to_string(),
            ty_off,
            comments,
        });
        extents.push(Span {
            start: name_span.start,
            end: ty_end,
        });
        cur.idx = stop_idx;

        if cur.peek().is_none() {
            break;
        }
        cur.eat_punct(b',')?;
    }
    attach_comments(
        src,
        list_start,
        cur.range_end,
        extents
            .into_iter()
            .zip(fields.iter_mut())
            .map(|(extent, field)| (extent, &mut field.comments))
            .collect(),
    );
    Some(fields)
}

fn attach_comments(src: &str, start: usize, end: usize, mut elements: Vec<(Span, &mut Comments)>) {
    let Some((first, _)) = elements.first() else {
        return;
    };
    let leading: Vec<Comment> = gap_comments(src, start, first.start)
        .into_iter()
        .map(|(comment, _)| comment)
        .collect();
    elements[0].1.leading.extend(leading);
    for index in 0..elements.len() {
        let gap_end = elements.get(index + 1).map_or(end, |(next, _)| next.start);
        for (comment, after_break) in gap_comments(src, elements[index].0.end, gap_end) {
            match elements.get_mut(index + 1) {
                Some((_, next)) if after_break => next.leading.push(comment),
                _ => elements[index].1.trailing.push(comment),
            }
        }
    }
}

fn gap_comments(src: &str, start: usize, end: usize) -> Vec<(Comment, bool)> {
    let bytes = src.as_bytes();
    let mut comments = Vec::new();
    let mut index = start;
    let mut line_break = false;
    let mut after_break = false;
    while index < end {
        let stop = match (bytes[index], bytes.get(index + 1)) {
            (b'\n' | b'\r', _) => {
                line_break = true;
                after_break = true;
                index += 1;
                continue;
            }
            (b'/', Some(b'/')) => bytes[index..end]
                .iter()
                .position(|&byte| byte == b'\n' || byte == b'\r')
                .map_or(end, |offset| index + offset),
            (b'/', Some(b'*')) => src[index + 2..end]
                .find("*/")
                .map_or(end, |offset| index + 2 + offset + 2),
            _ => {
                index += 1;
                continue;
            }
        };
        let line_start = src[..index].rfind(['\n', '\r']).map_or(0, |at| at + 1);
        comments.push((
            Comment {
                text: src[index..stop].to_string(),
                column: index - line_start,
                own_line: line_break,
            },
            after_break,
        ));
        line_break = false;
        index = stop;
    }
    comments
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
