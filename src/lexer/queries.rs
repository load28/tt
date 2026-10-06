//! Questions codegen asks of a piece of TypeScript text, answered over the
//! lexer's tokens so strings, comments, templates, regular expressions, and
//! brackets are read exactly as the lexer reads them.

use super::{Token, TokenKind, TplPart, lex, lex_with_kind, number_end};
use crate::SourceKind;

/// The index of the token closing the bracket opened at `open`, counting
/// every bracket kind.
fn close_of(tokens: &[Token], open: usize) -> Option<usize> {
    Token::balancing_close(tokens, open)
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
    if !src.as_bytes()[from..end].iter().any(|byte| {
        matches!(
            byte,
            b',' | b'(' | b')' | b'[' | b']' | b'{' | b'}' | b'<' | b'>'
        )
    }) {
        return false;
    }
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
    primary_expression(src, from, end, kind).is_some()
}

/// True when `src[from..end]` is a primary expression whose value a member
/// access appended to it reads: a primary expression that does not end in
/// an optional chain.
///
/// A `.name`, `[key]`, or call written after an optional chain continues
/// that chain (ECMA-262 §13.3.9, `OptionalChain`), so it is skipped when
/// the chain short-circuits; a parenthesized expression is a
/// `PrimaryExpression` and ends the chain. `a?.b` is primary, but only
/// `(a?.b).c` reads `c` of the value `a?.b` evaluates to.
pub(crate) fn is_member_receiver(src: &str, from: usize, end: usize, kind: SourceKind) -> bool {
    let tokens = lex_with_kind(src, from, end, kind);
    if let Some(first) = tokens.first()
        && let TokenKind::Punct(byte) = first.kind
        && byte.is_ascii_digit()
    {
        let literal_end = number_end(src.as_bytes(), first.span.start, end);
        if tokens.iter().all(|token| token.span.start < literal_end)
            && src.as_bytes()[first.span.start..literal_end]
                .iter()
                .all(|byte| byte.is_ascii_digit() || *byte == b'_')
        {
            return false;
        }
    }
    primary_expression(src, from, end, kind) == Some(Primary::Closed)
}

/// Whether a primary expression ends in an optional chain.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Primary {
    Closed,
    OptionalChain,
}

fn primary_expression(src: &str, from: usize, end: usize, kind: SourceKind) -> Option<Primary> {
    crate::work::tick("primary expression checks");
    let tokens = lex_with_kind(src, from, end, kind);
    let word = |index: usize| {
        let token = &tokens[index];
        matches!(token.kind, TokenKind::Ident).then(|| &src[token.span.start..token.span.end])
    };
    let head = tokens.first()?;
    let mut shape = Primary::Closed;
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
                return None;
            }
            1
        }
        _ if head.opens_bracket() => close_of(&tokens, 0)? + 1,
        TokenKind::Str | TokenKind::Template(_) => 1,
        TokenKind::Punct(byte) if byte.is_ascii_digit() => tokens
            .iter()
            .position(|token| !token.facts.ends_expression() || token.facts.line_break_before())
            .filter(|&index| index > 0)
            .unwrap_or(tokens.len()),
        _ => return None,
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
            TokenKind::Punct(b'.') => name(at + 1)?,
            TokenKind::OptChain => {
                shape = Primary::OptionalChain;
                match tokens.get(at + 1).map(|token| &token.kind) {
                    Some(TokenKind::Punct(b'(' | b'[')) => close_of(&tokens, at + 1)? + 1,
                    _ => name(at + 1)?,
                }
            }
            TokenKind::Punct(b'!')
                if !matches!(
                    tokens.get(at + 1).map(|token| &token.kind),
                    Some(TokenKind::Punct(b'='))
                ) =>
            {
                at + 1
            }
            TokenKind::Punct(b'(' | b'[') => close_of(&tokens, at)? + 1,
            TokenKind::Template(_) => at + 1,
            _ => return None,
        };
    }
    Some(shape)
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

/// A statement the source ends by automatic semicolon insertion (ECMA-262
/// §12.10.1), because the statement after it cannot continue it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AutomaticSemicolon {
    /// Where the ended statement's last token ends.
    pub(crate) end: usize,
    /// Where the next statement's first token starts.
    pub(crate) next: usize,
}

/// Every statement boundary in `tokens` that an automatic semicolon makes,
/// template interpolations included, ordered by the next statement's start.
pub(crate) fn automatic_semicolons(tokens: &[Token]) -> Vec<AutomaticSemicolon> {
    let mut found = Vec::new();
    collect_automatic_semicolons(tokens, &mut found);
    found.sort_unstable_by_key(|boundary| boundary.next);
    found
}

fn collect_automatic_semicolons(tokens: &[Token], found: &mut Vec<AutomaticSemicolon>) {
    crate::stack::grow(|| collect_automatic_semicolons_grown(tokens, found));
}

fn collect_automatic_semicolons_grown(tokens: &[Token], found: &mut Vec<AutomaticSemicolon>) {
    for (index, token) in tokens.iter().enumerate() {
        if index > 0 && token.facts.asi_before() && token.facts.statement_start() {
            found.push(AutomaticSemicolon {
                end: tokens[index - 1].span.end,
                next: token.span.start,
            });
        }
        if let TokenKind::Template(parts) = &token.kind {
            for part in parts {
                if let TplPart::Interp { tokens, .. } = part {
                    collect_automatic_semicolons(tokens, found);
                }
            }
        }
    }
}

/// True when `text`, written on the line after a statement that ends with
/// an operand, would continue that statement instead of starting one
/// ([`statement_continues_after`] on its first token). ECMA-262 §12.10.2
/// ("Interesting Cases of Automatic Semicolon Insertion") names `(`, `[`,
/// a template, `+`, `-`, and `/`; TypeScript adds `<`. The answer comes
/// from the same token facts that find the source's own boundaries.
pub(crate) fn continues_statement(text: &str, kind: SourceKind) -> bool {
    let bytes = text.as_bytes();
    let (start, _) = crate::scanner::skip_trivia(bytes, 0, bytes.len());
    let line = &text[start..crate::scanner::line_end(bytes, start, bytes.len())];
    if line.is_empty() {
        return false;
    }
    let probe = format!("x\n{line}");
    statement_continues_after(&lex_with_kind(&probe, 0, probe.len(), kind), 1)
}

/// True when, in `tokens`, the token after the one ending at `end`
/// continues that token's statement: no automatic semicolon precedes it,
/// it starts no statement, and it is not a separator or a closing bracket.
pub(crate) fn statement_continues_after(tokens: &[Token], end: usize) -> bool {
    let index = tokens.partition_point(|token| token.span.start < end);
    if let Some(enclosing) = index.checked_sub(1).map(|at| &tokens[at])
        && enclosing.span.end > end
    {
        let TokenKind::Template(parts) = &enclosing.kind else {
            return false;
        };
        return parts.iter().any(|part| match part {
            TplPart::Interp { span, tokens } => {
                span.start <= end && end <= span.end && statement_continues_after(tokens, end)
            }
            TplPart::Raw(_) => false,
        });
    }
    if index == 0 || tokens[index - 1].span.end != end {
        return false;
    }
    tokens.get(index).is_some_and(|next| {
        !next.facts.boundary_before()
            && !matches!(
                next.kind,
                TokenKind::Punct(b';' | b'}' | b')' | b']' | b',')
            )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn a_member_receiver_ends_any_optional_chain() {
        for (text, receiver) in [
            ("s.trim()", true),
            ("a?.b", false),
            ("a?.[0]!.b", false),
            ("f?.()", false),
            ("a.b?.c!", false),
            ("(a?.b)", true),
            ("(a?.b).c", true),
            ("[a?.b]", true),
            ("f(a?.b)", true),
            ("a + b", false),
            ("5", false),
            ("1_000", false),
            ("5.5", true),
            ("1e3", true),
            ("0xff", true),
            ("5n", true),
        ] {
            assert_eq!(
                is_member_receiver(text, 0, text.len(), SourceKind::TypeScript),
                receiver,
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
    fn a_statement_continues_into_a_line_that_starts_with_an_operator_or_bracket() {
        for (text, continues) in [
            ("(a)", true),
            ("[a]", true),
            ("`a`", true),
            ("+a", true),
            ("-a", true),
            ("/a/.test(s)", true),
            ("<T>a", true),
            ("  /* c */ (a)", true),
            ("\n  (a)", true),
            ("a(b)", false),
            ("++a", false),
            ("--a", false),
            ("let a", false),
            ("{ a }", false),
            (";(a)", false),
            ("// (a)", false),
            ("", false),
        ] {
            assert_eq!(
                continues_statement(text, SourceKind::TypeScript),
                continues,
                "{text:?}"
            );
        }
    }

    #[test]
    fn automatic_semicolons_are_found_in_template_interpolations() {
        let src = "const a = 1\nb\nlet t = `${(() => { c\nd })()}`\n";
        let tokens = lex_with_kind(src, 0, src.len(), SourceKind::TypeScript);
        let starts: Vec<&str> = automatic_semicolons(&tokens)
            .iter()
            .map(|boundary| &src[boundary.next..boundary.next + 1])
            .collect();
        assert_eq!(starts, ["b", "l", "d"]);
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
