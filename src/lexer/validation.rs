//! Lexical errors that must be diagnosed before host-parser recovery.

use super::*;

/// Shares lexical rejection across output, projection, and expression parsing.
pub(crate) fn host_syntax_error(src: &str, kind: SourceKind) -> Option<(Span, &'static str)> {
    if ["=======", "<<<<<<<", ">>>>>>>", "|||||||"]
        .iter()
        .any(|marker| src.contains(marker))
        && let Some(span) = conflict_marker(src, &lex_with_kind(src, 0, src.len(), kind))
    {
        return Some((span, "merge conflict marker encountered"));
    }
    if kind.is_tsx()
        && let Some(span) = invalid_jsx_entity(src)
    {
        return Some((span, "malformed JSX character reference"));
    }
    if kind.is_tsx()
        && let Some(span) = invalid_jsx_namespace_member(src)
    {
        return Some((
            span,
            "a JSX namespace name cannot be followed by member access",
        ));
    }
    None
}

/// Finds numeric JSX character references that are malformed enough to reach
/// an internal unwrap in the SWC lexer. Only opaque JSX text is inspected, so
/// an HTML-looking string, comment, or template chunk remains valid
/// TypeScript text.
fn invalid_jsx_entity(src: &str) -> Option<Span> {
    fn in_tokens(src: &str, tokens: &[Token]) -> Option<Span> {
        for token in tokens {
            match &token.kind {
                TokenKind::JsxRaw => {
                    let bytes = src.as_bytes();
                    let mut at = token.span.start;
                    while at + 1 < token.span.end {
                        if bytes[at] == b'&' && bytes[at + 1] == b'#' {
                            let mut cursor = at + 2;
                            let hexadecimal = bytes.get(cursor) == Some(&b'x')
                                || bytes.get(cursor) == Some(&b'X');
                            if hexadecimal {
                                cursor += 1;
                            }
                            let digit_start = cursor;
                            while cursor < token.span.end
                                && if hexadecimal {
                                    bytes[cursor].is_ascii_hexdigit()
                                } else {
                                    bytes[cursor].is_ascii_digit()
                                }
                            {
                                cursor += 1;
                            }
                            if digit_start == cursor || bytes.get(cursor) != Some(&b';') {
                                return Some(Span {
                                    start: at,
                                    end: (cursor + 1).min(token.span.end),
                                });
                            }
                            at = cursor + 1;
                            continue;
                        }
                        at += 1;
                    }
                }
                TokenKind::Template(parts) => {
                    for part in parts.iter() {
                        if let TplPart::Interp { tokens, .. } = part
                            && let Some(span) = in_tokens(src, tokens)
                        {
                            return Some(span);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    if let Some(span) = in_tokens(src, &lex_with_kind(src, 0, src.len(), SourceKind::Tsx)) {
        return Some(span);
    }

    // The hand lexer intentionally gives up on incomplete JSX, which is
    // exactly the shape that triggers SWC's entity panic. Keep a small raw
    // scanner for that recovery path, while ignoring quoted TypeScript text.
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut jsx_open = false;
    while i + 1 < bytes.len() {
        match bytes[i] {
            b'\'' | b'"' | b'`' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i = (i + 2).min(bytes.len());
                    } else if bytes[i] == quote {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(bytes.len());
            }
            b'<' if bytes.get(i + 1) == Some(&b'>')
                || bytes
                    .get(i + 1)
                    .is_some_and(|byte| byte.is_ascii_alphabetic()) =>
            {
                jsx_open = true;
                i += 1;
            }
            b'&' if jsx_open && bytes.get(i + 1) == Some(&b'#') => {
                let mut cursor = i + 2;
                let hexadecimal = matches!(bytes.get(cursor), Some(b'x' | b'X'));
                if hexadecimal {
                    cursor += 1;
                }
                let digit_start = cursor;
                while cursor < bytes.len()
                    && if hexadecimal {
                        bytes[cursor].is_ascii_hexdigit()
                    } else {
                        bytes[cursor].is_ascii_digit()
                    }
                {
                    cursor += 1;
                }
                if digit_start == cursor || bytes.get(cursor) != Some(&b';') {
                    return Some(Span {
                        start: i,
                        end: (cursor + 1).min(bytes.len()),
                    });
                }
                i = cursor + 1;
            }
            _ => i += 1,
        }
    }
    None
}

fn conflict_marker(src: &str, tokens: &[Token]) -> Option<Span> {
    for (index, token) in tokens.iter().enumerate() {
        if let TokenKind::Template(parts) = &token.kind {
            for part in parts.iter() {
                if let TplPart::Interp { tokens, .. } = part
                    && let Some(span) = conflict_marker(src, tokens)
                {
                    return Some(span);
                }
            }
        }
        if !matches!(
            token.kind,
            TokenKind::Punct(b'=' | b'<' | b'>') | TokenKind::OrOr
        ) {
            continue;
        }
        let start = token.span.start;
        let tail = &src.as_bytes()[start..];
        let delimiter = tail[0];
        if tail.len() < 7 || !tail[..7].iter().all(|byte| *byte == delimiter) {
            continue;
        }
        if delimiter != b'=' && tail.get(7) != Some(&b' ') {
            continue;
        }
        // A line break in leading trivia also counts when it is inside a
        // comment. This is the host lexer's token-boundary contract. Literal,
        // comment, and JSX text contents never enter this token branch.
        let previous_end = index
            .checked_sub(1)
            .map_or(0, |previous| tokens[previous].span.end);
        if index == 0 || src[previous_end..start].contains(['\n', '\r']) {
            return Some(Span {
                start,
                end: start + 7,
            });
        }
    }
    None
}
