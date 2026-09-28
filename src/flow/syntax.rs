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
/// The classification is per opening brace, from its immediate left
/// context: `=> {` is an arrow body; `) {` is a function or method body
/// unless the parenthesis belongs to a control head (`if`/`for`/`while`/
/// `switch`/`catch`/`with`, `for await`) or a `class ... extends call()`
/// heritage clause. Everything else — object literals, class and
/// namespace bodies, control-statement bodies, bare blocks — is
/// transparent or irrelevant: it never *provides* a function to return
/// from, and never blocks an outer one from counting.
pub(crate) fn in_function_body(src: &str, tokens: &[Token], at: usize) -> bool {
    let mut stack: Vec<bool> = Vec::new();
    for (k, t) in tokens.iter().enumerate().take(at) {
        match t.kind {
            TokenKind::Punct(b'{') => stack.push(function_body_brace(src, tokens, k)),
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
pub(crate) fn function_depth_at(src: &str, tokens: &[Token], at: usize) -> usize {
    let mut stack = Vec::new();
    for (index, token) in tokens.iter().enumerate().take(at) {
        match token.kind {
            TokenKind::Punct(b'{') => stack.push(function_body_brace(src, tokens, index)),
            TokenKind::Punct(b'}') => {
                stack.pop();
            }
            _ => {}
        }
    }
    stack.into_iter().filter(|is_function| *is_function).count()
}

pub(crate) fn user_function_depth_at(
    src: &str,
    tokens: &[Token],
    at: usize,
    tt_owned: &std::collections::HashSet<usize>,
) -> usize {
    let mut stack = Vec::new();
    for (index, token) in tokens.iter().enumerate().take(at) {
        match token.kind {
            TokenKind::Punct(b'{') => stack.push(
                function_body_brace(src, tokens, index)
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
pub(crate) fn function_target_at(src: &str, tokens: &[Token], at: usize) -> Option<FunctionTarget> {
    user_function_target_at(src, tokens, at, &std::collections::HashSet::new())
}

/// Returns the innermost user-written function enclosing `at`, skipping the
/// match body braces and arm arrows in `tt_owned`, which open no function.
pub(crate) fn user_function_target_at(
    src: &str,
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
                .then(|| function_target_brace(src, tokens, index))
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

pub(super) fn function_target_brace(
    src: &str,
    tokens: &[Token],
    brace: usize,
) -> Option<FunctionTarget> {
    if !function_body_brace(src, tokens, brace) {
        return None;
    }
    if matches!(
        tokens.get(brace.wrapping_sub(1)).map(|token| &token.kind),
        Some(TokenKind::Arrow)
    ) {
        return Some(FunctionTarget::Ordinary);
    }
    let close = (0..brace)
        .rev()
        .find(|&index| matches!(tokens[index].kind, TokenKind::Punct(b')')))?;
    let open = find_open(tokens, close)?;
    let word = |index: usize| match tokens.get(index) {
        Some(token) if matches!(token.kind, TokenKind::Ident) => {
            Some(&src[token.span.start..token.span.end])
        }
        _ => None,
    };
    let before = open.checked_sub(1)?;
    if word(before) == Some("constructor") {
        return Some(FunctionTarget::Constructor);
    }
    let generator = matches!(tokens[before].kind, TokenKind::Punct(b'*'))
        || (before >= 1 && matches!(tokens[before - 1].kind, TokenKind::Punct(b'*')));
    Some(if generator {
        FunctionTarget::Generator
    } else {
        FunctionTarget::Ordinary
    })
}

/// Heads whose parenthesized clause is followed by a *control* body, not a
/// function body.
pub(super) const CONTROL_PAREN_WORDS: &[&str] = &["if", "for", "while", "switch", "catch", "with"];

/// Statement-position words a return-type walk aborts on: meeting one at
/// the top level means the walk left the annotation and entered the
/// preceding statement (or the brace never had an annotation at all).
pub(super) const NON_TYPE_WORDS: &[&str] = &[
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "declare",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "finally",
    "for",
    "function",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "module",
    "namespace",
    "new",
    "of",
    "return",
    "switch",
    "throw",
    "try",
    "var",
    "while",
    "with",
    "yield",
];

/// Whether the `{` at token index `k` opens a function body (see
/// [`in_function_body`]).
pub(super) fn function_body_brace(src: &str, tokens: &[Token], k: usize) -> bool {
    let Some(prev) = k.checked_sub(1) else {
        return false;
    };
    match tokens[prev].kind {
        // `=> {` — an arrow body.
        TokenKind::Arrow => true,
        // `) {` — a parameter list's body, unless the parenthesis is a
        // control head or a heritage-clause call.
        TokenKind::Punct(b')') => paren_heads_function(src, tokens, prev),
        // Anything else may still be `) : <type> {` — a return-type
        // annotation between the parameter list and the body. Walk back
        // over the type (balanced brackets; names, operators and function
        // arrows at its top level); the `:` straight after a `)` is the
        // annotation's colon and the parenthesis decides as above. A token
        // no type contains at its top level ends the walk: the brace is
        // not a body.
        _ => annotated_paren(src, tokens, k)
            .is_some_and(|close| paren_heads_function(src, tokens, close)),
    }
}

/// The `)` that token `k` follows directly or through a return-type
/// annotation `) : <type>`.
fn annotated_paren(src: &str, tokens: &[Token], k: usize) -> Option<usize> {
    let prev = k.checked_sub(1)?;
    if matches!(tokens[prev].kind, TokenKind::Punct(b')')) {
        return Some(prev);
    }
    let mut depth = 0usize;
    let mut j = k;
    while j > 0 {
        j -= 1;
        let t = &tokens[j];
        match t.kind {
            TokenKind::Punct(b'>' | b')' | b']' | b'}') => depth += 1,
            TokenKind::Punct(b'<' | b'(' | b'[' | b'{') => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
            }
            TokenKind::Punct(b':') if depth == 0 => {
                return (j >= 1 && matches!(tokens[j - 1].kind, TokenKind::Punct(b')')))
                    .then(|| j - 1);
            }
            TokenKind::Ident if depth == 0 => {
                if NON_TYPE_WORDS.contains(&&src[t.span.start..t.span.end]) {
                    return None;
                }
            }
            TokenKind::Arrow if depth == 0 => return None,
            TokenKind::Str | TokenKind::Arrow => {}
            TokenKind::Punct(b'.' | b'|' | b'&') => {}
            TokenKind::Punct(c) if c.is_ascii_digit() => {}
            _ => {
                if depth == 0 {
                    return None;
                }
            }
        }
    }
    None
}

/// Whether the parameter list closed at token index `close` heads a
/// function body — i.e. it is not a control head (`if (…)`, `for (…)`,
/// `for await (…)`, …) and not a `class … extends call(…)` heritage
/// clause.
pub(super) fn paren_heads_function(src: &str, tokens: &[Token], close: usize) -> bool {
    let word = |i: usize| match tokens.get(i) {
        Some(t) if matches!(t.kind, TokenKind::Ident) => Some(&src[t.span.start..t.span.end]),
        _ => None,
    };
    let Some(open) = find_open(tokens, close) else {
        return false;
    };
    let Some(before) = open.checked_sub(1) else {
        return false;
    };
    match tokens[before].kind {
        TokenKind::Ident => {
            let name = word(before).unwrap_or_default();
            if CONTROL_PAREN_WORDS.contains(&name) {
                return false;
            }
            // `for await (…) {` — still a control body.
            if name == "await" && word(before.wrapping_sub(1)) == Some("for") {
                return false;
            }
            // `class A extends mixin(B) {` — a class body: walk back over
            // the (possibly dotted) callee to the word before it.
            let mut j = before;
            while j >= 2
                && matches!(tokens[j - 1].kind, TokenKind::Punct(b'.'))
                && matches!(tokens[j - 2].kind, TokenKind::Ident)
            {
                j -= 2;
            }
            !(j >= 1 && word(j - 1) == Some("extends"))
        }
        // `function* (…) {` — an anonymous generator.
        TokenKind::Punct(b'*') => word(before.wrapping_sub(1)) == Some("function"),
        // `f<T>(…) {` — a generic parameter list's body.
        TokenKind::Punct(b'>') => true,
        _ => false,
    }
}

/// The index of the token opening the bracket closed at `close`.
pub(super) fn find_open(tokens: &[Token], close: usize) -> Option<usize> {
    let mut depth = 0usize;
    for k in (0..=close).rev() {
        match tokens[k].kind {
            TokenKind::Punct(b')' | b']' | b'}') => depth += 1,
            TokenKind::Punct(b'(' | b'[' | b'{') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
    }
    None
}
