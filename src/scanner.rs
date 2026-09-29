//! Low-level byte scanning over tt/TypeScript source.
//!
//! All scanning is byte-based: every character the scanner makes decisions on
//! is ASCII, and UTF-8 continuation bytes (0x80+) never compare equal to any
//! ASCII byte, so multi-byte characters pass through opaquely. The one
//! question asked of a whole non-ASCII code point is whether it belongs to an
//! identifier ([`identifier_char_len`]), so a keyword is never read out of
//! the middle of a word such as `étry`.

const IDENT_CHAR: [bool; 128] = {
    let mut table = [false; 128];
    let mut b = 0;
    while b < 128 {
        let c = b as u8;
        table[b] = c.is_ascii_alphanumeric() || c == b'_' || c == b'$';
        b += 1;
    }
    table
};

#[inline]
pub(crate) fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$'
}

#[inline]
pub(crate) fn is_ident_char(b: u8) -> bool {
    b.is_ascii() && IDENT_CHAR[b as usize]
}

#[inline]
pub(crate) fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// The byte at `i`, or None at or past `end`.
#[inline]
pub(crate) fn at(src: &[u8], i: usize, end: usize) -> Option<u8> {
    if i < end { Some(src[i]) } else { None }
}

/// Whether an identifier starts at `i`: an ASCII identifier-start byte, or
/// the lead byte of a non-ASCII code point that is identifier material.
pub(crate) fn starts_identifier(src: &[u8], i: usize, end: usize) -> bool {
    at(src, i, end).is_some_and(|b| {
        is_ident_start(b) || (!b.is_ascii() && identifier_char_len(src, i, end).is_some())
    })
}

/// The byte length of the identifier character at `i`, or `None` when the
/// byte there cannot continue an identifier. Outside strings, comments,
/// templates, regexes, and JSX text, a non-ASCII code point in valid
/// TypeScript is either identifier material or one of ECMA-262's
/// non-ASCII `WhiteSpace`/`LineTerminator` code points, so only those
/// separate; the code point is consumed whole.
#[inline]
pub(crate) fn identifier_char_len(src: &[u8], i: usize, end: usize) -> Option<usize> {
    let b = at(src, i, end)?;
    if b.is_ascii() {
        return is_ident_char(b).then_some(1);
    }
    non_ascii_identifier_char_len(src, b, i, end)
}

fn non_ascii_identifier_char_len(src: &[u8], b: u8, i: usize, end: usize) -> Option<usize> {
    let len = match b {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => return None,
    };
    let c = std::str::from_utf8(src.get(i..i + len)?)
        .ok()?
        .chars()
        .next()?;
    (i + len <= end && !c.is_whitespace() && c != '\u{FEFF}').then_some(len)
}

/// Reads the identifier starting at `i`; returns the end index (exclusive).
#[inline]
pub(crate) fn ident_end(src: &[u8], i: usize, end: usize) -> usize {
    let bytes = &src[..end.min(src.len())];
    let mut j = i + identifier_char_len(src, i, end).unwrap_or(1);
    while let Some(&b) = bytes.get(j) {
        if b.is_ascii() {
            if !IDENT_CHAR[b as usize] {
                break;
            }
            j += 1;
            continue;
        }
        match non_ascii_identifier_char_len(src, b, j, end) {
            Some(len) => j += len,
            None => break,
        }
    }
    j
}

/// Skips whitespace and comments; returns the index of the next significant byte.
pub(crate) fn skip_ws_comments(src: &[u8], i: usize, end: usize) -> usize {
    skip_trivia(src, i, end).0
}

/// The byte length of the ECMA-262 `LineTerminator` (§12.3) at `i`: LF, CR,
/// U+2028 LINE SEPARATOR, or U+2029 PARAGRAPH SEPARATOR. A CR LF pair is two
/// terminators, which answers every question asked of them the same way.
#[inline]
pub(crate) fn line_terminator_len(src: &[u8], i: usize, end: usize) -> Option<usize> {
    match at(src, i, end)? {
        b'\n' | b'\r' => Some(1),
        0xE2 if i + 3 <= end && src[i + 1] == 0x80 && matches!(src[i + 2], 0xA8 | 0xA9) => Some(3),
        _ => None,
    }
}

/// Whether `src[from..to]` contains a line terminator.
pub(crate) fn contains_line_terminator(src: &[u8], from: usize, to: usize) -> bool {
    line_end(src, from, to) < to
}

/// The byte length of the white space or line terminator at `i`: the ASCII
/// ones, and the non-ASCII code points that cannot belong to an identifier
/// ([`identifier_char_len`]), which are ECMA-262 `WhiteSpace` and
/// `LineTerminator` and the byte-order mark.
#[inline]
fn space_len(src: &[u8], i: usize, end: usize) -> Option<usize> {
    let b = at(src, i, end)?;
    if b.is_ascii() {
        return is_ws(b).then_some(1);
    }
    non_ascii_space_len(src, b, i, end)
}

fn non_ascii_space_len(src: &[u8], b: u8, i: usize, end: usize) -> Option<usize> {
    let len = match b {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => return None,
    };
    let c = std::str::from_utf8(src.get(i..(i + len).min(end))?)
        .ok()?
        .chars()
        .next()?;
    (c.is_whitespace() || c == '\u{FEFF}').then_some(len)
}

/// The index just past the line break at `i` — one line terminator, or a
/// CR LF pair as one break — or `None` when none starts there.
pub(crate) fn line_break_end(src: &[u8], i: usize, end: usize) -> Option<usize> {
    if src.get(i..(i + 2).min(end)) == Some(b"\r\n") {
        return Some(i + 2);
    }
    line_terminator_len(src, i, end).map(|len| i + len)
}

/// Skips white space from `i`; line terminators too when `lines` is set.
/// Comments are not skipped.
pub(crate) fn skip_space(src: &[u8], mut i: usize, end: usize, lines: bool) -> usize {
    while let Some(len) = space_len(src, i, end) {
        if !lines && line_terminator_len(src, i, end).is_some() {
            break;
        }
        i += len;
    }
    i
}

/// The index just past the `/* … */` comment starting at `i`, or `end`
/// when it is unterminated.
pub(crate) fn block_comment_end(src: &[u8], i: usize, end: usize) -> usize {
    find_subslice(src, b"*/", i + 2, end).map_or(end, |close| close + 2)
}

/// Skips trivia — white space, line terminators, and comments — from `i`.
/// Returns the index of the next significant byte and whether a line
/// terminator was crossed, inside a block comment included (ECMA-262
/// §12.4: a multi-line comment containing a line terminator is one).
#[inline]
pub(crate) fn skip_trivia(src: &[u8], mut i: usize, end: usize) -> (usize, bool) {
    let bytes = &src[..end.min(src.len())];
    let mut line_break = false;
    loop {
        match bytes.get(i) {
            Some(b' ' | b'\t' | 0x0b | 0x0c) => i += 1,
            Some(b'\n' | b'\r') => {
                line_break = true;
                i += 1;
            }
            Some(&b) if b == b'/' || !b.is_ascii() => {
                return skip_comments_and_spaces(src, i, end, line_break);
            }
            _ => return (i, line_break),
        }
    }
}

fn skip_comments_and_spaces(
    src: &[u8],
    mut i: usize,
    end: usize,
    mut line_break: bool,
) -> (usize, bool) {
    let bytes = &src[..end.min(src.len())];
    while let Some(&b) = bytes.get(i) {
        match b {
            b' ' | b'\t' | 0x0b | 0x0c => i += 1,
            b'\n' | b'\r' => {
                line_break = true;
                i += 1;
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => i = line_end(src, i, end),
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let close = block_comment_end(src, i, end);
                line_break |= contains_line_terminator(src, i, close);
                i = close;
            }
            b if b.is_ascii() => break,
            b => match non_ascii_space_len(src, b, i, end) {
                Some(len) => {
                    line_break |= line_terminator_len(src, i, end).is_some();
                    i += len;
                }
                None => break,
            },
        }
    }
    (i, line_break)
}

pub(crate) fn find_subslice(src: &[u8], needle: &[u8], from: usize, end: usize) -> Option<usize> {
    if from >= end {
        return None;
    }
    src[from..end]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| from + p)
}

/// The index of the line terminator ending the line that contains `from`,
/// or `end`.
pub(crate) fn line_end(src: &[u8], from: usize, end: usize) -> usize {
    let mut i = from;
    while i < end {
        match src[i] {
            b'\n' | b'\r' => return i,
            0xE2 if line_terminator_len(src, i, end).is_some() => return i,
            _ => i += 1,
        }
    }
    end
}

/// `src[i]` is `'` or `"` — returns the index just past the closing quote.
pub(crate) fn scan_string(src: &[u8], mut i: usize, end: usize) -> usize {
    let quote = src[i];
    i += 1;
    while i < end {
        match src[i] {
            b'\\' => i += 2,
            b'\n' => return i, // unterminated string: stop at the newline
            b if b == quote => return i + 1,
            _ => i += 1,
        }
    }
    i.min(end)
}

/// `src[i]` is `/` where a regex literal is allowed — returns the index just
/// past the literal (including flags), or None if it doesn't scan as a regex.
pub(crate) fn scan_regex(src: &[u8], mut i: usize, end: usize) -> Option<usize> {
    i += 1;
    let mut in_class = false;
    while i < end {
        if line_terminator_len(src, i, end).is_some() {
            return None;
        }
        match src[i] {
            b'\\' => i += 2,
            b'[' => {
                in_class = true;
                i += 1;
            }
            b']' => {
                in_class = false;
                i += 1;
            }
            b'/' if !in_class => {
                i += 1;
                while i < end && is_ident_char(src[i]) {
                    i += 1;
                }
                return Some(i.min(end));
            }
            _ => i += 1,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{ident_end, identifier_char_len, starts_identifier};

    #[test]
    fn non_ascii_letters_continue_an_identifier_and_white_space_ends_it() {
        for word in ["étry", "名try", "tryé", "a\u{200C}b", "𝑥try"] {
            let src = format!("{word} ");
            assert!(starts_identifier(src.as_bytes(), 0, src.len()), "{word}");
            assert_eq!(
                ident_end(src.as_bytes(), 0, src.len()),
                word.len(),
                "{word}"
            );
        }
        for separator in [
            "\u{00A0}", "\u{1680}", "\u{2000}", "\u{200A}", "\u{2028}", "\u{2029}", "\u{202F}",
            "\u{205F}", "\u{3000}", "\u{FEFF}",
        ] {
            let src = format!("x{separator}try");
            let bytes = src.as_bytes();
            assert_eq!(
                identifier_char_len(bytes, 1, bytes.len()),
                None,
                "{separator:?}"
            );
            assert_eq!(ident_end(bytes, 0, bytes.len()), 1, "{separator:?}");
            assert!(!starts_identifier(bytes, 1, bytes.len()), "{separator:?}");
        }
        let cut = "é".as_bytes();
        assert_eq!(identifier_char_len(cut, 0, 1), None);
        assert_eq!(identifier_char_len(cut, 1, 2), None);
    }
}
