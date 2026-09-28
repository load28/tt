//! Low-level byte scanning over tt/TypeScript source.
//!
//! All scanning is byte-based: every character the scanner makes decisions on
//! is ASCII, and UTF-8 continuation bytes (0x80+) never compare equal to any
//! ASCII byte, so multi-byte characters pass through opaquely. The one
//! question asked of a whole non-ASCII code point is whether it belongs to an
//! identifier ([`identifier_char_len`]), so a keyword is never read out of
//! the middle of a word such as `étry`.

pub(crate) fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$'
}

pub(crate) fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

pub(crate) fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// The byte at `i`, or None at or past `end`.
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
pub(crate) fn identifier_char_len(src: &[u8], i: usize, end: usize) -> Option<usize> {
    let b = at(src, i, end)?;
    if b.is_ascii() {
        return is_ident_char(b).then_some(1);
    }
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
pub(crate) fn ident_end(src: &[u8], i: usize, end: usize) -> usize {
    let mut j = i + identifier_char_len(src, i, end).unwrap_or(1);
    while let Some(len) = identifier_char_len(src, j, end) {
        j += len;
    }
    j
}

/// Skips whitespace and comments; returns the index of the next significant byte.
pub(crate) fn skip_ws_comments(src: &[u8], mut i: usize, end: usize) -> usize {
    loop {
        while i < end && is_ws(src[i]) {
            i += 1;
        }
        if at(src, i, end) == Some(b'/') && at(src, i + 1, end) == Some(b'/') {
            while i < end && src[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if at(src, i, end) == Some(b'/') && at(src, i + 1, end) == Some(b'*') {
            i = match find_subslice(src, b"*/", i + 2, end) {
                Some(e) => e + 2,
                None => end,
            };
            continue;
        }
        return i;
    }
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

pub(crate) fn line_end(src: &[u8], from: usize, end: usize) -> usize {
    match src[from..end].iter().position(|&b| b == b'\n') {
        Some(p) => from + p,
        None => end,
    }
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

/// Whether a slash after the preceding significant token begins a regular
/// expression literal. Both the lexer and balanced-region scanner use this
/// policy so delimiters inside regex literals never affect structural scans.
pub(crate) fn regex_allowed(prev_sig: u8, prev_word: &str) -> bool {
    if !prev_word.is_empty() {
        return matches!(
            prev_word,
            "return"
                | "typeof"
                | "instanceof"
                | "in"
                | "of"
                | "new"
                | "delete"
                | "void"
                | "throw"
                | "case"
                | "do"
                | "else"
                | "yield"
                | "await"
        );
    }
    prev_sig == 0 || b"(,=:[!&|?{};~+-*%^<>".contains(&prev_sig)
}

/// `src[i]` is a backtick — returns the index just past the closing backtick.
pub(crate) fn skip_template(src: &[u8], mut i: usize, end: usize) -> usize {
    i += 1;
    while i < end {
        match src[i] {
            b'\\' => i += 2,
            b'`' => return i + 1,
            b'$' if at(src, i + 1, end) == Some(b'{') => {
                i = match find_matching(src, i + 1, end) {
                    Some(close) => close + 1,
                    None => end,
                };
            }
            _ => i += 1,
        }
    }
    i.min(end)
}

/// `src[i]` is one of `( { [ <` — returns the index of the matching closer,
/// or None if unbalanced. Skips strings, templates and comments. When matching
/// `< >`, `=>` is skipped so arrow/function types don't miscount.
pub(crate) fn find_matching(src: &[u8], mut i: usize, end: usize) -> Option<usize> {
    let open = src[i];
    let close = match open {
        b'{' => b'}',
        b'(' => b')',
        b'[' => b']',
        b'<' => b'>',
        _ => return None,
    };
    let mut depth = 0usize;
    let mut prev_word = "";
    let mut prev_sig = 0;
    while i < end {
        let c = src[i];
        if c == b'/' && at(src, i + 1, end) == Some(b'/') {
            i = line_end(src, i, end);
            continue;
        }
        if c == b'/' && at(src, i + 1, end) == Some(b'*') {
            i = match find_subslice(src, b"*/", i + 2, end) {
                Some(e) => e + 2,
                None => end,
            };
            continue;
        }
        if c == b'"' || c == b'\'' {
            i = scan_string(src, i, end);
            prev_word = "";
            prev_sig = c;
            continue;
        }
        if c == b'`' {
            i = skip_template(src, i, end);
            prev_word = "";
            prev_sig = b'`';
            continue;
        }
        if c == b'/'
            && regex_allowed(prev_sig, prev_word)
            && let Some(regex_end) = scan_regex(src, i, end)
        {
            i = regex_end;
            prev_word = "";
            prev_sig = b'/';
            continue;
        }
        if starts_identifier(src, i, end) {
            let word_end = ident_end(src, i, end);
            prev_word = std::str::from_utf8(&src[i..word_end]).unwrap_or("");
            prev_sig = src[word_end - 1];
            i = word_end;
            continue;
        }
        if open == b'<' && c == b'=' && at(src, i + 1, end) == Some(b'>') {
            i += 2;
            prev_word = "";
            prev_sig = b'>';
            continue;
        }
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        prev_word = "";
        prev_sig = c;
        i += 1;
    }
    None
}

/// `src[i]` is `/` where a regex literal is allowed — returns the index just
/// past the literal (including flags), or None if it doesn't scan as a regex.
pub(crate) fn scan_regex(src: &[u8], mut i: usize, end: usize) -> Option<usize> {
    i += 1;
    let mut in_class = false;
    while i < end {
        match src[i] {
            b'\\' => i += 2,
            b'\n' => return None,
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

/// Scans a type annotation until a top-level `,` or closing bracket.
pub(crate) fn scan_type_end(src: &[u8], mut i: usize, end: usize) -> usize {
    let mut depth = 0usize;
    while i < end {
        let c = src[i];
        if c == b'/' && at(src, i + 1, end) == Some(b'/') {
            i = line_end(src, i, end);
            continue;
        }
        if c == b'/' && at(src, i + 1, end) == Some(b'*') {
            i = match find_subslice(src, b"*/", i + 2, end) {
                Some(e) => e + 2,
                None => end,
            };
            continue;
        }
        if c == b'"' || c == b'\'' {
            i = scan_string(src, i, end);
            continue;
        }
        if c == b'`' {
            i = skip_template(src, i, end);
            continue;
        }
        if c == b'=' && at(src, i + 1, end) == Some(b'>') {
            i += 2;
            continue;
        }
        match c {
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b')' | b']' | b'}' => {
                if depth == 0 {
                    return i;
                }
                depth -= 1;
            }
            b'>' => {
                depth = depth.saturating_sub(1);
            }
            b',' if depth == 0 => return i,
            _ => {}
        }
        i += 1;
    }
    i
}

/// True if the range contains an `await` token in code position (including
/// inside template interpolations, excluding strings and comments).
/// True when `src[i..end]` has a `,` outside every bracket, string,
/// template and comment — the one operator that binds looser than an
/// initializer, an assignment right-hand side, a `return` operand or a
/// single call argument, and so the one reason such a position has to keep
/// the parentheses codegen wrapped a value in.
///
/// A `,` inside type arguments (`a as Map<K, V>`) counts as top level here:
/// the answer is only ever used to *keep* parentheses, so erring that way
/// costs a pair of parentheses and never a meaning.
pub(crate) fn has_top_level_comma(src: &[u8], mut i: usize, end: usize) -> bool {
    let mut prev_word = "";
    let mut prev_sig = 0u8;
    while i < end {
        let c = src[i];
        if c == b'/' && at(src, i + 1, end) == Some(b'/') {
            i = line_end(src, i, end);
            continue;
        }
        if c == b'/' && at(src, i + 1, end) == Some(b'*') {
            i = match find_subslice(src, b"*/", i + 2, end) {
                Some(e) => e + 2,
                None => end,
            };
            continue;
        }
        if c == b'"' || c == b'\'' {
            i = scan_string(src, i, end);
            prev_word = "";
            prev_sig = c;
            continue;
        }
        if c == b'`' {
            i = skip_template(src, i, end);
            prev_word = "";
            prev_sig = b'`';
            continue;
        }
        if c == b'/'
            && regex_allowed(prev_sig, prev_word)
            && let Some(regex_end) = scan_regex(src, i, end)
        {
            i = regex_end;
            prev_word = "";
            prev_sig = b'/';
            continue;
        }
        if starts_identifier(src, i, end) {
            let word_end = ident_end(src, i, end);
            prev_word = std::str::from_utf8(&src[i..word_end]).unwrap_or("");
            prev_sig = src[word_end - 1];
            i = word_end;
            continue;
        }
        if matches!(c, b'(' | b'[' | b'{') {
            match find_matching(src, i, end) {
                Some(close) => {
                    i = close + 1;
                    prev_word = "";
                    prev_sig = src[close];
                    continue;
                }
                // Unbalanced: this pass cannot see the top level any more.
                None => return true,
            }
        }
        if c == b',' {
            return true;
        }
        prev_word = "";
        prev_sig = c;
        i += 1;
    }
    false
}

/// True when `src[i..end]` is one *primary* expression — a single operand
/// with nothing at its top level but member access, calls, indexing,
/// non-null assertions and tagged templates.
///
/// This is the question a receiver position asks: member access binds
/// tighter than every operator, so only a primary receiver can lose the
/// parentheses codegen wrapped it in. A keyword operand (`await x`,
/// `new C`, `x as T`) is not primary — `(await x).f` and `await x.f` are
/// different expressions.
pub(crate) fn is_primary_expression(src: &[u8], from: usize, end: usize) -> bool {
    let mut i = skip_ws_comments(src, from, end);
    if i >= end {
        return false;
    }
    let head = src[i];
    if starts_identifier(src, i, end) {
        let word_end = ident_end(src, i, end);
        let word = std::str::from_utf8(&src[i..word_end]).unwrap_or("");
        // An operand-taking keyword binds looser than member access.
        if matches!(
            word,
            "await"
                | "class"
                | "delete"
                | "function"
                | "new"
                | "typeof"
                | "void"
                | "yield"
                | "async"
        ) {
            return false;
        }
        i = word_end;
    } else if matches!(head, b'(' | b'[' | b'{') {
        match find_matching(src, i, end) {
            Some(close) => i = close + 1,
            None => return false,
        }
    } else if head == b'`' {
        i = skip_template(src, i, end);
    } else if head == b'"' || head == b'\'' {
        i = scan_string(src, i, end);
    } else if head.is_ascii_digit() {
        while i < end && (src[i].is_ascii_alphanumeric() || src[i] == b'.' || src[i] == b'_') {
            i += 1;
        }
    } else {
        return false;
    }
    loop {
        let next = skip_ws_comments(src, i, end);
        if next >= end {
            return true;
        }
        match src[next] {
            b'.' => match member_name_end(src, skip_ws_comments(src, next + 1, end), end) {
                Some(name_end) => i = name_end,
                None => return false,
            },
            b'?' if at(src, next + 1, end) == Some(b'.') => {
                let after = skip_ws_comments(src, next + 2, end);
                if let Some(name_end) = member_name_end(src, after, end) {
                    i = name_end;
                } else if after < end && matches!(src[after], b'(' | b'[') {
                    match find_matching(src, after, end) {
                        Some(close) => i = close + 1,
                        None => return false,
                    }
                } else {
                    return false;
                }
            }
            b'!' if at(src, next + 1, end) != Some(b'=') => i = next + 1,
            b'(' | b'[' => match find_matching(src, next, end) {
                Some(close) => i = close + 1,
                None => return false,
            },
            b'`' => i = skip_template(src, next, end),
            _ => return false,
        }
    }
}

/// The end of the member name at `i` after `.` or `?.`: an identifier, or a
/// private name (`#name`, ECMA-262 `PrivateIdentifier`), which is one token
/// with no gap after the `#`.
fn member_name_end(src: &[u8], i: usize, end: usize) -> Option<usize> {
    let name = if at(src, i, end) == Some(b'#') {
        i + 1
    } else {
        i
    };
    starts_identifier(src, name, end).then(|| ident_end(src, name, end))
}

pub(crate) fn contains_await(src: &[u8], mut i: usize, end: usize) -> bool {
    while i < end {
        let c = src[i];
        if c == b'/' && at(src, i + 1, end) == Some(b'/') {
            i = line_end(src, i, end);
            continue;
        }
        if c == b'/' && at(src, i + 1, end) == Some(b'*') {
            i = match find_subslice(src, b"*/", i + 2, end) {
                Some(e) => e + 2,
                None => end,
            };
            continue;
        }
        if c == b'"' || c == b'\'' {
            i = scan_string(src, i, end);
            continue;
        }
        if c == b'`' {
            i += 1;
            while i < end {
                match src[i] {
                    b'\\' => i += 2,
                    b'`' => {
                        i += 1;
                        break;
                    }
                    b'$' if at(src, i + 1, end) == Some(b'{') => {
                        let close = find_matching(src, i + 1, end).unwrap_or(end);
                        if contains_await(src, i + 2, close) {
                            return true;
                        }
                        i = (close + 1).min(end);
                    }
                    _ => i += 1,
                }
            }
            continue;
        }
        if starts_identifier(src, i, end) {
            let j = ident_end(src, i, end);
            if matches!(&src[i..j], b"function" | b"class") {
                let mut body = j;
                while body < end && src[body] != b'{' {
                    body += 1;
                }
                if body < end {
                    i = find_matching(src, body, end).map_or(end, |close| close + 1);
                    continue;
                }
            }
            if &src[i..j] == b"await" {
                return true;
            }
            i = j;
            continue;
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{contains_await, ident_end, identifier_char_len, starts_identifier};

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

    #[test]
    fn await_scan_stops_at_nested_function_and_class_bodies() {
        let nested =
            b"function nested() { await later(); } class C { async m() { await later(); } }";
        assert!(!contains_await(nested, 0, nested.len()));

        let outer = b"await now(); function nested() { await later(); }";
        assert!(contains_await(outer, 0, outer.len()));
    }
}
