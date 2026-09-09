//! Low-level byte scanning over tt/TypeScript source.
//!
//! All scanning is byte-based: every character the scanner makes decisions on
//! is ASCII, and UTF-8 continuation bytes (0x80+) never compare equal to any
//! ASCII byte, so multi-byte characters pass through opaquely.

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

/// Reads the identifier starting at `i`; returns the end index (exclusive).
pub(crate) fn ident_end(src: &[u8], i: usize, end: usize) -> usize {
    let mut j = i + 1;
    while j < end && is_ident_char(src[j]) {
        j += 1;
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
        if is_ident_start(c) {
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
        if is_ident_start(c) {
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
    if is_ident_start(head) {
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
            b'.' => {
                let name = skip_ws_comments(src, next + 1, end);
                if name < end && is_ident_start(src[name]) {
                    i = ident_end(src, name, end);
                } else {
                    return false;
                }
            }
            b'?' if at(src, next + 1, end) == Some(b'.') => {
                let after = skip_ws_comments(src, next + 2, end);
                if after < end && is_ident_start(src[after]) {
                    i = ident_end(src, after, end);
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
        if is_ident_start(c) {
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
    use super::contains_await;

    #[test]
    fn await_scan_stops_at_nested_function_and_class_bodies() {
        let nested =
            b"function nested() { await later(); } class C { async m() { await later(); } }";
        assert!(!contains_await(nested, 0, nested.len()));

        let outer = b"await now(); function nested() { await later(); }";
        assert!(contains_await(outer, 0, outer.len()));
    }
}
