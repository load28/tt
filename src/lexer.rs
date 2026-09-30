//! Tokenization of tt/TypeScript source — the layer between the byte
//! scanner and the parser, in the spirit of swc's lexer.
//!
//! [`lex`] turns a byte range into a stream of *significant* tokens:
//! whitespace and comments are trivia and produce no tokens (verbatim
//! emission copies original bytes, so trivia never needs representing).
//! Every token carries its absolute byte [`Span`], and the whole stream is
//! ordered and non-overlapping, so any construct the parser lifts maps
//! back to an exact byte range of the source.
//!
//! The lexer drives the token facts machine ([`facts`]) as it goes: the
//! machine's grammar position decides regex-vs-division and whether a `<`
//! opens JSX, and every token carries the [`TokenFacts`] the machine
//! recorded for it — line terminators before it, automatic semicolons,
//! statement starts. Template literals are lexed hierarchically: a
//! [`TokenKind::Template`] token carries its raw chunks and the pre-lexed
//! token stream of every `${ }` interpolation.
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

mod comments;
mod facts;
mod names;
pub(crate) mod pragmas;
mod queries;
mod validation;
pub(crate) use comments::{comments, directive_governed_lines, leading_documentation};
pub(crate) use facts::{TokenFacts, statement_only_keyword};
pub(crate) use names::identifier_names_with_prefix;
pub(crate) use queries::{
    AutomaticSemicolon, automatic_semicolons, contains_await, continues_statement,
    has_top_level_comma, is_primary_expression, statement_continues_after, type_parameter_names,
};
pub(crate) use validation::{host_syntax_check, host_syntax_error, host_syntax_error_in};

/// One significant token.
#[derive(Debug)]
pub(crate) struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub facts: TokenFacts,
}

impl Token {
    /// Whether this token opens a bracket pair: `(`, `[`, `{`, or a `<`
    /// that opens type arguments or parameters
    /// ([`TokenFacts::opens_type_arguments`]). Every walk that balances
    /// brackets asks this, so a `,` inside `f<A, B>` is never top-level.
    pub(crate) fn opens_bracket(&self) -> bool {
        match self.kind {
            TokenKind::Punct(b'(' | b'[' | b'{') => true,
            TokenKind::Punct(b'<') => self.facts.opens_type_arguments(),
            _ => false,
        }
    }

    /// Whether this token closes a bracket pair: `)`, `]`, `}`, or the `>`
    /// that closes type arguments or parameters.
    pub(crate) fn closes_bracket(&self) -> bool {
        match self.kind {
            TokenKind::Punct(b')' | b']' | b'}') => true,
            TokenKind::Punct(b'>') => self.facts.closes_type_arguments(),
            _ => false,
        }
    }
}

/// Whether `tokens[k]` is a statement-only keyword where it stands: an
/// identifier spelling one ([`statement_only_keyword`]) that is not in a
/// type position. After an `as` assertion (`x as const`) or a `<` that
/// opens type parameters (`<const T>`), `const` is part of the type and
/// continues the expression instead of ending it.
pub(crate) fn statement_keyword_at(src: &str, tokens: &[Token], k: usize) -> bool {
    let token = &tokens[k];
    if !matches!(token.kind, TokenKind::Ident)
        || !statement_only_keyword(&src[token.span.start..token.span.end])
    {
        return false;
    }
    let Some(previous) = k.checked_sub(1).map(|p| &tokens[p]) else {
        return true;
    };
    let in_type = match previous.kind {
        TokenKind::Ident => &src[previous.span.start..previous.span.end] == "as",
        TokenKind::Punct(b'<') => previous.facts.opens_type_arguments(),
        _ => false,
    };
    !in_type
}

/// What a [`Token`] is. Only the distinctions the parser consumes exist;
/// everything else is a single-byte `Punct`.
#[derive(Debug)]
pub(crate) enum TokenKind {
    /// Identifier or keyword: ASCII `[A-Za-z_$][A-Za-z0-9_$]*` characters
    /// and every non-ASCII code point that is not ECMA-262 white space or a
    /// line terminator.
    Ident,
    /// `'...'` / `"..."` string literal (possibly unterminated at a
    /// newline or EOF, exactly as the byte scanner tolerates).
    Str,
    /// A template literal with its interpolations pre-lexed. Boxed so the
    /// overwhelmingly common one-byte `Punct` keeps [`Token`] small — the
    /// token stream of a large file is the compiler's biggest allocation.
    Template(Box<[TplPart]>),
    /// A regex literal: a `/` where the grammar expects an operand.
    Regex,
    /// A raw JSX run. Its bytes are opaque to the tt parser; JSX expression
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

pub(crate) enum TypeScriptTokens<'a> {
    Shared(&'a [Token]),
    Lexed(Vec<Token>),
}

impl<'a> TypeScriptTokens<'a> {
    pub(crate) fn of(src: &str, kind: SourceKind, tokens: &'a [Token]) -> Self {
        if kind == SourceKind::TypeScript {
            TypeScriptTokens::Shared(tokens)
        } else {
            TypeScriptTokens::Lexed(lex(src, 0, src.len()))
        }
    }

    pub(crate) fn tokens(&self) -> &[Token] {
        match self {
            TypeScriptTokens::Shared(tokens) => tokens,
            TypeScriptTokens::Lexed(tokens) => tokens,
        }
    }
}

/// Lexes a source range under its TypeScript surface kind.
pub(crate) fn lex_with_kind(
    src_str: &str,
    start: usize,
    end: usize,
    source_kind: SourceKind,
) -> Vec<Token> {
    if start == 0 && end == src_str.len() {
        crate::work::tick("whole-text lexes");
    }
    lex_region(
        src_str,
        start,
        end,
        source_kind,
        facts::Start::Statements,
        false,
        None,
    )
    .0
}

/// What the facts machine decided lexing `src`, nested regions included:
/// the validator that holds the machine to SWC's reading.
#[cfg(test)]
pub(crate) fn trace(src: &str, source_kind: SourceKind) -> Trace {
    let mut trace = Trace::default();
    lex_region(
        src,
        0,
        src.len(),
        source_kind,
        facts::Start::Statements,
        false,
        Some(&mut trace),
    );
    trace
}

/// Whether `offset` lies inside a comment of `src[start..end]`, a range
/// between two significant tokens and therefore trivia only — the same
/// comment rules [`lex_with_kind`] skips by. A position is inside a comment
/// after its opening delimiter and up to its end: a line comment's end of
/// line, a block comment's `*/`, or `end` for an unterminated block comment.
pub(crate) fn comment_at(src: &str, start: usize, end: usize, offset: usize) -> bool {
    let bytes = src.as_bytes();
    let mut i = start;
    while i < end {
        if bytes[i] == b'/' && at(bytes, i + 1, end) == Some(b'/') {
            let close = line_end(bytes, i, end);
            if i < offset && offset <= close {
                return true;
            }
            i = close;
        } else if bytes[i] == b'/' && at(bytes, i + 1, end) == Some(b'*') {
            let (close, inside) = match find_subslice(bytes, b"*/", i + 2, end) {
                Some(e) => (e + 2, offset < e + 2),
                None => (end, offset <= end),
            };
            if i < offset && inside {
                return true;
            }
            i = close;
        } else {
            i += 1;
        }
    }
    false
}

/// What the facts validator reads back from lexing, nested regions
/// included: every statement span a machine recognizes, in completion
/// order, and every position where a machine expected an operand and the
/// lexer read a regular expression or a JSX element there.
#[derive(Debug, Default)]
pub(crate) struct Trace {
    pub(crate) statements: Vec<Span>,
    pub(crate) regexes: Vec<usize>,
    pub(crate) elements: Vec<usize>,
}

type TraceSink<'t> = Option<&'t mut Trace>;

/// The byte just past the numeric literal starting at `i` (a digit, or a
/// `.` before one): digits and separators, a fraction, an exponent, a radix
/// prefix, and a BigInt suffix.
fn number_end(src: &[u8], i: usize, end: usize) -> usize {
    let digits = |mut j: usize, hex: bool| {
        while let Some(b) = at(src, j, end) {
            if b.is_ascii_digit() || b == b'_' || (hex && b.is_ascii_hexdigit()) {
                j += 1;
            } else {
                break;
            }
        }
        j
    };
    let mut j;
    if src[i] == b'0'
        && matches!(
            at(src, i + 1, end),
            Some(b'x' | b'X' | b'o' | b'O' | b'b' | b'B')
        )
    {
        j = digits(i + 2, true);
    } else {
        j = digits(i, false);
        if at(src, j, end) == Some(b'.') {
            j = digits(j + 1, false);
        }
        if matches!(at(src, j, end), Some(b'e' | b'E')) {
            let mut k = j + 1;
            if matches!(at(src, k, end), Some(b'+' | b'-')) {
                k += 1;
            }
            if at(src, k, end).is_some_and(|b| b.is_ascii_digit()) {
                j = digits(k, false);
            }
        }
    }
    if at(src, j, end) == Some(b'n') {
        j += 1;
    }
    j
}

/// Lexes `src[start..end]` from grammar position `mode`. A `braced` region
/// is a JavaScript expression container or template interpolation and ends
/// at its unmatched `}`, whose index is returned.
fn lex_region(
    src_str: &str,
    start: usize,
    end: usize,
    source_kind: SourceKind,
    mode: facts::Start,
    braced: bool,
    mut trace: TraceSink<'_>,
) -> (Vec<Token>, usize) {
    let src = src_str.as_bytes();
    let mut machine = facts::Machine::new(src_str, end, mode, trace.is_some());
    let mut brace_depth = 0usize;
    // Significant tokens run about one per six source bytes across real
    // TypeScript and one per four in tt source and the TypeScript the
    // compiler generates, so sizing up front for one per three spares the
    // repeated doubling that dominated lexing on large files.
    let mut tokens: Vec<Token> = Vec::with_capacity((end - start) / 3 + 8);
    let mut i = start;
    if start == 0 && !braced && src.starts_with(b"#!") {
        i = line_end(src, 0, end);
    }
    let mut number_until = 0usize;
    let span = |start: usize, end: usize| Span { start, end };
    let tok = |kind: facts::Tk, start: usize, end: usize, line_break: bool| facts::Tok {
        kind,
        text: if kind == facts::Tk::Word {
            &src_str[start..end]
        } else {
            ""
        },
        span: Span { start, end },
        line_break,
    };

    let close = loop {
        let (next, line_break) = skip_trivia(src, i, end);
        i = next;
        if i >= end {
            break end;
        }
        let c = src[i];
        if braced && c == b'}' && brace_depth == 0 {
            break i;
        }

        if i < number_until {
            let word = starts_identifier(src, i, end);
            let (kind, e) = if word {
                (TokenKind::Ident, ident_end(src, i, end))
            } else {
                (TokenKind::Punct(c), i + 1)
            };
            tokens.push(Token {
                kind,
                span: span(i, e),
                facts: machine.continuation(span(i, e)),
            });
            i = e;
            continue;
        }

        if c == b'"' || c == b'\'' {
            let e = scan_string(src, i, end);
            let facts = machine.push(tok(facts::Tk::Str, i, e, line_break));
            tokens.push(Token {
                kind: TokenKind::Str,
                span: span(i, e),
                facts,
            });
            i = e;
            continue;
        }

        if c == b'`' {
            let (e, parts) = lex_template(src_str, i, end, source_kind, trace.as_deref_mut());
            let facts = machine.push(tok(facts::Tk::Template, i, e, line_break));
            tokens.push(Token {
                kind: TokenKind::Template(parts.into_boxed_slice()),
                span: span(i, e),
                facts,
            });
            i = e;
            continue;
        }

        if source_kind.is_tsx()
            && c == b'<'
            && machine.operand_expected(i, line_break)
            && let Some(jsx) = scan_jsx(src_str, i, end, source_kind, trace.as_deref_mut())
        {
            if let Some(trace) = trace.as_deref_mut() {
                trace.elements.push(i);
            }
            let first = tokens.len();
            tokens.extend(jsx.tokens);
            let facts = machine.push(tok(facts::Tk::Jsx, i, jsx.end, line_break));
            if let Some(opening) = tokens.get_mut(first) {
                opening.facts = facts;
            }
            if let Some(closing) = tokens.last_mut() {
                closing.facts = closing.facts.ending_expression();
            }
            i = jsx.end;
            continue;
        }

        if c == b'/'
            && machine.operand_expected(i, line_break)
            && let Some(e) = scan_regex(src, i, end)
        {
            if let Some(trace) = trace.as_deref_mut() {
                trace.regexes.push(i);
            }
            let facts = machine.push(tok(facts::Tk::Regex, i, e, line_break));
            tokens.push(Token {
                kind: TokenKind::Regex,
                span: span(i, e),
                facts,
            });
            i = e;
            continue;
        }

        if starts_identifier(src, i, end) {
            let j = ident_end(src, i, end);
            let facts = machine.push(tok(facts::Tk::Word, i, j, line_break));
            tokens.push(Token {
                kind: TokenKind::Ident,
                span: span(i, j),
                facts,
            });
            i = j;
            continue;
        }

        if c.is_ascii_digit()
            || (c == b'.'
                && at(src, i + 1, end).is_some_and(|b| b.is_ascii_digit())
                && machine.operand_expected(i, line_break))
        {
            number_until = number_end(src, i, end);
            let facts = machine.push(tok(facts::Tk::Number, i, number_until, line_break));
            tokens.push(Token {
                kind: TokenKind::Punct(c),
                span: span(i, i + 1),
                facts,
            });
            i += 1;
            continue;
        }

        if braced {
            if c == b'{' {
                brace_depth += 1;
            }
            if c == b'}' {
                brace_depth -= 1;
            }
        }
        let (kind, machine_kind, len) = match (c, at(src, i + 1, end)) {
            (b'=', Some(b'>')) => (TokenKind::Arrow, facts::Tk::Arrow, 2),
            (b'|', Some(b'|')) => (TokenKind::OrOr, facts::Tk::OrOr, 2),
            (b'|', Some(b'>')) => (TokenKind::PipeOp, facts::Tk::Pipe, 2),
            (b'?', Some(b'.')) if !at(src, i + 2, end).is_some_and(|b| b.is_ascii_digit()) => {
                (TokenKind::OptChain, facts::Tk::OptChain, 2)
            }
            (b'?', Some(b'?')) => (TokenKind::Coalesce, facts::Tk::Coalesce, 2),
            _ => (TokenKind::Punct(c), facts::Tk::Punct(c), 1),
        };
        let facts = machine.push(tok(machine_kind, i, i + len, line_break));
        tokens.push(Token {
            kind,
            span: span(i, i + len),
            facts,
        });
        i += len;
    };
    let statements = machine.finish();
    if let Some(trace) = trace {
        trace.statements.extend(statements);
    }
    (tokens, close)
}

struct JsxExpression {
    open: usize,
    close: usize,
    tokens: Vec<Token>,
}

fn jsx_expression(
    src: &str,
    open: usize,
    end: usize,
    kind: SourceKind,
    trace: TraceSink<'_>,
) -> Option<JsxExpression> {
    let (tokens, close) = lex_region(
        src,
        open + 1,
        end,
        kind,
        facts::Start::Expression,
        true,
        trace,
    );
    (close < end).then_some(JsxExpression {
        open,
        close,
        tokens,
    })
}

/// Finds a JSX namespace name followed by member access (`<ns:name.member`).
///
/// JSX admits either a namespace name or a member chain, but never both in
/// one element name. SWC 45.x reaches an internal `unreachable` while
/// recovering this malformed shape, so the shared syntax boundary rejects it
/// before entering SWC. The tt lexer supplies the lexical isolation here:
/// strings, comments, regex literals, JSX text, and template raw chunks never
/// appear as punctuation tokens.
pub(crate) fn invalid_jsx_namespace_member(tokens: &[Token]) -> Option<Span> {
    fn in_tokens(tokens: &[Token]) -> Option<Span> {
        for (index, token) in tokens.iter().enumerate() {
            if let TokenKind::Template(parts) = &token.kind {
                for part in parts.iter() {
                    if let TplPart::Interp { tokens, .. } = part
                        && let Some(span) = in_tokens(tokens)
                    {
                        return Some(span);
                    }
                }
            }

            let mut cursor = index;
            if !matches!(tokens[cursor].kind, TokenKind::Punct(b'<')) {
                continue;
            }
            cursor += 1;
            if matches!(
                tokens.get(cursor).map(|token| &token.kind),
                Some(TokenKind::Punct(b'/'))
            ) {
                cursor += 1;
            }
            let shape = (
                tokens.get(cursor),
                tokens.get(cursor + 1),
                tokens.get(cursor + 2),
                tokens.get(cursor + 3),
                tokens.get(cursor + 4),
            );
            if let (
                Some(Token {
                    kind: TokenKind::Ident,
                    ..
                }),
                Some(Token {
                    kind: TokenKind::Punct(b':'),
                    ..
                }),
                Some(Token {
                    kind: TokenKind::Ident,
                    ..
                }),
                Some(Token {
                    kind: TokenKind::Punct(b'.'),
                    span,
                    ..
                }),
                Some(Token {
                    kind: TokenKind::Ident,
                    ..
                }),
            ) = shape
            {
                return Some(*span);
            }
        }
        None
    }

    in_tokens(tokens)
}

/// `src[start]` is a backtick — lexes the template into raw chunks and
/// recursively lexed `${ }` interpolations. Returns the index just past
/// the closing backtick (or `end` if unterminated).
///
/// An interpolation whose `}` is missing runs to `end` and ends the
/// template, as TypeScript's scanner reads it: after `${` come expression
/// tokens, whatever follows. The parts still cover the source once, in
/// order — no raw chunk follows an interpolation that never closed.
fn lex_template(
    src_str: &str,
    start: usize,
    end: usize,
    source_kind: SourceKind,
    mut trace: TraceSink<'_>,
) -> (usize, Vec<TplPart>) {
    let src = src_str.as_bytes();
    let mut parts: Vec<TplPart> = Vec::new();
    let mut raw_start = start; // includes the opening backtick
    let push_raw = |parts: &mut Vec<TplPart>, start: usize, end: usize| {
        // Empty quasis separate adjacent interpolations and own their
        // delimiters when lowered. Preserve the alternating template shape.
        parts.push(TplPart::Raw(Span { start, end }));
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
            let (tokens, close) = lex_region(
                src_str,
                i + 2,
                end,
                source_kind,
                facts::Start::Expression,
                true,
                trace.as_deref_mut(),
            );
            push_raw(&mut parts, raw_start, i);
            parts.push(TplPart::Interp {
                span: Span {
                    start: i + 2,
                    end: close,
                },
                tokens,
            });
            if close == end {
                return (end, parts);
            }
            i = (close + 1).min(end);
            raw_start = i;
            continue;
        }
        i += 1;
    }
    push_raw(&mut parts, raw_start, end);
    (end, parts)
}

struct ScannedJsx {
    end: usize,
    tokens: Vec<Token>,
}

/// Parses one complete JSX element or fragment and exposes only its
/// JavaScript expression containers to the tt lexer. Every tag, attribute,
/// and text run becomes an opaque token, so words in JSX text can never be
/// claimed as tt syntax.
fn scan_jsx(
    src_str: &str,
    start: usize,
    end: usize,
    source_kind: SourceKind,
    mut trace: TraceSink<'_>,
) -> Option<ScannedJsx> {
    let src = src_str.as_bytes();
    let opening = scan_jsx_opening(src_str, start, end, source_kind, trace.as_deref_mut())?;
    let mut tokens = jsx_region_tokens(start, opening.end, opening.expressions);
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
                facts: TokenFacts::default(),
            });
            return Some(ScannedJsx {
                end: close_end,
                tokens,
            });
        }
        if src[i] == b'<' {
            let child = scan_jsx(src_str, i, end, source_kind, trace.as_deref_mut())?;
            if raw_start < i {
                tokens.push(Token {
                    kind: TokenKind::JsxRaw,
                    span: Span {
                        start: raw_start,
                        end: i,
                    },
                    facts: TokenFacts::default(),
                });
            }
            tokens.extend(child.tokens);
            i = child.end;
            raw_start = i;
            continue;
        }
        if src[i] == b'{' {
            let expression = jsx_expression(src_str, i, end, source_kind, trace.as_deref_mut())?;
            let close = expression.close;
            if raw_start < i + 1 {
                tokens.push(Token {
                    kind: TokenKind::JsxRaw,
                    span: Span {
                        start: raw_start,
                        end: i + 1,
                    },
                    facts: TokenFacts::default(),
                });
            }
            tokens.extend(expression.tokens);
            tokens.push(Token {
                kind: TokenKind::JsxRaw,
                span: Span {
                    start: close,
                    end: close + 1,
                },
                facts: TokenFacts::default(),
            });
            i = close + 1;
            raw_start = i;
            continue;
        }
        i += 1;
    }
}

fn jsx_region_tokens(start: usize, end: usize, expressions: Vec<JsxExpression>) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut raw_start = start;
    for JsxExpression {
        open,
        close,
        tokens: expression_tokens,
    } in expressions
    {
        tokens.push(Token {
            kind: TokenKind::JsxRaw,
            span: Span {
                start: raw_start,
                end: open + 1,
            },
            facts: TokenFacts::default(),
        });
        tokens.extend(expression_tokens);
        tokens.push(Token {
            kind: TokenKind::JsxRaw,
            span: Span {
                start: close,
                end: close + 1,
            },
            facts: TokenFacts::default(),
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
            facts: TokenFacts::default(),
        });
    }
    tokens
}

struct JsxOpening {
    end: usize,
    name: Option<String>,
    self_closing: bool,
    expressions: Vec<JsxExpression>,
}

/// Returns `(end, opening_name, self_closing)`. `None` means the `<` starts
/// ordinary TypeScript syntax, not a complete JSX opening construct.
fn scan_jsx_opening(
    src_str: &str,
    start: usize,
    end: usize,
    source_kind: SourceKind,
    mut trace: TraceSink<'_>,
) -> Option<JsxOpening> {
    let src = src_str.as_bytes();
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
        i = facts::type_arguments_end(src, i, end)?;
    }
    loop {
        i = skip_trivia(src, i, end).0;
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
                let expression =
                    jsx_expression(src_str, i, end, source_kind, trace.as_deref_mut())?;
                let close = expression.close;
                expressions.push(expression);
                i = close + 1;
            }
            (Some(b), _) if b == b'"' || b == b'\'' => i = scan_string(src, i, end),
            (Some(b), _) if is_jsx_name_start(b) => {
                i = scan_jsx_name(src, i, end)?;
                i = skip_trivia(src, i, end).0;
                if at(src, i, end) == Some(b'=') {
                    i += 1;
                    i = skip_trivia(src, i, end).0;
                    match at(src, i, end)? {
                        b'"' | b'\'' => i = scan_string(src, i, end),
                        b'{' => {
                            let expression =
                                jsx_expression(src_str, i, end, source_kind, trace.as_deref_mut())?;
                            let close = expression.close;
                            expressions.push(expression);
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
            i = skip_trivia(src, i, end).0;
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
