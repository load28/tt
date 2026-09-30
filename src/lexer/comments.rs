use super::{Token, TokenKind, TplPart};
use crate::ast::Span;
use crate::scanner::{block_comment_end, line_end, skip_space};

pub(crate) fn comments(src: &str, tokens: &[Token]) -> Vec<Span> {
    let mut found = Vec::new();
    collect(src, tokens, 0, src.len(), &mut found);
    found
}

fn collect(src: &str, tokens: &[Token], start: usize, end: usize, found: &mut Vec<Span>) {
    let mut gap = start;
    for token in tokens {
        in_gap(src, gap, token.span.start, found);
        if let TokenKind::Template(parts) = &token.kind {
            for part in parts {
                if let TplPart::Interp { span, tokens } = part {
                    collect(src, tokens, span.start, span.end, found);
                }
            }
        }
        gap = token.span.end;
    }
    in_gap(src, gap, end, found);
}

fn in_gap(src: &str, start: usize, end: usize, found: &mut Vec<Span>) {
    let bytes = src.as_bytes();
    let mut at = start;
    while at < end {
        at = skip_space(bytes, at, end, true);
        let close = match (bytes.get(at), bytes.get(at + 1)) {
            (Some(b'/'), Some(b'/')) if at + 1 < end => line_end(bytes, at, end),
            (Some(b'/'), Some(b'*')) if at + 1 < end => block_comment_end(bytes, at, end),
            _ => return,
        };
        found.push(Span {
            start: at,
            end: close,
        });
        at = close;
    }
}

pub(crate) fn directive_governed_lines(src: &str, comments: &[Span]) -> Vec<Span> {
    let bytes = src.as_bytes();
    let mut governed: Vec<Span> = comments
        .iter()
        .filter(|comment| is_directive(bytes, **comment))
        .filter_map(|comment| {
            let mut line = next_line(bytes, comment.end)?;
            loop {
                let end = line_end(bytes, line, bytes.len());
                let text = src[line..end].trim_matches(|c: char| c.is_ascii_whitespace());
                if !text.is_empty() && !text.starts_with("//") {
                    return Some(Span { start: line, end });
                }
                line = next_line(bytes, end)?;
            }
        })
        .collect();
    governed.dedup();
    governed
}

fn next_line(bytes: &[u8], at: usize) -> Option<usize> {
    let end = line_end(bytes, at, bytes.len());
    crate::scanner::line_break_end(bytes, end, bytes.len())
}

fn is_directive(bytes: &[u8], comment: Span) -> bool {
    let text = &bytes[comment.start..comment.end];
    let rest = if let Some(line) = text.strip_prefix(b"//") {
        line.strip_prefix(b"/").unwrap_or(line)
    } else {
        let last_line = text
            .iter()
            .rposition(|&byte| byte == b'\n' || byte == b'\r')
            .map_or(text, |at| &text[at + 1..]);
        let slashes = last_line
            .iter()
            .position(|&byte| byte != b'/' && byte != b'*')
            .unwrap_or(last_line.len());
        &last_line[slashes..]
    };
    let rest = &rest[rest
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(rest.len())..];
    rest.starts_with(b"@ts-expect-error") || rest.starts_with(b"@ts-ignore")
}

pub(crate) fn leading_documentation(
    src: &str,
    comments: &[Span],
    governed: &[Span],
    statement: usize,
) -> Option<Span> {
    let bytes = src.as_bytes();
    if governed
        .iter()
        .any(|line| line.start <= statement && statement <= line.end)
    {
        return None;
    }
    let mut first = None;
    let mut next = statement;
    for comment in comments[..comments.partition_point(|comment| comment.end <= statement)]
        .iter()
        .rev()
    {
        if skip_space(bytes, comment.end, next, true) != next {
            break;
        }
        let line = crate::lines::line_start_before(src, comment.start);
        if skip_space(bytes, line, comment.start, false) != comment.start {
            break;
        }
        first = Some(*comment);
        next = comment.start;
    }
    let first = first?;
    let run = &comments[comments.partition_point(|comment| comment.start < first.start)
        ..comments.partition_point(|comment| comment.end <= statement)];
    let documentation = run.iter().find(|comment| {
        let text = &bytes[comment.start..comment.end];
        text.starts_with(b"/**") && !text.starts_with(b"/**/")
    })?;
    Some(Span {
        start: documentation.start,
        end: statement,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourceKind;
    use crate::lexer::lex_with_kind;

    fn read(src: &str) -> (Vec<Span>, Vec<Span>) {
        let tokens = lex_with_kind(src, 0, src.len(), SourceKind::TypeScript);
        let comments = comments(src, &tokens);
        let governed = directive_governed_lines(src, &comments);
        (comments, governed)
    }

    fn lines<'a>(src: &'a str, spans: &[Span]) -> Vec<&'a str> {
        spans
            .iter()
            .map(|span| &src[span.start..span.end])
            .collect()
    }

    #[test]
    fn comments_are_read_between_tokens_only() {
        let src = "const a = \"//x\"; // one\nconst t = `${/* two */ b} // no`;\n/* three */\n";
        let (comments, _) = read(src);
        assert_eq!(
            lines(src, &comments),
            ["// one", "/* two */", "/* three */"]
        );
    }

    #[test]
    fn a_directive_governs_the_next_line_that_is_not_blank_or_a_line_comment() {
        let src = "// @ts-expect-error\n\n// note\nconst a: number = \"\";\n/* @ts-ignore */\nb();\n// @ts-nocheck\nc();\n  //   @ts-ignore: why\n  d();\n";
        let (_, governed) = read(src);
        assert_eq!(
            lines(src, &governed),
            ["const a: number = \"\";", "b();", "  d();"]
        );
    }

    #[test]
    fn a_block_directive_counts_from_its_last_line() {
        let src = "/*\n @ts-ignore */\na();\n/** @ts-ignore\n * text */\nb();\n";
        let (_, governed) = read(src);
        assert_eq!(lines(src, &governed), ["a();"]);
    }

    #[test]
    fn documentation_runs_from_the_first_jsdoc_to_the_statement() {
        let src = "x(); /** trailing */\n// plain\n/** doc */\n// more\nexport const a = 1;\n";
        let (comments, governed) = read(src);
        let statement = src.find("export").unwrap();
        let span = leading_documentation(src, &comments, &governed, statement).unwrap();
        assert_eq!(&src[span.start..span.end], "/** doc */\n// more\n");
        let plain = "// plain\nconst b = 1;\n";
        let (comments, governed) = read(plain);
        assert_eq!(leading_documentation(plain, &comments, &governed, 9), None);
    }

    #[test]
    fn a_governed_statement_keeps_its_comments_in_place() {
        let src = "/** doc */\n// @ts-ignore\nconst a = 1;\n";
        let (comments, governed) = read(src);
        let statement = src.find("const").unwrap();
        assert_eq!(
            leading_documentation(src, &comments, &governed, statement),
            None
        );
    }
}
