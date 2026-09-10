//! Assignment-target grammar shared by direct diagnostics and symbol probes.
//!
//! A target is a reference, a parenthesized target, or an array/object pattern.
//! Defaults and computed property names are expressions, not write targets.

use super::*;

#[derive(Clone)]
pub(super) struct Target {
    pub(super) root: usize,
    pub(super) members: usize,
}

fn reference(tokens: &[Token], start: usize) -> Option<(usize, Vec<Target>)> {
    let (mut end, mut targets) = match tokens.get(start)?.kind {
        TokenKind::Ident => (
            start + 1,
            vec![Target {
                root: start,
                members: 0,
            }],
        ),
        TokenKind::Punct(b'(') => {
            let close = find_close_at(tokens, start)?;
            let (end, targets) = target(tokens, start + 1)?;
            if end != close {
                return None;
            }
            (close + 1, targets)
        }
        _ => return None,
    };
    loop {
        let next = match tokens.get(end).map(|t| &t.kind) {
            Some(TokenKind::Punct(b'.') | TokenKind::OptChain)
                if matches!(tokens.get(end + 1)?.kind, TokenKind::Ident) =>
            {
                end + 2
            }
            Some(TokenKind::Punct(b'[')) => find_close_at(tokens, end)? + 1,
            Some(TokenKind::Punct(b'!')) if !punct_at(tokens, end + 1, b'=') => {
                end += 1;
                continue;
            }
            _ => break,
        };
        if targets.len() != 1 {
            return None;
        }
        targets[0].members += 1;
        end = next;
    }
    Some((end, targets))
}

fn target(tokens: &[Token], start: usize) -> Option<(usize, Vec<Target>)> {
    if !matches!(tokens.get(start)?.kind, TokenKind::Punct(b'[' | b'{')) {
        return reference(tokens, start);
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
        let (end, nested) = target(tokens, from)?;
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
pub(super) fn writes(src: &str, tokens: &[Token]) -> std::collections::HashSet<usize> {
    let mut out = std::collections::HashSet::new();
    for start in 0..tokens.len() {
        let Some((end, targets)) = target(tokens, start) else {
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
