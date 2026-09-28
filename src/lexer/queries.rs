//! Questions codegen asks of a piece of TypeScript text, answered over the
//! lexer's tokens so strings, comments, templates, regular expressions, and
//! brackets are read exactly as the lexer reads them.

use super::{Token, TokenKind, TplPart, lex, lex_with_kind};
use crate::SourceKind;

/// The index of the token closing the bracket opened at `open`, counting
/// every bracket kind.
fn close_of(tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        match token.kind {
            _ if token.opens_bracket() => depth += 1,
            _ if token.closes_bracket() => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// True when `src[from..end]` has a `,` outside every bracket, string,
/// template and comment — the one operator that binds looser than an
/// initializer, an assignment right-hand side, a `return` operand or a
/// single call argument, and so the one reason such a position has to keep
/// the parentheses codegen wrapped a value in.
///
/// A `,` inside type arguments (`a as Map<K, V>`, `f<A, B>(x)`) is inside
/// a bracket pair, as the token facts record it. Anything past an
/// unbalanced closer counts as top level: the answer is only ever used to
/// *keep* parentheses, so erring that way costs a pair of parentheses and
/// never a meaning.
pub(crate) fn has_top_level_comma(src: &str, from: usize, end: usize, kind: SourceKind) -> bool {
    let mut depth = 0usize;
    for token in &lex_with_kind(src, from, end, kind) {
        match token.kind {
            _ if token.opens_bracket() => depth += 1,
            _ if token.closes_bracket() => match depth.checked_sub(1) {
                Some(outer) => depth = outer,
                None => return true,
            },
            TokenKind::Punct(b',') if depth == 0 => return true,
            _ => {}
        }
    }
    depth > 0
}

/// True when `src[from..end]` is one *primary* expression — a single operand
/// with nothing at its top level but member access, calls, indexing,
/// non-null assertions and tagged templates.
///
/// This is the question a receiver position asks: member access binds
/// tighter than every operator, so only a primary receiver can lose the
/// parentheses codegen wrapped it in. A keyword operand (`await x`,
/// `new C`, `x as T`) is not primary — `(await x).f` and `await x.f` are
/// different expressions.
pub(crate) fn is_primary_expression(src: &str, from: usize, end: usize, kind: SourceKind) -> bool {
    let tokens = lex_with_kind(src, from, end, kind);
    let word = |index: usize| {
        let token = &tokens[index];
        matches!(token.kind, TokenKind::Ident).then(|| &src[token.span.start..token.span.end])
    };
    let Some(head) = tokens.first() else {
        return false;
    };
    let mut at = match head.kind {
        TokenKind::Ident => {
            if matches!(
                word(0),
                Some(
                    "await"
                        | "class"
                        | "delete"
                        | "function"
                        | "new"
                        | "typeof"
                        | "void"
                        | "yield"
                        | "async"
                )
            ) {
                return false;
            }
            1
        }
        _ if head.opens_bracket() => match close_of(&tokens, 0) {
            Some(close) => close + 1,
            None => return false,
        },
        TokenKind::Str | TokenKind::Template(_) => 1,
        TokenKind::Punct(byte) if byte.is_ascii_digit() => tokens
            .iter()
            .position(|token| !token.facts.ends_expression() || token.facts.line_break_before())
            .filter(|&index| index > 0)
            .unwrap_or(tokens.len()),
        _ => return false,
    };
    let name = |at: usize| {
        let at = match tokens.get(at).map(|token| &token.kind) {
            Some(TokenKind::Punct(b'#')) => at + 1,
            _ => at,
        };
        matches!(
            tokens.get(at).map(|token| &token.kind),
            Some(TokenKind::Ident)
        )
        .then_some(at + 1)
    };
    while let Some(token) = tokens.get(at) {
        at = match token.kind {
            TokenKind::Punct(b'.') => match name(at + 1) {
                Some(next) => next,
                None => return false,
            },
            TokenKind::OptChain => match tokens.get(at + 1).map(|token| &token.kind) {
                Some(TokenKind::Punct(b'(' | b'[')) => match close_of(&tokens, at + 1) {
                    Some(close) => close + 1,
                    None => return false,
                },
                _ => match name(at + 1) {
                    Some(next) => next,
                    None => return false,
                },
            },
            TokenKind::Punct(b'!')
                if !matches!(
                    tokens.get(at + 1).map(|token| &token.kind),
                    Some(TokenKind::Punct(b'='))
                ) =>
            {
                at + 1
            }
            TokenKind::Punct(b'(' | b'[') => match close_of(&tokens, at) {
                Some(close) => close + 1,
                None => return false,
            },
            TokenKind::Template(_) => at + 1,
            _ => return false,
        };
    }
    true
}

/// True if `src[from..end]` contains an `await` in code position, template
/// interpolations included, outside the bodies of nested functions and
/// classes, where an `await` belongs to them.
pub(crate) fn contains_await(src: &str, from: usize, end: usize) -> bool {
    fn scan(src: &str, tokens: &[Token]) -> bool {
        let mut at = 0usize;
        while let Some(token) = tokens.get(at) {
            match &token.kind {
                TokenKind::Ident => match &src[token.span.start..token.span.end] {
                    "await" => return true,
                    "function" | "class" => {
                        if let Some(body) = (at..tokens.len())
                            .find(|&index| matches!(tokens[index].kind, TokenKind::Punct(b'{')))
                        {
                            at = close_of(tokens, body).map_or(tokens.len(), |close| close + 1);
                            continue;
                        }
                    }
                    _ => {}
                },
                TokenKind::Template(parts) => {
                    for part in parts.iter() {
                        if let TplPart::Interp { tokens, .. } = part
                            && scan(src, tokens)
                        {
                            return true;
                        }
                    }
                }
                _ => {}
            }
            at += 1;
        }
        false
    }
    scan(src, &lex(src, from, end))
}

/// The names a type parameter list (`<T, const U extends V = W>`, brackets
/// included) declares, in order.
pub(crate) fn type_parameter_names(generics: &str) -> Vec<String> {
    let tokens = lex(generics, 0, generics.len());
    let mut names = Vec::new();
    let mut depth = 0usize;
    let mut expecting = true;
    for token in &tokens {
        match token.kind {
            _ if token.opens_bracket() => depth += 1,
            _ if token.closes_bracket() => depth = depth.saturating_sub(1),
            TokenKind::Punct(b',') if depth == 1 => expecting = true,
            TokenKind::Ident if depth == 1 && expecting => {
                let word = &generics[token.span.start..token.span.end];
                if !matches!(word, "const" | "in" | "out") {
                    names.push(word.to_owned());
                    expecting = false;
                }
            }
            _ => {}
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn await_scan_stops_at_nested_function_and_class_bodies() {
        let nested =
            "function nested() { await later(); } class C { async m() { await later(); } }";
        assert!(!contains_await(nested, 0, nested.len()));

        let outer = "await now(); function nested() { await later(); }";
        assert!(contains_await(outer, 0, outer.len()));

        let template = "`${await now()}`";
        assert!(contains_await(template, 0, template.len()));
    }

    #[test]
    fn primary_expressions_are_single_operands() {
        for (text, primary) in [
            ("s.trim()", true),
            ("a?.[0]!.b", true),
            ("f`x`", true),
            ("(await p)", true),
            ("await p", false),
            ("a + b", false),
            ("x as T", false),
            ("1.5", true),
            ("0xff.toString()", true),
            ("this.#x", true),
            ("/a/.test(s)", false),
        ] {
            assert_eq!(
                is_primary_expression(text, 0, text.len(), SourceKind::TypeScript),
                primary,
                "{text}"
            );
        }
    }

    #[test]
    fn a_comma_counts_only_at_the_top_level() {
        for (text, comma) in [
            ("a, b", true),
            ("f(a, b)", false),
            ("`${a, b}`", false),
            ("/,/.test(s)", false),
            ("x / 2, y", true),
            ("a as Map<K, V>", false),
            ("f<A, B>(x)", false),
            ("a < b, c > d", true),
        ] {
            assert_eq!(
                has_top_level_comma(text, 0, text.len(), SourceKind::TypeScript),
                comma,
                "{text}"
            );
        }
    }

    #[test]
    fn type_parameter_names_skip_modifiers_and_constraints() {
        assert_eq!(
            type_parameter_names(
                "<const T extends Map<K, V> = {}, in out U, V = (a: string) => void>"
            ),
            ["T", "U", "V"]
        );
    }
}
