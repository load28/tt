//! TypeScript-owned identifier positions used by the lossless tt parser.
//!
//! Some TypeScript declarations have the same local token shape as tt syntax.
//! SWC is the compiler's TypeScript syntax substrate, so it identifies those
//! declaration names from the host AST instead of duplicating TypeScript's
//! surrounding grammar in token heuristics.

use std::collections::HashSet;

use swc_common::input::StringInput;
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap, Span};
use swc_ecma_ast::{
    ClassMethod, FnDecl, FnExpr, GetterProp, MethodProp, PrivateMethod, PropName, SetterProp,
};
use swc_ecma_parser::lexer::Lexer;
use swc_ecma_parser::{Parser, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};

pub(super) fn owned_match_names(src: &str, source_kind: crate::SourceKind) -> HashSet<usize> {
    let source_map: Lrc<SourceMap> = Default::default();
    let file = source_map.new_source_file(Lrc::new(FileName::Anon), src.to_owned());
    let source_start = file.start_pos.0;
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: source_kind.is_tsx(),
            decorators: true,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*file),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let Ok(module) = parser.parse_module() else {
        return HashSet::new();
    };
    if !parser.take_errors().is_empty() {
        return HashSet::new();
    }

    let mut collector = MatchNameCollector {
        source_start,
        offsets: HashSet::new(),
    };
    module.visit_with(&mut collector);
    collector.offsets
}

struct MatchNameCollector {
    source_start: u32,
    offsets: HashSet<usize>,
}

impl MatchNameCollector {
    fn insert(&mut self, name: &str, span: Span) {
        if name == "match"
            && let Ok(offset) = usize::try_from(span.lo.0.saturating_sub(self.source_start))
        {
            self.offsets.insert(offset);
        }
    }

    fn insert_prop_name(&mut self, name: &PropName) {
        if let PropName::Ident(name) = name {
            self.insert(name.sym.as_ref(), name.span);
        }
    }
}

impl Visit for MatchNameCollector {
    fn visit_fn_decl(&mut self, node: &FnDecl) {
        self.insert(node.ident.sym.as_ref(), node.ident.span);
        node.visit_children_with(self);
    }

    fn visit_fn_expr(&mut self, node: &FnExpr) {
        if let Some(name) = &node.ident {
            self.insert(name.sym.as_ref(), name.span);
        }
        node.visit_children_with(self);
    }

    fn visit_class_method(&mut self, node: &ClassMethod) {
        self.insert_prop_name(&node.key);
        node.visit_children_with(self);
    }

    fn visit_private_method(&mut self, node: &PrivateMethod) {
        self.insert(node.key.name.as_ref(), node.key.span);
        node.visit_children_with(self);
    }

    fn visit_method_prop(&mut self, node: &MethodProp) {
        self.insert_prop_name(&node.key);
        node.visit_children_with(self);
    }

    fn visit_getter_prop(&mut self, node: &GetterProp) {
        self.insert_prop_name(&node.key);
        node.visit_children_with(self);
    }

    fn visit_setter_prop(&mut self, node: &SetterProp) {
        self.insert_prop_name(&node.key);
        node.visit_children_with(self);
    }
}
