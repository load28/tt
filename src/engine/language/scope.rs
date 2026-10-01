//! TypeScript's completion rules where lowering moved the code.
//!
//! TypeScript derives two facts about a completion position from the syntax
//! around it (`internal/ls/completions.go` in typescript-go), each by
//! walking up from the position: `getClosestSymbolDeclaration` names the
//! variable whose initializer holds it, stopping at a function body, an
//! arrow body, or a binding pattern, and `shouldIncludeSymbol` leaves that
//! variable out (`const a = /* no 'a' here */`);
//! `tryGetFunctionLikeBodyCompletionContainer` finds the function-like body
//! around it, stopping at a class, and `getGlobalCompletions` offers a
//! function body's keywords (`KeywordCompletionFiltersFunctionLikeBodyKeywords`)
//! there instead of every keyword (`KeywordCompletionFiltersAll`).
//!
//! In the served text a construct's code no longer stands where it is
//! written: a `match` initializing `const value` runs in a block before the
//! declaration. As far as a walk stays in text the user wrote, the served
//! syntax is the source's and TypeScript's answer stands. Where it reaches
//! text the compiler wrote, the construct's place in the source decides
//! instead ([`crate::program_syntax::CompletionScope`]).

use swc_common::Spanned;
use swc_ecma_ast::{ArrowExpr, CatchClause, Class, Constructor, Function, VarDeclarator};
use swc_ecma_visit::{Visit, VisitWith};

use super::*;

/// The keywords `KeywordCompletionFiltersAll` offers and
/// `KeywordCompletionFiltersFunctionLikeBodyKeywords` does not
/// (`getTypescriptKeywordCompletions`): `declare`, `module`, `namespace`,
/// `abstract`, and the type keywords (`typeKeywords` in
/// `internal/ls/utilities.go`) that are contextual keywords, which
/// `isFunctionLikeBodyKeyword` admits only as `async`, `await`, `using`,
/// `as`, `satisfies`, or `type`.
const MODULE_LEVEL_KEYWORDS: [&str; 18] = [
    "abstract",
    "any",
    "asserts",
    "bigint",
    "boolean",
    "declare",
    "infer",
    "keyof",
    "module",
    "namespace",
    "never",
    "number",
    "object",
    "readonly",
    "string",
    "symbol",
    "unique",
    "unknown",
];

/// A keyword `KeywordCompletionFiltersAll` offers and no other filter
/// does: an answer holding it is TypeScript's global completion outside
/// every function-like body, and one without it was filtered otherwise (a
/// type position's `KeywordCompletionFiltersTypeKeywords`, a class body's
/// keywords, or no keywords at all), which the construct's place does not
/// change.
const ALL_FILTER_KEYWORD: &str = "namespace";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    /// A variable or parameter declaration the position is written in.
    Declaration,
    /// A function-like declaration whose body holds the position.
    Body,
    /// A class, where the function-like body walk stops.
    Class,
}

/// The nodes TypeScript's two walks stop at that hold a served offset,
/// outermost first.
struct Ancestors {
    at: usize,
    origin: crate::host_input::HostOrigin,
    found: Vec<(Stop, usize, usize)>,
}

impl Ancestors {
    fn holds(&self, span: swc_common::Span) -> bool {
        let (start, end) = (self.origin.byte(span.lo), self.origin.byte(span.hi));
        start <= self.at && self.at <= end
    }

    fn stop(&mut self, stop: Stop, span: swc_common::Span) {
        if self.holds(span) {
            let (start, end) = (self.origin.byte(span.lo), self.origin.byte(span.hi));
            self.found.push((stop, start, end));
        }
    }
}

impl Visit for Ancestors {
    fn visit_var_declarator(&mut self, node: &VarDeclarator) {
        self.stop(Stop::Declaration, node.span);
        node.visit_children_with(self);
    }

    fn visit_function(&mut self, node: &Function) {
        for param in &node.params {
            self.stop(Stop::Declaration, param.span);
        }
        if node.body.as_ref().is_some_and(|body| self.holds(body.span)) {
            self.stop(Stop::Body, node.span);
        }
        node.visit_children_with(self);
    }

    fn visit_arrow_expr(&mut self, node: &ArrowExpr) {
        for param in &node.params {
            self.stop(Stop::Declaration, param.span());
        }
        if self.holds(node.body.span()) {
            self.stop(Stop::Body, node.span);
        }
        node.visit_children_with(self);
    }

    fn visit_constructor(&mut self, node: &Constructor) {
        for param in &node.params {
            self.stop(Stop::Declaration, param.span());
        }
        if node.body.as_ref().is_some_and(|body| self.holds(body.span)) {
            self.stop(Stop::Body, node.span);
        }
        node.visit_children_with(self);
    }

    fn visit_class(&mut self, node: &Class) {
        self.stop(Stop::Class, node.span);
        node.visit_children_with(self);
    }

    fn visit_catch_clause(&mut self, node: &CatchClause) {
        if let Some(param) = &node.param {
            self.stop(Stop::Declaration, param.span());
        }
        node.visit_children_with(self);
    }
}

/// Which of TypeScript's two walks from the served offset `at` (a byte of
/// `text.code`) stop in text the user wrote, so that its answer is the
/// source's: `(declaration, container)`. `None` when the served text does
/// not parse.
fn decided_in_source(
    text: ServedText<'_>,
    kind: crate::SourceKind,
    at: usize,
) -> Option<(bool, bool)> {
    let input = crate::host_input::HostInput::new(text.code);
    let module = input.parser(kind).parse_program().ok()?;
    let mut ancestors = Ancestors {
        at,
        origin: input.origin(),
        found: Vec::new(),
    };
    module.visit_with(&mut ancestors);
    let (mut declaration, mut container) = (false, false);
    for &(stop, start, end) in ancestors.found.iter().rev() {
        if mapper::to_source_span(text.mappings, start, end).is_none() {
            break;
        }
        match stop {
            Stop::Declaration => declaration = true,
            Stop::Body => {
                declaration = true;
                container = true;
            }
            Stop::Class => container = true,
        }
        if declaration && container {
            break;
        }
    }
    Some((declaration, container))
}

/// What the construct around source byte `at` says TypeScript's walks
/// find when they leave the user's text: the declaration to leave out,
/// and whether the position is in a function-like body. `None` when `at`
/// is in no construct.
fn construct_scope(doc: &ServiceDoc, at: usize) -> Option<(Option<&str>, bool)> {
    let scope = doc
        .completion_scopes
        .iter()
        .filter(|scope| scope.source.start < at && at < scope.source.end)
        .min_by_key(|scope| scope.source.end - scope.source.start)?;
    let hosted = scope
        .hosts
        .iter()
        .any(|host| host.start <= at && at <= host.end);
    if scope.isolated && !hosted {
        return Some((None, true));
    }
    Some((scope.declaration.as_deref(), scope.function_body))
}

/// Restates TypeScript's completion answer at `served` (a UTF-16 offset of
/// `text.code`) for source byte `at`, where the walks TypeScript makes
/// leave the user's text inside a construct: the variable being declared
/// is left out, and in a function-like body a function body's keywords are
/// offered.
pub(super) fn restate_completions(
    answer: &mut CompletionAnswer,
    doc: &ServiceDoc,
    text: ServedText<'_>,
    kind: crate::SourceKind,
    served: usize,
    at: usize,
) {
    let Some((declaration, function_body)) = construct_scope(doc, at) else {
        return;
    };
    let Some((declared, contained)) =
        decided_in_source(text, kind, mapper::from_utf16(text.code, served))
    else {
        return;
    };
    if !declared && let Some(name) = declaration {
        answer
            .items
            .retain(|item| item.label != name || item.source.is_some());
    }
    let keyword = |item: &CompletionItem, label: &str| {
        item.kind == Some(CompletionItemKind::Keyword) && item.label == label
    };
    if !contained
        && function_body
        && answer
            .items
            .iter()
            .any(|item| keyword(item, ALL_FILTER_KEYWORD))
    {
        answer.items.retain(|item| {
            !MODULE_LEVEL_KEYWORDS
                .iter()
                .any(|label| keyword(item, label))
        });
    }
}
