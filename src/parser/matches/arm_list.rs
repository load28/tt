//! Match arm boundaries, strict list recognition, and list recovery.

use super::*;

/// Where an arm is in the arm grammar: its pattern, its `if` guard, or its
/// body after `=>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum ArmPart {
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
pub(in super::super) struct ArmOutline {
    pub(in super::super) start: usize,
    pub(in super::super) guard: Option<usize>,
    pub(in super::super) arrow: Option<usize>,
    pub(in super::super) end: usize,
}

impl ArmOutline {
    /// The part of the arm the token at `index` is in.
    pub(in super::super) fn part_at(&self, index: usize) -> ArmPart {
        if self.arrow.is_some_and(|arrow| arrow < index) {
            ArmPart::Body
        } else if self.guard.is_some_and(|guard| guard < index) {
            ArmPart::Guard
        } else {
            ArmPart::Pattern
        }
    }

    /// The token just past the arm's pattern.
    pub(in super::super) fn pattern_end(&self) -> usize {
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
pub(in super::super) fn outline_arms(src: &str, tokens: &[Token]) -> Vec<ArmOutline> {
    let open_arm = |start| ArmOutline {
        start,
        guard: None,
        arrow: None,
        end: tokens.len(),
    };
    let mut arms = Vec::new();
    let mut arm = open_arm(0);
    let mut index = 0;
    while let Some(token) = tokens.get(index) {
        crate::work::tick("arm outline steps");
        match token.kind {
            _ if token.opens_bracket() => match Token::balancing_close(tokens, index) {
                Some(close) => index = close,
                None => break,
            },
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
        index += 1;
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
pub(super) fn recover_match_arms(mut cur: Cursor) -> Option<Vec<Span>> {
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
pub(super) fn parse_strict_arm_list<'t, T>(
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
pub(super) fn arms_tail(arms: &Cursor, last_start: usize) -> Option<ArmsTail> {
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
