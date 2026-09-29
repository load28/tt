use std::collections::HashSet;

use super::{Token, TokenKind, TplPart, lex_with_kind};
use crate::SourceKind;
use crate::scanner::{is_ident_char, is_ident_start};

pub(crate) fn identifier_names_with_prefix(
    src: &str,
    source_kind: SourceKind,
    prefix: &str,
) -> HashSet<String> {
    let mut names = HashSet::new();
    if !src.contains(prefix) && !src.contains('\\') {
        return names;
    }
    let tokens = lex_with_kind(src, 0, src.len(), source_kind);
    collect(src, &tokens, prefix, &mut names);
    names
}

fn collect(src: &str, tokens: &[Token], prefix: &str, names: &mut HashSet<String>) {
    let mut covered = 0;
    for token in tokens {
        match &token.kind {
            TokenKind::Template(parts) => {
                for part in parts.iter() {
                    if let TplPart::Interp { tokens, .. } = part {
                        collect(src, tokens, prefix, names);
                    }
                }
            }
            TokenKind::JsxRaw => {
                let bytes = src.as_bytes();
                let mut i = token.span.start;
                while i < token.span.end {
                    let starts_name = (is_ident_start(bytes[i]) || bytes[i] == b'\\')
                        && (i == token.span.start || !is_ident_char(bytes[i - 1]));
                    match starts_name
                        .then(|| identifier_name(src, i, token.span.end))
                        .flatten()
                    {
                        Some((name, end)) => {
                            insert(names, prefix, name);
                            i = end;
                        }
                        None => i += 1,
                    }
                }
            }
            TokenKind::Ident | TokenKind::Punct(b'\\') if token.span.start >= covered => {
                if let Some((name, end)) = identifier_name(src, token.span.start, src.len()) {
                    covered = end;
                    insert(names, prefix, name);
                }
            }
            _ => {}
        }
    }
}

fn insert(names: &mut HashSet<String>, prefix: &str, name: String) {
    if name.starts_with(prefix) {
        names.insert(name);
    }
}

fn identifier_name(src: &str, start: usize, end: usize) -> Option<(String, usize)> {
    let bytes = src.as_bytes();
    let mut name = String::new();
    let mut i = start;
    while i < end {
        if is_ident_char(bytes[i]) {
            if name.is_empty() && !is_ident_start(bytes[i]) {
                return None;
            }
            name.push(char::from(bytes[i]));
            i += 1;
        } else if bytes[i] == b'\\' {
            let (character, next) = unicode_escape(src, i, end)?;
            name.push(character);
            i = next;
        } else {
            break;
        }
    }
    (!name.is_empty()).then_some((name, i))
}

fn unicode_escape(src: &str, start: usize, end: usize) -> Option<(char, usize)> {
    let bytes = src.as_bytes();
    if bytes.get(start + 1) != Some(&b'u') {
        return None;
    }
    let (digits, next) = if bytes.get(start + 2) == Some(&b'{') {
        let close = (start + 3..end).find(|&i| bytes[i] == b'}')?;
        (&src[start + 3..close], close + 1)
    } else {
        let close = start + 6;
        (src.get(start + 2..close)?, close)
    };
    if next > end || digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    Some((char::from_u32(value)?, next))
}
