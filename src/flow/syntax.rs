//! Function-boundary syntax classification.

use super::*;

/// Whether token index `at` of this statement stream stands inside a
/// **function-like boundary written in the stream** — the placement
/// question `try`, let-else, and `if let` ask: the exits their lowering
/// emits (`return`, a labeled `break`) belong to that boundary, not to the
/// stream's own region. At the top level of a module (or of a tt
/// construct's own statement region: an isolated value region or a
/// `result` block's body) there is none; inside a `function`, a method, or
/// an arrow body written in the region there is, and so there is inside a
/// class body or a class static block written there ([`FunctionTarget`]):
/// code there is not evaluated by the region, and neither `return` nor a
/// `break` to a label outside it is allowed there (ECMA-262 §15.7.1: a
/// `ClassStaticBlockStatementList` is parsed `[~Return]` and may contain no
/// undefined break target).
///
/// The classification is per opening brace, and it is the lexer's
/// ([`function_target_brace`]). Everything else — object literals,
/// namespace bodies, control-statement bodies, bare blocks — is
/// transparent: it never *provides* a boundary, and never blocks an outer
/// one from counting.
pub(crate) fn in_function_body(tokens: &[Token], at: usize) -> bool {
    let mut stack: Vec<bool> = Vec::new();
    for (k, t) in tokens.iter().enumerate().take(at) {
        match t.kind {
            TokenKind::Punct(b'{') => stack.push(function_target_brace(tokens, k).is_some()),
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

/// Number of function-like boundaries ([`FunctionTarget`]: user-written
/// function bodies, concise arrow bodies, class bodies, and class static
/// blocks) enclosing a token, skipping the braces and arrows of tt
/// constructs in `tt_owned`. Each boundary is its own Result scope for a
/// `try` written in it, so a speculative `result` claim counts them to find
/// the `try`s whose nearest Result scope is the candidate block.
pub(crate) fn user_function_depth_at(
    tokens: &[Token],
    at: usize,
    tt_owned: &std::collections::HashSet<usize>,
) -> usize {
    let mut stack = Vec::new();
    for (index, token) in tokens.iter().enumerate().take(at) {
        match token.kind {
            TokenKind::Punct(b'{') => stack.push(
                function_target_brace(tokens, index).is_some()
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

/// The innermost function-like boundary around a token: the kind of
/// user-written function an early `return` at the token would leave, or the
/// class code that has none. Constructors and generators syntactically
/// accept `return`, but a propagated Result would change their JavaScript
/// completion contract. A class static block and the rest of a class body
/// outside its methods are boundaries too: code there is not evaluated by the
/// function the class is written in (ECMA-262 §15.7), so a `return` written
/// there cannot reach that function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionTarget {
    Ordinary,
    Constructor,
    Generator,
    StaticBlock,
    ClassElement,
}

pub(crate) struct FunctionTargets {
    braced: Vec<Option<(usize, FunctionTarget)>>,
    arrows: Vec<(usize, usize)>,
    spans: Vec<(usize, usize)>,
    interpolations: Vec<(crate::ast::Span, FunctionTargets)>,
}

impl FunctionTargets {
    pub(crate) fn new(
        tokens: &[Token],
        tt_owned: &dyn Fn(&[Token]) -> std::collections::HashSet<usize>,
    ) -> Self {
        let owned = tt_owned(tokens);
        let mut braced = Vec::with_capacity(tokens.len() + 1);
        let mut stack: Vec<Option<(usize, FunctionTarget)>> = Vec::new();
        let mut interpolations = Vec::new();
        braced.push(None);
        for (index, token) in tokens.iter().enumerate() {
            match &token.kind {
                TokenKind::Punct(b'{') => {
                    let own = (!owned.contains(&index)
                        && !index
                            .checked_sub(1)
                            .is_some_and(|previous| owned.contains(&previous)))
                    .then(|| function_target_brace(tokens, index))
                    .flatten()
                    .map(|target| (index, target));
                    let innermost = own.or_else(|| stack.last().copied().flatten());
                    stack.push(innermost);
                }
                TokenKind::Punct(b'}') => {
                    stack.pop();
                }
                TokenKind::Template(parts) => {
                    for part in parts.iter() {
                        if let crate::lexer::TplPart::Interp { span, tokens } = part {
                            interpolations.push((*span, FunctionTargets::new(tokens, tt_owned)));
                        }
                    }
                }
                _ => {}
            }
            braced.push(stack.last().copied().flatten());
        }
        let arrows = tokens
            .iter()
            .enumerate()
            .filter(|(arrow, token)| {
                matches!(token.kind, TokenKind::Arrow) && !owned.contains(arrow)
            })
            .map(|(arrow, _)| (arrow, concise_arrow_end(tokens, arrow + 1)))
            .collect();
        FunctionTargets {
            braced,
            arrows,
            spans: tokens
                .iter()
                .map(|token| (token.span.start, token.span.end))
                .collect(),
            interpolations,
        }
    }

    #[cfg(test)]
    pub(crate) fn at(&self, at: usize) -> Option<FunctionTarget> {
        self.innermost(at).map(|(_, target)| target)
    }

    fn innermost(&self, at: usize) -> Option<(usize, FunctionTarget)> {
        let braced = self.braced[at.min(self.braced.len() - 1)];
        let before = self.arrows.partition_point(|(arrow, _)| *arrow < at);
        let concise_arrow = self.arrows[..before]
            .iter()
            .rev()
            .find(|(_, end)| *end > at)
            .map(|(arrow, _)| (*arrow, FunctionTarget::Ordinary));
        match (braced, concise_arrow) {
            (Some(braced), Some(arrow)) => Some(if braced.0 > arrow.0 { braced } else { arrow }),
            (Some(innermost), None) | (None, Some(innermost)) => Some(innermost),
            (None, None) => None,
        }
    }

    pub(crate) fn at_offset(&self, offset: usize) -> Option<FunctionTarget> {
        self.innermost_at_offset(offset).map(|(_, target)| target)
    }

    /// The byte offset where the innermost user function enclosing `offset`
    /// opens (its body's `{` or its concise arrow's `=>`).
    pub(crate) fn boundary_at_offset(&self, offset: usize) -> Option<usize> {
        self.innermost_at_offset(offset).map(|(start, _)| start)
    }

    fn innermost_at_offset(&self, offset: usize) -> Option<(usize, FunctionTarget)> {
        let at = self.spans.partition_point(|(start, _)| *start < offset);
        let own = |at: usize| {
            self.innermost(at)
                .map(|(token, target)| (self.spans[token].0, target))
        };
        match at.checked_sub(1) {
            Some(token) if self.spans[token].1 > offset => self
                .interpolations
                .iter()
                .find(|(span, _)| span.start <= offset && offset < span.end)
                .and_then(|(_, inner)| inner.innermost_at_offset(offset))
                .or_else(|| own(token)),
            _ => own(at),
        }
    }
}

/// Returns the innermost user function enclosing `at`.
#[cfg(test)]
pub(crate) fn function_target_at(tokens: &[Token], at: usize) -> Option<FunctionTarget> {
    user_function_target_at(tokens, at, &std::collections::HashSet::new())
}

/// Returns the innermost user-written function enclosing `at`, skipping the
/// match body braces and arm arrows in `tt_owned`, which open no function.
#[cfg(test)]
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
            _ if tokens[index].opens_bracket() => depth += 1,
            _ if tokens[index].closes_bracket() => {
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

/// The function-like boundary the `{` at `brace` opens, if it opens one:
/// the lexer's function-body, class-body, and static-block facts
/// ([`crate::lexer::TokenFacts`]).
pub(super) fn function_target_brace(tokens: &[Token], brace: usize) -> Option<FunctionTarget> {
    let facts = tokens.get(brace)?.facts;
    if facts.function_body() {
        Some(if facts.constructor_body() {
            FunctionTarget::Constructor
        } else if facts.generator_body() {
            FunctionTarget::Generator
        } else {
            FunctionTarget::Ordinary
        })
    } else if facts.static_block() {
        Some(FunctionTarget::StaticBlock)
    } else if facts.class_body() {
        Some(FunctionTarget::ClassElement)
    } else {
        None
    }
}

/// Whether the `{` at token index `k` opens a function body (see
/// [`in_function_body`]).
pub(super) fn function_body_brace(tokens: &[Token], k: usize) -> bool {
    tokens[k].facts.function_body()
}
