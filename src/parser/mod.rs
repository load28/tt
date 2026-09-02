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
mod keywords;
mod lets;
mod literals;
mod matches;
mod parse;
mod pipes;
mod results;
mod tries;
mod variants;

#[cfg(test)]
mod tests;

use crate::ast::*;
use crate::lexer::{self, Token, TokenKind, TplPart};
use crate::val;
use cursor::Cursor;

pub(crate) use cursor::{dotted_at, find_close_at};
pub(crate) use keywords::is_reserved;
use keywords::*;
#[cfg(test)]
use parse::visit_programs;
pub(crate) use parse::{
    Parser, lex_and_parse_with_kind, parse, parse_with_kind, projection_recoveries,
    unclaimed_candidates,
};

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
