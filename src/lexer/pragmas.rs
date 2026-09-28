#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeadingComment {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) pragma: Option<FilePragma>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilePragma {
    TripleSlashDirective,
    TypeCheck,
    Jsx,
}

pub(crate) fn leading_comments(src: &str, mut at: usize) -> Vec<LeadingComment> {
    let bytes = src.as_bytes();
    let mut comments = Vec::new();
    loop {
        while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
            at += 1;
        }
        let (end, pragma) = match (bytes.get(at), bytes.get(at + 1)) {
            (Some(b'/'), Some(b'/')) => {
                let end = bytes[at..]
                    .iter()
                    .position(|&b| b == b'\n' || b == b'\r')
                    .map_or(bytes.len(), |line_end| at + line_end);
                (end, line_pragma(&bytes[at..end]))
            }
            (Some(b'/'), Some(b'*')) => {
                let end = bytes[at + 2..]
                    .windows(2)
                    .position(|w| w == b"*/")
                    .map_or(bytes.len(), |close| at + 2 + close + 2);
                (end, block_pragma(&bytes[at..end]))
            }
            _ => return comments,
        };
        comments.push(LeadingComment {
            start: at,
            end,
            pragma,
        });
        at = end;
    }
}

pub(crate) fn after_file_pragmas(src: &str, at: usize) -> usize {
    let bytes = src.as_bytes();
    leading_comments(src, at)
        .iter()
        .rev()
        .find(|comment| comment.pragma.is_some())
        .map_or(at, |pragma| {
            let mut end = pragma.end;
            while matches!(bytes.get(end), Some(b' ' | b'\t' | b'\r')) {
                end += 1;
            }
            match bytes.get(end) {
                Some(b'\n') => end + 1,
                _ => pragma.end,
            }
        })
}

fn line_pragma(comment: &[u8]) -> Option<FilePragma> {
    let body = comment.strip_prefix(b"//")?;
    if let Some(directive) = body.strip_prefix(b"/") {
        let tag = trim_start(directive).strip_prefix(b"<")?;
        return ["reference", "amd-module", "amd-dependency"]
            .iter()
            .any(|name| names(tag, name))
            .then_some(FilePragma::TripleSlashDirective);
    }
    let name = trim_start(body).strip_prefix(b"@")?;
    ["ts-check", "ts-nocheck"]
        .iter()
        .any(|pragma| names(name, pragma))
        .then_some(FilePragma::TypeCheck)
}

fn block_pragma(comment: &[u8]) -> Option<FilePragma> {
    comment
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == b'@')
        .any(|(at, _)| {
            ["jsx", "jsxfrag", "jsximportsource", "jsxruntime"]
                .iter()
                .any(|pragma| names(&comment[at + 1..], pragma))
        })
        .then_some(FilePragma::Jsx)
}

fn names(text: &[u8], name: &str) -> bool {
    text.len() >= name.len()
        && text[..name.len()].eq_ignore_ascii_case(name.as_bytes())
        && text
            .get(name.len())
            .is_none_or(|b| b.is_ascii_whitespace() || matches!(b, b'/' | b'*'))
}

fn trim_start(text: &[u8]) -> &[u8] {
    let skip = text
        .iter()
        .position(|b| !matches!(b, b' ' | b'\t'))
        .unwrap_or(text.len());
    &text[skip..]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<Option<FilePragma>> {
        leading_comments(src, 0)
            .into_iter()
            .map(|comment| comment.pragma)
            .collect()
    }

    #[test]
    fn documented_file_pragmas_are_recognized() {
        assert_eq!(
            kinds(
                "/// <reference path=\"a.d.ts\" />\n///<amd-module name=\"m\"/>\n/// <amd-dependency path=\"x\" />\n// @ts-nocheck\n//@ts-check\n/** @jsx h */\n/* @jsxImportSource preact */\nlet x;"
            ),
            [
                Some(FilePragma::TripleSlashDirective),
                Some(FilePragma::TripleSlashDirective),
                Some(FilePragma::TripleSlashDirective),
                Some(FilePragma::TypeCheck),
                Some(FilePragma::TypeCheck),
                Some(FilePragma::Jsx),
                Some(FilePragma::Jsx),
            ]
        );
    }

    #[test]
    fn statement_comments_are_not_pragmas() {
        assert_eq!(
            kinds(
                "// @ts-expect-error\n// @ts-ignore\n/** Docs. */\n/// plain note\n// @ts-checked\n/* @jsxs */\nlet x;"
            ),
            [None, None, None, None, None, None]
        );
    }

    #[test]
    fn text_goes_after_the_last_pragma_line() {
        let src = "// license\n/// <reference types=\"node\" />\n// @ts-expect-error\nlet x;";
        assert_eq!(
            &src[after_file_pragmas(src, 0)..],
            "// @ts-expect-error\nlet x;"
        );
        let src = "/** Docs. */\nlet x;";
        assert_eq!(after_file_pragmas(src, 0), 0);
        let src = "\r\n// @ts-nocheck\r\nlet x;";
        assert_eq!(&src[after_file_pragmas(src, 0)..], "let x;");
    }
}
