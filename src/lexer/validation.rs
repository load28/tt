//! Lexical errors that must be diagnosed before host-parser recovery.

use super::*;

/// Shares lexical rejection across output, projection, and expression parsing.
pub(crate) fn host_syntax_error(src: &str, kind: SourceKind) -> Option<(Span, &'static str)> {
    host_syntax_check(src, kind).0
}

/// Protections required before host parsing even when missing delimiters can
/// be recovered. Literal/comment text is excluded by the shared token model.
pub(crate) fn host_lexical_error(src: &str, kind: SourceKind) -> Option<(Span, &'static str)> {
    let markers = has_conflict_marker_text(src);
    if !markers && !(kind.is_tsx() && src.as_bytes().contains(&b'<')) {
        return None;
    }
    lexical_error(src, kind, markers, &lex_with_kind(src, 0, src.len(), kind))
}

pub(crate) fn host_lexical_error_in(
    src: &str,
    kind: SourceKind,
    tokens: &[Token],
) -> Option<(Span, &'static str)> {
    lexical_error(src, kind, has_conflict_marker_text(src), tokens)
}

/// [`host_syntax_error`], and the tokens it lexed `src` into when a failure
/// was possible, for a caller that reads the same text's tokens next.
///
/// A conflict marker needs its marker text, a TSX namespaced member its
/// `<`, and an unbalanced delimiter a delimiter byte; text with none of
/// them cannot fail, and is not lexed.
pub(crate) fn host_syntax_check(
    src: &str,
    kind: SourceKind,
) -> (Option<(Span, &'static str)>, Option<Vec<Token>>) {
    let markers = has_conflict_marker_text(src);
    if markers
        || src.bytes().any(|byte| match byte {
            b'(' | b')' | b'[' | b']' | b'{' | b'}' => true,
            b'<' => kind.is_tsx(),
            _ => false,
        })
    {
        let tokens = lex_with_kind(src, 0, src.len(), kind);
        (host_syntax_error_in(src, kind, &tokens), Some(tokens))
    } else {
        (None, None)
    }
}

/// Whether `src` holds one of the four seven-byte conflict-marker runs
/// (`=======`, `<<<<<<<`, `>>>>>>>`, `|||||||`) anywhere, literal and
/// comment text included.
///
/// A run of seven equal bytes covers one of every seven consecutive
/// positions, so the scan reads every seventh byte and measures the run
/// only around a marker byte.
fn has_conflict_marker_text(src: &str) -> bool {
    const RUN: usize = 7;
    let bytes = src.as_bytes();
    let mut probe = RUN - 1;
    while let Some(&byte) = bytes.get(probe) {
        if !matches!(byte, b'=' | b'<' | b'>' | b'|') {
            probe += RUN;
            continue;
        }
        let start = bytes[..probe]
            .iter()
            .rposition(|other| *other != byte)
            .map_or(0, |before| before + 1);
        let end = bytes[probe..]
            .iter()
            .position(|other| *other != byte)
            .map_or(bytes.len(), |after| probe + after);
        if end - start >= RUN {
            return true;
        }
        probe = end + RUN - 1;
    }
    false
}

pub(crate) fn host_syntax_error_in(
    src: &str,
    kind: SourceKind,
    tokens: &[Token],
) -> Option<(Span, &'static str)> {
    host_lexical_error_in(src, kind, tokens).or_else(|| {
        unbalanced_delimiter(tokens).map(|span| (span, "unbalanced TypeScript delimiter"))
    })
}

fn lexical_error(
    src: &str,
    kind: SourceKind,
    markers: bool,
    tokens: &[Token],
) -> Option<(Span, &'static str)> {
    if markers && let Some(span) = conflict_marker(src, tokens) {
        return Some((span, "merge conflict marker encountered"));
    }
    if kind.is_tsx()
        && let Some(span) = invalid_jsx_namespace_member(tokens)
    {
        return Some((
            span,
            "a JSX namespace name cannot be followed by member access",
        ));
    }
    None
}

/// Finds an unmatched `()`, `[]`, or `{}` delimiter without interpreting
/// delimiters inside strings, comments, templates, regexes, or JSX text.
fn unbalanced_delimiter(tokens: &[Token]) -> Option<Span> {
    fn walk(tokens: &[Token], stack: &mut Vec<(u8, Span)>) -> Option<Span> {
        crate::stack::grow(|| walk_grown(tokens, stack))
    }

    fn walk_grown(tokens: &[Token], stack: &mut Vec<(u8, Span)>) -> Option<Span> {
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
                        let TplPart::Interp { span, tokens } = part else {
                            continue;
                        };
                        // An interpolation that runs to the template's end
                        // never met its `}`: its `${` is still open.
                        if span.end == token.span.end {
                            let open = Span {
                                start: span.start - 2,
                                end: span.start,
                            };
                            stack.push((b'{', open));
                        }
                        if let Some(span) = walk(tokens, stack) {
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
    if let Some(span) = walk(tokens, &mut stack) {
        return Some(span);
    }
    stack.last().map(|(_, span)| *span)
}

fn conflict_marker(src: &str, tokens: &[Token]) -> Option<Span> {
    crate::stack::grow(|| conflict_marker_grown(src, tokens))
}

fn conflict_marker_grown(src: &str, tokens: &[Token]) -> Option<Span> {
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
        if index == 0
            || crate::scanner::contains_line_terminator(src.as_bytes(), previous_end, start)
        {
            return Some(Span {
                start,
                end: start + 7,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn searched(src: &str) -> bool {
        ["=======", "<<<<<<<", ">>>>>>>", "|||||||"]
            .iter()
            .any(|marker| src.contains(marker))
    }

    #[test]
    fn the_marker_scan_answers_what_a_search_for_each_marker_answers() {
        let alphabet = ['=', '<', 'a'];
        let mut texts = vec![String::new()];
        for _ in 0..10 {
            texts = texts
                .iter()
                .flat_map(|text| {
                    alphabet.iter().map(move |byte| {
                        let mut longer = text.clone();
                        longer.push(*byte);
                        longer
                    })
                })
                .collect();
            for text in &texts {
                assert_eq!(has_conflict_marker_text(text), searched(text), "{text:?}");
            }
        }
        for run in 1..=16 {
            for byte in ['=', '<', '>', '|'] {
                for before in 0..8 {
                    for after in 0..8 {
                        let text = format!(
                            "{}{}{}",
                            "x".repeat(before),
                            byte.to_string().repeat(run),
                            "y".repeat(after)
                        );
                        assert_eq!(has_conflict_marker_text(&text), run >= 7, "{text:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn text_that_cannot_fail_the_check_is_not_lexed() {
        let work = crate::work::measure(|| {
            for (text, kind) in [
                ("a + b", SourceKind::Tsx),
                ("a < b", SourceKind::TypeScript),
            ] {
                let (error, tokens) = host_syntax_check(text, kind);
                assert!(error.is_none() && tokens.is_none(), "{text:?}");
            }
        });
        assert_eq!(work.get("whole-text lexes"), None);
        let work = crate::work::measure(|| {
            let (error, tokens) = host_syntax_check("<a:b.c", SourceKind::Tsx);
            assert!(error.is_some() && tokens.is_some());
        });
        assert_eq!(work["whole-text lexes"], 1);
    }
}
