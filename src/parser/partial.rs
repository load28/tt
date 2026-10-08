//! The pattern a position is in, read with the parser's grammar from text
//! that may not parse yet.
//!
//! The parser claims a construct only once all of it parses, but an editor
//! asks about a pattern while it is being typed: `match (s) { Circle(r) =>
//! r, Po` has an arm with no body and a body with no closing brace. The
//! queries here answer from the same rules the parser commits with: the
//! match head ([`matches::match_body_open`]), the arm walk
//! ([`matches::outline_arms`]), the arm pattern grammar
//! ([`matches::parse_arm_pattern`]), and the heads that introduce a single
//! pattern ([`iflets::if_let_pattern`], [`lets::let_else_pattern`]). A reader
//! of these answers holds no grammar of its own.

use super::cursor::{Cursor, find_close_at};
use super::matches::{self, ArmPart};
use super::{Parser, iflets, lets};
use crate::ast::Pattern;
use crate::lexer::{Token, TokenKind};

/// The pattern that the text before a token belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PatternSite {
    /// A match arm's pattern, in the body whose `{` is the token at `open`,
    /// starting at the arm's first token `start`.
    Arm { open: usize, start: usize },
    /// The one pattern of an `if let` or a let-else, starting at its first
    /// alternative's token `start`.
    Single { start: usize },
}

/// An arm whose pattern is finished — it reached its `=>`.
#[derive(Debug)]
pub(crate) struct ArmHeader {
    /// The pattern, when it parses with the arm pattern grammar.
    pub(crate) pattern: Option<Pattern>,
    /// Whether an `if` guard follows the pattern.
    pub(crate) guarded: bool,
    pub(crate) start: usize,
}

/// The pattern that the tokens before index `before` are being written in,
/// if any: an arm of the innermost match body enclosing them while that arm
/// is still in its pattern, or the alternatives right after an `if let` or
/// a let-else's declaration keyword. A guard or an arm body is an
/// expression, and a `(` anywhere else is a call.
pub(crate) fn pattern_site_at(src: &str, tokens: &[Token], before: usize) -> Option<PatternSite> {
    let mut open = Vec::new();
    for (index, token) in tokens.iter().enumerate().take(before) {
        match token.kind {
            _ if token.opens_bracket() => open.push(index),
            _ if token.closes_bracket() => {
                open.pop();
            }
            _ => {}
        }
    }
    let parens = open
        .iter()
        .rev()
        .take_while(|&&index| matches!(tokens[index].kind, TokenKind::Punct(b'(')))
        .count();
    let head_end = if parens > 0 {
        open[open.len() - parens]
    } else {
        before
    };
    if let Some(brace) = open.len().checked_sub(parens + 1).map(|index| open[index])
        && opens_match_body(src, tokens, brace)
    {
        let body = &tokens[brace + 1..body_close(tokens, brace)];
        let at = before.checked_sub(brace + 1)?;
        let arm = matches::outline_arms(src, body)
            .into_iter()
            .find(|arm| at <= arm.end)?;
        return (arm.part_at(at) == ArmPart::Pattern).then_some(PatternSite::Arm {
            open: brace,
            start: brace + 1 + arm.start,
        });
    }
    single_pattern_site(src, tokens, head_end, parens > 0)
}

/// The finished arms of the match body whose `{` is the token at `open`, in
/// source order.
pub(crate) fn arm_headers(src: &str, tokens: &[Token], open: usize) -> Vec<ArmHeader> {
    let parser = Parser::new(src);
    let body = &tokens[open + 1..body_close(tokens, open)];
    matches::outline_arms(src, body)
        .into_iter()
        .filter(|arm| arm.arrow.is_some())
        .map(|arm| {
            let end = arm.pattern_end();
            let mut cur = Cursor::new(&parser, &body[..end], arm.start, body[end].span.start);
            ArmHeader {
                pattern: matches::parse_arm_pattern(&mut cur).filter(|_| cur.peek().is_none()),
                guarded: arm.guard.is_some(),
                start: body[arm.start].span.start,
            }
        })
        .collect()
}

pub(crate) fn tuple_arm_headers(
    src: &str,
    tokens: &[Token],
    open: usize,
    position: usize,
) -> Vec<ArmHeader> {
    let parser = Parser::new(src);
    let body = &tokens[open + 1..body_close(tokens, open)];
    matches::outline_arms(src, body)
        .into_iter()
        .filter(|arm| arm.arrow.is_some())
        .filter_map(|arm| {
            let end = arm.pattern_end();
            if !matches!(body.get(arm.start)?.kind, TokenKind::Punct(b'(')) {
                return None;
            }
            let close = find_close_at(body, arm.start).filter(|&close| close < end)?;
            let mut element = arm.start + 1;
            let mut index = 0;
            let mut k = element;
            let mut depth = 0usize;
            let element_end = loop {
                if k >= close {
                    break (index == position).then_some(close)?;
                }
                let token = &body[k];
                if token.opens_bracket() {
                    depth += 1;
                } else if token.closes_bracket() {
                    depth = depth.saturating_sub(1);
                } else if depth == 0 && matches!(token.kind, TokenKind::Punct(b',')) {
                    if index == position {
                        break k;
                    }
                    index += 1;
                    element = k + 1;
                }
                k += 1;
            };
            let mut cur = Cursor::new(
                &parser,
                &body[..element_end],
                element,
                body[element_end].span.start,
            );
            Some(ArmHeader {
                pattern: matches::parse_arm_pattern(&mut cur).filter(|_| cur.peek().is_none()),
                guarded: arm.guard.is_some(),
                start: body[arm.start].span.start,
            })
        })
        .collect()
}

pub(crate) fn pattern_of(text: &str) -> Option<Pattern> {
    let parser = Parser::new(text);
    let tokens = crate::lexer::lex(text, 0, text.len());
    let mut cur = Cursor::new(&parser, &tokens, 0, text.len());
    matches::parse_arm_pattern(&mut cur).filter(|_| cur.peek().is_none())
}

/// Whether the `{` at `brace` opens the body of a match.
pub(crate) fn opens_match_body(src: &str, tokens: &[Token], brace: usize) -> bool {
    (0..brace).rev().any(|keyword| {
        super::parse::match_keyword_at(src, tokens, keyword)
            && matches::match_body_open(tokens, keyword + 1) == Some(brace)
    })
}

/// The token index of the `}` closing the body opened at `open`, or the end
/// of the input while it is unwritten.
fn body_close(tokens: &[Token], open: usize) -> usize {
    find_close_at(tokens, open).unwrap_or(tokens.len())
}

/// The `if let` or let-else whose pattern alternatives run up to
/// `head_end`. With `parens`, `head_end` is the `(` of the alternative
/// being written, and its tag is the token before it.
fn single_pattern_site(
    src: &str,
    tokens: &[Token],
    head_end: usize,
    parens: bool,
) -> Option<PatternSite> {
    let until = if parens {
        let tag = head_end.checked_sub(1)?;
        matches!(tokens[tag].kind, TokenKind::Ident).then_some(tag)?
    } else {
        head_end
    };
    let parser = Parser::new(src);
    (0..until).rev().find_map(|keyword| {
        let (start, nested) = iflets::if_let_pattern(src, tokens, keyword)
            .map(|start| (start, true))
            .or_else(|| lets::let_else_pattern(src, tokens, keyword).map(|start| (start, false)))?;
        (start <= until && alternatives_reach(&parser, tokens, start, until, nested))
            .then_some(PatternSite::Single { start })
    })
}

/// Whether the tokens `start..until` are whole alternatives, each followed
/// by `|`, so that `until` is where the next alternative begins.
fn alternatives_reach(
    parser: &Parser,
    tokens: &[Token],
    start: usize,
    until: usize,
    nested: bool,
) -> bool {
    let range_end = tokens
        .get(until)
        .map_or(parser.src.len(), |token| token.span.start);
    let mut cur = Cursor::new(parser, &tokens[..until], start, range_end);
    while cur.peek().is_some() {
        if matches::parse_alternative(&mut cur, nested).is_none() || cur.eat_punct(b'|').is_none() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(source: &str) -> Vec<Token> {
        crate::lexer::lex_with_kind(source, 0, source.len(), crate::SourceKind::TypeScript)
    }

    fn site(source: &str, needle: &str) -> Option<PatternSite> {
        let tokens = lex(source);
        let offset = source.find(needle).expect("needle") + needle.len();
        let before = tokens.partition_point(|token| token.span.start < offset);
        pattern_site_at(source, &tokens, before)
    }

    #[test]
    fn both_committed_match_heads_open_arm_sites() {
        for source in [
            "match (s) { A => 1, ",
            "match s { A => 1, ",
            "match s.t { A => 1, ",
        ] {
            assert!(
                matches!(site(source, "1, "), Some(PatternSite::Arm { .. })),
                "{source}"
            );
        }
        assert_eq!(site("x.match (s) { A => 1, ", "1, "), None);
    }

    #[test]
    fn an_arm_is_a_pattern_site_only_until_its_guard_or_arrow() {
        assert!(site("match (s) { A(", "A(").is_some());
        assert_eq!(site("match (s) { A(x) if f(", "f("), None);
        assert_eq!(site("match (s) { A(x) => f(", "f("), None);
        assert!(site("match (s) { A(x) => f(1, 2), B(", "B(").is_some());
    }

    #[test]
    fn single_pattern_heads_are_the_parsers() {
        assert!(site("if let A(x) | B(", "B(").is_some());
        assert!(site("const A(", "A(").is_some());
        assert_eq!(site("x.const A(", "A("), None);
        assert_eq!(site("const enum(", "enum("), None);
        assert_eq!(site("if let A(x) = B(", "B("), None);
    }

    #[test]
    fn the_arm_walk_splits_only_at_top_level_commas() {
        let source = "A(x, y) if f(1, 2) => [3, 4], B => { g(5, 6) }, ";
        let tokens = lex(source);
        let arms = matches::outline_arms(source, &tokens);
        assert_eq!(arms.len(), 3);
        assert!(arms[0].guard.is_some() && arms[0].arrow.is_some());
        assert!(arms[1].guard.is_none() && arms[1].arrow.is_some());
        assert_eq!(arms[2].start, tokens.len());
    }

    #[test]
    fn a_comma_inside_type_arguments_is_not_an_arm_separator() {
        let source = "match (s) { A(x) if f<P, Q>(x) => new Map<P, Q>(), B => g<P, Q>(1), ";
        let tokens = lex(source);
        let open = tokens
            .iter()
            .position(|token| matches!(token.kind, TokenKind::Punct(b'{')))
            .expect("body");
        let headers = arm_headers(source, &tokens, open);
        assert_eq!(headers.len(), 2);
        assert!(headers[0].guarded && headers[0].pattern.is_some());
        assert!(!headers[1].guarded && headers[1].pattern.is_some());
        assert!(matches!(
            site(source, "g<P, Q>(1), "),
            Some(PatternSite::Arm { .. })
        ));
        assert_eq!(site(source, "f<P,"), None);
        assert!(site("match (s) { A(x) if f<P, Q>(x) => 1, B(", "B(").is_some());
    }
}
