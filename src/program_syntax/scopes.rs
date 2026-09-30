use std::collections::HashSet;

use swc_common::{BytePos, Spanned};
use swc_ecma_ast::{
    ArrowExpr, ArrowFunctionBody, AssignTargetPat, BlockStmt, BreakStmt, CatchClause, ClassDecl,
    ClassExpr, ContinueStmt, Decl, Expr, FnDecl, FnExpr, ForInStmt, ForOfStmt, ForStmt, Function,
    Ident, LabeledStmt, ObjectPatProp, Pat, SimpleAssignTarget, Stmt, SwitchStmt, VarDecl,
    VarDeclKind, VarDeclOrExpr,
};
use swc_ecma_visit::{Visit, VisitWith};

use super::projection::TtBindings;

pub(super) fn pattern_names(pattern: &Pat, names: &mut Vec<String>) {
    match pattern {
        Pat::Ident(binding) => names.push(binding.id.sym.to_string()),
        Pat::Array(array) => {
            for element in array.elems.iter().flatten() {
                pattern_names(element, names);
            }
        }
        Pat::Rest(rest) => pattern_names(&rest.arg, names),
        Pat::Object(object) => {
            for property in &object.props {
                match property {
                    ObjectPatProp::KeyValue(pair) => pattern_names(&pair.value, names),
                    ObjectPatProp::Assign(assign) => names.push(assign.key.id.sym.to_string()),
                    ObjectPatProp::Rest(rest) => pattern_names(&rest.arg, names),
                }
            }
        }
        Pat::Assign(assign) => pattern_names(&assign.left, names),
        Pat::Invalid(_) | Pat::Expr(_) => {}
    }
}

fn declaration_names(declaration: &VarDecl, names: &mut Vec<String>) {
    for declarator in &declaration.decls {
        pattern_names(&declarator.name, names);
    }
}

pub(super) fn reads_outer(
    expression: &Expr,
    names: &HashSet<String>,
    tt: &TtBindings,
    at: &dyn Fn(BytePos) -> usize,
) -> bool {
    let mut walk = Walk {
        names,
        tt,
        at,
        scopes: Vec::new(),
        assigning: false,
        found: false,
    };
    expression.visit_with(&mut walk);
    walk.found
}

struct Walk<'a> {
    names: &'a HashSet<String>,
    tt: &'a TtBindings,
    at: &'a dyn Fn(BytePos) -> usize,
    scopes: Vec<HashSet<String>>,
    assigning: bool,
    found: bool,
}

impl Walk<'_> {
    fn reference(&mut self, ident: &Ident) {
        let name = ident.sym.as_ref();
        if !self.names.contains(name) || self.scopes.iter().any(|scope| scope.contains(name)) {
            return;
        }
        let position = (self.at)(ident.span.lo);
        let bound_by_tt = self.tt.scopes.iter().any(|(span, bound)| {
            span.start.0 <= position
                && position < span.end.0
                && bound.iter().any(|bound| bound == name)
        });
        if !bound_by_tt {
            self.found = true;
        }
    }

    fn scoped(&mut self, names: Vec<String>, visit: impl FnOnce(&mut Self)) {
        self.scopes.push(names.into_iter().collect());
        visit(self);
        self.scopes.pop();
    }

    fn block_names(&self, statements: &[Stmt]) -> Vec<String> {
        let mut names = Vec::new();
        for statement in statements {
            match statement {
                Stmt::Decl(Decl::Var(declaration)) if declaration.kind != VarDeclKind::Var => {
                    declaration_names(declaration, &mut names);
                }
                Stmt::Decl(Decl::Using(declaration)) => {
                    for declarator in &declaration.decls {
                        pattern_names(&declarator.name, &mut names);
                    }
                }
                Stmt::Decl(Decl::Fn(function)) => names.push(function.ident.sym.to_string()),
                Stmt::Decl(Decl::Class(class)) => names.push(class.ident.sym.to_string()),
                Stmt::Decl(Decl::TsEnum(declaration)) => {
                    names.push(declaration.id.sym.to_string());
                }
                _ => {}
            }
            let start = (self.at)(statement.span().lo);
            for (span, declared) in &self.tt.statements {
                if span.start.0 == start {
                    names.extend(declared.iter().cloned());
                }
            }
        }
        names
    }
}

fn hoisted_names(statements: &[Stmt], names: &mut Vec<String>) {
    struct Hoisted<'n>(&'n mut Vec<String>);
    impl Visit for Hoisted<'_> {
        fn visit_var_decl(&mut self, node: &VarDecl) {
            if node.kind == VarDeclKind::Var {
                declaration_names(node, self.0);
            }
            node.visit_children_with(self);
        }
        fn visit_function(&mut self, _: &Function) {}
        fn visit_arrow_expr(&mut self, _: &ArrowExpr) {}
        fn visit_class(&mut self, _: &swc_ecma_ast::Class) {}
    }
    for statement in statements {
        statement.visit_with(&mut Hoisted(names));
    }
}

impl Visit for Walk<'_> {
    fn visit_ident(&mut self, node: &Ident) {
        self.reference(node);
    }

    fn visit_binding_ident(&mut self, node: &swc_ecma_ast::BindingIdent) {
        if self.assigning {
            self.reference(&node.id);
        }
        node.type_ann.visit_with(self);
    }

    fn visit_simple_assign_target(&mut self, node: &SimpleAssignTarget) {
        match node {
            SimpleAssignTarget::Ident(binding) => self.reference(&binding.id),
            _ => node.visit_children_with(self),
        }
    }

    fn visit_assign_target_pat(&mut self, node: &AssignTargetPat) {
        let assigning = std::mem::replace(&mut self.assigning, true);
        node.visit_children_with(self);
        self.assigning = assigning;
    }

    fn visit_labeled_stmt(&mut self, node: &LabeledStmt) {
        node.body.visit_with(self);
    }

    fn visit_break_stmt(&mut self, _: &BreakStmt) {}

    fn visit_continue_stmt(&mut self, _: &ContinueStmt) {}

    fn visit_fn_decl(&mut self, node: &FnDecl) {
        node.function.visit_with(self);
    }

    fn visit_class_decl(&mut self, node: &ClassDecl) {
        self.scoped(vec![node.ident.sym.to_string()], |walk| {
            node.class.visit_with(walk)
        });
    }

    fn visit_fn_expr(&mut self, node: &FnExpr) {
        let names = node
            .ident
            .iter()
            .map(|ident| ident.sym.to_string())
            .collect();
        self.scoped(names, |walk| node.function.visit_with(walk));
    }

    fn visit_class_expr(&mut self, node: &ClassExpr) {
        let names = node
            .ident
            .iter()
            .map(|ident| ident.sym.to_string())
            .collect();
        self.scoped(names, |walk| node.class.visit_with(walk));
    }

    fn visit_function(&mut self, node: &Function) {
        let mut names = Vec::new();
        for param in &node.params {
            pattern_names(&param.pat, &mut names);
        }
        if let Some(body) = &node.body {
            hoisted_names(&body.stmts, &mut names);
        }
        let assigning = std::mem::replace(&mut self.assigning, false);
        self.scoped(names, |walk| node.visit_children_with(walk));
        self.assigning = assigning;
    }

    fn visit_arrow_expr(&mut self, node: &ArrowExpr) {
        let mut names = Vec::new();
        for param in &node.params {
            pattern_names(param, &mut names);
        }
        if let ArrowFunctionBody::FunctionBody(body) = &*node.body {
            hoisted_names(&body.stmts, &mut names);
        }
        let assigning = std::mem::replace(&mut self.assigning, false);
        self.scoped(names, |walk| node.visit_children_with(walk));
        self.assigning = assigning;
    }

    fn visit_function_body(&mut self, node: &swc_ecma_ast::FunctionBody) {
        let names = self.block_names(&node.stmts);
        self.scoped(names, |walk| node.visit_children_with(walk));
    }

    fn visit_block_stmt(&mut self, node: &BlockStmt) {
        let names = self.block_names(&node.stmts);
        self.scoped(names, |walk| node.visit_children_with(walk));
    }

    fn visit_switch_stmt(&mut self, node: &SwitchStmt) {
        let statements: Vec<Stmt> = node
            .cases
            .iter()
            .flat_map(|case| case.cons.iter().cloned())
            .collect();
        let names = self.block_names(&statements);
        self.scoped(names, |walk| node.visit_children_with(walk));
    }

    fn visit_catch_clause(&mut self, node: &CatchClause) {
        let mut names = Vec::new();
        if let Some(param) = &node.param {
            pattern_names(param, &mut names);
        }
        self.scoped(names, |walk| node.visit_children_with(walk));
    }

    fn visit_for_stmt(&mut self, node: &ForStmt) {
        let mut names = Vec::new();
        if let Some(VarDeclOrExpr::VarDecl(declaration)) = &node.init
            && declaration.kind != VarDeclKind::Var
        {
            declaration_names(declaration, &mut names);
        }
        self.scoped(names, |walk| node.visit_children_with(walk));
    }

    fn visit_for_in_stmt(&mut self, node: &ForInStmt) {
        let mut names = Vec::new();
        head_names(&node.left, &mut names);
        self.scoped(names, |walk| node.visit_children_with(walk));
    }

    fn visit_for_of_stmt(&mut self, node: &ForOfStmt) {
        let mut names = Vec::new();
        head_names(&node.left, &mut names);
        self.scoped(names, |walk| node.visit_children_with(walk));
    }
}

fn head_names(head: &swc_ecma_ast::ForHead, names: &mut Vec<String>) {
    match head {
        swc_ecma_ast::ForHead::VarDecl(declaration) if declaration.kind != VarDeclKind::Var => {
            declaration_names(declaration, names);
        }
        swc_ecma_ast::ForHead::UsingDecl(declaration) => {
            for declarator in &declaration.decls {
                pattern_names(&declarator.name, names);
            }
        }
        _ => {}
    }
}
