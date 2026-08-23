//! Tokenization of rl/TypeScript source — the layer between the byte
//! scanner and the parser, in the spirit of swc's lexer.
//!
//! [`lex`] turns a byte range into a stream of *significant* tokens:
//! whitespace and comments are trivia and produce no tokens (verbatim
//! emission copies original bytes, so trivia never needs representing).
//! Every token carries its absolute byte [`Span`], and the whole stream is
//! ordered and non-overlapping, so any construct the parser lifts maps
//! back to an exact byte range of the source.
//!
//! Like the previous byte scan loop, the lexer decides regex-vs-division
//! with the preceding-token heuristic, and template literals are lexed
//! hierarchically: a [`TokenKind::Template`] token carries its raw chunks
//! and the pre-lexed token stream of every `${ }` interpolation.
//!
//! The only multi-byte operators fused into single tokens are the five the
//! parser must treat as units: `=>` (never an `=` or a `< >` bracket),
//! `||` (never an or-pattern separator), `?.`/`??` (never ternary
//! openers), and `|>` (the pipeline operator — never a union `|` followed
//! by a comparison, because that byte sequence cannot occur in valid
//! TypeScript). Everything else significant is a one-byte
//! [`TokenKind::Punct`].

use crate::SourceKind;
use crate::ast::Span;
use crate::scanner::*;

/// One significant token.
#[derive(Debug)]
pub(crate) struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// What a [`Token`] is. Only the distinctions the parser consumes exist;
/// everything else is a single-byte `Punct`.
#[derive(Debug)]
pub(crate) enum TokenKind {
    /// ASCII identifier or keyword (`[A-Za-z_$][A-Za-z0-9_$]*`).
    Ident,
    /// `'...'` / `"..."` string literal (possibly unterminated at a
    /// newline or EOF, exactly as the byte scanner tolerates).
    Str,
    /// A template literal with its interpolations pre-lexed. Boxed so the
    /// overwhelmingly common one-byte `Punct` keeps [`Token`] small — the
    /// token stream of a large file is the compiler's biggest allocation.
    Template(Box<[TplPart]>),
    /// A regex literal, decided by the preceding-token heuristic.
    Regex,
    /// A raw JSX run. Its bytes are opaque to the rl parser; JSX expression
    /// containers are lexed recursively and appear as ordinary tokens between
    /// these runs.
    JsxRaw,
    /// `=>`
    Arrow,
    /// `||`
    OrOr,
    /// `?.`
    OptChain,
    /// `??`
    Coalesce,
    /// `|>`
    PipeOp,
    /// Any other significant byte.
    Punct(u8),
}

/// One piece of a [`TokenKind::Template`], in source order. `Raw` spans
/// include the surrounding backticks and the literal text (matching
/// [`crate::ast::TemplateChunk::Raw`]); an interpolation's span excludes
/// its `${` and `}` delimiters.
#[derive(Debug)]
pub(crate) enum TplPart {
    Raw(Span),
    Interp { span: Span, tokens: Vec<Token> },
}

/// Lexes `src[start..end]` into significant tokens.
pub(crate) fn lex(src_str: &str, start: usize, end: usize) -> Vec<Token> {
    lex_with_kind(src_str, start, end, SourceKind::TypeScript)
}

/// Lexes a source range under its TypeScript surface kind.
pub(crate) fn lex_with_kind(
    src_str: &str,
    start: usize,
    end: usize,
    source_kind: SourceKind,
) -> Vec<Token> {
    let src = src_str.as_bytes();
    // Significant tokens run about one per six source bytes across real
    // TypeScript, so sizing up front spares the repeated doubling that
    // dominated lexing on large files.
    let mut tokens: Vec<Token> = Vec::with_capacity((end - start) / 6 + 8);
    let mut i = start;
    // Regex-heuristic state, same rules as the previous scan loop: the last
    // identifier scanned, or the last significant byte otherwise.
    let mut prev_word: &str = "";
    let mut prev_sig: u8 = 0;

    let span = |start: usize, end: usize| Span { start, end };

    while i < end {
        let c = src[i];

        if is_ws(c) {
            i += 1;
            continue;
        }

        // comments — trivia
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
            let e = scan_string(src, i, end);
            tokens.push(Token {
                kind: TokenKind::Str,
                span: span(i, e),
            });
            prev_sig = c;
            prev_word = "";
            i = e;
            continue;
        }

        if c == b'`' {
            let (e, parts) = lex_template(src_str, i, end, source_kind);
            tokens.push(Token {
                kind: TokenKind::Template(parts.into_boxed_slice()),
                span: span(i, e),
            });
            prev_sig = b'`';
            prev_word = "";
            i = e;
            continue;
        }

        if source_kind.is_tsx()
            && c == b'<'
            && jsx_allowed(prev_sig, prev_word)
            && let Some(jsx) = scan_jsx(src_str, i, end, source_kind)
        {
            tokens.extend(jsx.tokens);
            prev_sig = b'>';
            prev_word = "";
            i = jsx.end;
            continue;
        }

        if c == b'/'
            && regex_allowed(prev_sig, prev_word)
            && let Some(e) = scan_regex(src, i, end)
        {
            tokens.push(Token {
                kind: TokenKind::Regex,
                span: span(i, e),
            });
            prev_sig = b'/';
            prev_word = "";
            i = e;
            continue;
        }

        if is_ident_start(c) {
            let j = ident_end(src, i, end);
            tokens.push(Token {
                kind: TokenKind::Ident,
                span: span(i, j),
            });
            prev_word = &src_str[i..j];
            prev_sig = src[j - 1];
            i = j;
            continue;
        }

        let (kind, len) = match (c, at(src, i + 1, end)) {
            (b'=', Some(b'>')) => (TokenKind::Arrow, 2),
            (b'|', Some(b'|')) => (TokenKind::OrOr, 2),
            (b'|', Some(b'>')) => (TokenKind::PipeOp, 2),
            (b'?', Some(b'.')) => (TokenKind::OptChain, 2),
            (b'?', Some(b'?')) => (TokenKind::Coalesce, 2),
            _ => (TokenKind::Punct(c), 1),
        };
        tokens.push(Token {
            kind,
            span: span(i, i + len),
        });
        prev_sig = src[i + len - 1];
        prev_word = "";
        i += len;
    }
    tokens
}

/// `src[start]` is a backtick — lexes the template into raw chunks and
/// recursively lexed `${ }` interpolations. Returns the index just past
/// the closing backtick (or `end` if unterminated).
fn lex_template(
    src_str: &str,
    start: usize,
    end: usize,
    source_kind: SourceKind,
) -> (usize, Vec<TplPart>) {
    let src = src_str.as_bytes();
    let mut parts: Vec<TplPart> = Vec::new();
    let mut raw_start = start; // includes the opening backtick
    let push_raw = |parts: &mut Vec<TplPart>, start: usize, end: usize| {
        if start < end {
            parts.push(TplPart::Raw(Span { start, end }));
        }
    };
    let mut i = start + 1;
    while i < end {
        let c = src[i];
        if c == b'\\' {
            i = (i + 2).min(end);
            continue;
        }
        if c == b'`' {
            i += 1;
            push_raw(&mut parts, raw_start, i);
            return (i, parts);
        }
        if c == b'$' && at(src, i + 1, end) == Some(b'{') {
            push_raw(&mut parts, raw_start, i);
            let close = find_matching(src, i + 1, end).unwrap_or(end);
            parts.push(TplPart::Interp {
                span: Span {
                    start: i + 2,
                    end: close,
                },
                tokens: lex_with_kind(src_str, i + 2, close, source_kind),
            });
            i = (close + 1).min(end);
            raw_start = i;
            continue;
        }
        i += 1;
    }
    push_raw(&mut parts, raw_start, end);
    (end, parts)
}

fn jsx_allowed(prev_sig: u8, prev_word: &str) -> bool {
    if prev_sig == 0 {
        return true;
    }
    if matches!(prev_word, "return" | "throw" | "yield" | "await" | "case") {
        return true;
    }
    b"([{,:;=!?&|+-*%^~>".contains(&prev_sig)
}

struct ScannedJsx {
    end: usize,
    tokens: Vec<Token>,
}

/// Parses one complete JSX element or fragment and exposes only its
/// JavaScript expression containers to the rl lexer. Every tag, attribute,
/// and text run becomes an opaque token, so words in JSX text can never be
/// claimed as rl syntax.
fn scan_jsx(
    src_str: &str,
    start: usize,
    end: usize,
    source_kind: SourceKind,
) -> Option<ScannedJsx> {
    let src = src_str.as_bytes();
    let opening = scan_jsx_opening(src, start, end)?;
    let mut tokens = jsx_region_tokens(
        src_str,
        start,
        opening.end,
        &opening.expressions,
        source_kind,
    );
    let mut i = opening.end;
    if opening.self_closing {
        return Some(ScannedJsx { end: i, tokens });
    }
    let mut raw_start = i;

    loop {
        if i >= end {
            return None;
        }
        if src[i] == b'<' && at(src, i + 1, end) == Some(b'/') {
            let close_end = scan_jsx_closing(src, i, end, opening.name.as_deref())?;
            tokens.push(Token {
                kind: TokenKind::JsxRaw,
                span: Span {
                    start: raw_start,
                    end: close_end,
                },
            });
            return Some(ScannedJsx {
                end: close_end,
                tokens,
            });
        }
        if src[i] == b'<' {
            let child = scan_jsx(src_str, i, end, source_kind)?;
            if raw_start < i {
                tokens.push(Token {
                    kind: TokenKind::JsxRaw,
                    span: Span {
                        start: raw_start,
                        end: i,
                    },
                });
            }
            tokens.extend(child.tokens);
            i = child.end;
            raw_start = i;
            continue;
        }
        if src[i] == b'{' {
            let close = find_matching(src, i, end)?;
            if raw_start < i + 1 {
                tokens.push(Token {
                    kind: TokenKind::JsxRaw,
                    span: Span {
                        start: raw_start,
                        end: i + 1,
                    },
                });
            }
            tokens.extend(lex_with_kind(src_str, i + 1, close, source_kind));
            tokens.push(Token {
                kind: TokenKind::JsxRaw,
                span: Span {
                    start: close,
                    end: close + 1,
                },
            });
            i = close + 1;
            raw_start = i;
            continue;
        }
        i += 1;
    }
}

fn jsx_region_tokens(
    src_str: &str,
    start: usize,
    end: usize,
    expressions: &[(usize, usize)],
    source_kind: SourceKind,
) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut raw_start = start;
    for &(open, close) in expressions {
        tokens.push(Token {
            kind: TokenKind::JsxRaw,
            span: Span {
                start: raw_start,
                end: open + 1,
            },
        });
        tokens.extend(lex_with_kind(src_str, open + 1, close, source_kind));
        tokens.push(Token {
            kind: TokenKind::JsxRaw,
            span: Span {
                start: close,
                end: close + 1,
            },
        });
        raw_start = close + 1;
    }
    if raw_start < end {
        tokens.push(Token {
            kind: TokenKind::JsxRaw,
            span: Span {
                start: raw_start,
                end,
            },
        });
    }
    tokens
}

struct JsxOpening {
    end: usize,
    name: Option<String>,
    self_closing: bool,
    expressions: Vec<(usize, usize)>,
}

/// Returns `(end, opening_name, self_closing)`. `None` means the `<` starts
/// ordinary TypeScript syntax, not a complete JSX opening construct.
fn scan_jsx_opening(src: &[u8], start: usize, end: usize) -> Option<JsxOpening> {
    let mut i = start + 1;
    if at(src, i, end) == Some(b'>') {
        return Some(JsxOpening {
            end: i + 1,
            name: None,
            self_closing: false,
            expressions: Vec::new(),
        });
    }
    let name_start = i;
    i = scan_jsx_name(src, i, end)?;
    let name = String::from_utf8(src[name_start..i].to_vec()).ok()?;
    let mut expressions = Vec::new();
    if at(src, i, end) == Some(b'<') {
        i = find_matching(src, i, end)? + 1;
    }
    loop {
        while i < end && is_ws(src[i]) {
            i += 1;
        }
        match (at(src, i, end), at(src, i + 1, end)) {
            (Some(b'/'), Some(b'>')) => {
                return Some(JsxOpening {
                    end: i + 2,
                    name: Some(name),
                    self_closing: true,
                    expressions,
                });
            }
            (Some(b'>'), _) => {
                return Some(JsxOpening {
                    end: i + 1,
                    name: Some(name),
                    self_closing: false,
                    expressions,
                });
            }
            (Some(b'{'), _) => {
                let close = find_matching(src, i, end)?;
                expressions.push((i, close));
                i = close + 1;
            }
            (Some(b), _) if b == b'"' || b == b'\'' => i = scan_string(src, i, end),
            (Some(b), _) if is_jsx_name_start(b) => {
                i = scan_jsx_name(src, i, end)?;
                while i < end && is_ws(src[i]) {
                    i += 1;
                }
                if at(src, i, end) == Some(b'=') {
                    i += 1;
                    while i < end && is_ws(src[i]) {
                        i += 1;
                    }
                    match at(src, i, end)? {
                        b'"' | b'\'' => i = scan_string(src, i, end),
                        b'{' => {
                            let close = find_matching(src, i, end)?;
                            expressions.push((i, close));
                            i = close + 1;
                        }
                        _ => return None,
                    }
                }
            }
            _ => return None,
        }
    }
}

fn scan_jsx_closing(src: &[u8], start: usize, end: usize, opening: Option<&str>) -> Option<usize> {
    let mut i = start + 2;
    match opening {
        None => {
            if at(src, i, end) != Some(b'>') {
                return None;
            }
            Some(i + 1)
        }
        Some(opening) => {
            let name_start = i;
            i = scan_jsx_name(src, i, end)?;
            if &src[name_start..i] != opening.as_bytes() {
                return None;
            }
            while i < end && is_ws(src[i]) {
                i += 1;
            }
            (at(src, i, end) == Some(b'>')).then_some(i + 1)
        }
    }
}

fn is_jsx_name_start(b: u8) -> bool {
    is_ident_start(b) || b >= 0x80
}

fn is_jsx_name_char(b: u8) -> bool {
    is_ident_char(b) || matches!(b, b'-' | b'.' | b':') || b >= 0x80
}

fn scan_jsx_name(src: &[u8], mut i: usize, end: usize) -> Option<usize> {
    if !is_jsx_name_start(at(src, i, end)?) {
        return None;
    }
    i += 1;
    while i < end && is_jsx_name_char(src[i]) {
        i += 1;
    }
    Some(i)
}
