//! Assignment-target grammar shared by direct diagnostics and symbol probes.
//!
//! A target is a reference or an array/object pattern. Defaults and computed
//! property names are expressions, not write targets.

use swc_common::Spanned;

use super::*;
use crate::host_input::HostInput;

#[derive(Clone)]
pub(super) struct Target {
    pub(super) root: usize,
    pub(super) members: usize,
}

fn property_name_at(tokens: &[Token], k: usize) -> bool {
    match k.checked_sub(1).map(|at| &tokens[at].kind) {
        Some(TokenKind::OptChain) => true,
        Some(TokenKind::Punct(b'.')) => {
            !(k >= 3 && punct_at(tokens, k - 2, b'.') && punct_at(tokens, k - 3, b'.'))
        }
        _ => false,
    }
}

/// The reference that starts at `start`: an identifier or a parenthesized
/// operand, followed by member steps and non-null assertions. The tokens
/// delimit it; a reference with a wrapper in it (parentheses, `!`, `as`,
/// `satisfies`, `<T>`) is read as TypeScript reads it, through the one
/// definition of an access path ([`super::reference::access_path`]).
fn reference(
    src: &str,
    source_kind: SourceKind,
    tokens: &[Token],
    start: usize,
) -> Option<(usize, Vec<Target>)> {
    let first = tokens.get(start)?;
    let mut end = match first.kind {
        TokenKind::Ident if property_name_at(tokens, start) => return None,
        TokenKind::Ident => start + 1,
        // After a complete operand, `(` opens a call's arguments, not a
        // parenthesized operand: `f(x).y = 1` writes to what `f` returns.
        TokenKind::Punct(b'(') if start == 0 || !tokens[start - 1].facts.ends_expression() => {
            find_close_at(tokens, start)? + 1
        }
        _ => return None,
    };
    let mut wrapped = !matches!(first.kind, TokenKind::Ident);
    let mut members = 0;
    loop {
        end = match tokens.get(end).map(|t| &t.kind) {
            Some(TokenKind::Punct(b'.') | TokenKind::OptChain)
                if matches!(tokens.get(end + 1)?.kind, TokenKind::Ident) =>
            {
                members += 1;
                end + 2
            }
            Some(TokenKind::Punct(b'[')) => {
                members += 1;
                find_close_at(tokens, end)? + 1
            }
            Some(TokenKind::Punct(b'!')) if !punct_at(tokens, end + 1, b'=') => {
                wrapped = true;
                end + 1
            }
            _ => break,
        };
    }
    if !wrapped {
        return Some((
            end,
            vec![Target {
                root: start,
                members,
            }],
        ));
    }
    let from = first.span.start;
    let text = &src[from..tokens[end - 1].span.end];
    let input = HostInput::new(text);
    let mut parser = input.parser(source_kind);
    let expression = parser.parse_expr().ok()?;
    if !parser.take_errors().is_empty() || input.byte(expression.span().hi) != text.len() {
        return None;
    }
    let (root, members) = super::reference::access_path(&expression)?;
    let root_at = from + input.byte(root.span.lo);
    let root = start
        + tokens[start..end]
            .iter()
            .position(|token| token.span.start == root_at)?;
    Some((end, vec![Target { root, members }]))
}

fn target(
    src: &str,
    source_kind: SourceKind,
    tokens: &[Token],
    start: usize,
) -> Option<(usize, Vec<Target>)> {
    crate::stack::grow(|| target_grown(src, source_kind, tokens, start))
}

fn target_grown(
    src: &str,
    source_kind: SourceKind,
    tokens: &[Token],
    start: usize,
) -> Option<(usize, Vec<Target>)> {
    if !matches!(tokens.get(start)?.kind, TokenKind::Punct(b'[' | b'{')) {
        return reference(src, source_kind, tokens, start);
    }
    let object = punct_at(tokens, start, b'{');
    let close = find_close_at(tokens, start)?;
    let mut targets = Vec::new();
    for (mut from, to) in list_entries(tokens, start) {
        let rest = (0..3).all(|n| punct_at(tokens, from + n, b'.'));
        if rest {
            from += 3;
        }
        if object && !rest {
            let key_end = if punct_at(tokens, from, b'[') {
                find_close_at(tokens, from)? + 1
            } else {
                from + 1
            };
            if punct_at(tokens, key_end, b':') {
                from = key_end + 1;
            }
        }
        let (end, nested) = target(src, source_kind, tokens, from)?;
        // A default initializer is a read; only the target to its left writes.
        if end != to && assignment_op_at(tokens, end) != Some(1) {
            return None;
        }
        targets.extend(nested);
    }
    Some((close + 1, targets))
}

/// Root identifiers of property writes, indexed in source coordinates so the
/// scope walk resolves each occurrence where it is written.
pub(super) fn writes(
    src: &str,
    source_kind: SourceKind,
    tokens: &[Token],
) -> std::collections::HashSet<usize> {
    let mut out = std::collections::HashSet::new();
    for start in 0..tokens.len() {
        let Some((end, targets)) = target(src, source_kind, tokens, start) else {
            continue;
        };
        let prefix = start.checked_sub(1).is_some_and(|at| {
            matches!(tokens[at].kind, TokenKind::Ident)
                && &src[tokens[at].span.start..tokens[at].span.end] == "delete"
        }) || start.checked_sub(2).is_some_and(|at| incdec_at(tokens, at));
        let loop_target = start.checked_sub(2).is_some_and(|at| {
            matches!(tokens[at].kind, TokenKind::Ident)
                && &src[tokens[at].span.start..tokens[at].span.end] == "for"
                && punct_at(tokens, at + 1, b'(')
        }) && matches!(tokens.get(end), Some(t) if matches!(t.kind, TokenKind::Ident)
            && matches!(&src[t.span.start..t.span.end], "of" | "in"));
        if prefix
            || assignment_op_at(tokens, end).is_some()
            || incdec_at(tokens, end)
            || loop_target
        {
            for target in targets.into_iter().filter(|target| target.members > 0) {
                out.insert(tokens[target.root].span.start);
            }
        }
    }
    out
}
