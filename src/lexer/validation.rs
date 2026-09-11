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
        && let Some(span) = invalid_jsx_namespace_member(src)
    {
        return Some((
            span,
            "a JSX namespace name cannot be followed by member access",
        ));
    }
    if let Some(span) = unbalanced_delimiter(src, kind) {
        return Some((span, "unbalanced TypeScript delimiter"));
    }
    None
}

/// Finds an unmatched `()`, `[]`, or `{}` delimiter without interpreting
/// delimiters inside strings, comments, templates, regexes, or JSX text.
fn unbalanced_delimiter(src: &str, kind: SourceKind) -> Option<Span> {
    fn walk(tokens: &[Token], stack: &mut Vec<(u8, Span)>) -> Option<Span> {
        for token in tokens {
            match &token.kind {
                TokenKind::Punct(byte @ (b'(' | b'[' | b'{')) => stack.push((*byte, token.span)),
                TokenKind::Punct(byte @ (b')' | b']' | b'}')) => {
                    let expected = match byte {
                        b')' => b'(',
                        b']' => b'[',
                        b'}' => b'{',
                        _ => unreachable!(),
                    };
                    if stack.pop().is_none_or(|(open, _)| open != expected) {
                        return Some(token.span);
                    }
                }
                TokenKind::Template(parts) => {
                    for part in parts.iter() {
                        if let TplPart::Interp { tokens, .. } = part
                            && let Some(span) = walk(tokens, stack)
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

    let mut stack = Vec::new();
    if let Some(span) = walk(&lex_with_kind(src, 0, src.len(), kind), &mut stack) {
        return Some(span);
    }
    stack.last().map(|(_, span)| *span)
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
