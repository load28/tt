//! The lexical shapes of the `val` binding modifier.
//!
//! A `val` identifier is a modifier in one of two shapes, both on the
//! keyword's own line (an ASI-separated `val\nconst x = 1;` is an expression
//! statement naming a variable `val`, then a declaration):
//!
//! - `val const|let|var ...` — [`ValShape::Declaration`]. An identifier
//!   expression is never followed on its line by a declaration keyword in
//!   TypeScript, so this shape is tt's alone and the parser claims it here.
//! - `val <binding>` at the start of a list entry, after `(` or `,` and
//!   TypeScript's parameter-property modifiers — [`ValShape::Parameter`].
//!   Whether that entry is a formal parameter is a question about the whole
//!   list (`(val [a]) => a` against `c ? (val [0]) : w => w`), so the parser
//!   lifts the keyword and records the candidate, and the host grammar
//!   answers it ([`super::host::rejected_val_candidates`]).

use super::*;

/// The shape a `val` identifier has, before the host grammar has placed the
/// binding it precedes.
pub(super) enum ValShape {
    Declaration,
    Parameter,
}

/// Words that may sit between a parameter-list entry's start and its
/// binding — TypeScript's parameter property modifiers. `val` is allowed
/// after them (`constructor(private val x: X)`).
pub(crate) fn is_param_modifier(word: &str) -> bool {
    matches!(
        word,
        "public" | "private" | "protected" | "readonly" | "override"
    )
}

/// Words that turn `val <word>` into a valid TypeScript expression or type,
/// so a `val` in front of one is an ordinary identifier: `(val as User)`,
/// `for (val of items)`, `(val in obj)`, `<T, val extends U ? X : Y>`.
/// Excluding them keeps the tt reading of every remaining candidate a
/// well-formed TypeScript projection.
fn is_operator_word(word: &str) -> bool {
    is_reserved(word)
        || matches!(
            word,
            "as" | "satisfies"
                | "is"
                | "keyof"
                | "infer"
                | "asserts"
                | "implements"
                | "readonly"
                | "out"
                | "unique"
        )
}

/// The shape of the undotted identifier `val` at token index `idx`, or
/// `None` when it can only be an ordinary identifier.
pub(super) fn shape(src: &str, tokens: &[Token], idx: usize) -> Option<ValShape> {
    let val = tokens.get(idx)?;
    let next = tokens.get(idx + 1)?;
    if src[val.span.end..next.span.start].contains('\n') {
        return None;
    }
    let word = |token: &Token| &src[token.span.start..token.span.end];
    let binding_start = |token: &Token| match token.kind {
        TokenKind::Ident => !is_operator_word(word(token)),
        TokenKind::Punct(b'{' | b'[') => true,
        _ => false,
    };

    if matches!(next.kind, TokenKind::Ident) && matches!(word(next), "const" | "let" | "var") {
        return Some(ValShape::Declaration);
    }

    let rest = tokens.get(idx + 1..idx + 4).is_some_and(|dots| {
        dots.iter()
            .all(|dot| matches!(dot.kind, TokenKind::Punct(b'.')))
    });
    let binding = if rest { tokens.get(idx + 4)? } else { next };
    if !binding_start(binding) {
        return None;
    }
    let mut k = idx;
    while let Some(before) = k.checked_sub(1) {
        match tokens[before].kind {
            TokenKind::Ident if is_param_modifier(word(&tokens[before])) => k = before,
            TokenKind::Punct(b'(' | b',') => return Some(ValShape::Parameter),
            _ => return None,
        }
    }
    None
}

/// The byte span the parser drops for a `val` modifier: the keyword plus
/// the spaces and tabs right after it, so `val const x` emits `const x`
/// rather than ` const x`. A comment after the keyword is kept.
pub(super) fn modifier_end(src: &str, keyword_end: usize) -> usize {
    let bytes = src.as_bytes();
    let mut end = keyword_end;
    while end < bytes.len() && (bytes[end] == b' ' || bytes[end] == b'\t') {
        end += 1;
    }
    end
}
