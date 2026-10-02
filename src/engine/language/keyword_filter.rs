use swc_common::Spanned;
use swc_ecma_ast::{
    Class, TsAsExpr, TsConstAssertion, TsExprWithTypeArgs, TsInterfaceDecl, TsSatisfiesExpr,
    TsType, TsTypeAliasDecl, TsTypeAnn, TsTypeAssertion, TsTypeParam, TsTypePredicate, TsTypeQuery,
    TsTypeRef,
};
use swc_ecma_visit::{Visit, VisitWith};

use crate::lexer::{Token, TokenKind, TplPart};

const TYPESCRIPT_KEYWORDS: [&str; 85] = [
    "abstract",
    "accessor",
    "any",
    "as",
    "asserts",
    "assert",
    "bigint",
    "boolean",
    "break",
    "case",
    "catch",
    "class",
    "continue",
    "const",
    "constructor",
    "debugger",
    "declare",
    "default",
    "defer",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "from",
    "function",
    "get",
    "if",
    "immediate",
    "implements",
    "import",
    "in",
    "infer",
    "instanceof",
    "interface",
    "intrinsic",
    "is",
    "keyof",
    "let",
    "module",
    "namespace",
    "never",
    "new",
    "null",
    "number",
    "object",
    "package",
    "private",
    "protected",
    "public",
    "override",
    "out",
    "readonly",
    "require",
    "global",
    "return",
    "satisfies",
    "set",
    "static",
    "string",
    "super",
    "switch",
    "symbol",
    "this",
    "throw",
    "true",
    "try",
    "type",
    "typeof",
    "undefined",
    "unique",
    "unknown",
    "using",
    "var",
    "void",
    "while",
    "with",
    "yield",
    "async",
    "await",
    "of",
];

const SCANNED_KEYWORDS: [&str; 18] = [
    "true",
    "false",
    "typeof",
    "extends",
    "keyof",
    "any",
    "unknown",
    "number",
    "bigint",
    "object",
    "boolean",
    "string",
    "symbol",
    "void",
    "undefined",
    "never",
    "intrinsic",
    "null",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Gap {
    Equals,
    As,
    Satisfies,
    Extends,
}

#[derive(Default)]
struct TypeSyntax {
    origin: Option<crate::host_input::HostOrigin>,
    types: Vec<(usize, usize)>,
    values: Vec<(usize, usize)>,
    colons: Vec<usize>,
    angles: Vec<usize>,
    asserts: Vec<usize>,
    gaps: Vec<(usize, usize, Gap)>,
}

impl TypeSyntax {
    fn byte(&self, position: swc_common::BytePos) -> usize {
        self.origin
            .expect("the visitor is built with an origin")
            .byte(position)
    }

    fn span(&self, span: swc_common::Span) -> (usize, usize) {
        (self.byte(span.lo), self.byte(span.hi))
    }

    fn gap(&mut self, from: swc_common::BytePos, to: swc_common::BytePos, gap: Gap) {
        let (from, to) = (self.byte(from), self.byte(to));
        self.gaps.push((from, to, gap));
    }
}

impl Visit for TypeSyntax {
    fn visit_ts_type(&mut self, node: &TsType) {
        let span = self.span(node.span());
        self.types.push(span);
        node.visit_children_with(self);
    }

    fn visit_ts_type_query(&mut self, node: &TsTypeQuery) {
        let span = self.span(node.expr_name.span());
        self.values.push(span);
        node.visit_children_with(self);
    }

    fn visit_ts_type_ann(&mut self, node: &TsTypeAnn) {
        let start = self.byte(node.span.lo);
        self.colons.push(start);
        node.visit_children_with(self);
    }

    fn visit_ts_type_ref(&mut self, node: &TsTypeRef) {
        if let Some(arguments) = &node.type_params {
            let start = self.byte(arguments.span.lo);
            self.angles.push(start);
        }
        node.visit_children_with(self);
    }

    fn visit_ts_type_assertion(&mut self, node: &TsTypeAssertion) {
        let start = self.byte(node.span.lo);
        self.angles.push(start);
        node.visit_children_with(self);
    }

    fn visit_ts_type_predicate(&mut self, node: &TsTypePredicate) {
        if node.asserts {
            let start = self.byte(node.span.lo);
            self.asserts.push(start);
        }
        node.visit_children_with(self);
    }

    fn visit_ts_type_alias_decl(&mut self, node: &TsTypeAliasDecl) {
        let before = node
            .type_params
            .as_ref()
            .map_or(node.id.span.hi, |parameters| parameters.span.hi);
        self.gap(before, node.type_ann.span().lo, Gap::Equals);
        node.visit_children_with(self);
    }

    fn visit_ts_type_param(&mut self, node: &TsTypeParam) {
        if let Some(constraint) = &node.constraint {
            self.gap(node.name.span.hi, constraint.span().lo, Gap::Extends);
        }
        if let Some(default) = &node.default {
            let before = node
                .constraint
                .as_ref()
                .map_or(node.name.span.hi, |constraint| constraint.span().hi);
            self.gap(before, default.span().lo, Gap::Equals);
        }
        node.visit_children_with(self);
    }

    fn visit_ts_as_expr(&mut self, node: &TsAsExpr) {
        self.gap(node.expr.span().hi, node.type_ann.span().lo, Gap::As);
        node.visit_children_with(self);
    }

    fn visit_ts_const_assertion(&mut self, node: &TsConstAssertion) {
        self.gap(node.expr.span().hi, node.span.hi, Gap::As);
        node.visit_children_with(self);
    }

    fn visit_ts_satisfies_expr(&mut self, node: &TsSatisfiesExpr) {
        self.gap(node.expr.span().hi, node.type_ann.span().lo, Gap::Satisfies);
        node.visit_children_with(self);
    }

    fn visit_class(&mut self, node: &Class) {
        for implemented in &node.implements {
            let span = self.span(implemented.span);
            self.types.push(span);
        }
        node.visit_children_with(self);
    }

    fn visit_ts_interface_decl(&mut self, node: &TsInterfaceDecl) {
        let body = self.span(node.body.span);
        self.types.push(body);
        node.extends
            .iter()
            .for_each(|heritage: &TsExprWithTypeArgs| {
                let span = self.span(heritage.span);
                self.types.push(span);
            });
        node.visit_children_with(self);
    }
}

fn flatten(tokens: &[Token], out: &mut Vec<(usize, usize, Lexeme)>) {
    for token in tokens {
        match &token.kind {
            TokenKind::Template(parts) => {
                for part in parts.iter() {
                    match part {
                        TplPart::Raw(span) => out.push((span.start, span.end, Lexeme::Text)),
                        TplPart::Interp { tokens, .. } => flatten(tokens, out),
                    }
                }
            }
            TokenKind::Str | TokenKind::Regex | TokenKind::JsxRaw => {
                out.push((token.span.start, token.span.end, Lexeme::Text))
            }
            TokenKind::Ident => out.push((token.span.start, token.span.end, Lexeme::Word)),
            TokenKind::OptChain => out.push((token.span.start, token.span.end, Lexeme::OptChain)),
            TokenKind::Punct(byte) => {
                out.push((token.span.start, token.span.end, Lexeme::Punct(*byte)))
            }
            _ => out.push((token.span.start, token.span.end, Lexeme::Other)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lexeme {
    Word,
    Text,
    OptChain,
    Punct(u8),
    Other,
}

struct Tokens<'a> {
    code: &'a str,
    all: Vec<(usize, usize, Lexeme)>,
}

impl Tokens<'_> {
    fn text(&self, index: usize) -> &str {
        let (start, end, _) = self.all[index];
        &self.code[start..end]
    }

    fn is_word(&self, index: usize, word: &str) -> bool {
        self.all[index].2 == Lexeme::Word && self.text(index) == word
    }

    fn before(&self, offset: usize) -> Option<usize> {
        self.all.iter().rposition(|&(_, end, _)| end <= offset)
    }

    fn matching_open(&self, close: usize, open: u8, closing: u8) -> Option<usize> {
        let mut depth = 0usize;
        for index in (0..close).rev() {
            match self.all[index].2 {
                Lexeme::Punct(byte) if byte == closing => depth += 1,
                Lexeme::Punct(byte) if byte == open => {
                    if depth == 0 {
                        return Some(index);
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }
        None
    }

    fn possibly_type_arguments(&self, context: usize) -> bool {
        let mut index = Some(context);
        let mut remaining = 0usize;
        while let Some(at) = index {
            match self.all[at].2 {
                Lexeme::Punct(b'<') => {
                    let mut called = at.checked_sub(1);
                    if called.is_some_and(|before| self.all[before].2 == Lexeme::OptChain) {
                        called = called.and_then(|before| before.checked_sub(1));
                    }
                    let Some(called) = called.filter(|&before| {
                        self.all[before].2 == Lexeme::Word
                            && !TYPESCRIPT_KEYWORDS.contains(&self.text(before))
                    }) else {
                        return false;
                    };
                    if remaining == 0 {
                        return true;
                    }
                    remaining -= 1;
                    index = Some(called);
                }
                Lexeme::Punct(b'>') => remaining += 1,
                Lexeme::Punct(b'}') => index = self.matching_open(at, b'{', b'}'),
                Lexeme::Punct(b')') => index = self.matching_open(at, b'(', b')'),
                Lexeme::Punct(b']') => index = self.matching_open(at, b'[', b']'),
                Lexeme::Punct(b',' | b'.' | b'|' | b'?' | b':' | b'&') => {}
                Lexeme::Punct(byte) if byte.is_ascii_digit() => {}
                Lexeme::Text => {}
                Lexeme::Other if &self.code[self.all[at].0..self.all[at].1] == "=>" => {}
                Lexeme::Word => {
                    let word = self.text(at);
                    if TYPESCRIPT_KEYWORDS.contains(&word) && !SCANNED_KEYWORDS.contains(&word) {
                        return false;
                    }
                }
                _ => return false,
            }
            index = index.and_then(|current| current.checked_sub(1));
        }
        false
    }
}

pub(super) fn offers_all_keywords(
    code: &str,
    kind: crate::SourceKind,
    module: &swc_ecma_ast::Program,
    origin: crate::host_input::HostOrigin,
    at: usize,
) -> bool {
    let lexed = crate::lexer::lex_with_kind(code, 0, code.len(), kind);
    if crate::lexer::comments(code, &lexed).iter().any(|comment| {
        comment.start < at
            && (at < comment.end || at == comment.end && code[comment.start..].starts_with("//"))
    }) {
        return false;
    }
    let mut all = Vec::new();
    flatten(&lexed, &mut all);
    all.sort_by_key(|&(start, end, _)| (start, std::cmp::Reverse(end)));
    let tokens = Tokens { code, all };
    if tokens
        .all
        .iter()
        .any(|&(start, end, lexeme)| lexeme == Lexeme::Text && start < at && at < end)
    {
        return false;
    }
    let mut syntax = TypeSyntax {
        origin: Some(origin),
        ..TypeSyntax::default()
    };
    module.visit_with(&mut syntax);

    let previous = tokens.all.iter().rposition(|&(start, _, _)| start < at);
    let touching = previous.filter(|&index| {
        let (_, end, lexeme) = tokens.all[index];
        lexeme == Lexeme::Word && at <= end
    });
    let context = match touching {
        Some(index) => tokens.before(tokens.all[index].0),
        None => previous,
    };

    let location = touching
        .or_else(|| tokens.all.iter().position(|&(_, end, _)| end > at))
        .filter(|&index| tokens.all[index].2 == Lexeme::Word)
        .map(|index| tokens.all[index].0);
    let part_of_type = location.is_some_and(|start| {
        let innermost = |spans: &[(usize, usize)]| {
            spans
                .iter()
                .filter(|&&(from, to)| from <= start && start < to)
                .map(|&(from, to)| to - from)
                .min()
        };
        match (innermost(&syntax.types), innermost(&syntax.values)) {
            (Some(ty), Some(value)) => ty < value,
            (Some(_), None) => true,
            _ => false,
        }
    });

    let Some(context) = context else {
        return !part_of_type;
    };
    let (context_start, _, context_lexeme) = tokens.all[context];
    let value_location = tokens.is_word(context, "typeof")
        || tokens.is_word(context, "asserts") && syntax.asserts.contains(&context_start);
    let in_gap = |kind: Gap| {
        syntax
            .gaps
            .iter()
            .any(|&(from, to, gap)| gap == kind && from <= context_start && context_start < to)
    };
    let type_context = match context_lexeme {
        Lexeme::Punct(b':') => syntax.colons.contains(&context_start),
        Lexeme::Punct(b'=') => in_gap(Gap::Equals),
        Lexeme::Punct(b'<') => syntax.angles.contains(&context_start),
        Lexeme::Word if tokens.text(context) == "as" => in_gap(Gap::As),
        Lexeme::Word if tokens.text(context) == "satisfies" => in_gap(Gap::Satisfies),
        Lexeme::Word if tokens.text(context) == "extends" => in_gap(Gap::Extends),
        _ => false,
    };
    let possibly_type_argument = matches!(context_lexeme, Lexeme::Punct(b'<' | b','))
        && tokens.possibly_type_arguments(context);
    !(!value_location && (part_of_type || type_context || possibly_type_argument))
}
