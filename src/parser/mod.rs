//! Structural parsing of tt source into the AST.
//!
//! The parser is **infallible**: it never reports an error. The source is
//! first lexed into a significant-token stream ([`crate::lexer`]); the
//! parser walks that stream and lifts every construct that *fully* parses
//! as tt syntax — a `variant` declaration, a `match` expression, a `try` or
//! let-else statement, a `val` binding modifier, a relative `.tt` import
//! specifier — into a typed AST node; everything else, including any candidate that deviates even
//! slightly from tt syntax, is left as a verbatim byte range. This is how
//! the "every valid TypeScript file is a valid .tt file" contract is
//! implemented: construct-hood is a purely structural decision made here,
//! and all tt-level *errors* (duplicate cases, misplaced wildcard,
//! non-exhaustive match, bad field types) are the semantic phase's job
//! ([`crate::sema`]).
//!
//! Plain TypeScript enums keep working: only a declaration beginning with the
//! contextual `variant` keyword is treated as a tt variant. TypeScript `enum`
//! declarations are never lifted and pass through byte-for-byte.
//!
//! Nested code (match scrutinees, arm bodies, template interpolations) is
//! parsed recursively from sub-slices of the same token stream, with
//! absolute byte spans, so every later phase can report exact positions.
//!
//! Module layout: this file owns the main token loop and shared token
//! rules; [`cursor`] is the token cursor sub-parsers consume; [`variants`]
//! parses tt `variant` declarations; [`matches`] parses `match` expressions;
//! [`tries`] parses `try` statements; [`lets`] parses let-else statements;
//! [`results`] parses `result { ... }` computation blocks;
//! [`imports`] lifts relative `.tt` module specifiers out of static
//! import/re-export statements. The `val` binding modifier is recognized
//! here too, through the shared structural rule in [`crate::val`].

mod cursor;
mod iflets;
mod imports;
mod lets;
mod literals;
mod matches;
mod pipes;
mod results;
mod tries;
mod variants;

use crate::ast::*;
use crate::lexer::{self, Token, TokenKind, TplPart};
use crate::val;
use cursor::Cursor;

pub(crate) use cursor::{dotted_at, find_close_at};

pub(super) enum Claim<T> {
    Parsed(T),
    NotTt,
    /// Structurally recognized tt intent which did not complete. The source
    /// still passes through; this fact is consumed only if output verify
    /// later proves that the verbatim text is not TypeScript either.
    Unclaimed(UnclaimedTtCandidate),
    Malformed {
        error: crate::error::TtError,
        recovery: RecoveryNode,
    },
}

// Words that can never be a variant tag, match pattern tag, or binding name.
// Meeting one of these while trying to parse a tt construct aborts the
// attempt, so ordinary TypeScript (e.g. a class method named `match`) is
// left untouched.
//
// Written as a `match` rather than a slice scan: the token loop asks this of
// every identifier in the file, and rustc compiles a string `match` into a
// length switch plus a handful of comparisons, where `[&str]::contains`
// walks the whole list.
pub(crate) fn is_reserved(word: &str) -> bool {
    matches!(
        word,
        "async"
            | "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "variant"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "let"
            | "new"
            | "null"
            | "of"
            | "return"
            | "static"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
    )
}

// Words that terminate the expression currently being scanned — an
// undotted occurrence means whatever follows starts a new expression, so
// the pipeline-head tracker resets there. Prefix operators that *continue*
// an expression (`new`, `typeof`, `void`, `delete`, `await`) and
// expression-capable keywords (`function`, `class`) are deliberately
// absent; `in`/`of` reset for the sake of `for` heads (their rare binary
// use next to a pipeline needs parens, documented). A `match` for the same
// reason as [`is_reserved`] — every identifier in the file reaches it.
fn is_pipe_boundary_word(word: &str) -> bool {
    matches!(
        word,
        "break"
            | "case"
            | "catch"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "do"
            | "else"
            | "export"
            | "finally"
            | "for"
            | "if"
            | "in"
            | "let"
            | "of"
            | "return"
            | "switch"
            | "throw"
            | "var"
            | "while"
            | "with"
            | "yield"
    )
}

/// Parses a whole source file into a [`Program`].
pub(crate) fn parse(src: &str) -> Program {
    lex_and_parse(src).0
}

pub(crate) fn parse_with_kind(src: &str, source_kind: crate::SourceKind) -> Program {
    lex_and_parse_with_kind(src, source_kind).0
}

/// [`parse`], also returning the file's token stream — the `val` analysis
/// ([`crate::val::check`]) reads the same tokens, and lexing twice is the
/// compiler's most expensive avoidable work.
pub(crate) fn lex_and_parse(src: &str) -> (Program, Vec<Token>) {
    lex_and_parse_with_kind(src, crate::SourceKind::TypeScript)
}

pub(crate) fn lex_and_parse_with_kind(
    src: &str,
    source_kind: crate::SourceKind,
) -> (Program, Vec<Token>) {
    let parser = Parser {
        src,
        bytes: src.as_bytes(),
        flow_queries: crate::flow::FlowBodyQueries::default(),
    };
    let tokens = lexer::lex_with_kind(src, 0, src.len(), source_kind);
    let program = parser.parse_tokens(&tokens, 0, src.len());
    (program, tokens)
}

/// Visits every recursively nested parse region exactly once.
///
/// Parser side tables use one structural traversal so adding a new nested
/// [`Program`] shape cannot make recovery and rollback collection drift.
fn visit_programs(program: &Program, visit: &mut impl FnMut(&Program)) {
    visit(program);
    for segment in &program.segments {
        match segment {
            Segment::Verbatim(_)
            | Segment::Variant(_)
            | Segment::TtImport(_)
            | Segment::ValModifier(_) => {}
            Segment::Match(expr) => {
                visit_programs(&expr.scrutinee, visit);
                for arm in &expr.arms {
                    if let Some(guard) = &arm.guard {
                        visit_programs(&guard.expr, visit);
                    }
                    visit_programs(&arm.body, visit);
                }
            }
            Segment::TupleMatch(expr) => {
                for (_, scrutinee) in &expr.scrutinees {
                    visit_programs(scrutinee, visit);
                }
                for arm in &expr.arms {
                    if let Some(guard) = &arm.guard {
                        visit_programs(&guard.expr, visit);
                    }
                    visit_programs(&arm.body, visit);
                }
            }
            Segment::Try(stmt) => visit_programs(&stmt.expr, visit),
            Segment::TryExpr(expr) => visit_programs(&expr.expr, visit),
            Segment::LetElse(stmt) => {
                visit_programs(&stmt.expr, visit);
                visit_programs(&stmt.else_body, visit);
            }
            Segment::IfLet(stmt) => {
                visit_programs(&stmt.expr, visit);
                visit_programs(&stmt.body, visit);
                let mut next = stmt.else_part.as_ref();
                while let Some(else_part) = next {
                    match else_part {
                        IfLetElse::Block(block) => {
                            visit_programs(block, visit);
                            break;
                        }
                        IfLetElse::IfLet(chained) => {
                            visit_programs(&chained.expr, visit);
                            visit_programs(&chained.body, visit);
                            next = chained.else_part.as_ref();
                        }
                    }
                }
            }
            Segment::Template(template) => {
                for chunk in &template.chunks {
                    if let TemplateChunk::Interp(interp) = chunk {
                        visit_programs(interp, visit);
                    }
                }
            }
            Segment::Pipe(pipe) => {
                if let Some(head) = &pipe.head {
                    visit_programs(head, visit);
                }
                for step in &pipe.steps {
                    visit_programs(&step.body, visit);
                }
            }
            Segment::ResultBlock(block) => {
                for item in &block.items {
                    let ResultItem::Stmts(stmts) = item;
                    visit_programs(stmts, visit);
                }
                if let Some(value) = &block.value {
                    visit_programs(value, visit);
                }
            }
        }
    }
}

/// Collects parser-owned recovery nodes from the recursively nested AST.
pub(crate) fn projection_recoveries(program: &Program) -> Vec<RecoveryNode> {
    let mut out = Vec::new();
    visit_programs(program, &mut |program| {
        out.extend(program.recoveries.iter().cloned());
    });
    out.sort_by_key(|node| (node.span.start, std::cmp::Reverse(node.span.end)));
    out
}

/// Collects structurally recognized, rolled-back tt candidates from every
/// nested source region. Output verification uses these facts to explain a
/// passthrough parse failure without scanning source text for keywords.
pub(crate) fn unclaimed_candidates(program: &Program) -> Vec<UnclaimedTtCandidate> {
    let mut out = Vec::new();
    visit_programs(program, &mut |program| {
        if let Some(candidates) = &program.unclaimed {
            out.extend(candidates.0.iter().copied());
        }
    });
    out.sort_by_key(|candidate| (candidate.extent.start, candidate.extent.end));
    out
}

/// Shared state for one parse: the source in both views and memoized semantic
/// queries. Recursion carries explicit token slices and byte ranges.
pub(crate) struct Parser<'a> {
    pub src: &'a str,
    pub bytes: &'a [u8],
    flow_queries: crate::flow::FlowBodyQueries,
}

impl Parser<'_> {
    pub(super) fn body_diverges(&self, span: Span, tokens: &[Token], program: &Program) -> bool {
        self.flow_queries.diverges(self.src, span, tokens, program)
    }
}

fn flush_verbatim(segments: &mut Vec<Segment>, start: usize, end: usize) {
    if start < end {
        segments.push(Segment::Verbatim(Span { start, end }));
    }
}

/// The byte where a segment starts in the source (approximate for variants —
/// the name offset — which is fine: rewinding only compares against a
/// pipeline head start, and a variant declaration cannot sit inside one).
fn segment_start(seg: &Segment) -> usize {
    match seg {
        Segment::Verbatim(span) => span.start,
        Segment::Variant(d) => d.name_off,
        Segment::Match(m) => m.keyword_off,
        Segment::TupleMatch(m) => m.keyword_off,
        Segment::Try(t) => t.keyword_off,
        Segment::TryExpr(expr) => expr.span.start,
        Segment::LetElse(l) => l.keyword_off,
        Segment::IfLet(s) => s.keyword_off,
        Segment::TtImport(d) => d.spec.start,
        Segment::Template(t) => match t.chunks.first() {
            Some(TemplateChunk::Raw(span)) => span.start,
            Some(TemplateChunk::Interp(_)) | None => 0, // first chunk is always Raw
        },
        Segment::Pipe(p) => p.head_span.start,
        Segment::ResultBlock(b) => b.keyword_off,
        Segment::ValModifier(span) => span.start,
    }
}

/// Pops (and truncates) segments back to `boundary` so a pipeline head can
/// re-own bytes that were already lifted (a template or match inside the
/// head). Segments are contiguous, so the returned byte — the new "flushed
/// up to here" position — is the start of the last popped segment, or
/// `boundary` when a verbatim segment crossing it was truncated.
fn rewind_segments(segments: &mut Vec<Segment>, boundary: usize, seg_start: usize) -> usize {
    let mut cover = seg_start;
    while let Some(last) = segments.last_mut() {
        match last {
            Segment::Verbatim(span) => {
                if span.start >= boundary {
                    cover = span.start;
                    segments.pop();
                } else if span.end > boundary {
                    span.end = boundary;
                    cover = boundary;
                    break;
                } else {
                    break;
                }
            }
            other => {
                let s = segment_start(other);
                if s >= boundary {
                    cover = s;
                    segments.pop();
                } else {
                    break;
                }
            }
        }
    }
    cover
}

/// Bounds the expression containing an unclaimed operator and stops before
/// the enclosing statement or delimiter. This parser-owned synchronization
/// point prevents recovery from consuming the next independent construct.
fn recovery_expression_span(
    tokens: &[Token],
    start_idx: usize,
    operator_idx: usize,
    range_end: usize,
) -> Span {
    let mut depth = 0usize;
    let mut recovery_end = tokens
        .get(operator_idx)
        .map_or(range_end, |token| token.span.end);
    for token in tokens.iter().skip(operator_idx + 1) {
        match token.kind {
            TokenKind::Punct(b'(' | b'[' | b'{') => depth += 1,
            TokenKind::Punct(b')' | b']' | b'}') if depth == 0 => break,
            TokenKind::Punct(b')' | b']' | b'}') => depth -= 1,
            TokenKind::Punct(b';' | b',') if depth == 0 => break,
            _ => recovery_end = token.span.end,
        }
    }
    Span {
        start: tokens
            .get(start_idx)
            .map_or(tokens[operator_idx].span.start, |token| token.span.start),
        end: recovery_end,
    }
}

fn recovery_statement_span(tokens: &[Token], start_idx: usize, range_end: usize) -> Span {
    let recovery_end = (start_idx..tokens.len())
        .find(|&idx| matches!(tokens[idx].kind, TokenKind::Punct(b'{')))
        .and_then(|open| find_close_at(tokens, open))
        .and_then(|close| tokens.get(close))
        .map_or(range_end, |token| token.span.end);
    Span {
        start: tokens[start_idx].span.start,
        end: recovery_end,
    }
}

fn starts_statement(src: &str, tokens: &[Token], idx: usize, in_ternary: bool) -> bool {
    if in_ternary {
        return false;
    }
    let Some(previous) = idx.checked_sub(1).and_then(|idx| tokens.get(idx)) else {
        return true;
    };
    if matches!(previous.kind, TokenKind::Punct(b'{' | b'}' | b';' | b':')) {
        return true;
    }
    if matches!(previous.kind, TokenKind::Ident)
        && matches!(&src[previous.span.start..previous.span.end], "else" | "do")
    {
        return true;
    }
    if !matches!(previous.kind, TokenKind::Punct(b')')) {
        return false;
    }
    let mut depth = 0usize;
    for open in (0..idx).rev() {
        match tokens[open].kind {
            TokenKind::Punct(b')') => depth += 1,
            TokenKind::Punct(b'(') => {
                depth -= 1;
                if depth == 0 {
                    return open.checked_sub(1).is_some_and(|before| {
                        matches!(tokens[before].kind, TokenKind::Ident)
                            && matches!(
                                &src[tokens[before].span.start..tokens[before].span.end],
                                "if" | "for" | "while" | "with"
                            )
                    });
                }
            }
            _ => {}
        }
    }
    false
}

/// The update of a C-style `for` header follows its second top-level
/// semicolon. Its `try` is an expression candidate, while the test position
/// keeps statement-propagation ownership so Evaluation IR can report the
/// repeated-evaluation reason.
fn in_for_update(src: &str, tokens: &[Token], idx: usize) -> bool {
    let mut depth = 0usize;
    let mut separators = 0usize;
    for cursor in (0..idx).rev() {
        match tokens[cursor].kind {
            TokenKind::Punct(b')') => depth += 1,
            TokenKind::Punct(b'(') => {
                if depth == 0 {
                    return separators >= 2
                        && cursor.checked_sub(1).is_some_and(|before| {
                            matches!(tokens[before].kind, TokenKind::Ident)
                                && &src[tokens[before].span.start..tokens[before].span.end] == "for"
                        });
                }
                depth -= 1;
            }
            TokenKind::Punct(b';') if depth == 0 => separators += 1,
            _ => {}
        }
    }
    false
}

/// A colon normally admits a following statement (labels, `case`, match
/// arms), but a colon inside an object literal introduces a value expression.
/// This recognizes the object-literal brace from the token that introduced it
/// instead of guessing from the spelling of the property.
fn follows_object_member_colon(src: &str, tokens: &[Token], idx: usize) -> bool {
    if !idx
        .checked_sub(1)
        .and_then(|previous| tokens.get(previous))
        .is_some_and(|token| matches!(token.kind, TokenKind::Punct(b':')))
    {
        return false;
    }

    let mut depth = 0usize;
    for open in (0..idx - 1).rev() {
        match tokens[open].kind {
            TokenKind::Punct(b'}') => depth += 1,
            TokenKind::Punct(b'{') if depth > 0 => depth -= 1,
            TokenKind::Punct(b'{') => {
                let Some(before) = open.checked_sub(1).and_then(|at| tokens.get(at)) else {
                    return false;
                };
                return match before.kind {
                    TokenKind::Punct(b'(' | b'[' | b',' | b'=' | b':' | b'?') => true,
                    TokenKind::Ident => {
                        matches!(&src[before.span.start..before.span.end], "return" | "yield")
                    }
                    _ => false,
                };
            }
            _ => {}
        }
    }
    false
}

/// Returns true for the `= try` portion of a declaration. A declaration try
/// without its required semicolon is an incomplete tt statement, not a
/// misplaced expression, and must retain its rollback candidate.
fn follows_declaration_equals(src: &str, tokens: &[Token], idx: usize) -> bool {
    if !idx
        .checked_sub(1)
        .and_then(|previous| tokens.get(previous))
        .is_some_and(|token| matches!(token.kind, TokenKind::Punct(b'=')))
    {
        return false;
    }

    let mut depth = 0usize;
    for at in (0..idx - 1).rev() {
        match tokens[at].kind {
            TokenKind::Punct(b')' | b']' | b'}') => depth += 1,
            TokenKind::Punct(b'(' | b'[' | b'{') if depth > 0 => depth -= 1,
            // This `=` belongs to a destructuring default, not the
            // declaration initializer. Its `try` must therefore use the
            // expression-placement path below rather than declaration
            // recovery.
            TokenKind::Punct(b'(' | b'[' | b'{') => return false,
            TokenKind::Punct(b';') if depth == 0 => return false,
            TokenKind::Ident
                if depth == 0
                    && matches!(
                        &src[tokens[at].span.start..tokens[at].span.end],
                        "const" | "let" | "var"
                    ) =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

/// A spread operand begins with three adjacent dot tokens. The last dot is
/// not member access, even though the generic property-name test sees it
/// immediately before the operand keyword.
fn follows_spread_operator(tokens: &[Token], idx: usize) -> bool {
    idx >= 3
        && tokens[idx - 3..idx]
            .iter()
            .all(|token| matches!(token.kind, TokenKind::Punct(b'.')))
}

impl Parser<'_> {
    /// Parses a lexed token range covering `bytes[start..end]` into a
    /// [`Program`] whose segments cover the byte range exactly, in source
    /// order. Bytes between lifted constructs — trivia included — become
    /// verbatim segments.
    pub(crate) fn parse_tokens(&self, tokens: &[Token], start: usize, end: usize) -> Program {
        self.parse_tokens_with_context(tokens, start, end, false)
    }

    pub(super) fn parse_expression_tokens(
        &self,
        tokens: &[Token],
        start: usize,
        end: usize,
    ) -> Program {
        self.parse_tokens_with_context(tokens, start, end, true)
    }

    fn parse_tokens_with_context(
        &self,
        tokens: &[Token],
        start: usize,
        end: usize,
        expression_root: bool,
    ) -> Program {
        let mut segments: Vec<Segment> = Vec::new();
        let mut unclaimed: Vec<UnclaimedTtCandidate> = Vec::new();
        let mut recoveries: Vec<RecoveryNode> = Vec::new();
        let mut malformed = Vec::new();
        let mut stray_pipes: Vec<usize> = Vec::new();
        let mut stray_if_lets: Vec<usize> = Vec::new();
        let stray_results: Vec<usize> = Vec::new();
        let mut seg_start = start;
        let mut i = 0usize;

        // Expression-start tracking for pipeline heads: the index of the
        // token starting the expression currently being scanned, plus a
        // taint flag for unparenthesized ternary punctuation (a tainted
        // head aborts the claim — the normative "parenthesize ternaries"
        // rule). Brackets save and restore the enclosing expression's
        // state, so `f(a(b) |> g)` finds `a(b)`, not `b`.
        let mut expr: (usize, bool) = (0, false);
        let mut expr_stack: Vec<(usize, bool)> = Vec::new();

        while i < tokens.len() {
            let tok = &tokens[i];
            let word = match tok.kind {
                TokenKind::Template(ref parts) => {
                    flush_verbatim(&mut segments, seg_start, tok.span.start);
                    segments.push(Segment::Template(self.build_template(parts)));
                    seg_start = tok.span.end;
                    i += 1;
                    continue;
                }
                TokenKind::PipeOp => {
                    if !expr.1
                        && expr.0 < i
                        && let Some(attempt) = pipes::parse_pipeline(self, tokens, expr.0, i)
                    {
                        let (next_i, pipe) = match attempt {
                            pipes::Attempt::Parsed(next_i, pipe) => (next_i, pipe),
                            pipes::Attempt::MalformedOptional {
                                next,
                                head_span,
                                error_span,
                                extent,
                            } => {
                                seg_start =
                                    rewind_segments(&mut segments, head_span.start, seg_start);
                                malformed.push(
                                    crate::error::TtError::span(
                                        error_span.start,
                                        error_span.end,
                                        "pipeline: invalid optional postfix tail".to_string(),
                                    )
                                    .code(
                                        crate::diagnostics::DiagnosticCode::MalformedPipelinePostfix,
                                    )
                                    .owner(extent.start, extent.end)
                                    .help(
                                        "an optional postfix step starts with `?.name`, \
                                         `?.[key]`, or `?.(args)` and continues only with member, \
                                         index, or call operations",
                                    ),
                                );
                                recoveries.push(RecoveryNode {
                                    span: extent,
                                    kind: RecoveryKind::Expression,
                                });
                                i = next;
                                expr = (i, false);
                                continue;
                            }
                        };
                        // The head may span constructs already lifted as
                        // segments (a template, a match) — rewind them and
                        // let the head's sub-program own those bytes.
                        let head_start = pipe.head_span.start;
                        let pipe_end = pipe.steps.last().map(|s| s.span.end).unwrap_or(end);
                        seg_start = rewind_segments(&mut segments, head_start, seg_start);
                        flush_verbatim(&mut segments, seg_start, head_start);
                        segments.push(Segment::Pipe(pipe));
                        seg_start = pipe_end;
                        i = next_i;
                        expr = (i, false);
                        continue;
                    }
                    stray_pipes.push(tok.span.start);
                    recoveries.push(RecoveryNode {
                        span: recovery_expression_span(tokens, expr.0.min(i), i, end),
                        kind: RecoveryKind::Expression,
                    });
                    i += 1;
                    continue;
                }
                TokenKind::Ident => &self.src[tok.span.start..tok.span.end],
                _ => {
                    self.track_expr_boundary(tok, i, tokens, &mut expr, &mut expr_stack);
                    i += 1;
                    continue;
                }
            };

            // property access like `str.match(...)` never starts a construct
            let dotted = cursor::dotted_at(tokens, 0, i);

            if !dotted && (word == "variant" || word == "export") {
                let (kw_idx, exported) = if word == "variant" {
                    (Some(i), false)
                } else {
                    match tokens.get(i + 1) {
                        Some(t)
                            if matches!(t.kind, TokenKind::Ident)
                                && &self.src[t.span.start..t.span.end] == "variant" =>
                        {
                            (Some(i + 1), true)
                        }
                        _ => (None, false),
                    }
                };
                if let Some(kw_idx) = kw_idx {
                    match variants::parse_variant(
                        Cursor::new(self, tokens, kw_idx + 1, end),
                        exported,
                    ) {
                        Claim::Parsed((cur, byte_end, decl)) => {
                            flush_verbatim(&mut segments, seg_start, tok.span.start);
                            segments.push(Segment::Variant(decl));
                            seg_start = byte_end;
                            i = cur.idx;
                            expr = (i, false);
                            continue;
                        }
                        Claim::Malformed { error, recovery } => {
                            malformed.push(error);
                            recoveries.push(recovery);
                        }
                        Claim::Unclaimed(candidate) => unclaimed.push(candidate),
                        Claim::NotTt => {}
                    }
                }
            }

            // Static import / re-export of a relative `.tt` path — only
            // the specifier string is lifted; the clause before it and
            // the rest of the statement stay verbatim.
            if !dotted
                && (word == "import" || word == "export")
                && let Some((cur, decl)) =
                    imports::parse_tt_import(Cursor::new(self, tokens, i + 1, end), word)
            {
                flush_verbatim(&mut segments, seg_start, decl.spec.start);
                seg_start = decl.spec.end;
                segments.push(Segment::TtImport(decl));
                i = cur.idx;
                expr = (i, false);
                continue;
            }

            if !dotted && word == "match" {
                match matches::parse_match(Cursor::new(self, tokens, i + 1, end), tok.span) {
                    Claim::Parsed((cur, byte_end, parsed)) => {
                        flush_verbatim(&mut segments, seg_start, tok.span.start);
                        segments.push(match parsed {
                            matches::ParsedMatch::Single(expr) => Segment::Match(expr),
                            matches::ParsedMatch::Tuple(expr) => Segment::TupleMatch(expr),
                        });
                        seg_start = byte_end;
                        i = cur.idx;
                        continue;
                    }
                    Claim::Malformed { error, recovery } => {
                        malformed.push(error);
                        recoveries.push(recovery);
                    }
                    Claim::Unclaimed(candidate) => unclaimed.push(candidate),
                    Claim::NotTt => {}
                }
            }

            // `try <expr>;` — never valid TypeScript in expression
            // position (`try { ... }` blocks and member names are
            // structurally excluded by the sub-parser).
            if (!dotted || follows_spread_operator(tokens, i)) && word == "try" {
                let misplaced = (expression_root && i == 0)
                    || !starts_statement(self.src, tokens, i, expr.1)
                    || in_for_update(self.src, tokens, i)
                    || follows_object_member_colon(self.src, tokens, i);
                let parenthesized_declaration_operand =
                    follows_declaration_equals(self.src, tokens, i)
                        && tokens
                            .get(i + 1)
                            .is_some_and(|token| matches!(token.kind, TokenKind::Punct(b'(')));
                if misplaced
                    && (!follows_declaration_equals(self.src, tokens, i)
                        || parenthesized_declaration_operand)
                    && let Some((next_i, parsed)) =
                        tries::parse_try_expr(Cursor::new(self, tokens, i + 1, end), tok.span)
                {
                    flush_verbatim(&mut segments, seg_start, tok.span.start);
                    let span = parsed.span;
                    segments.push(Segment::TryExpr(parsed));
                    seg_start = span.end;
                    i = next_i;
                    continue;
                }
                match tries::parse_try_stmt(Cursor::new(self, tokens, i + 1, end), tok.span) {
                    Claim::Parsed((cur, byte_end, mut stmt)) => {
                        stmt.in_function = crate::flow::in_function_body(self.src, tokens, i);
                        flush_verbatim(&mut segments, seg_start, tok.span.start);
                        segments.push(Segment::Try(stmt));
                        seg_start = byte_end;
                        i = cur.idx;
                        continue;
                    }
                    Claim::Unclaimed(candidate) => unclaimed.push(candidate),
                    Claim::NotTt => {}
                    Claim::Malformed { .. } => unreachable!("try rollback is not malformed"),
                }
            }

            // `const|let|var <binding> = try <expr>;` — the `= try`
            // sequence is never valid TypeScript — and
            // `const|let|var Tag(...) = <expr> else { ... };` — a
            // declaration keyword is never followed by `<ident>(` in
            // valid TypeScript.
            if !dotted && (word == "const" || word == "let" || word == "var") {
                if let Some((cur, byte_end, mut stmt)) =
                    tries::parse_try_decl(Cursor::new(self, tokens, i + 1, end), tok.span)
                {
                    stmt.in_function = crate::flow::in_function_body(self.src, tokens, i);
                    flush_verbatim(&mut segments, seg_start, tok.span.start);
                    segments.push(Segment::Try(stmt));
                    seg_start = byte_end;
                    i = cur.idx;
                    expr = (i, false);
                    continue;
                }
                if let Some((cur, byte_end, mut stmt)) =
                    lets::parse_let_else(Cursor::new(self, tokens, i + 1, end), tok.span)
                {
                    stmt.in_function = crate::flow::in_function_body(self.src, tokens, i);
                    flush_verbatim(&mut segments, seg_start, tok.span.start);
                    segments.push(Segment::LetElse(stmt));
                    seg_start = byte_end;
                    i = cur.idx;
                    expr = (i, false);
                    continue;
                }
            }

            // `if let ...` — an undotted `if` followed by `let` is never
            // valid TypeScript, so a candidate that fails to parse cannot
            // be passed through either; it is recorded for sema.
            if !dotted
                && word == "if"
                && matches!(tokens.get(i + 1),
                    Some(t) if matches!(t.kind, TokenKind::Ident)
                        && &self.src[t.span.start..t.span.end] == "let")
            {
                if let Some((cur, byte_end, mut stmt)) =
                    iflets::parse_if_let(Cursor::new(self, tokens, i + 1, end), tok.span)
                {
                    stmt.in_function = crate::flow::in_function_body(self.src, tokens, i);
                    flush_verbatim(&mut segments, seg_start, tok.span.start);
                    segments.push(Segment::IfLet(stmt));
                    seg_start = byte_end;
                    i = cur.idx;
                    expr = (i, false);
                    continue;
                }
                stray_if_lets.push(tok.span.start);
                recoveries.push(RecoveryNode {
                    span: recovery_statement_span(tokens, i, end),
                    kind: RecoveryKind::Statement,
                });
            }

            // `result { ... }` is contextual: only a body with a nearest
            // direct tt `try` is claimed. Otherwise `result` remains an
            // ordinary identifier that a block statement may follow.
            if !dotted
                && word == "result"
                && matches!(tokens.get(i + 1), Some(t) if matches!(t.kind, TokenKind::Punct(b'{')))
            {
                let (attempt, nested) =
                    results::parse_result_block(Cursor::new(self, tokens, i + 1, end), tok.span);
                let _ = nested;
                match attempt {
                    results::Attempt::Claimed(cur, byte_end, block) => {
                        flush_verbatim(&mut segments, seg_start, tok.span.start);
                        segments.push(Segment::ResultBlock(*block));
                        seg_start = byte_end;
                        i = cur.idx;
                        continue;
                    }
                    results::Attempt::LegacyBinding {
                        span,
                        arrow,
                        recovery,
                    } => {
                        malformed.push(
                            crate::error::TtError::span(
                                span.start,
                                span.end,
                                "`<-` is no longer valid in a `result` block".to_string(),
                            )
                            .code(crate::DiagnosticCode::ResultLegacyBinding)
                            .suggest(
                                "replace `<-` with `= try`; then add an explicit Result return",
                                arrow.start,
                                arrow.end,
                                "= try",
                            )
                            .owner(recovery.start, recovery.end),
                        );
                        recoveries.push(RecoveryNode {
                            span: recovery,
                            kind: RecoveryKind::Expression,
                        });
                    }
                    results::Attempt::Pass => {}
                }
            }

            // `val` — a binding modifier, dropped from the output. The
            // two accepted shapes (`val const|let|var` on one line, and
            // `val <binding>` at the start of a parameter-list entry)
            // cannot occur in valid TypeScript, so every other `val` is
            // an ordinary identifier and stays verbatim.
            if !dotted && word == "val" && val::modifier_at(self.src, tokens, i).is_some() {
                flush_verbatim(&mut segments, seg_start, tok.span.start);
                let end = val::modifier_end(self.src, tok.span.end);
                segments.push(Segment::ValModifier(Span {
                    start: tok.span.start,
                    end,
                }));
                seg_start = end;
                i += 1;
                continue;
            }

            if !dotted && is_pipe_boundary_word(word) {
                expr = (i + 1, false);
            }
            i += 1;
        }

        flush_verbatim(&mut segments, seg_start, end);
        Program {
            segments,
            unclaimed: (!unclaimed.is_empty()).then(|| Box::new(UnclaimedTtCandidates(unclaimed))),
            recoveries,
            malformed,
            stray_pipes,
            stray_if_lets,
            stray_results,
        }
    }

    /// Advances the pipeline-head tracker over one non-identifier token.
    /// Openers save the enclosing expression's state; closers restore it —
    /// a `}` only when what follows can *continue* an expression (an object
    /// literal or function-expression body), otherwise it closed a block
    /// and the next token starts fresh. `?`/`:` reset while carrying the
    /// ternary taint that makes a later claim abort.
    fn track_expr_boundary(
        &self,
        tok: &Token,
        i: usize,
        tokens: &[Token],
        expr: &mut (usize, bool),
        stack: &mut Vec<(usize, bool)>,
    ) {
        match tok.kind {
            TokenKind::Punct(b'(' | b'[' | b'{') => {
                stack.push(*expr);
                *expr = (i + 1, false);
            }
            TokenKind::Punct(b')' | b']') => {
                *expr = stack.pop().unwrap_or((i + 1, false));
            }
            TokenKind::Punct(b'}') => {
                let outer = stack.pop().unwrap_or((i + 1, false));
                *expr = if self.brace_ends_expression(tokens, i) {
                    outer
                } else {
                    (i + 1, false)
                };
            }
            TokenKind::Punct(b';' | b',') => *expr = (i + 1, false),
            TokenKind::Punct(b'=') if pipes::is_assignment_eq(self.bytes, tok.span) => {
                *expr = (i + 1, false);
            }
            TokenKind::Punct(b':') => *expr = (i + 1, expr.1),
            TokenKind::Punct(b'?') => *expr = (i + 1, true),
            TokenKind::Arrow => *expr = (i + 1, false),
            _ => {}
        }
    }

    /// True when the token after the `}` at `close_idx` continues the
    /// surrounding expression (`.m`, `)`, an operator, `|>`, ...) rather
    /// than starting a new statement or expression.
    pub(super) fn brace_ends_expression(&self, tokens: &[Token], close_idx: usize) -> bool {
        match tokens.get(close_idx + 1) {
            None => true,
            Some(t) => match &t.kind {
                TokenKind::Ident => {
                    matches!(&self.src[t.span.start..t.span.end], "instanceof" | "in")
                }
                TokenKind::Str | TokenKind::Template(_) | TokenKind::Regex => false,
                TokenKind::JsxRaw => true,
                TokenKind::Punct(c) => {
                    !matches!(c, b'(' | b'[' | b'{' | b'!' | b'~') && !c.is_ascii_digit()
                }
                _ => true, // |>, =>, ||, ?., ?? all continue an expression
            },
        }
    }

    /// Turns a lexed template token into the AST template, recursively
    /// parsing each interpolation's token stream.
    fn build_template(&self, parts: &[TplPart]) -> Template {
        let chunks = parts
            .iter()
            .map(|part| match part {
                TplPart::Raw(span) => TemplateChunk::Raw(*span),
                TplPart::Interp { span, tokens } => TemplateChunk::Interp(
                    self.parse_tokens_with_context(tokens, span.start, span.end, true),
                ),
            })
            .collect();
        Template { chunks }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_incomplete_try_keeps_a_structural_rollback_fact() {
        let source = "function f() {\n  const n = try g()\n  return n;\n}\n";
        let program = parse(source);
        let candidates = unclaimed_candidates(&program);
        assert_eq!(candidates.len(), 1, "{candidates:#?}");
        let candidate = candidates[0];
        assert_eq!(candidate.kind, UnclaimedTtKind::Try);
        assert_eq!(
            &source[candidate.keyword.start..candidate.keyword.end],
            "try"
        );
        assert_eq!(
            &source[candidate.extent.start..candidate.extent.end],
            "try g()"
        );
    }

    #[test]
    fn valid_typescript_try_shapes_are_not_tt_candidates() {
        let source = "try { f(); } catch (error) { g(error); }\n\
                      const object = { try: 1 };\n\
                      object.try();\n";
        assert!(unclaimed_candidates(&parse(source)).is_empty());
    }

    #[test]
    fn a_template_interpolation_recovery_is_collected_once() {
        let program = parse("const value = `x=${1 |> }`;\n");
        let recoveries = projection_recoveries(&program);
        assert_eq!(recoveries.len(), 1, "{recoveries:#?}");
    }
}
