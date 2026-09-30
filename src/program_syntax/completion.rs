//! Where TypeScript's completion rules place each tt construct.
//!
//! TypeScript decides which declaration a completion position is written
//! in by walking up from it (`internal/ls/completions.go` in
//! typescript-go): `getClosestSymbolDeclaration` finds the variable
//! declaration whose initializer holds it, stopping at a function body, an
//! arrow body, or a binding pattern, and `shouldIncludeSymbol` leaves that
//! declaration out (`const a = /* no 'a' here */`). Lowering
//! moves a construct's code away from the place it is written (a `match`
//! runs before the declaration it initializes), so the served TypeScript
//! no longer holds those facts. The projection does: each construct stands
//! where it is written, a value `match` and a `result` block as the region
//! function their statements belong to, with the scrutinee evaluated where
//! the construct stands.

use swc_ecma_ast::{
    ArrowExpr, ArrowFunctionBody, CatchClause, Constructor, ExprStmt, FnExpr, Function, Pat,
    VarDeclarator,
};
use swc_ecma_visit::{Visit, VisitWith};

use super::*;

/// What TypeScript's completion rules say at one tt construct's place, and
/// which of its parts are evaluated there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletionScope {
    /// The construct's source span.
    pub(crate) source: SourceSpan,
    /// Whether the construct's statements and arms form a region function:
    /// then only `hosts` are evaluated at the construct's place, and every
    /// other position in it is inside a function body with no declaration
    /// of its own.
    pub(crate) isolated: bool,
    /// The parts of an isolated construct evaluated at its place (a
    /// `match`'s scrutinees).
    pub(crate) hosts: Vec<SourceSpan>,
    /// The name of the variable declaration whose initializer holds the
    /// construct, when TypeScript leaves it out of completions there.
    pub(crate) declaration: Option<String>,
}

/// What the builder records for [`completion_scopes`].
#[derive(Debug, Default)]
pub(super) struct CompletionMarks {
    /// Where each region function starts, with the scrutinee statements
    /// its body opens with.
    pub(super) regions: HashMap<ProjectedByte, Vec<ProjectedSpan>>,
    /// Each declaration-form propagation statement, with the name its
    /// binding declares when that binding is an identifier.
    pub(super) bindings: HashMap<ProjectedSpan, Option<String>>,
}

#[derive(Debug, Clone)]
struct Scope {
    declaration: Option<String>,
}

struct Walk<'a> {
    start: HostOrigin,
    marks: &'a CompletionMarks,
    scopes: Vec<(ProjectedSpan, Scope)>,
}

impl Walk<'_> {
    fn span(&self, span: swc_common::Span) -> ProjectedSpan {
        projected_span(span, self.start)
    }

    fn with(&mut self, span: swc_common::Span, scope: Scope, visit: impl FnOnce(&mut Self)) {
        let span = self.span(span);
        self.scopes.push((span, scope));
        visit(self);
    }

    fn declared() -> Scope {
        Scope { declaration: None }
    }

    fn body() -> Scope {
        Scope { declaration: None }
    }

    fn params(&mut self, params: &[Pat]) {
        for param in params {
            let scope = Self::declared();
            self.with(param.span(), scope, |walk| param.visit_with(walk));
        }
    }

    fn is_region(&self, span: swc_common::Span) -> bool {
        self.marks.regions.contains_key(&self.span(span).start)
    }

    fn region_body(&mut self, function: swc_common::Span, statements: &[swc_ecma_ast::Stmt]) {
        let hosts = self
            .marks
            .regions
            .get(&self.span(function).start)
            .cloned()
            .unwrap_or_default();
        for statement in statements {
            if hosts.contains(&self.span(statement.span())) {
                statement.visit_with(self);
            } else {
                self.with(statement.span(), Self::body(), |walk| {
                    statement.visit_with(walk)
                });
            }
        }
    }
}

impl Visit for Walk<'_> {
    fn visit_var_declarator(&mut self, node: &VarDeclarator) {
        let scope = Self::declared();
        self.with(node.name.span(), scope, |walk| node.name.visit_with(walk));
        if let Some(init) = &node.init {
            let declaration = match &node.name {
                Pat::Ident(binding) => Some(binding.id.sym.to_string()),
                _ => None,
            };
            self.with(init.span(), Scope { declaration }, |walk| {
                init.visit_with(walk)
            });
        }
    }

    fn visit_function(&mut self, node: &Function) {
        for param in &node.params {
            let scope = Self::declared();
            self.with(param.span, scope, |walk| param.visit_with(walk));
        }
        if let Some(body) = &node.body {
            self.with(body.span, Self::body(), |walk| body.visit_with(walk));
        }
    }

    fn visit_fn_expr(&mut self, node: &FnExpr) {
        if !self.is_region(node.function.span) {
            node.visit_children_with(self);
            return;
        }
        if let Some(body) = &node.function.body {
            self.region_body(node.function.span, &body.stmts);
        }
    }

    fn visit_arrow_expr(&mut self, node: &ArrowExpr) {
        if self.is_region(node.span)
            && let ArrowFunctionBody::FunctionBody(body) = &*node.body
        {
            self.region_body(node.span, &body.stmts);
            return;
        }
        self.params(&node.params);
        self.with(node.body.span(), Self::body(), |walk| {
            node.body.visit_with(walk)
        });
    }

    fn visit_constructor(&mut self, node: &Constructor) {
        let scope = Self::declared();
        for param in &node.params {
            self.with(param.span(), scope.clone(), |walk| param.visit_with(walk));
        }
        if let Some(body) = &node.body {
            self.with(body.span, Self::body(), |walk| body.visit_with(walk));
        }
    }

    fn visit_catch_clause(&mut self, node: &CatchClause) {
        if let Some(param) = &node.param {
            self.params(std::slice::from_ref(param));
        }
        node.body.visit_with(self);
    }

    fn visit_expr_stmt(&mut self, node: &ExprStmt) {
        match self.marks.bindings.get(&self.span(node.span)) {
            Some(declaration) => {
                let scope = Scope {
                    declaration: declaration.clone(),
                };
                self.with(node.span, scope, |walk| node.visit_children_with(walk));
            }
            None => node.visit_children_with(self),
        }
    }
}

/// The source span a projected span's copied and placeholder text came from.
fn source_of(segments: &[ProjectionSourceSegment], projected: ProjectedSpan) -> Option<SourceSpan> {
    let inside = segments.iter().filter(|segment| {
        segment.kind != ProjectionSegmentKind::SourceBoundary
            && segment.kind != ProjectionSegmentKind::AutomaticSemicolon
            && projected.start <= segment.projected.start
            && segment.projected.end <= projected.end
    });
    inside.fold(None, |span: Option<SourceSpan>, segment| {
        Some(match span {
            None => segment.source,
            Some(span) => SourceSpan {
                start: span.start.min(segment.source.start),
                end: span.end.max(segment.source.end),
            },
        })
    })
}

/// Each construct's [`CompletionScope`], from the parsed projection.
pub(super) fn completion_scopes(
    module: &Module,
    start: HostOrigin,
    pending: &[PendingOverlay],
    segments: &[ProjectionSourceSegment],
    marks: &CompletionMarks,
) -> Vec<CompletionScope> {
    let mut walk = Walk {
        start,
        marks,
        scopes: Vec::new(),
    };
    module.visit_with(&mut walk);
    pending
        .iter()
        .filter(|entry| entry.category != SyntaxCategory::Item)
        .map(|entry| {
            let at = entry.projected.start;
            let scope = walk
                .scopes
                .iter()
                .rev()
                .find(|(span, _)| span.start <= at && at < span.end)
                .map(|(_, scope)| scope.clone())
                .unwrap_or(Scope { declaration: None });
            let region = marks
                .regions
                .get(&ProjectedByte(entry.projected.start.0 + 1))
                .filter(|_| {
                    entry.marker == OverlayMarker::DecisionCallExpression
                        || entry.synthetic_return.is_some()
                });
            CompletionScope {
                source: entry.source,
                isolated: region.is_some(),
                hosts: region
                    .into_iter()
                    .flatten()
                    .filter_map(|host| source_of(segments, *host))
                    .collect(),
                declaration: scope.declaration,
            }
        })
        .collect()
}
