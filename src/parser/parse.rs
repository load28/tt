//! Whole-file parsing, recovery collection, and parser implementation.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use super::*;

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
    crate::work::tick("source parses");
    let tokens = lexer::lex_with_kind(src, 0, src.len(), source_kind);
    let parse = |host_rejected_vals: &[usize], host_owned_matches: Vec<Span>| {
        Parser {
            src,
            bytes: src.as_bytes(),
            source_kind,
            host_owned_matches,
            host_rejected_vals: host_rejected_vals.to_vec(),
            flow_queries: crate::flow::FlowBodyQueries::default(),
            passed_results: RefCell::default(),
        }
        .parse_tokens(&tokens, 0, src.len())
    };
    let mut program = parse(&[], Vec::new());
    let host_rejected_vals = host::rejected_val_candidates(src, source_kind, &program);
    if !host_rejected_vals.is_empty() {
        program = parse(&host_rejected_vals, Vec::new());
    }
    let host_owned_matches = host::owned_match_names_in_mixed(src, source_kind, &program);
    if !host_owned_matches.is_empty() {
        program = parse(&host_rejected_vals, host_owned_matches);
    }
    (program, tokens)
}

/// Every `val` modifier of the parse, keyed by the keyword's byte offset —
/// the parser's decision, which the `val` analysis reads over the token
/// stream instead of re-deriving it.
pub(crate) fn val_modifiers(program: &Program) -> HashMap<usize, ValModifier> {
    let mut modifiers = HashMap::new();
    visit_programs(program, &mut |region| {
        for segment in &region.segments {
            if let Segment::ValModifier(modifier) = segment {
                modifiers.insert(modifier.span.start, *modifier);
            }
        }
    });
    modifiers
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PipelineShape {
    pub head: Option<Span>,
    pub span: Span,
    pub first_call: Option<Span>,
}

#[derive(Debug, Clone)]
pub(crate) struct ArmScope {
    pub span: Span,
    pub bindings: Vec<Span>,
}

pub(crate) fn arm_scopes(program: &Program) -> Vec<ArmScope> {
    let mut scopes = Vec::new();
    visit_programs(program, &mut |region| {
        for segment in &region.segments {
            match segment {
                Segment::Match(expr) => {
                    for arm in &expr.arms {
                        let mut bindings = Vec::new();
                        pattern_bindings(&arm.pattern, &mut bindings);
                        scopes.push(ArmScope {
                            span: Span {
                                start: arm.pattern_span.end,
                                end: arm.body_span.end,
                            },
                            bindings,
                        });
                    }
                }
                Segment::TupleMatch(expr) => {
                    for arm in &expr.arms {
                        let mut bindings = Vec::new();
                        if let TuplePattern::Elems(elements) = &arm.pattern {
                            for element in elements {
                                pattern_bindings(element, &mut bindings);
                            }
                        }
                        scopes.push(ArmScope {
                            span: Span {
                                start: arm.pattern_span.end,
                                end: arm.body_span.end,
                            },
                            bindings,
                        });
                    }
                }
                _ => {}
            }
        }
    });
    scopes
}

fn pattern_bindings(pattern: &Pattern, out: &mut Vec<Span>) {
    match pattern {
        Pattern::Tags(alternatives) => {
            if let Some(first) = alternatives.first() {
                tag_bindings(first, out);
            }
        }
        Pattern::Instances(alternatives) => {
            for binding in alternatives
                .iter()
                .filter_map(|alternative| alternative.bindings.as_ref())
                .flatten()
            {
                binding_names(binding, out);
            }
        }
        Pattern::Wildcard | Pattern::Literals(_) => {}
    }
}

fn tag_bindings(pattern: &TagPattern, out: &mut Vec<Span>) {
    crate::stack::grow(|| {
        for binding in pattern.bindings.iter().flatten() {
            binding_names(binding, out);
        }
    });
}

fn binding_names(binding: &Binding, out: &mut Vec<Span>) {
    match &binding.nested {
        Some(nested) => tag_bindings(nested, out),
        None => out.push(binding.alias_span.unwrap_or(binding.name_span)),
    }
}

pub(crate) fn pipeline_shapes(program: &Program) -> Vec<PipelineShape> {
    let mut shapes = Vec::new();
    visit_programs(program, &mut |region| {
        for segment in &region.segments {
            if let Segment::Pipe(pipe) = segment {
                let end = pipe.steps.last().map_or(pipe.head_span.end, |s| s.span.end);
                shapes.push(PipelineShape {
                    head: (pipe.head_kind != PipeHeadKind::Flow).then_some(pipe.head_span),
                    span: Span {
                        start: pipe.head_span.start,
                        end,
                    },
                    first_call: pipe
                        .steps
                        .first()
                        .filter(|step| step.kind == PipeStepKind::Call)
                        .map(|step| step.span),
                });
            }
        }
    });
    shapes
}

/// Visits every recursively nested parse region exactly once.
///
/// Parser side tables use one structural traversal so adding a new nested
/// [`Program`] shape cannot make recovery and rollback collection drift.
pub(super) fn visit_programs(program: &Program, visit: &mut impl FnMut(&Program)) {
    crate::stack::grow(|| visit_programs_grown(program, visit));
}

fn visit_programs_grown(program: &Program, visit: &mut impl FnMut(&Program)) {
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
        for node in &program.recoveries {
            match &node.kind {
                RecoveryKind::MatchArms(arms) => out.extend(arms.iter().map(|span| RecoveryNode {
                    span: *span,
                    kind: RecoveryKind::ListElement,
                })),
                _ => out.push(node.clone()),
            }
        }
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
    /// The file's surface, for re-lexing a piece of it.
    pub source_kind: crate::SourceKind,
    host_owned_matches: Vec<Span>,
    /// Keyword offsets of parameter-shaped `val` candidates whose binding
    /// the host grammar does not read as a formal parameter, sorted.
    host_rejected_vals: Vec<usize>,
    flow_queries: crate::flow::FlowBodyQueries,
    passed_results: RefCell<HashSet<usize>>,
}

impl<'a> Parser<'a> {
    pub(super) fn new(src: &'a str) -> Self {
        Parser {
            src,
            bytes: src.as_bytes(),
            source_kind: crate::SourceKind::TypeScript,
            host_owned_matches: Vec::new(),
            host_rejected_vals: Vec::new(),
            flow_queries: crate::flow::FlowBodyQueries::default(),
            passed_results: RefCell::default(),
        }
    }
}

impl Parser<'_> {
    /// The modifier kind of the undotted identifier `val` at token index
    /// `idx`, recording a parameter-shaped one as a host candidate.
    fn val_modifier_at(
        &self,
        tokens: &[Token],
        idx: usize,
        candidates: &mut Vec<Span>,
    ) -> Option<ValModifierKind> {
        match vals::shape(self.src, tokens, idx)? {
            vals::ValShape::Declaration => Some(ValModifierKind::Declaration),
            vals::ValShape::Parameter => {
                let keyword = tokens[idx].span.start;
                if self.host_rejected_vals.binary_search(&keyword).is_ok() {
                    return None;
                }
                candidates.push(Span {
                    start: keyword,
                    end: tokens[idx + 1].span.start,
                });
                Some(ValModifierKind::Parameter)
            }
        }
    }

    fn host_owns_match_name(&self, candidate: Span) -> bool {
        let preceding = self
            .host_owned_matches
            .partition_point(|owned| owned.start <= candidate.start);
        preceding.checked_sub(1).is_some_and(|index| {
            let owned = self.host_owned_matches[index];
            candidate.end <= owned.end
        })
    }

    pub(super) fn body_diverges(&self, span: Span, tokens: &[Token], program: &Program) -> bool {
        self.flow_queries.diverges(self.src, span, tokens, program)
    }
}

enum ExprFrame {
    Resume((usize, bool)),
    StatementHeader,
    ForHeader(bool),
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
        Segment::ValModifier(modifier) => modifier.span.start,
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
/// point prevents recovery from consuming the next independent construct:
/// as in TypeScript, an expression ends at a statement boundary and before
/// a statement keyword, which cannot continue it.
fn recovery_expression_span(
    src: &str,
    tokens: &[Token],
    start_idx: usize,
    operator_idx: usize,
    range_end: usize,
) -> Span {
    let mut depth = 0usize;
    let mut recovery_end = tokens
        .get(operator_idx)
        .map_or(range_end, |token| token.span.end);
    for (idx, token) in tokens.iter().enumerate().skip(operator_idx + 1) {
        if depth == 0
            && (token.facts.boundary_before()
                || !cursor::dotted_at(tokens, operator_idx + 1, idx)
                    && crate::lexer::statement_keyword_at(src, tokens, idx)
                    && &src[token.span.start..token.span.end] != "try")
        {
            break;
        }
        match token.kind {
            _ if token.opens_bracket() => depth += 1,
            TokenKind::Punct(b')' | b']' | b'}') if depth == 0 => break,
            _ if token.closes_bracket() => depth -= 1,
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

/// A spread operand begins with three adjacent dot tokens. The last dot is
/// not member access, even though the generic property-name test sees it
/// immediately before the operand keyword.
/// Whether the token at `k` is a `match` keyword that may start a tt match:
/// undotted, or the operand of a spread, whose third dot is punctuation.
pub(super) fn match_keyword_at(src: &str, tokens: &[Token], k: usize) -> bool {
    let token = &tokens[k];
    matches!(token.kind, TokenKind::Ident)
        && &src[token.span.start..token.span.end] == "match"
        && (!cursor::dotted_at(tokens, 0, k) || follows_spread_operator(tokens, k))
}

fn follows_spread_operator(tokens: &[Token], idx: usize) -> bool {
    idx >= 3
        && tokens[idx - 3..idx]
            .iter()
            .all(|token| matches!(token.kind, TokenKind::Punct(b'.')))
}

/// Whether a parsed `match (...) { ... }` still overlaps a possible host
/// declaration position. The expression tracker already models where a host
/// operator or delimiter requires an operand; those positions need no second
/// parser. A preceding prefix, a statement start or member name (the
/// lexer's [`crate::lexer::TokenFacts`]), or a comma remains ambiguous and
/// is delegated to the host AST without enumerating TypeScript modifiers.
fn match_may_be_host_owned(tokens: &[Token], idx: usize, expr: (usize, bool)) -> bool {
    expr.0 < idx
        || tokens[idx].facts.statement_start()
        || tokens[idx].facts.member()
        || idx
            .checked_sub(1)
            .is_some_and(|previous| matches!(tokens[previous].kind, TokenKind::Punct(b',')))
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
        crate::stack::grow(|| self.parse_token_range(tokens, start, end, expression_root))
    }

    fn parse_token_range(
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
        let mut host_candidates = HostCandidates::default();
        let mut stray_pipes: Vec<usize> = Vec::new();
        let mut stray_if_lets: Vec<crate::ast::StrayIfLet> = Vec::new();
        let mut seg_start = start;
        let mut i = 0usize;

        // Expression-start tracking for pipeline heads: the index of the
        // token starting the expression currently being scanned, plus a
        // taint flag for unparenthesized ternary punctuation (a tainted
        // head aborts the claim — the normative "parenthesize ternaries"
        // rule). Brackets save and restore the enclosing expression's
        // state, so `f(a(b) |> g)` finds `a(b)`, not `b`.
        let mut expr: (usize, bool) = (0, false);
        let mut expr_stack: Vec<ExprFrame> = Vec::new();

        while i < tokens.len() {
            let tok = &tokens[i];
            if tok.facts.boundary_before() {
                expr = (i, false);
            }
            let word = match tok.kind {
                TokenKind::Template(ref parts) => {
                    flush_verbatim(&mut segments, seg_start, tok.span.start);
                    segments.push(Segment::Template(self.build_template(tok.span, parts)));
                    seg_start = tok.span.end;
                    i += 1;
                    continue;
                }
                TokenKind::PipeOp => {
                    if !expr.1
                        && expr.0 < i
                        && let Some(attempt) = pipes::parse_pipeline(self, tokens, expr.0, i, end)
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
                        if !pipes::asserted(self.src, tokens, expr.0, i) {
                            expr = (i, false);
                        }
                        continue;
                    }
                    stray_pipes.push(tok.span.start);
                    recoveries.push(RecoveryNode {
                        span: recovery_expression_span(self.src, tokens, expr.0.min(i), i, end),
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

            if !dotted && (word == "variant" || word == "export" || word == "declare") {
                let word_at = |k: usize| {
                    tokens
                        .get(k)
                        .filter(|t| matches!(t.kind, TokenKind::Ident))
                        .map(|t| &self.src[t.span.start..t.span.end])
                };
                let (kw_idx, exported, declared) = match word {
                    "variant" => (Some(i), false, false),
                    "declare"
                        if word_at(i + 1) == Some("variant")
                            && !tokens[i + 1].facts.line_break_before() =>
                    {
                        (Some(i + 1), false, true)
                    }
                    "export" if word_at(i + 1) == Some("variant") => (Some(i + 1), true, false),
                    "export"
                        if word_at(i + 1) == Some("declare")
                            && word_at(i + 2) == Some("variant")
                            && !tokens[i + 2].facts.line_break_before() =>
                    {
                        (Some(i + 2), true, true)
                    }
                    _ => (None, false, false),
                };
                if word == "export"
                    && word_at(i + 1) == Some("default")
                    && word_at(i + 2) == Some("variant")
                {
                    match variants::parse_default_variant(
                        Cursor::new(self, tokens, i + 3, end),
                        tok.span.start,
                    ) {
                        Claim::Malformed { error, recovery } => {
                            malformed.push(error);
                            recoveries.push(recovery);
                            i += 3;
                            continue;
                        }
                        Claim::Unclaimed(candidate) => {
                            unclaimed.push(candidate);
                            i += 3;
                            continue;
                        }
                        Claim::Parsed(never) => match never {},
                        Claim::NotTt => {}
                    }
                }
                if let Some(kw_idx) = kw_idx {
                    match variants::parse_variant(
                        Cursor::new(self, tokens, kw_idx + 1, end),
                        exported,
                        declared,
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
                            i = kw_idx + 1;
                            continue;
                        }
                        Claim::Unclaimed(candidate) => {
                            unclaimed.push(candidate);
                            i = kw_idx + 1;
                            continue;
                        }
                        Claim::NotTt => {}
                    }
                }
            }

            // Literal import / re-export / import-equals / ambient module
            // name of a relative tt path — only the specifier string is
            // lifted; the syntax around it stays verbatim.
            if !dotted
                && (word == "import" || word == "export" || word == "module")
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

            // A spread's third dot is punctuation in the host grammar, not
            // member access. Keep the same structural distinction used for
            // `try` so every spread-capable host can own a match operand.
            if match_keyword_at(self.src, tokens, i) && !self.host_owns_match_name(tok.span) {
                let host_ambiguous = match_may_be_host_owned(tokens, i, expr);
                match matches::parse_match(Cursor::new(self, tokens, i + 1, end), tok.span) {
                    Claim::Parsed((cur, byte_end, parsed)) => {
                        if host_ambiguous {
                            host_candidates.matches.push(Span {
                                start: tok.span.start,
                                end: byte_end,
                            });
                        }
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
                        host_candidates.malformed_matches.push(recovery.span);
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
                let misplaced = !tok.facts.member() && !tok.facts.statement_start();
                if misplaced
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
                        stmt.in_function = crate::flow::in_function_body(tokens, i);
                        flush_verbatim(&mut segments, seg_start, tok.span.start);
                        segments.push(Segment::Try(stmt));
                        seg_start = byte_end;
                        i = cur.idx;
                        continue;
                    }
                    Claim::Unclaimed(candidate) => unclaimed.push(candidate),
                    // A statement-position rejection also protects host member
                    // signatures. Only expression positions may claim TryExpr.
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
                    stmt.in_function = crate::flow::in_function_body(tokens, i);
                    let mut first = i;
                    while first > 0
                        && tokens[first - 1].span.start >= seg_start
                        && matches!(tokens[first - 1].kind, TokenKind::Ident)
                        && matches!(
                            &self.src[tokens[first - 1].span.start..tokens[first - 1].span.end],
                            "export" | "declare"
                        )
                        && !cursor::dotted_at(tokens, 0, first - 1)
                    {
                        first -= 1;
                    }
                    stmt.owner_span.start = tokens[first].span.start;
                    flush_verbatim(&mut segments, seg_start, stmt.owner_span.start);
                    segments.push(Segment::Try(stmt));
                    seg_start = byte_end;
                    i = cur.idx;
                    expr = (i, false);
                    continue;
                }
                if lets::let_else_pattern(self.src, tokens, i).is_some()
                    && let Some((cur, byte_end, mut stmt)) =
                        lets::parse_let_else(Cursor::new(self, tokens, i + 1, end), tok.span)
                {
                    stmt.in_function = crate::flow::in_function_body(tokens, i);
                    if !stmt.in_function
                        && i > 0
                        && tokens[i - 1].span.start >= seg_start
                        && matches!(tokens[i - 1].kind, TokenKind::Ident)
                        && &self.src[tokens[i - 1].span.start..tokens[i - 1].span.end] == "export"
                        && !cursor::dotted_at(tokens, 0, i - 1)
                    {
                        stmt.exported = true;
                        stmt.owner_span.start = tokens[i - 1].span.start;
                    }
                    flush_verbatim(&mut segments, seg_start, stmt.owner_span.start);
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
            if iflets::if_let_pattern(self.src, tokens, i).is_some() {
                let parsed = iflets::parse_if_let(Cursor::new(self, tokens, i + 1, end), tok.span);
                if let Ok((cur, byte_end, mut stmt)) = parsed {
                    stmt.in_function = crate::flow::in_function_body(tokens, i);
                    stmt.expression_position = !tok.facts.statement_start();
                    if stmt.expression_position {
                        recoveries.push(RecoveryNode {
                            span: stmt.owner_span,
                            kind: RecoveryKind::Expression,
                        });
                    }
                    let expression_position = stmt.expression_position;
                    flush_verbatim(&mut segments, seg_start, tok.span.start);
                    segments.push(Segment::IfLet(stmt));
                    seg_start = byte_end;
                    i = cur.idx;
                    if !expression_position {
                        expr = (i, false);
                    }
                    continue;
                }
                if let Err(stray) = parsed
                    && !stray_if_lets.contains(&stray)
                {
                    stray_if_lets.push(stray);
                }
                recoveries.extend(iflets::stray_if_let_recoveries(self.src, tokens, i, end));
            }

            // `result { ... }` is contextual: only a body with a nearest
            // direct tt `try` is claimed. Otherwise `result` remains an
            // ordinary identifier that a block statement may follow.
            if !dotted
                && word == "result"
                && matches!(tokens.get(i + 1), Some(t) if matches!(t.kind, TokenKind::Punct(b'{')))
                && !self
                    .passed_results
                    .borrow()
                    .contains(&tokens[i + 1].span.start)
            {
                match results::parse_result_block(Cursor::new(self, tokens, i + 1, end), tok.span) {
                    results::Attempt::Claimed(cur, byte_end, block) => {
                        flush_verbatim(&mut segments, seg_start, tok.span.start);
                        segments.push(Segment::ResultBlock(*block));
                        seg_start = byte_end;
                        i = cur.idx;
                        continue;
                    }
                    results::Attempt::Pass => {
                        self.passed_results
                            .borrow_mut()
                            .insert(tokens[i + 1].span.start);
                    }
                }
            }

            // `val` — a binding modifier, dropped from the output. Its
            // shapes are in `vals`; a parameter-shaped one stays a modifier
            // only when the host grammar reads its binding as a formal
            // parameter. Every other `val` is an ordinary identifier.
            if !dotted
                && word == "val"
                && let Some(kind) = self.val_modifier_at(tokens, i, &mut host_candidates.vals)
            {
                flush_verbatim(&mut segments, seg_start, tok.span.start);
                let end = vals::modifier_end(self.src, tok.span.end);
                segments.push(Segment::ValModifier(ValModifier {
                    span: Span {
                        start: tok.span.start,
                        end,
                    },
                    kind,
                }));
                seg_start = end;
                i += 1;
                continue;
            }

            let separates =
                word != "in" || matches!(expr_stack.last(), Some(ExprFrame::ForHeader(false)));
            if !dotted && separates && is_pipe_boundary_word(word) {
                expr = (i + 1, false);
            }
            i += 1;
        }

        flush_verbatim(&mut segments, seg_start, end);
        Program {
            span: Span { start, end },
            expression_root,
            segments,
            host_candidates: (!host_candidates.matches.is_empty()
                || !host_candidates.malformed_matches.is_empty()
                || !host_candidates.vals.is_empty())
            .then(|| Box::new(host_candidates)),
            unclaimed: (!unclaimed.is_empty()).then(|| Box::new(UnclaimedTtCandidates(unclaimed))),
            recoveries,
            malformed,
            stray_pipes,
            stray_if_lets,
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
        stack: &mut Vec<ExprFrame>,
    ) {
        let fresh = (i + 1, false);
        let restore = |frame: Option<ExprFrame>| match frame {
            Some(ExprFrame::Resume(outer)) => outer,
            Some(ExprFrame::StatementHeader | ExprFrame::ForHeader(_)) | None => fresh,
        };
        match tok.kind {
            TokenKind::JsxRaw => *expr = fresh,
            TokenKind::Punct(b'(') if self.opens_statement_header(tokens, i) => {
                stack.push(if self.opens_for_header(tokens, i) {
                    ExprFrame::ForHeader(false)
                } else {
                    ExprFrame::StatementHeader
                });
                *expr = fresh;
            }
            _ if tok.opens_bracket() => {
                stack.push(ExprFrame::Resume(*expr));
                *expr = fresh;
            }
            TokenKind::Punct(b'}') => {
                let outer = restore(stack.pop());
                *expr = if self.brace_ends_expression(tokens, i) {
                    outer
                } else {
                    (i + 1, false)
                };
            }
            _ if tok.closes_bracket() => {
                *expr = restore(stack.pop());
            }
            TokenKind::Punct(b';') => {
                if let Some(ExprFrame::ForHeader(initialized)) = stack.last_mut() {
                    *initialized = true;
                }
                *expr = (i + 1, false);
            }
            TokenKind::Punct(b',') => *expr = (i + 1, false),
            TokenKind::Punct(b'=') if pipes::is_assignment_eq(self.bytes, tok.span) => {
                *expr = (i + 1, false);
            }
            TokenKind::Punct(b'*')
                if i.checked_sub(1)
                    .and_then(|previous| tokens.get(previous))
                    .is_some_and(|previous| {
                        matches!(previous.kind, TokenKind::Ident)
                            && &self.src[previous.span.start..previous.span.end] == "yield"
                    }) =>
            {
                // `yield*` is one prefix host operator. The delegated value,
                // not the `*`, starts a pipeline head.
                *expr = (i + 1, false);
            }
            TokenKind::Punct(b':') => *expr = (i + 1, expr.1),
            TokenKind::Punct(b'?') => *expr = (i + 1, true),
            TokenKind::Arrow => *expr = (i + 1, false),
            _ => {}
        }
    }

    fn opens_for_header(&self, tokens: &[Token], open_idx: usize) -> bool {
        let word_at = |k: usize| {
            tokens
                .get(k)
                .filter(|t| matches!(t.kind, TokenKind::Ident))
                .map(|t| &self.src[t.span.start..t.span.end])
        };
        match open_idx.checked_sub(1).and_then(word_at) {
            Some("for") => true,
            Some("await") => open_idx
                .checked_sub(2)
                .is_some_and(|k| word_at(k) == Some("for")),
            _ => false,
        }
    }

    fn opens_statement_header(&self, tokens: &[Token], open_idx: usize) -> bool {
        let word_at = |k: usize| {
            tokens
                .get(k)
                .filter(|t| matches!(t.kind, TokenKind::Ident) && !cursor::dotted_at(tokens, 0, k))
                .map(|t| &self.src[t.span.start..t.span.end])
        };
        let Some(keyword_idx) = open_idx.checked_sub(1) else {
            return false;
        };
        match word_at(keyword_idx) {
            Some("if" | "while" | "for" | "with") => true,
            Some("await") => keyword_idx
                .checked_sub(1)
                .is_some_and(|k| word_at(k) == Some("for")),
            _ => false,
        }
    }

    /// True when the token after the `}` at `close_idx` continues the
    /// surrounding expression (`.m`, `)`, an operator, `|>`, ...) rather
    /// than starting a new statement: no statement boundary
    /// ([`crate::lexer::TokenFacts::boundary_before`]) separates them.
    pub(super) fn brace_ends_expression(&self, tokens: &[Token], close_idx: usize) -> bool {
        tokens
            .get(close_idx + 1)
            .is_none_or(|next| !next.facts.boundary_before())
    }

    /// Turns a lexed template token into the AST template, recursively
    /// parsing each interpolation's token stream.
    fn build_template(&self, span: Span, parts: &[TplPart]) -> Template {
        let chunks = parts
            .iter()
            .map(|part| match part {
                TplPart::Raw(span) => TemplateChunk::Raw(*span),
                TplPart::Interp { span, tokens } => TemplateChunk::Interp(
                    self.parse_tokens_with_context(tokens, span.start, span.end, true),
                ),
            })
            .collect();
        Template { span, chunks }
    }
}
