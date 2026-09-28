//! Function-boundary syntax classification.

use super::*;

/// Whether a `return` emitted at token index `at` of this statement
/// stream would leave a **user-written function inside the stream** — the
/// placement question `try` asks: its lowering emits a `return`, and that
/// `return` must have a function of the user's to exit. At the top level
/// of a module (or of a tt construct's own statement region, which
/// forms an isolated value region) there is none; inside a `function`, a method, or
/// an arrow body written in the region there is.
///
/// The classification is per opening brace, and it is the lexer's: a `{`
/// with the `function_body` fact ([`crate::lexer::TokenFacts`]) opens a
/// body after `=>` or after a parameter list and its return type.
/// Everything else — object literals, class and namespace bodies,
/// control-statement bodies, bare blocks — is transparent or irrelevant:
/// it never *provides* a function to return from, and never blocks an
/// outer one from counting.
pub(crate) fn in_function_body(tokens: &[Token], at: usize) -> bool {
    let mut stack: Vec<bool> = Vec::new();
    for (k, t) in tokens.iter().enumerate().take(at) {
        match t.kind {
            TokenKind::Punct(b'{') => stack.push(function_body_brace(tokens, k)),
            TokenKind::Punct(b'}') => {
                stack.pop();
            }
            _ => {}
        }
    }
    stack.iter().any(|&is_function| is_function)
}

/// Number of braced user-written function bodies enclosing a token. This is
/// the lexical-boundary fact speculative `result` claiming needs: an inner
/// function has its own Result scope even when the enclosing candidate is
/// already inside another function.
pub(crate) fn function_depth_at(tokens: &[Token], at: usize) -> usize {
    let mut stack = Vec::new();
    for (index, token) in tokens.iter().enumerate().take(at) {
        match token.kind {
            TokenKind::Punct(b'{') => stack.push(function_body_brace(tokens, index)),
            TokenKind::Punct(b'}') => {
                stack.pop();
            }
            _ => {}
        }
    }
    stack.into_iter().filter(|is_function| *is_function).count()
}

pub(crate) fn user_function_depth_at(
    tokens: &[Token],
    at: usize,
    tt_owned: &std::collections::HashSet<usize>,
) -> usize {
    let mut stack = Vec::new();
    for (index, token) in tokens.iter().enumerate().take(at) {
        match token.kind {
            TokenKind::Punct(b'{') => stack.push(
                function_body_brace(tokens, index)
                    && !tt_owned.contains(&index)
                    && !index
                        .checked_sub(1)
                        .is_some_and(|previous| tt_owned.contains(&previous)),
            ),
            TokenKind::Punct(b'}') => {
                stack.pop();
            }
            _ => {}
        }
    }
    let braced = stack.into_iter().filter(|is_function| *is_function).count();
    let concise = tokens
        .iter()
        .enumerate()
        .take(at)
        .filter(|(arrow, token)| {
            matches!(token.kind, TokenKind::Arrow) && !tt_owned.contains(arrow)
        })
        .filter(|(arrow, _)| {
            !matches!(
                tokens.get(arrow + 1).map(|token| &token.kind),
                Some(TokenKind::Punct(b'{'))
            ) && concise_arrow_end(tokens, arrow + 1) > at
        })
        .count();
    braced + concise
}

/// Whether `at` is directly enclosed by a class static block. A nested
/// user-written function remains its own Result scope, so callers combine
/// this with [`function_target_at`] rather than treating every nested token
/// as statically owned.
pub(crate) fn in_static_block(src: &str, tokens: &[Token], at: usize) -> bool {
    let mut stack: Vec<bool> = Vec::new();
    for (index, token) in tokens.iter().enumerate().take(at) {
        match token.kind {
            TokenKind::Punct(b'{') => stack.push(
                index
                    .checked_sub(1)
                    .and_then(|before| tokens.get(before))
                    .is_some_and(|previous| {
                        matches!(previous.kind, TokenKind::Ident)
                            && &src[previous.span.start..previous.span.end] == "static"
                    }),
            ),
            TokenKind::Punct(b'}') => {
                stack.pop();
            }
            _ => {}
        }
    }
    stack.into_iter().any(|is_static| is_static)
}

/// The kind of user-written function that an early `return` at a token can
/// reach. Constructors and generators syntactically accept `return`, but a
/// propagated Result would change their JavaScript completion contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionTarget {
    Ordinary,
    Constructor,
    Generator,
}

/// Returns the innermost user function enclosing `at`.
pub(crate) fn function_target_at(tokens: &[Token], at: usize) -> Option<FunctionTarget> {
    user_function_target_at(tokens, at, &std::collections::HashSet::new())
}

/// Returns the innermost user-written function enclosing `at`, skipping the
/// match body braces and arm arrows in `tt_owned`, which open no function.
pub(crate) fn user_function_target_at(
    tokens: &[Token],
    at: usize,
    tt_owned: &std::collections::HashSet<usize>,
) -> Option<FunctionTarget> {
    let mut stack: Vec<Option<(usize, FunctionTarget)>> = Vec::new();
    for (index, token) in tokens.iter().enumerate().take(at) {
        match token.kind {
            TokenKind::Punct(b'{') => stack.push(
                (!tt_owned.contains(&index)
                    && !index
                        .checked_sub(1)
                        .is_some_and(|previous| tt_owned.contains(&previous)))
                .then(|| function_target_brace(tokens, index))
                .flatten()
                .map(|target| (index, target)),
            ),
            TokenKind::Punct(b'}') => {
                stack.pop();
            }
            _ => {}
        }
    }
    let braced = stack.into_iter().rev().flatten().next();
    let concise_arrow = tokens
        .iter()
        .enumerate()
        .take(at)
        .filter(|(arrow, token)| {
            matches!(token.kind, TokenKind::Arrow) && !tt_owned.contains(arrow)
        })
        .filter(|(arrow, _)| concise_arrow_end(tokens, arrow + 1) > at)
        .map(|(arrow, _)| (arrow, FunctionTarget::Ordinary))
        .next_back();
    match (braced, concise_arrow) {
        (Some(braced), Some(arrow)) => Some(if braced.0 > arrow.0 {
            braced.1
        } else {
            arrow.1
        }),
        (Some((_, target)), None) | (None, Some((_, target))) => Some(target),
        (None, None) => None,
    }
}

/// Token index just past a concise arrow body, or the body start for a
/// braced arrow. Balanced groups are one expression atom; a top-level
/// comma, semicolon, enclosing closer, or statement boundary
/// ([`crate::lexer::TokenFacts::boundary_before`]) ends the body.
pub(super) fn concise_arrow_end(tokens: &[Token], from: usize) -> usize {
    crate::work::tick("concise arrow scans");
    if matches!(
        tokens.get(from).map(|token| &token.kind),
        Some(TokenKind::Punct(b'{'))
    ) {
        return from;
    }
    let mut index = from;
    let mut depth = 0usize;
    while index < tokens.len() {
        if depth == 0 && index > from && tokens[index].facts.boundary_before() {
            return index;
        }
        match tokens[index].kind {
            TokenKind::Punct(b'(' | b'[' | b'{') => depth += 1,
            TokenKind::Punct(b')' | b']' | b'}') => {
                if depth == 0 {
                    return index;
                }
                depth -= 1;
            }
            TokenKind::Punct(b',' | b';') if depth == 0 => return index,
            _ => {}
        }
        index += 1;
    }
    tokens.len()
}

/// The kind of user function the `{` at `brace` opens, if it opens one:
/// the lexer's function-body facts ([`crate::lexer::TokenFacts`]).
pub(super) fn function_target_brace(tokens: &[Token], brace: usize) -> Option<FunctionTarget> {
    let facts = tokens.get(brace)?.facts;
    facts
        .function_body()
        .then_some(if facts.constructor_body() {
            FunctionTarget::Constructor
        } else if facts.generator_body() {
            FunctionTarget::Generator
        } else {
            FunctionTarget::Ordinary
        })
}

/// Whether the `{` at token index `k` opens a function body (see
/// [`in_function_body`]).
pub(super) fn function_body_brace(tokens: &[Token], k: usize) -> bool {
    tokens[k].facts.function_body()
}
