//! Token cursor — the parser's view over the lexed token stream.
//!
//! A [`Cursor`] is a cheap `Copy` handle (token slice + position), so a
//! sub-parser takes one by value, advances its own copy, and returns it on
//! success; on failure the caller's original cursor is untouched — that is
//! the whole backtracking story, mirroring how the byte parser re-scanned
//! from a saved offset.
//!
//! Byte positions still matter: constructs and sub-programs are byte
//! ranges of the original source. `range_end` is the byte just past the
//! cursor's token range, used as the stop position for open-ended scans
//! (an arm body running to the end of the region, a type annotation
//! running to the closing paren) when the tokens run out.

use crate::ast::Span;
use crate::lexer::{Token, TokenKind};

use super::Parser;

#[derive(Clone, Copy)]
pub(super) struct Cursor<'t> {
    pub(super) parser: &'t Parser<'t>,
    pub(super) tokens: &'t [Token],
    pub(super) idx: usize,
    /// Byte offset just past this token range in the source.
    pub(super) range_end: usize,
}

impl<'t> Cursor<'t> {
    pub(super) fn new(
        parser: &'t Parser<'t>,
        tokens: &'t [Token],
        idx: usize,
        range_end: usize,
    ) -> Self {
        Cursor {
            parser,
            tokens,
            idx,
            range_end,
        }
    }

    pub(super) fn peek(&self) -> Option<&'t Token> {
        self.tokens.get(self.idx)
    }

    pub(super) fn bump(&mut self) -> Option<&'t Token> {
        let t = self.tokens.get(self.idx)?;
        self.idx += 1;
        Some(t)
    }

    /// Whether a line terminator comes before the token under the cursor.
    pub(super) fn line_break_before(&self) -> bool {
        self.peek()
            .is_some_and(|token| token.facts.line_break_before())
    }

    pub(super) fn text(&self, t: &Token) -> &'t str {
        &self.parser.src[t.span.start..t.span.end]
    }

    pub(super) fn at_punct(&self, b: u8) -> bool {
        matches!(self.peek().map(|t| &t.kind), Some(TokenKind::Punct(x)) if *x == b)
    }

    pub(super) fn eat_punct(&mut self, b: u8) -> Option<Span> {
        if self.at_punct(b) {
            return self.bump().map(|t| t.span);
        }
        None
    }

    /// Consumes the current token if it is an ASCII identifier, the only
    /// kind a tt construct names or binds.
    pub(super) fn eat_ident(&mut self) -> Option<(&'t str, Span)> {
        let token = self.peek()?;
        match token.kind {
            TokenKind::Ident if self.text(token).is_ascii() => {
                let t = self.bump()?;
                Some((&self.parser.src[t.span.start..t.span.end], t.span))
            }
            _ => None,
        }
    }

    /// The token index of the closer matching the opener at `self.idx`,
    /// which must open a bracket pair ([`Token::opens_bracket`]): `(`, `[`,
    /// `{`, or a `<` the token facts record as opening type arguments or
    /// parameters. Only the matching pair is counted, so a stray closer of
    /// another kind never ends the group.
    pub(super) fn find_close(&self) -> Option<usize> {
        find_close_at(self.tokens, self.idx)
    }

    /// The byte where the token at `idx` starts, or `range_end` past the
    /// last token — the natural stop position for open-ended scans.
    pub(super) fn stop_byte_at(&self, idx: usize) -> usize {
        match self.tokens.get(idx) {
            Some(t) => t.span.start,
            None => self.range_end,
        }
    }

    /// A cursor over `tokens[from..to]` whose open-ended scans stop at
    /// byte `range_end` (typically the closing delimiter of the region).
    pub(super) fn sub(&self, from: usize, to: usize, range_end: usize) -> Cursor<'t> {
        Cursor {
            parser: self.parser,
            tokens: &self.tokens[from..to],
            idx: 0,
            range_end,
        }
    }
}

/// See [`Cursor::find_close`].
pub(crate) fn find_close_at(tokens: &[Token], open_idx: usize) -> Option<usize> {
    Token::matching_close(tokens, open_idx)
}

/// Whether the `{` at `k`, in an expression scan that started at `from`,
/// begins an expression: an object literal (ECMA-262 PrimaryExpression),
/// an arrow body, or a type literal, each stepped over as one group. It
/// is a block that follows the expression only when the token before it
/// ends an expression ([`crate::lexer::TokenFacts::ends_expression`]).
pub(super) fn brace_begins_expression(tokens: &[Token], from: usize, k: usize) -> bool {
    k <= from || !tokens[k - 1].facts.ends_expression()
}

/// True when the identifier at `k` (within a scan that started at `from`)
/// is a property name, not a keyword or a binding: the token before it is a
/// member-access dot (`.` or the `?.` of optional chaining), or the `#` of a
/// private name (`#name`, ECMA-262 `PrivateIdentifier`), which the lexer
/// splits into `#` and the identifier with no gap between them. The last
/// dot of a spread's `...` is punctuation, not member access.
pub(crate) fn dotted_at(tokens: &[Token], from: usize, k: usize) -> bool {
    k > from
        && match tokens[k - 1].kind {
            TokenKind::Punct(b'.') => !spread_ends_at(tokens, k - 1),
            TokenKind::OptChain => true,
            TokenKind::Punct(b'#') => tokens[k - 1].span.end == tokens[k].span.start,
            _ => false,
        }
}

pub(super) fn spread_ends_at(tokens: &[Token], last: usize) -> bool {
    last >= 2
        && tokens[last - 2..=last]
            .windows(2)
            .all(|pair| pair[0].span.end == pair[1].span.start)
        && tokens[last - 2..=last]
            .iter()
            .all(|token| matches!(token.kind, TokenKind::Punct(b'.')))
}

/// The index just past a construct that carries its own top-level braces
/// — `match ( ... ) { ... }` or `result { ... }` — when `tokens[k]` is its
/// undotted keyword `word`. Expression scanners step over the whole shape
/// so their bare-`{` abort does not reject it; whether it really is a tt
/// construct is the recursive parse's decision.
pub(super) fn skip_braced_construct(tokens: &[Token], word: &str, k: usize) -> Option<usize> {
    match word {
        "match" => skip_match_shape(tokens, k),
        "result" => {
            if !matches!(tokens.get(k + 1)?.kind, TokenKind::Punct(b'{')) {
                return None;
            }
            Some(find_close_at(tokens, k + 1)? + 1)
        }
        _ => None,
    }
}

/// See [`skip_braced_construct`] — the `match ( ... ) { ... }` shape.
fn skip_match_shape(tokens: &[Token], k: usize) -> Option<usize> {
    if !matches!(tokens.get(k + 1)?.kind, TokenKind::Punct(b'(')) {
        return None;
    }
    let close_paren = find_close_at(tokens, k + 1)?;
    if !super::matches::opens_match_body(tokens.get(close_paren + 1)?) {
        return None;
    }
    let close_brace = find_close_at(tokens, close_paren + 1)?;
    Some(close_brace + 1)
}
