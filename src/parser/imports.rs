//! Structural parsing of literal import/re-export module specifiers.
//!
//! Only module-specifier strings are lifted; surrounding syntax stays verbatim.
//! Static declarations, dynamic imports, and import types share the same
//! relative `.tt`/`.ttx` rewrite. Computed imports and `import.meta` remain
//! untouched, as do TypeScript import-assignment declarations.
//!
//! Alongside the specifier, the clause's imported names are collected for
//! the declaration-collection API (project-wide exhaustiveness). Name
//! collection is best-effort and never changes whether a specifier is
//! lifted: an entry that doesn't parse as `[type] name [as alias]` is
//! skipped, costing only the exhaustiveness information for that binding.

use super::cursor::Cursor;
use super::is_reserved;
use crate::ast::{Span, TtImportDecl, TtImportNames, TtSpecifier};

use crate::lexer::{Token, TokenKind};

/// `cur` is positioned just past an `import` or `export` keyword (`kw`).
/// Returns the advanced cursor and the lifted import (specifier span plus
/// imported names) when the clause identifies an import/re-export module string
/// recognized by `tt_spec_span`.
pub(super) fn parse_tt_import<'t>(
    mut cur: Cursor<'t>,
    kw: &str,
) -> Option<(Cursor<'t>, TtImportDecl)> {
    let first = cur.peek()?;

    if kw == "import" {
        match first.kind {
            // `import "spec";` — side-effect import, the specifier is right here.
            TokenKind::Str => {
                let (spec, kind) = tt_spec_span(&cur, first.span)?;
                cur.bump();
                return Some((
                    cur,
                    TtImportDecl {
                        spec,
                        kind,
                        names: TtImportNames::None,
                    },
                ));
            }
            // Both dynamic imports and import types have a literal first
            // argument. Require the argument boundary so concatenations are
            // never mistaken for a complete module specifier.
            TokenKind::Punct(b'(') => {
                cur.bump();
                let token = cur.peek()?;
                if !matches!(token.kind, TokenKind::Str) {
                    return None;
                }
                let (spec, kind) = tt_spec_span(&cur, token.span)?;
                cur.bump();
                if !matches!(cur.peek()?.kind, TokenKind::Punct(b')' | b',')) {
                    return None;
                }
                return Some((
                    cur,
                    TtImportDecl {
                        spec,
                        kind,
                        names: TtImportNames::None,
                    },
                ));
            }
            TokenKind::Punct(b'.') | TokenKind::OptChain => return None,
            _ => {}
        }
        clause_then_spec(cur, true)
    } else {
        // A re-export starts with `{`, `*`, or `type` followed by `{`/`*`;
        // anything else (`const`, `default`, `enum`, ...) is not one.
        // Re-exports bring nothing into local scope, so no names.
        match first.kind {
            TokenKind::Punct(b'{' | b'*') => clause_then_spec(cur, false),
            TokenKind::Ident if cur.text(first) == "type" => {
                match cur.tokens.get(cur.idx + 1).map(|t| &t.kind) {
                    Some(TokenKind::Punct(b'{' | b'*')) => {
                        cur.bump();
                        clause_then_spec(cur, false)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

/// Consumes an import/re-export clause token by token until `from`, then
/// expects the specifier string. Clauses are only identifiers (bindings,
/// contextual `type`/`as`), `{ ... }` lists, `*`, and `,` — any other
/// token (a reserved word, `=`, `;`, ...) means this is not a static
/// import clause. `local` is true for `import` declarations, whose clause
/// names enter local scope and are collected.
fn clause_then_spec(mut cur: Cursor<'_>, local: bool) -> Option<(Cursor<'_>, TtImportDecl)> {
    let mut namespace: Option<String> = None;
    let mut named: Option<Vec<(String, Option<String>)>> = None;
    loop {
        let t = cur.peek()?;
        match t.kind {
            TokenKind::Punct(b'{') => {
                let open = cur.idx;
                let close = cur.find_close()?;
                if local {
                    named.get_or_insert_default().extend(named_entries(cur.sub(
                        open + 1,
                        close,
                        cur.tokens[close].span.start,
                    )));
                }
                cur.idx = close + 1;
            }
            TokenKind::Punct(b'*') => {
                cur.bump();
                // `* as ns` — remember the namespace name for `import`.
                if local
                    && matches!(cur.peek(), Some(t) if matches!(t.kind, TokenKind::Ident) && cur.text(t) == "as")
                    && let Some(ns) = cur.tokens.get(cur.idx + 1)
                    && matches!(ns.kind, TokenKind::Ident)
                {
                    namespace = Some(cur.text(ns).to_string());
                }
            }
            TokenKind::Punct(b',') => {
                cur.bump();
            }
            TokenKind::Ident => {
                let word = cur.text(t);
                if word == "from" {
                    cur.bump();
                    let spec_tok = cur.peek()?;
                    if !matches!(spec_tok.kind, TokenKind::Str) {
                        return None;
                    }
                    let (spec, kind) = tt_spec_span(&cur, spec_tok.span)?;
                    cur.bump();
                    let names = match (namespace, named) {
                        _ if !local => TtImportNames::None,
                        (Some(ns), _) => TtImportNames::Namespace(ns),
                        (None, Some(entries)) => TtImportNames::Named(entries),
                        // only a default binding (tt variants are named exports)
                        (None, None) => TtImportNames::None,
                    };
                    return Some((cur, TtImportDecl { spec, kind, names }));
                }
                if is_reserved(word) {
                    return None;
                }
                cur.bump();
            }
            _ => return None,
        }
    }
}

/// Best-effort parse of `{ [type] name [as alias], ... }` entries as
/// (exported name, alias) pairs. An entry that doesn't fit the shape is
/// skipped — never a parse failure.
fn named_entries(cur: Cursor) -> Vec<(String, Option<String>)> {
    let mut entries = Vec::new();
    // split the brace contents on top-level commas
    let mut entry: Vec<&str> = Vec::new();
    let mut flush = |entry: &mut Vec<&str>| {
        let words: Vec<&str> = match entry.first() {
            Some(&"type") if entry.len() > 1 => entry[1..].to_vec(),
            _ => entry.clone(),
        };
        match words.as_slice() {
            [name] => entries.push((name.to_string(), None)),
            [name, "as", alias] => entries.push((name.to_string(), Some(alias.to_string()))),
            _ => {} // exotic entry (string name, ...) — skip
        }
        entry.clear();
    };
    let mut clean = true; // entry contained only identifiers
    for t in cur.tokens {
        match t.kind {
            TokenKind::Punct(b',') => {
                if clean {
                    flush(&mut entry);
                } else {
                    entry.clear();
                }
                clean = true;
            }
            TokenKind::Ident => entry.push(&cur.parser.src[t.span.start..t.span.end]),
            _ => clean = false,
        }
    }
    if clean {
        flush(&mut entry);
    }
    entries
}

/// `span` is a lexed string token; returns it back if its content is a
/// relative path ending in `.tt`.
fn tt_spec_span(cur: &Cursor, span: Span) -> Option<(Span, TtSpecifier)> {
    let src = cur.parser.bytes;
    let quote = src[span.start];
    // The lexer tolerates unterminated strings (stopping at a newline or
    // EOF) — require a real closing quote.
    if span.end < span.start + 2 || src[span.end - 1] != quote {
        return None;
    }
    let spec = &src[span.start + 1..span.end - 1];
    if let Some(module) = crate::stdlib::StdModule::from_specifier(spec) {
        return Some((span, TtSpecifier::Std(module)));
    }
    let relative = spec.starts_with(b"./") || spec.starts_with(b"../");
    if relative && spec.ends_with(b".tt") {
        Some((span, TtSpecifier::Relative(crate::SourceKind::TypeScript)))
    } else if relative && spec.ends_with(b".ttx") {
        Some((span, TtSpecifier::Relative(crate::SourceKind::Tsx)))
    } else {
        None
    }
}

/// The module's local export specifiers, `export { a, b as c };` and
/// `export type { a as c };`, as (local name, exported name) pairs in source
/// order.
///
/// A clause followed by `from` re-exports another module and binds no local
/// name, so it is not included; an entry other than `[type] name [as name]`
/// is skipped. Only a module-level statement exports from the module, so the
/// scan reads clauses at bracket depth 0 alone.
pub(crate) fn local_export_specifiers(src: &str, tokens: &[Token]) -> Vec<(String, String)> {
    let text = |token: &Token| &src[token.span.start..token.span.end];
    let is_word = |idx: usize, word: &str| {
        tokens
            .get(idx)
            .is_some_and(|token| matches!(token.kind, TokenKind::Ident) && text(token) == word)
    };
    let mut specifiers = Vec::new();
    let mut depth = 0usize;
    let mut idx = 0usize;
    while let Some(token) = tokens.get(idx) {
        match token.kind {
            TokenKind::Punct(b'(' | b'[' | b'{') => depth += 1,
            TokenKind::Punct(b')' | b']' | b'}') => depth = depth.saturating_sub(1),
            TokenKind::Ident
                if depth == 0
                    && text(token) == "export"
                    && !super::cursor::dotted_at(tokens, 0, idx) =>
            {
                let open = if is_word(idx + 1, "type") {
                    idx + 2
                } else {
                    idx + 1
                };
                if matches!(
                    tokens.get(open).map(|token| &token.kind),
                    Some(TokenKind::Punct(b'{'))
                ) && let Some(close) = super::cursor::find_close_at(tokens, open)
                {
                    if !is_word(close + 1, "from") {
                        specifiers.extend(export_entries(src, &tokens[open + 1..close]));
                    }
                    idx = close + 1;
                    continue;
                }
            }
            _ => {}
        }
        idx += 1;
    }
    specifiers
}

fn export_entries(src: &str, tokens: &[Token]) -> Vec<(String, String)> {
    tokens
        .split(|token| matches!(token.kind, TokenKind::Punct(b',')))
        .filter_map(|entry| {
            let words = entry
                .iter()
                .map(|token| {
                    matches!(token.kind, TokenKind::Ident)
                        .then(|| &src[token.span.start..token.span.end])
                })
                .collect::<Option<Vec<&str>>>()?;
            let words = match words.as_slice() {
                ["type", rest @ ..] if !rest.is_empty() => rest,
                all => all,
            };
            match words {
                [name] => Some((name.to_string(), name.to_string())),
                [local, "as", exported] => Some((local.to_string(), exported.to_string())),
                _ => None,
            }
        })
        .collect()
}
