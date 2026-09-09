//! Structural parsing of tt pipeline expressions (`head |> step |> ...`).
//!
//! Contract safety: `|>` (a `|` immediately followed by `>`) cannot occur
//! anywhere in valid TypeScript — after a `|` (bitwise OR / union type) an
//! expression or a type must follow, and `>` starts neither — so the lexer
//! fuses it into a single [`TokenKind::PipeOp`] token and claiming it never
//! affects the passthrough contract. Inside strings, comments, regexes, and
//! template text the byte pair is trivia/literal content and never lexes as
//! a token.
//!
//! The *head* is found by the main token loop, which tracks the start of
//! the expression currently being scanned (see `expr start tracking` in
//! [`super`]); this module owns the forward scan over the steps. A head
//! that is exactly the bare identifier `flow` is the composition keyword
//! (`flow |> f |> g` builds a function instead of flowing a value) — a
//! contextual keyword, so a variable named `flow` still pipes when
//! parenthesized (`(flow) |> f`). A step
//! runs to the next top-level `|>` or to a terminator the pipeline cannot
//! contain at its top level (`;`, `,`, an unmatched closer, or the region
//! end). A top-level `?`, `:`, `=` (assignment), `=>`, or statement-only
//! keyword aborts the claim — ternaries and arrow functions must be
//! parenthesized (a normative rule, like the match scrutinee parens) — and
//! the unclaimed `|>` is recorded for the semantic phase to report.

use super::cursor::dotted_at;
use crate::ast::{PipeExpr, PipeHeadKind, PipeStep, PipeStepKind, Span};
use crate::lexer::{Token, TokenKind};

/// True for a `=` Punct that is (the start of) an assignment operator —
/// not part of `==`/`===`/`!=`/`<=`/`>=`. Compound assignments (`+=`,
/// `&&=`, ...) classify as assignment via their trailing `=`.
pub(super) fn is_assignment_eq(bytes: &[u8], span: Span) -> bool {
    let prev = span.start.checked_sub(1).map(|p| bytes[p]);
    let next = bytes.get(span.end).copied();
    next != Some(b'=') && !matches!(prev, Some(b'=' | b'!' | b'<' | b'>'))
}

/// True when the head is exactly the bare `flow` keyword — one identifier
/// token spelled `flow`. A dotted or called head (`a.flow`, `flow()`) has
/// more tokens and stays an ordinary value head.
fn head_kind(
    parser: &super::Parser,
    tokens: &[Token],
    head_idx: usize,
    pipe_idx: usize,
) -> PipeHeadKind {
    if pipe_idx - head_idx != 1 || !matches!(tokens[head_idx].kind, TokenKind::Ident) {
        return PipeHeadKind::Expression;
    }
    match &parser.src[tokens[head_idx].span.start..tokens[head_idx].span.end] {
        "flow" => PipeHeadKind::Flow,
        "super" => PipeHeadKind::BareSuper,
        _ => PipeHeadKind::Expression,
    }
}

/// `tokens[pipe_idx]` is a `|>` token and `tokens[head_idx..pipe_idx]` is
/// the head expression (non-empty, guaranteed by the caller). Parses the
/// step chain; on success returns the token index just past the last step
/// and the parsed pipeline. `None` leaves everything unclaimed (the caller
/// records the stray `|>` for sema).
pub(super) enum Attempt {
    Parsed(usize, PipeExpr),
    MalformedOptional {
        next: usize,
        head_span: Span,
        error_span: Span,
        extent: Span,
    },
}

pub(super) fn parse_pipeline(
    parser: &super::Parser,
    tokens: &[Token],
    head_idx: usize,
    pipe_idx: usize,
) -> Option<Attempt> {
    let head_span = Span {
        start: tokens[head_idx].span.start,
        end: tokens[pipe_idx - 1].span.end,
    };
    let case_test = head_idx.checked_sub(1).is_some_and(|before| {
        matches!(tokens[before].kind, TokenKind::Ident)
            && &parser.src[tokens[before].span.start..tokens[before].span.end] == "case"
    });

    let mut steps: Vec<PipeStep> = Vec::new();
    let mut k = pipe_idx;
    while matches!(tokens.get(k).map(|t| &t.kind), Some(TokenKind::PipeOp)) {
        k += 1;
        let step_from = k;
        let mut depth = 0usize;
        while let Some(t) = tokens.get(k) {
            match &t.kind {
                TokenKind::PipeOp if depth == 0 => break,
                TokenKind::JsxRaw if depth == 0 => break,
                TokenKind::Punct(b';' | b',') if depth == 0 => break,
                TokenKind::Punct(b')' | b']' | b'}') if depth == 0 => break,
                TokenKind::Punct(b':') if depth == 0 && case_test => break,
                TokenKind::Punct(b'?' | b':') if depth == 0 => return None,
                TokenKind::Arrow if depth == 0 => return None,
                TokenKind::Punct(b'=') if depth == 0 && is_assignment_eq(parser.bytes, t.span) => {
                    return None;
                }
                TokenKind::Ident
                    if depth == 0
                        && !dotted_at(tokens, step_from, k)
                        && super::tries::STMT_ONLY_WORDS
                            .contains(&&parser.src[t.span.start..t.span.end]) =>
                {
                    return None;
                }
                TokenKind::Punct(b'(' | b'[' | b'{') => depth += 1,
                TokenKind::Punct(b')' | b']' | b'}') => depth -= 1,
                _ => {}
            }
            k += 1;
        }
        if k == step_from {
            return None; // empty step (`x |> |> f`, `x |>;`, trailing `|>`)
        }

        // A step starting with `.` + identifier is the existing postfix
        // form. An optional postfix commits at `?.` and validates its whole
        // tail atomically: no prefix of an unsupported tail may be emitted.
        let kind = match &tokens[step_from].kind {
            TokenKind::Punct(b'.') => {
                if !matches!(
                    tokens.get(step_from + 1).map(|t| &t.kind),
                    Some(TokenKind::Ident)
                ) {
                    return None;
                }
                PipeStepKind::Postfix { optional: false }
            }
            TokenKind::OptChain => {
                if let Err(error_span) = validate_optional_tail(tokens, step_from, k) {
                    let (next, end) = malformed_pipeline_end(tokens, k);
                    return Some(Attempt::MalformedOptional {
                        next,
                        head_span,
                        error_span,
                        extent: Span {
                            start: head_span.start,
                            end,
                        },
                    });
                }
                PipeStepKind::Postfix { optional: true }
            }
            _ => PipeStepKind::Call,
        };

        let span = Span {
            start: tokens[step_from].span.start,
            end: tokens[k - 1].span.end,
        };
        steps.push(PipeStep {
            span,
            kind,
            body: parser.parse_expression_tokens(&tokens[step_from..k], span.start, span.end),
        });
    }
    if steps.is_empty() {
        return None;
    }

    let head_kind = head_kind(parser, tokens, head_idx, pipe_idx);
    let head = (head_kind != PipeHeadKind::Flow).then(|| {
        parser.parse_expression_tokens(&tokens[head_idx..pipe_idx], head_span.start, head_span.end)
    });
    Some(Attempt::Parsed(
        k,
        PipeExpr {
            head_span,
            head_kind,
            head,
            steps,
        },
    ))
}

/// Validates the complete optional postfix tail in `tokens[from..to]`.
///
/// The grammar is intentionally expressed as one repeated postfix model:
/// the first operation is optional (`?.name`, `?.[key]`, `?.(args)`), then
/// ordinary and optional member/index/call operations may follow. Delimited
/// operands remain ordinary recursively parsed programs; this function owns
/// only the chain boundary, not TypeScript expression syntax inside them.
fn validate_optional_tail(tokens: &[Token], from: usize, to: usize) -> Result<(), Span> {
    let mut k = optional_operation(tokens, from, to)?;
    while k < to {
        k = match tokens[k].kind {
            TokenKind::Punct(b'.') => named_operation(tokens, k, to)?,
            TokenKind::OptChain => optional_operation(tokens, k, to)?,
            TokenKind::Punct(b'(' | b'[') => delimited_operation(tokens, k, to)?,
            _ => return Err(tokens[k].span),
        };
    }
    Ok(())
}

fn optional_operation(tokens: &[Token], at: usize, to: usize) -> Result<usize, Span> {
    debug_assert!(matches!(tokens[at].kind, TokenKind::OptChain));
    let Some(next) = tokens.get(at + 1).filter(|_| at + 1 < to) else {
        return Err(tokens[at].span);
    };
    match next.kind {
        TokenKind::Ident => Ok(at + 2),
        TokenKind::Punct(b'(' | b'[') => delimited_operation(tokens, at + 1, to),
        _ => Err(Span {
            start: tokens[at].span.start,
            end: next.span.end,
        }),
    }
}

fn named_operation(tokens: &[Token], at: usize, to: usize) -> Result<usize, Span> {
    let Some(name) = tokens.get(at + 1).filter(|_| at + 1 < to) else {
        return Err(tokens[at].span);
    };
    if matches!(name.kind, TokenKind::Ident) {
        Ok(at + 2)
    } else {
        Err(Span {
            start: tokens[at].span.start,
            end: name.span.end,
        })
    }
}

fn delimited_operation(tokens: &[Token], at: usize, to: usize) -> Result<usize, Span> {
    let Some(close) = super::cursor::find_close_at(tokens, at).filter(|close| *close < to) else {
        return Err(tokens[at].span);
    };
    Ok(close + 1)
}

/// Finds the recovery extent of a pipeline after one optional step has
/// committed but failed. This is deliberately more permissive than parsing:
/// it consumes the remaining top-level `|>` steps up to the host delimiter so
/// one broken pipeline produces one owned diagnostic rather than a cascade of
/// stray-pipe errors.
fn malformed_pipeline_end(tokens: &[Token], mut k: usize) -> (usize, usize) {
    let mut depth = 0usize;
    let mut end = tokens
        .get(k.saturating_sub(1))
        .map_or(0, |token| token.span.end);
    while let Some(token) = tokens.get(k) {
        match token.kind {
            TokenKind::JsxRaw if depth == 0 => break,
            TokenKind::Punct(b';' | b',') if depth == 0 => break,
            TokenKind::Punct(b')' | b']' | b'}') if depth == 0 => break,
            TokenKind::Punct(b'(' | b'[' | b'{') => depth += 1,
            TokenKind::Punct(b')' | b']' | b'}') => depth -= 1,
            _ => {}
        }
        end = token.span.end;
        k += 1;
    }
    (k, end)
}
