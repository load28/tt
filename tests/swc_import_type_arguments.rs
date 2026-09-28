//! Exercise the vendored parser's type arguments of an import type directly
//! (`vendor/swc_ecma_parser/TT-PATCH.md`): as in TypeScript's
//! `parseTypeArgumentsOfTypeReference`, a `<` on the next line does not
//! continue the type.
use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::{Decl, Expr, ModuleItem, Stmt, TsType};
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer};

fn parse(source: &str, tsx: bool) -> Result<swc_ecma_ast::Module, swc_ecma_parser::error::Error> {
    let cm: Lrc<SourceMap> = Default::default();
    let file = cm.new_source_file(Lrc::new(FileName::Anon), source.to_owned());
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*file),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let module = parser.parse_module()?;
    match parser.take_errors().into_iter().next() {
        Some(error) => Err(error),
        None => Ok(module),
    }
}

fn annotation(module: &swc_ecma_ast::Module) -> TsType {
    let Some(ModuleItem::Stmt(Stmt::Decl(Decl::Var(declaration)))) = module.body.first() else {
        panic!("the first item is not a variable declaration: {module:?}");
    };
    *declaration.decls[0]
        .name
        .as_ident()
        .and_then(|ident| ident.type_ann.as_ref())
        .expect("the declaration is annotated")
        .type_ann
        .clone()
}

fn has_type_arguments(ty: &TsType) -> bool {
    match ty {
        TsType::TsImportType(import) => import.type_args.is_some(),
        TsType::TsTypeQuery(query) => {
            query.type_args.is_some()
                || matches!(&query.expr_name, swc_ecma_ast::TsTypeQueryExpr::Import(import) if import.type_args.is_some())
        }
        other => panic!("not an import type: {other:?}"),
    }
}

const TYPES: &[&str] = &[
    "import(\"x\")",
    "import(\"x\").A",
    "typeof import(\"x\")",
    "typeof import(\"x\").A",
];

#[test]
fn a_type_argument_list_on_the_next_line_starts_a_statement() {
    for ty in TYPES {
        let source = format!("let x: {ty}\n<any>y\n");
        let module = parse(&source, false).unwrap_or_else(|error| panic!("{source:?}: {error:?}"));
        assert_eq!(module.body.len(), 2, "{source:?}");
        assert!(!has_type_arguments(&annotation(&module)), "{source:?}");
        let ModuleItem::Stmt(Stmt::Expr(statement)) = &module.body[1] else {
            panic!("{source:?}: the second item is not an expression statement");
        };
        assert!(
            matches!(*statement.expr, Expr::TsTypeAssertion(_)),
            "{source:?}"
        );
        let source = format!("let x: {ty}\n<b>hi</b>\n");
        let module = parse(&source, true).unwrap_or_else(|error| panic!("{source:?}: {error:?}"));
        assert_eq!(module.body.len(), 2, "{source:?}");
        let ModuleItem::Stmt(Stmt::Expr(statement)) = &module.body[1] else {
            panic!("{source:?}: the second item is not an expression statement");
        };
        assert!(matches!(*statement.expr, Expr::JSXElement(_)), "{source:?}");
    }
}

#[test]
fn a_type_argument_list_on_the_same_line_belongs_to_the_import_type() {
    for ty in TYPES {
        for arguments in ["<any>", "<\nany>", "<<T>() => T>"] {
            let source = format!("let x: {ty}{arguments};\n");
            for tsx in [false, true] {
                let module =
                    parse(&source, tsx).unwrap_or_else(|error| panic!("{source:?}: {error:?}"));
                assert_eq!(module.body.len(), 1, "{source:?}");
                assert!(has_type_arguments(&annotation(&module)), "{source:?}");
            }
        }
    }
}
