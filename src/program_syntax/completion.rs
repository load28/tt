//! Where TypeScript's completion rules place each tt construct.
//!
//! TypeScript decides which declaration a completion position is written
//! in by walking up from it (`internal/ls/completions.go` in
//! typescript-go): `getClosestSymbolDeclaration` finds the variable
//! declaration whose initializer holds it, stopping at a function body, an
//! arrow body, or a binding pattern, and `shouldIncludeSymbol` leaves that
//! declaration out (`const a = /* no 'a' here */`);
//! `tryGetFunctionLikeBodyCompletionContainer` finds the function-like body
//! around it, stopping at a class, and `getGlobalCompletions` offers a
//! function body's keywords there instead of every keyword. Lowering
//! moves a construct's code away from the place it is written (a `match`
//! runs before the declaration it initializes), so the served TypeScript
//! no longer holds those facts. The projection does: each construct stands
//! where it is written, a value `match` and a `result` block as the region
//! function their statements belong to, with the scrutinee evaluated where
//! the construct stands.

use swc_ecma_ast::{
    ArrowExpr, ArrowFunctionBody, CatchClause, Class, Constructor, Decl, ExportDecl, ExprStmt,
    FnExpr, Function, Pat, VarDeclarator,
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
    /// Whether the construct is inside a function-like body, with no class
    /// in between.
    pub(crate) function_body: bool,
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
    function_body: bool,
}

struct Walk<'a> {
    start: HostOrigin,
    marks: &'a CompletionMarks,
    stack: Vec<Scope>,
    scopes: Vec<(ProjectedSpan, Scope)>,
}

impl Walk<'_> {
    fn span(&self, span: swc_common::Span) -> ProjectedSpan {
        projected_span(span, self.start)
    }

    fn with(&mut self, span: swc_common::Span, scope: Scope, visit: impl FnOnce(&mut Self)) {
        let span = self.span(span);
        self.scopes.push((span, scope.clone()));
        self.stack.push(scope);
        visit(self);
        self.stack.pop();
    }

    fn current(&self) -> Scope {
        self.stack.last().cloned().unwrap_or(Scope {
            declaration: None,
            function_body: false,
        })
    }

    fn declared(&self) -> Scope {
        Scope {
            declaration: None,
            ..self.current()
        }
    }

    fn body() -> Scope {
        Scope {
            declaration: None,
            function_body: true,
        }
    }

    fn params(&mut self, params: &[Pat]) {
        for param in params {
            let scope = self.declared();
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

impl Walk<'_> {
    /// A declarator's initializer names the declaration unless the
    /// declaration is exported: the binder gives an exported member a local
    /// symbol flagged `ExportValue` alone (`declareModuleMember` in
    /// typescript-go `internal/binder/binder.go`), which has no value
    /// declaration, and that local symbol is the one in scope, so
    /// `shouldIncludeSymbol` keeps it.
    fn declarator(&mut self, node: &VarDeclarator, exported: bool) {
        let scope = self.declared();
        self.with(node.name.span(), scope, |walk| node.name.visit_with(walk));
        if let Some(init) = &node.init {
            let declaration = match &node.name {
                Pat::Ident(binding) if !exported => Some(binding.id.sym.to_string()),
                _ => None,
            };
            let scope = Scope {
                declaration,
                ..self.current()
            };
            self.with(init.span(), scope, |walk| init.visit_with(walk));
        }
    }
}

impl Visit for Walk<'_> {
    fn visit_export_decl(&mut self, node: &ExportDecl) {
        match &node.decl {
            Decl::Var(declaration) => {
                for declarator in &declaration.decls {
                    self.declarator(declarator, true);
                }
            }
            _ => node.visit_children_with(self),
        }
    }

    fn visit_var_declarator(&mut self, node: &VarDeclarator) {
        self.declarator(node, false);
    }

    fn visit_function(&mut self, node: &Function) {
        for param in &node.params {
            let scope = self.declared();
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
        let scope = self.declared();
        for param in &node.params {
            self.with(param.span(), scope.clone(), |walk| param.visit_with(walk));
        }
        if let Some(body) = &node.body {
            self.with(body.span, Self::body(), |walk| body.visit_with(walk));
        }
    }

    fn visit_class(&mut self, node: &Class) {
        let scope = Scope {
            function_body: false,
            ..self.current()
        };
        self.with(node.span, scope, |walk| node.visit_children_with(walk));
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
                    ..self.current()
                };
                self.with(node.span, scope, |walk| node.visit_children_with(walk));
            }
            None => node.visit_children_with(self),
        }
    }
}

/// The source span a projected span's copied and placeholder text came from.
fn source_of(segments: &ProjectionSegments, projected: ProjectedSpan) -> Option<SourceSpan> {
    let inside = segments
        .starting_in(projected.start, ProjectedByte(projected.end.0 + 1))
        .into_iter()
        .map(|index| &segments[index])
        .inspect(|_| crate::work::tick("completion host segments"))
        .filter(|segment| {
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
    segments: &ProjectionSegments,
    marks: &CompletionMarks,
) -> Vec<CompletionScope> {
    let mut walk = Walk {
        start,
        marks,
        stack: Vec::new(),
        scopes: Vec::new(),
    };
    module.visit_with(&mut walk);
    let entries: Vec<&PendingOverlay> = pending
        .iter()
        .filter(|entry| entry.category != SyntaxCategory::Item)
        .collect();
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by_key(|&index| entries[index].projected.start);
    let mut innermost: Vec<Option<usize>> = vec![None; entries.len()];
    let mut open: Vec<usize> = Vec::new();
    let mut next = 0;
    for index in order {
        let at = entries[index].projected.start;
        while next < walk.scopes.len() && walk.scopes[next].0.start <= at {
            open.push(next);
            next += 1;
        }
        innermost[index] = open
            .iter()
            .rev()
            .copied()
            .inspect(|_| crate::work::tick("completion scope candidates"))
            .find(|&scope| at < walk.scopes[scope].0.end);
        open.retain(|&scope| at < walk.scopes[scope].0.end);
    }
    entries
        .into_iter()
        .zip(innermost)
        .map(|(entry, innermost)| {
            let scope = innermost
                .map(|index| walk.scopes[index].1.clone())
                .unwrap_or(Scope {
                    declaration: None,
                    function_body: false,
                });
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
                function_body: scope.function_body,
            }
        })
        .collect()
}
