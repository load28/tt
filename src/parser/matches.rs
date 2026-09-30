//! Structural parsing of tt `match` expressions.
//!
//! Purely structural: anything that does not fully parse as a tt match (a
//! method named `match`, `String.prototype.match` calls, ...) returns `None`
//! and passes through verbatim. tt-level errors — duplicate arms, a
//! misplaced wildcard, non-exhaustiveness — are the semantic phase's job.
//! The scrutinee and every arm body are recursively parsed sub-programs.

use super::Claim;
use super::cursor::{Cursor, find_close_at};
use super::is_reserved;
use super::literals::{at_literal, parse_literal_alternatives};
use crate::ast::{
    Arm, ArmsTail, Binding, GuardExpr, InstancePattern, MatchExpr, Pattern, RecoveryKind,
    RecoveryNode, Span, TagPattern, TupleArm, TupleMatchExpr, TuplePattern,
};
use crate::lexer::{Token, TokenKind};

/// What [`parse_match`] found: a single match or a tuple match. The arms
/// decide — a tuple match needs every arm to be a parenthesized tuple
/// pattern (or a final bare `_`) *and* the scrutinee to split at top-level
/// commas, so `match (a, b) { Tag => ... }` keeps meaning a single match
/// over a comma expression, exactly as before tuple matches existed.
pub(super) enum ParsedMatch {
    Single(MatchExpr),
    Tuple(TupleMatchExpr),
}

/// `cur` is positioned just past the `match` keyword (`kw_span`). On
/// success returns the advanced cursor, the byte just past the closing
/// brace, and the parsed expression.
pub(super) fn parse_match<'t>(
    cur: Cursor<'t>,
    kw_span: Span,
) -> Claim<(Cursor<'t>, usize, ParsedMatch)> {
    if let Some(parsed) = parse_match_complete(cur, kw_span, false) {
        return Claim::Parsed(parsed);
    }
    let body_open = match_body_open(cur.tokens, cur.idx);
    if body_open.is_some_and(|open| body_reads_as_arms(cur.parser.src, cur.tokens, open)) {
        if let Some(parsed) = parse_match_complete(cur, kw_span, true) {
            return Claim::Parsed(parsed);
        }
        let end = body_open
            .and_then(|open| super::cursor::find_close_at(cur.tokens, open))
            .and_then(|close| cur.tokens.get(close))
            .map_or(cur.range_end, |token| token.span.end);
        let value_element = body_open.and_then(|open| tuple_value_element(&cur, open));
        if let Some(arms) = recover_match_arms(cur).filter(|arms| !arms.is_empty()) {
            let first = arms[0];
            return Claim::Malformed {
                error: match value_element {
                    Some(element) => element.error(),
                    None => crate::error::TtError::span(
                        first.start,
                        first.end,
                        "invalid match arm: expected `<pattern> => <body>`".to_string(),
                    )
                    .code(crate::DiagnosticCode::MalformedMatch),
                },
                recovery: RecoveryNode {
                    span: Span {
                        start: kw_span.start,
                        end,
                    },
                    kind: RecoveryKind::MatchArms(arms),
                },
            };
        }
        let mut error = crate::error::TtError::span(
            kw_span.start,
            kw_span.end,
            "tt `match` could not be parsed".to_string(),
        )
        .code(crate::DiagnosticCode::MalformedMatch);
        // A scrutinee that starts with a bare identifier is the one shape
        // whose fix the parser can write: the text between the keyword and
        // the body is the expression, and it only needs parentheses around
        // it. Anything else gets the rule's form, which is advice.
        error = match (cur.peek(), body_open) {
            (_, Some(open)) if has_instance_call_pattern(cur.parser.src, cur.tokens, open) => {
                error.help("write property bindings with braces — `is Type { field }`")
            }
            (Some(token), Some(open))
                if matches!(token.kind, TokenKind::Ident)
                    && token.span.start < cur.tokens[open].span.start =>
            {
                let scrutinee = cur.parser.src[token.span.start..cur.tokens[open].span.start]
                    .trim_end()
                    .to_string();
                error.suggest(
                    "a match scrutinee is parenthesized — `match (<expression>) { ... }`",
                    token.span.start,
                    token.span.start + scrutinee.len(),
                    format!("({scrutinee})"),
                )
            }
            _ => match value_element {
                Some(element) => element.error(),
                None => error.help("write `match (<scrutinee>) { <pattern> => <body> }`"),
            },
        };
        Claim::Malformed {
            error,
            recovery: RecoveryNode {
                span: Span {
                    start: kw_span.start,
                    end,
                },
                kind: RecoveryKind::Expression,
            },
        }
    } else {
        Claim::NotTt
    }
}

/// The token index of the `{` that opens a match body, for the `match`
/// keyword whose next token is at `after_keyword`: the brace right after a
/// parenthesized scrutinee, `match (…) {`, or the first brace after an
/// identifier scrutinee, `match user {`, which the parser commits to and
/// reports with a fix.
pub(super) fn match_body_open(tokens: &[Token], after_keyword: usize) -> Option<usize> {
    match tokens.get(after_keyword)?.kind {
        TokenKind::Punct(b'(') => {
            let open = find_close_at(tokens, after_keyword)? + 1;
            opens_match_body(tokens.get(open)?).then_some(open)
        }
        TokenKind::Ident => (after_keyword..tokens.len())
            .find(|&index| matches!(tokens[index].kind, TokenKind::Punct(b'{')))
            .filter(|&index| opens_match_body(&tokens[index])),
        _ => None,
    }
}

/// Whether `token`, after a match head, opens the match's body.
///
/// Only a brace on the head's line does. A line terminator before the brace
/// ends the head: `match(x)` is then a complete call, which no production
/// continues with `{`, so TypeScript inserts a semicolon (ECMA-262
/// §12.10.1) and the brace opens a block statement. The lexer's token facts
/// read a match head the same way (TASK-491).
pub(super) fn opens_match_body(token: &Token) -> bool {
    matches!(token.kind, TokenKind::Punct(b'{')) && !token.facts.line_break_before()
}

fn has_instance_call_pattern(src: &str, tokens: &[Token], body_open: usize) -> bool {
    let mut index = body_open + 1;
    while index + 2 < tokens.len() {
        let token = &tokens[index];
        if matches!(token.kind, TokenKind::Ident)
            && &src[token.span.start..token.span.end] == "is"
            && matches!(tokens[index + 1].kind, TokenKind::Ident)
        {
            let mut tail = index + 2;
            while tail + 1 < tokens.len()
                && matches!(tokens[tail].kind, TokenKind::Punct(b'.'))
                && matches!(tokens[tail + 1].kind, TokenKind::Ident)
            {
                tail += 2;
            }
            return tokens
                .get(tail)
                .is_some_and(|token| matches!(token.kind, TokenKind::Punct(b'(')));
        }
        if matches!(token.kind, TokenKind::Arrow | TokenKind::Punct(b'}')) {
            break;
        }
        index += 1;
    }
    false
}

#[derive(Clone, Copy)]
enum TupleValueElement {
    Literal(Span),
    Instance(Span),
}

impl TupleValueElement {
    fn error(self) -> crate::error::TtError {
        let (span, what) = match self {
            TupleValueElement::Literal(span) => (span, "a literal pattern"),
            TupleValueElement::Instance(span) => (span, "an `is` pattern"),
        };
        crate::error::TtError::span(
            span.start,
            span.end,
            format!("{what} cannot be a tuple pattern element"),
        )
        .code(crate::DiagnosticCode::MalformedMatch)
        .help(
            "tuple pattern elements are tag patterns or `_`; test this value in an arm \
             guard or a nested `match`",
        )
    }
}

fn tuple_value_element(cur: &Cursor, body_open: usize) -> Option<TupleValueElement> {
    let close = super::cursor::find_close_at(cur.tokens, body_open)?;
    let body = cur.sub(body_open + 1, close, cur.tokens[close].span.start);
    let mut idx = 0;
    let mut arm_start = true;
    while let Some(token) = body.tokens.get(idx) {
        match token.kind {
            TokenKind::Punct(b',') => {
                idx += 1;
                arm_start = true;
                continue;
            }
            TokenKind::Punct(delimiter) if token.opens_bracket() => {
                let end = super::cursor::find_close_at(body.tokens, idx)?;
                if arm_start
                    && delimiter == b'('
                    && let Some(element) =
                        value_element(body.sub(idx + 1, end, body.tokens[end].span.start))
                {
                    return Some(element);
                }
                idx = end + 1;
            }
            _ => idx += 1,
        }
        arm_start = false;
    }
    None
}

fn value_element(elements: Cursor) -> Option<TupleValueElement> {
    let tokens = elements.tokens;
    let mut ranges = Vec::new();
    let mut from = 0;
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            _ if token.opens_bracket() => depth += 1,
            _ if token.closes_bracket() => depth = depth.saturating_sub(1),
            TokenKind::Punct(b',') if depth == 0 => {
                ranges.push((from, index));
                from = index + 1;
            }
            _ => {}
        }
    }
    if ranges.is_empty() {
        return None;
    }
    ranges.push((from, tokens.len()));
    ranges
        .into_iter()
        .filter(|(from, to)| from < to)
        .find_map(|(from, to)| {
            let span = Span {
                start: tokens[from].span.start,
                end: tokens[to - 1].span.end,
            };
            let element = elements.sub(from, to, span.end);
            let complete = |mut cur: Cursor, parse: fn(&mut Cursor) -> bool| {
                parse(&mut cur) && cur.peek().is_none()
            };
            if at_literal(&element)
                && complete(element, |cur| parse_literal_alternatives(cur).is_some())
            {
                Some(TupleValueElement::Literal(span))
            } else if complete(element, |cur| parse_instance_alternatives(cur).is_some()) {
                Some(TupleValueElement::Instance(span))
            } else {
                None
            }
        })
}

/// Whether the braces opening at `open` hold **arms** rather than
/// statements — the one question that separates a near-miss tt match from
/// TypeScript that merely looks like one.
///
/// `match` is not a reserved word, so it is an ordinary name in TypeScript:
/// a method (`class C { match(x) { ... } }`), a binding (`for (const match
/// of xs)`), a function. Every one of those is `match`, something, and a
/// block — the same silhouette a match has. What is *not* the same is what
/// the block contains, and that is where the answer has to come from
/// (TASK-229).
///
/// An arm list is `<pattern> => <body>`, comma-separated. So at the body's
/// own brace level:
///
/// - the first token cannot be a statement keyword (`return`, `const`, …),
///   because a pattern is a tag, a literal, `_`, or a tuple. `true` and
///   `false` are reserved words *and* literal patterns, so they stay.
/// - a `=>` must appear before any `;`, because a statement list separates
///   with semicolons and an arm list never reaches one at its own level.
///
/// Depth matters: the arrow in `return f(y => y)` belongs to a call, not to
/// the block, which is exactly why "contains an arrow anywhere" claimed
/// every un-annotated method whose body happened to use one.
fn body_reads_as_arms(src: &str, tokens: &[Token], open: usize) -> bool {
    let Some(first) = tokens.get(open + 1) else {
        return false;
    };
    if matches!(first.kind, TokenKind::Ident) {
        let word = &src[first.span.start..first.span.end];
        if is_reserved(word) && word != "true" && word != "false" {
            return false;
        }
    }
    let mut depth = 0usize;
    for token in &tokens[open + 1..] {
        match token.kind {
            _ if token.opens_bracket() => depth += 1,
            TokenKind::Punct(b'}') => {
                if depth == 0 {
                    return false; // the body ended with no arm in it
                }
                depth -= 1;
            }
            _ if token.closes_bracket() => depth = depth.saturating_sub(1),
            TokenKind::Punct(b';') if depth == 0 => return false,
            TokenKind::Arrow if depth == 0 => return true,
            _ => {}
        }
    }
    false
}

fn parse_match_complete<'t>(
    mut cur: Cursor<'t>,
    kw_span: Span,
    open_arms: bool,
) -> Option<(Cursor<'t>, usize, ParsedMatch)> {
    if !cur.at_punct(b'(') {
        return None;
    }
    let open = cur.idx;
    let close = cur.find_close()?;
    let scrutinee_span = Span {
        start: cur.tokens[open].span.start + 1,
        end: cur.tokens[close].span.start,
    };
    if cur.parser.src[scrutinee_span.start..scrutinee_span.end]
        .trim()
        .is_empty()
    {
        return None;
    }
    cur.idx = close + 1;

    if !cur.peek().is_some_and(opens_match_body) {
        return None;
    }
    let body_open = cur.idx;
    let body_close = cur.find_close()?;
    let byte_end = cur.tokens[body_close].span.end;
    let arms_cur = cur.sub(body_open + 1, body_close, cur.tokens[body_close].span.start);

    // Tuple attempt first (arm-driven). One side may have arity one when
    // the other proves tuple intent, so sema can report the exact mismatch.
    if let Some(parts) = split_scrutinees(&cur, open, close)
        && let Some(arms) = parse_tuple_arms(arms_cur, open_arms)
        && !arms.is_empty()
        && (parts.len() > 1
            || arms
                .iter()
                .any(|arm| matches!(&arm.pattern, TuplePattern::Elems(elems) if elems.len() > 1)))
    {
        let scrutinees = parts
            .iter()
            .map(|&(from, to)| {
                let span = Span {
                    start: cur.tokens[from].span.start,
                    end: cur.tokens[to - 1].span.end,
                };
                let program =
                    cur.parser
                        .parse_expression_tokens(&cur.tokens[from..to], span.start, span.end);
                (span, program)
            })
            .collect();
        cur.idx = body_close + 1;
        return Some((
            cur,
            byte_end,
            ParsedMatch::Tuple(TupleMatchExpr {
                keyword_off: kw_span.start,
                body_open: cur.tokens[body_open].span.start,
                body_close: cur.tokens[body_close].span.start,
                scrutinees,
                tail: arms_tail(&arms_cur, arms.last()?.pattern_span.start)?,
                arms: arms
                    .into_iter()
                    .map(|arm| arm.into_tuple_arm(cur.parser))
                    .collect(),
            }),
        ));
    }

    let arms = match parse_arms(arms_cur, open_arms) {
        Some(arms) if !arms.is_empty() => arms,
        _ => return None,
    };

    let scrutinee = cur.parser.parse_expression_tokens(
        &cur.tokens[open + 1..close],
        scrutinee_span.start,
        scrutinee_span.end,
    );
    cur.idx = body_close + 1;
    Some((
        cur,
        byte_end,
        ParsedMatch::Single(MatchExpr {
            keyword_off: kw_span.start,
            body_open: cur.tokens[body_open].span.start,
            body_close: cur.tokens[body_close].span.start,
            scrutinee_span,
            scrutinee,
            tail: arms_tail(&arms_cur, arms.last()?.pattern_span.start)?,
            arms: arms
                .into_iter()
                .map(|arm| arm.into_arm(cur.parser))
                .collect(),
        }),
    ))
}

/// Splits the scrutinee token range `(open..close)` at top-level commas
/// into `(from, to)` token ranges. `None` means an empty part; one part is
/// retained for tuple-arity recovery. Type arguments are a bracket pair
/// where the token facts say so (TASK-495); a comparison operator stays an
/// ordinary expression token and does not hide the tuple comma.
fn split_scrutinees(cur: &Cursor, open: usize, close: usize) -> Option<Vec<(usize, usize)>> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut from = open + 1;
    for k in open + 1..close {
        match cur.tokens[k].kind {
            _ if cur.tokens[k].opens_bracket() => depth += 1,
            _ if cur.tokens[k].closes_bracket() => depth = depth.saturating_sub(1),
            TokenKind::Punct(b',') if depth == 0 => {
                if k == from {
                    return None; // empty part
                }
                parts.push((from, k));
                from = k + 1;
            }
            _ => {}
        }
    }
    if from >= close {
        return None; // trailing comma leaves an empty last part
    }
    parts.push((from, close));
    Some(parts)
}

/// Where an arm is in the arm grammar: its pattern, its `if` guard, or its
/// body after `=>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ArmPart {
    Pattern,
    Guard,
    Body,
}

/// One arm of a match body as the arm grammar delimits it, before and
/// whether or not the arm itself parses. Indices are into the body's
/// tokens: `start` is the arm's first token, `guard` its top-level `if`,
/// `arrow` its top-level `=>`, and `end` the `,` that separates it from the
/// next arm, or the body's length for the last arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ArmOutline {
    pub(super) start: usize,
    pub(super) guard: Option<usize>,
    pub(super) arrow: Option<usize>,
    pub(super) end: usize,
}

impl ArmOutline {
    /// The part of the arm the token at `index` is in.
    pub(super) fn part_at(&self, index: usize) -> ArmPart {
        if self.arrow.is_some_and(|arrow| arrow < index) {
            ArmPart::Body
        } else if self.guard.is_some_and(|guard| guard < index) {
            ArmPart::Guard
        } else {
            ArmPart::Pattern
        }
    }

    /// The token just past the arm's pattern.
    pub(super) fn pattern_end(&self) -> usize {
        self.guard.or(self.arrow).unwrap_or(self.end)
    }
}

/// The arms of a match body, `tokens` being the text between its braces —
/// or up to the end of the input while the closing brace is unwritten.
///
/// This is the one walk that delimits arms, for the parser's strict and
/// recovering arm lists and for any reader of an unfinished body. An arm is
/// a pattern, an optional `if` guard, `=>`, and a body; it ends at the next
/// `,` outside every bracket, whether or not it reached its `=>`. A bracket
/// left open runs to the end, so the text inside a pattern or an argument
/// list being typed stays in its arm. A `,` after the last arm leaves an
/// empty final arm, the slot where the next one would be written.
pub(super) fn outline_arms(src: &str, tokens: &[Token]) -> Vec<ArmOutline> {
    let open_arm = |start| ArmOutline {
        start,
        guard: None,
        arrow: None,
        end: tokens.len(),
    };
    let mut arms = Vec::new();
    let mut arm = open_arm(0);
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            _ if token.opens_bracket() => depth += 1,
            _ if token.closes_bracket() => depth = depth.saturating_sub(1),
            _ if depth > 0 => {}
            TokenKind::Punct(b',') => {
                arm.end = index;
                arms.push(arm);
                arm = open_arm(index + 1);
            }
            TokenKind::Arrow if arm.arrow.is_none() => arm.arrow = Some(index),
            TokenKind::Ident
                if arm.arrow.is_none()
                    && arm.guard.is_none()
                    && &src[token.span.start..token.span.end] == "if" =>
            {
                arm.guard = Some(index);
            }
            _ => {}
        }
    }
    arms.push(arm);
    arms
}

/// The arms of the list under `cur`, each with a cursor over its own tokens
/// whose open-ended scans stop where its separator starts. The empty slot
/// after a final `,` is not an arm.
fn list_arms<'t>(cur: &Cursor<'t>) -> impl Iterator<Item = (ArmOutline, Cursor<'t>)> {
    let list = cur.sub(cur.idx, cur.tokens.len(), cur.range_end);
    outline_arms(cur.parser.src, list.tokens)
        .into_iter()
        .filter(move |arm| arm.start < list.tokens.len())
        .map(move |arm| {
            (
                arm,
                list.sub(arm.start, arm.end, list.stop_byte_at(arm.end)),
            )
        })
}

/// Parses one arm from its own tokens: the arm grammar must consume them
/// all.
fn parse_whole_arm<'t, T>(
    mut cur: Cursor<'t>,
    parse: fn(&mut Cursor<'t>) -> Option<T>,
) -> Option<T> {
    parse(&mut cur).filter(|_| cur.peek().is_none())
}

/// Parse independently recoverable list elements. A failed arm owns the
/// bytes [`outline_arms`] gives it, up to and including its separator.
fn parse_arm_list<'t, T>(
    cur: Cursor<'t>,
    parse: fn(&mut Cursor<'t>) -> Option<T>,
) -> (Vec<T>, Vec<Span>) {
    let list = cur.sub(cur.idx, cur.tokens.len(), cur.range_end);
    let mut arms = Vec::new();
    let mut errors = Vec::new();
    for (outline, arm) in list_arms(&list) {
        match parse_whole_arm(arm, parse) {
            Some(parsed) => arms.push(parsed),
            None => errors.push(Span {
                start: list.stop_byte_at(outline.start),
                end: list.stop_byte_at((outline.end + 1).min(list.tokens.len())),
            }),
        }
    }
    (arms, errors)
}

/// Recovery is admitted only inside a structurally committed match with a
/// complete scrutinee and body, and only when at least one arm fully parses.
fn recover_match_arms(mut cur: Cursor) -> Option<Vec<Span>> {
    if !cur.at_punct(b'(') {
        return None;
    }
    let close = cur.find_close()?;
    if close == cur.idx + 1 {
        return None;
    }
    cur.idx = close + 1;
    if !cur.at_punct(b'{') {
        return None;
    }
    let close = cur.find_close()?;
    let body = cur.sub(cur.idx + 1, close, cur.tokens[close].span.start);
    let (single, single_errors) = parse_arm_list(body, parse_open_arm);
    let (tuple, tuple_errors) = parse_arm_list(body, parse_open_tuple_arm);
    // A bare wildcard belongs to both grammars and supplies no evidence of
    // either form. Only discriminating patterns choose the recovery grammar.
    let single_form = single
        .iter()
        .any(|arm| !matches!(arm.pattern, Pattern::Wildcard));
    let tuple_form = tuple
        .iter()
        .any(|arm| !matches!(arm.pattern, TuplePattern::Wildcard));
    match (single_form, tuple_form) {
        (true, false) => Some(single_errors),
        (false, true) => Some(tuple_errors),
        _ => None,
    }
}

/// Strict recognition never recovers past a rejected candidate. Arm bodies
/// remain token slices until the enclosing match selects a complete grammar.
fn parse_strict_arm_list<'t, T>(
    cur: Cursor<'t>,
    parse: fn(&mut Cursor<'t>) -> Option<T>,
) -> Option<Vec<T>> {
    list_arms(&cur)
        .map(|(_, arm)| parse_whole_arm(arm, parse))
        .collect()
}

/// Where a fully parsed arm list ends. The list's tokens are exactly the
/// arms and their separators, so its last token is either the last arm's
/// final token or the comma after it.
fn arms_tail(arms: &Cursor, last_start: usize) -> Option<ArmsTail> {
    let separated = arms
        .tokens
        .last()
        .is_some_and(|t| matches!(t.kind, TokenKind::Punct(b',')));
    let last = arms.tokens.len().checked_sub(1 + usize::from(separated))?;
    Some(ArmsTail {
        last_start,
        last_end: arms.tokens[last].span.end,
        separated,
    })
}

fn parse_arms(cur: Cursor<'_>, open_arms: bool) -> Option<Vec<ArmSyntax<'_, Pattern>>> {
    parse_strict_arm_list(cur, if open_arms { parse_open_arm } else { parse_arm })
}

fn parse_arm<'t>(cur: &mut Cursor<'t>) -> Option<ArmSyntax<'t, Pattern>> {
    parse_single_arm(cur, false)
}

fn parse_open_arm<'t>(cur: &mut Cursor<'t>) -> Option<ArmSyntax<'t, Pattern>> {
    parse_single_arm(cur, true)
}

fn parse_single_arm<'t>(cur: &mut Cursor<'t>, open: bool) -> Option<ArmSyntax<'t, Pattern>> {
    let pattern_start = cur.peek()?.span.start;
    let pattern = parse_arm_pattern(cur)?;
    let pattern_end = cur.tokens.get(cur.idx.checked_sub(1)?)?.span.end;

    // Only tag and literal patterns take a guard — `_ if` never parses,
    // so it passes through.
    let allow_guard = !matches!(pattern, Pattern::Wildcard);
    let tail = parse_arm_tail(cur, allow_guard, open)?;

    Some(ArmSyntax {
        pattern,
        pattern_span: Span {
            start: pattern_start,
            end: pattern_end,
        },
        tail,
    })
}

/// Parses a single match arm's pattern at the cursor: `_`, literal
/// alternatives, `is` alternatives, or tag alternatives.
pub(super) fn parse_arm_pattern(cur: &mut Cursor) -> Option<Pattern> {
    let first = cur.peek()?;
    Some(match first.kind {
        TokenKind::Ident if cur.text(first) == "_" => {
            cur.bump();
            Pattern::Wildcard
        }
        _ if at_literal(cur) => Pattern::Literals(parse_literal_alternatives(cur)?),
        TokenKind::Ident if cur.text(first) == "is" => {
            Pattern::Instances(parse_instance_alternatives(cur)?)
        }
        TokenKind::Ident => Pattern::Tags(parse_tag_alternatives(cur)?),
        _ => return None,
    })
}

/// Parses `is Type (| is Type)*`. A constructor is an identifier or dotted
/// identifier path. Property bindings use braces and never contain nested
/// patterns in this first class-pattern surface.
fn parse_instance_alternatives(cur: &mut Cursor) -> Option<Vec<InstancePattern>> {
    let mut alternatives = vec![parse_instance_pattern(cur)?];
    while cur.at_punct(b'|') {
        cur.bump();
        let token = cur.peek()?;
        if !matches!(token.kind, TokenKind::Ident) || cur.text(token) != "is" {
            return None;
        }
        alternatives.push(parse_instance_pattern(cur)?);
    }
    Some(alternatives)
}

fn parse_instance_pattern(cur: &mut Cursor) -> Option<InstancePattern> {
    let is_token = cur.peek()?;
    if !matches!(is_token.kind, TokenKind::Ident) || cur.text(is_token) != "is" {
        return None;
    }
    let is_off = is_token.span.start;
    cur.bump();

    let (first, first_span) = cur.eat_ident()?;
    if is_reserved(first) {
        return None;
    }
    let mut path = first.to_string();
    let mut path_end = first_span.end;
    while cur.at_punct(b'.') {
        cur.bump();
        let (part, part_span) = cur.eat_ident()?;
        path.push('.');
        path.push_str(part);
        path_end = part_span.end;
    }

    let mut bindings = None;
    let mut list = None;
    let mut end = path_end;
    if cur.at_punct(b'{') {
        let open = cur.idx;
        let close = cur.find_close()?;
        bindings = Some(parse_bindings(
            cur.sub(open + 1, close, cur.tokens[close].span.start),
            false,
        )?);
        end = cur.tokens[close].span.end;
        list = Some(Span {
            start: cur.tokens[open].span.start,
            end,
        });
        cur.idx = close + 1;
    }
    Some(InstancePattern {
        path,
        path_span: Span {
            start: first_span.start,
            end: path_end,
        },
        is_off,
        end,
        bindings,
        list,
    })
}

/// Parses tuple arms: `(elem, elem, ...) (if guard)? => body` with an
/// optional final bare `_` arm. `None` unless *every* arm has that shape —
/// the caller then falls back to single-match arms.
fn parse_tuple_arms(cur: Cursor<'_>, open_arms: bool) -> Option<Vec<ArmSyntax<'_, TuplePattern>>> {
    parse_strict_arm_list(
        cur,
        if open_arms {
            parse_open_tuple_arm
        } else {
            parse_tuple_arm
        },
    )
}

fn parse_tuple_arm<'t>(cur: &mut Cursor<'t>) -> Option<ArmSyntax<'t, TuplePattern>> {
    parse_tuple_arm_with(cur, false)
}

fn parse_open_tuple_arm<'t>(cur: &mut Cursor<'t>) -> Option<ArmSyntax<'t, TuplePattern>> {
    parse_tuple_arm_with(cur, true)
}

fn parse_tuple_arm_with<'t>(
    cur: &mut Cursor<'t>,
    open: bool,
) -> Option<ArmSyntax<'t, TuplePattern>> {
    let first = cur.peek()?;
    let pattern_start = first.span.start;

    let pattern = match first.kind {
        TokenKind::Ident if cur.text(first) == "_" => {
            cur.bump();
            TuplePattern::Wildcard
        }
        TokenKind::Punct(b'(') => {
            let open = cur.idx;
            let close = cur.find_close()?;
            let elems = parse_tuple_elems(cur.sub(open + 1, close, cur.tokens[close].span.start))?;
            cur.idx = close + 1;
            TuplePattern::Elems(elems)
        }
        _ => return None,
    };
    let pattern_end = cur.tokens.get(cur.idx.checked_sub(1)?)?.span.end;

    let allow_guard = matches!(pattern, TuplePattern::Elems(_));
    let tail = parse_arm_tail(cur, allow_guard, open)?;

    Some(ArmSyntax {
        pattern,
        pattern_span: Span {
            start: pattern_start,
            end: pattern_end,
        },
        tail,
    })
}

/// Parses the comma-separated element patterns between a tuple pattern's
/// parens: each element a tag pattern (or-alternatives included) or `_`.
fn parse_tuple_elems(mut cur: Cursor) -> Option<Vec<Pattern>> {
    let mut elems = Vec::new();
    loop {
        let first = cur.peek()?;
        let pat = match first.kind {
            TokenKind::Ident if cur.text(first) == "_" => {
                cur.bump();
                Pattern::Wildcard
            }
            TokenKind::Ident => Pattern::Tags(parse_tag_alternatives(&mut cur)?),
            _ => return None,
        };
        elems.push(pat);
        if cur.peek().is_none() {
            break;
        }
        cur.eat_punct(b',')?;
        if cur.peek().is_none() {
            break; // trailing comma
        }
    }
    Some(elems)
}

/// Parses `Tag (| Tag)*` alternatives starting at the identifier under the
/// cursor. `||` lexes as a single OrOr token, so it can never be an
/// alternative separator — the candidate then fails the parse.
fn parse_tag_alternatives(cur: &mut Cursor) -> Option<Vec<TagPattern>> {
    let mut alts = vec![parse_tag_pattern(cur)?];
    while cur.at_punct(b'|') {
        cur.bump();
        alts.push(parse_tag_pattern(cur)?);
    }
    Some(alts)
}

/// Parses everything after an arm's pattern: an optional `if <cond>` guard
/// (refused when `allow_guard` is false), the `=>`, and the expression or
/// block body. Shared between single-match and tuple-match arms.
#[allow(clippy::type_complexity)]
fn parse_arm_tail<'t>(
    cur: &mut Cursor<'t>,
    allow_guard: bool,
    open: bool,
) -> Option<ArmTailSyntax<'t>> {
    let mut guard = None;
    if allow_guard
        && matches!(cur.peek(), Some(t) if matches!(t.kind, TokenKind::Ident) && cur.text(t) == "if")
    {
        cur.bump();
        let g_start = cur.stop_byte_at(cur.idx);
        let (arrow_idx, g_end) = match guard_end(cur) {
            Some(end) => end,
            None if open && guard_runs_to_end(cur) => (
                cur.tokens.len(),
                cur.tokens.last().map_or(g_start, |t| t.span.end),
            ),
            None => return None,
        };
        if cur.parser.src[g_start..g_end].trim().is_empty() {
            return None;
        }
        guard = Some((
            Span {
                start: g_start,
                end: g_end,
            },
            &cur.tokens[cur.idx..arrow_idx],
        ));
        cur.idx = arrow_idx;
    }

    let missing = |cur: &Cursor<'t>, guard| {
        let at = cur
            .idx
            .checked_sub(1)
            .and_then(|previous| cur.tokens.get(previous))
            .map_or(cur.range_end, |t| t.span.end);
        Some(ArmTailSyntax {
            guard,
            body_span: Span { start: at, end: at },
            body_tokens: &[],
            block: false,
            missing: true,
        })
    };
    if open && guard.is_some() && cur.peek().is_none() {
        return missing(cur, guard);
    }
    if !matches!(cur.peek().map(|t| &t.kind), Some(TokenKind::Arrow)) {
        return None;
    }
    cur.bump();
    if open && cur.peek().is_none() {
        return missing(cur, guard);
    }

    // body: `{ ... }` block or a single expression
    let body_span;
    let body_tokens;
    let mut block = false;
    if cur.at_punct(b'{') {
        let open = cur.idx;
        let close = cur.find_close()?;
        body_span = Span {
            start: cur.tokens[open].span.start + 1,
            end: cur.tokens[close].span.start,
        };
        body_tokens = &cur.tokens[open + 1..close];
        block = true;
        cur.idx = close + 1;
    } else {
        let body_start = cur.stop_byte_at(cur.idx);
        let (stop_idx, stop_byte) = expr_body_end(cur);
        body_span = Span {
            start: body_start,
            end: stop_byte,
        };
        if cur.parser.src[body_span.start..body_span.end]
            .trim()
            .is_empty()
        {
            return None;
        }
        body_tokens = &cur.tokens[cur.idx..stop_idx];
        cur.idx = stop_idx;
    }

    Some(ArmTailSyntax {
        guard,
        body_span,
        body_tokens,
        block,
        missing: false,
    })
}

/// Syntax recognition owns spans and borrowed token slices, never recursively
/// constructed bodies. Only a committed match materializes guards and bodies.
struct ArmSyntax<'t, P> {
    pattern: P,
    pattern_span: Span,
    tail: ArmTailSyntax<'t>,
}

impl ArmSyntax<'_, Pattern> {
    fn into_arm(self, parser: &super::Parser<'_>) -> Arm {
        let tail = self.tail.finish(parser);
        Arm {
            pattern: self.pattern,
            pattern_span: self.pattern_span,
            guard: tail.guard,
            body_span: tail.body_span,
            body: tail.body,
            block: tail.block,
            diverges: tail.diverges,
            missing: tail.missing,
        }
    }
}

impl ArmSyntax<'_, TuplePattern> {
    fn into_tuple_arm(self, parser: &super::Parser<'_>) -> TupleArm {
        let tail = self.tail.finish(parser);
        TupleArm {
            pattern: self.pattern,
            pattern_span: self.pattern_span,
            guard: tail.guard,
            body_span: tail.body_span,
            body: tail.body,
            block: tail.block,
            diverges: tail.diverges,
            missing: tail.missing,
        }
    }
}

struct ArmTailSyntax<'t> {
    guard: Option<(Span, &'t [Token])>,
    body_span: Span,
    body_tokens: &'t [Token],
    block: bool,
    missing: bool,
}

impl ArmTailSyntax<'_> {
    fn finish(self, parser: &super::Parser<'_>) -> ArmTail {
        let Self {
            guard,
            body_span,
            body_tokens,
            block,
            missing,
        } = self;
        let guard = guard.map(|(span, tokens)| GuardExpr {
            span,
            expr: parser.parse_expression_tokens(tokens, span.start, span.end),
        });
        #[cfg(test)]
        review_regressions::ARM_BODIES.set(review_regressions::ARM_BODIES.get() + 1);

        let body = if block {
            parser.parse_tokens(body_tokens, body_span.start, body_span.end)
        } else {
            parser.parse_expression_tokens(body_tokens, body_span.start, body_span.end)
        };
        // Whether control can reach the end of a block body is the same
        // question let-else asks of its `else` block, answered on the same CFG
        // (`crate::flow`). An expression body always yields, so the question
        // only arises for a block.
        let diverges = block && parser.body_diverges(body_span, body_tokens, &body);
        ArmTail {
            guard,
            body_span,
            body,
            block,
            diverges,
            missing,
        }
    }
}

/// What follows an arm's pattern: its guard, its body, and the two facts
/// about that body the later phases need.
struct ArmTail {
    guard: Option<GuardExpr>,
    body_span: Span,
    body: crate::ast::Program,
    /// True for a `{ ... }` block body.
    block: bool,
    /// True when every path out of a block body leaves it — the value the
    /// arm yields is always written, so a lowering's fall-through to
    /// `undefined` is unreachable. Always false for an expression body,
    /// which yields by being evaluated.
    diverges: bool,
    missing: bool,
}

/// Parses one `Tag` / `Tag(bindings...)` alternative starting at the
/// identifier under the cursor.
fn parse_tag_pattern(cur: &mut Cursor) -> Option<TagPattern> {
    parse_alternative(cur, true)
}

/// One tag alternative — a tag, optionally with a parenthesized binding
/// list — parsed at the cursor. Shared with the let-else and `if let`
/// parsers, whose or-pattern alternatives use the same grammar
/// (`allow_nested` is false for let-else, whose bindings stay alias-only).
pub(super) fn parse_alternative(cur: &mut Cursor, allow_nested: bool) -> Option<TagPattern> {
    let (tag, tag_span) = cur.eat_ident()?;
    if is_reserved(tag) {
        return None;
    }
    let mut bindings = None;
    let mut list = None;
    let mut end = tag_span.end;
    if cur.at_punct(b'(') {
        let open = cur.idx;
        let close = cur.find_close()?;
        bindings = Some(parse_bindings(
            cur.sub(open + 1, close, cur.tokens[close].span.start),
            allow_nested,
        )?);
        end = cur.tokens[close].span.end;
        list = Some(Span {
            start: cur.tokens[open].span.start,
            end,
        });
        cur.idx = close + 1;
    }
    Some(TagPattern {
        tag: tag.to_string(),
        tag_off: tag_span.start,
        end,
        bindings,
        list,
    })
}

/// Parses `a, b: alias, ...` between the parens of a pattern (shared with
/// the let-else pattern). With `allow_nested`, `b: Tag(...)` — an
/// identifier directly followed by parens — is a nested tag pattern
/// instead of an alias (match patterns only; let-else keeps aliases only).
/// None on failure.
pub(super) fn parse_bindings(mut cur: Cursor, allow_nested: bool) -> Option<Vec<Binding>> {
    let mut bindings = Vec::new();
    loop {
        if cur.peek().is_none() {
            break;
        }
        let (name, name_span) = cur.eat_ident()?;
        if is_reserved(name) {
            return None;
        }

        let mut alias = None;
        let mut alias_span = None;
        let mut nested = None;
        if cur.eat_punct(b':').is_some() {
            let (rhs, rhs_span) = cur.eat_ident()?;
            if is_reserved(rhs) {
                return None;
            }
            if allow_nested && cur.at_punct(b'(') {
                let open = cur.idx;
                let close = cur.find_close()?;
                let inner =
                    parse_bindings(cur.sub(open + 1, close, cur.tokens[close].span.start), true)?;
                cur.idx = close + 1;
                nested = Some(TagPattern {
                    tag: rhs.to_string(),
                    tag_off: rhs_span.start,
                    end: cur.tokens[close].span.end,
                    bindings: Some(inner),
                    list: Some(Span {
                        start: cur.tokens[open].span.start,
                        end: cur.tokens[close].span.end,
                    }),
                });
            } else {
                alias = Some(rhs.to_string());
                alias_span = Some(rhs_span);
            }
        }
        bindings.push(Binding {
            name: name.to_string(),
            name_span,
            alias,
            alias_span,
            nested,
        });

        if cur.peek().is_none() {
            break;
        }
        cur.eat_punct(b',')?;
    }
    Some(bindings)
}

/// Scans a guard condition from `cur.idx` until the arm's top-level `=>`,
/// returning the arrow's token index and byte offset. None on anything a
/// guard cannot contain at its top level (`,`, `;`, a closer) — the
/// candidate then passes through.
fn guard_end(cur: &Cursor) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut k = cur.idx;
    while k < cur.tokens.len() {
        let t = &cur.tokens[k];
        match t.kind {
            TokenKind::Arrow if depth == 0 => return Some((k, t.span.start)),
            _ if t.opens_bracket() => depth += 1,
            _ if t.closes_bracket() => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
            }
            TokenKind::Punct(b',' | b';') if depth == 0 => return None,
            _ => {}
        }
        k += 1;
    }
    None
}

fn guard_runs_to_end(cur: &Cursor) -> bool {
    let mut depth = 0usize;
    for t in &cur.tokens[cur.idx..] {
        match t.kind {
            TokenKind::Arrow if depth == 0 => return false,
            _ if t.opens_bracket() => depth += 1,
            _ if t.closes_bracket() => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
            TokenKind::Punct(b',' | b';') if depth == 0 => return false,
            _ => {}
        }
    }
    depth == 0
}

/// Scans an arm's expression body from `cur.idx` until a top-level `,` or
/// closing bracket, returning the stopping token index and byte offset
/// (the region end when the tokens run out).
fn expr_body_end(cur: &Cursor) -> (usize, usize) {
    let mut depth = 0usize;
    let mut k = cur.idx;
    while k < cur.tokens.len() {
        let t = &cur.tokens[k];
        match t.kind {
            _ if t.opens_bracket() => depth += 1,
            _ if t.closes_bracket() => {
                if depth == 0 {
                    return (k, t.span.start);
                }
                depth -= 1;
            }
            TokenKind::Punct(b',') if depth == 0 => return (k, t.span.start),
            _ => {}
        }
        k += 1;
    }
    (k, cur.range_end)
}

#[cfg(test)]
mod review_regressions {
    use std::cell::Cell;

    thread_local! {
        pub(super) static ARM_BODIES: Cell<usize> = const { Cell::new(0) };
    }

    #[test]
    fn strict_tuple_rejection_does_not_revisit_nested_fallback_bodies() {
        // Count parsed bodies, not elapsed time: this contract is independent
        // of machine speed and catches exponential speculative AST rebuilding.
        for (prefix, suffix, bodies_per_level) in [
            ("match (x) { A => 1, _ => ", " }", 2),
            ("match (x) { _ => ", ", A => 1 }", 2),
            ("match (x) { _ => ", " }", 1),
            ("match (x, x) { (A, _) => 1, _ => ", " }", 2),
        ] {
            for depth in [4, 12, 24] {
                let mut expression = "0".to_string();
                for _ in 0..depth {
                    expression = format!("{prefix}{expression}{suffix}");
                }
                ARM_BODIES.set(0);
                let source = format!("declare const x: any; const result = {expression};");
                let program = crate::parser::parse(&source);
                assert!(!program.segments.is_empty());
                assert_eq!(
                    ARM_BODIES.get(),
                    depth * bodies_per_level,
                    "{prefix}, depth {depth}"
                );
            }
        }
    }
}
