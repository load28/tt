//! TypeScript's completion rules where lowering moved the code.
//!
//! TypeScript decides which declaration a completion position is written in
//! from the syntax around it (`internal/ls/completions.go` in
//! typescript-go): `getClosestSymbolDeclaration` walks up from the position
//! to the variable declaration whose initializer holds it, stopping at a
//! function body, an arrow body, or a binding pattern, and
//! `shouldIncludeSymbol` leaves that variable out (`const a = /* no 'a'
//! here */`).
//!
//! In the served text a construct's code no longer stands where it is
//! written: a `match` initializing `const value` runs in a block before the
//! declaration. As far as the walk stays in text the user wrote, the served
//! syntax is the source's and TypeScript's answer stands. Where it reaches
//! text the compiler wrote, the construct's place in the source decides
//! instead ([`crate::program_syntax::CompletionScope`]).

use swc_common::Spanned;
use swc_ecma_ast::{ArrowExpr, CatchClause, Constructor, Function, VarDeclarator};
use swc_ecma_visit::{Visit, VisitWith};

use super::*;

/// The declarations and function bodies TypeScript's walk stops at that
/// hold a served offset, outermost first.
struct Ancestors {
    at: usize,
    origin: crate::host_input::HostOrigin,
    found: Vec<(usize, usize)>,
}

impl Ancestors {
    fn stop(&mut self, span: swc_common::Span) {
        let (start, end) = (self.origin.byte(span.lo), self.origin.byte(span.hi));
        if start <= self.at && self.at <= end {
            self.found.push((start, end));
        }
    }
}

impl Visit for Ancestors {
    fn visit_var_declarator(&mut self, node: &VarDeclarator) {
        self.stop(node.span);
        node.visit_children_with(self);
    }

    fn visit_function(&mut self, node: &Function) {
        self.stop(node.span);
        node.visit_children_with(self);
    }

    fn visit_arrow_expr(&mut self, node: &ArrowExpr) {
        self.stop(node.span);
        node.visit_children_with(self);
    }

    fn visit_constructor(&mut self, node: &Constructor) {
        self.stop(node.span);
        node.visit_children_with(self);
    }

    fn visit_catch_clause(&mut self, node: &CatchClause) {
        if let Some(param) = &node.param {
            self.stop(param.span());
        }
        node.visit_children_with(self);
    }
}

/// Whether TypeScript's walk from the served offset `at` (a byte of
/// `text.code`) stops in text the user wrote, so that its answer is the
/// source's. `None` when the served text does not parse.
fn decided_in_source(text: ServedText<'_>, kind: crate::SourceKind, at: usize) -> Option<bool> {
    let input = crate::host_input::HostInput::new(text.code);
    let module = input.parser(kind).parse_program().ok()?;
    let mut ancestors = Ancestors {
        at,
        origin: input.origin(),
        found: Vec::new(),
    };
    module.visit_with(&mut ancestors);
    Some(
        ancestors.found.last().is_some_and(|&(start, end)| {
            mapper::to_source_span(text.mappings, start, end).is_some()
        }),
    )
}

/// The declaration TypeScript's walk finds for source byte `at` when it
/// leaves the user's text inside a construct: the one whose initializer
/// holds the construct, unless `at` is in the construct's own region
/// function, which the walk stops at.
fn construct_declaration(doc: &ServiceDoc, at: usize) -> Option<&str> {
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
        return None;
    }
    scope.declaration.as_deref()
}

/// Leaves the variable being declared out of TypeScript's completion
/// answer at `served` (a UTF-16 offset of `text.code`) for source byte
/// `at`, when the construct around `at` is written in that variable's
/// initializer and lowering moved it out.
pub(super) fn restate_completions(
    answer: &mut CompletionAnswer,
    doc: &ServiceDoc,
    text: ServedText<'_>,
    kind: crate::SourceKind,
    served: usize,
    at: usize,
) {
    let Some(name) = construct_declaration(doc, at) else {
        return;
    };
    if decided_in_source(text, kind, mapper::from_utf16(text.code, served)) != Some(false) {
        return;
    }
    answer
        .items
        .retain(|item| item.label != name || item.source.is_some());
}
