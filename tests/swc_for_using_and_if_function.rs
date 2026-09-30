//! The vendored parser's `for (using …; …; …)` initializer and Annex B `if` function clause (TASK-624).
use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::{Decl, ForHead, ModuleItem, Stmt, VarDeclOrExpr};
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer};

fn parse(source: &str) -> Result<swc_ecma_ast::Module, swc_ecma_parser::error::Error> {
    let cm: Lrc<SourceMap> = Default::default();
    let file = cm.new_source_file(Lrc::new(FileName::Anon), source.to_owned());
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax::default()),
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

fn first_statement(source: &str) -> Stmt {
    let module = parse(source).unwrap_or_else(|error| panic!("{source:?}: {error:?}"));
    let Some(ModuleItem::Stmt(statement)) = module.body.into_iter().next() else {
        panic!("{source:?}: the first item is not a statement");
    };
    statement
}

#[test]
fn a_using_declaration_initializes_a_for_statement() {
    for (source, is_await, names) in [
        ("for (using q = r(); ; ) {}", false, 1),
        ("for (using q = r(), p = s(); q; ) {}", false, 2),
        ("for (using q: R = r();;) {}", false, 1),
        ("for (await using q = r(); ;) {}", true, 1),
        ("for (await using q = r(), p = s(); ; p) {}", true, 2),
    ] {
        let Stmt::For(statement) = first_statement(source) else {
            panic!("{source:?}: not a for statement");
        };
        let Some(VarDeclOrExpr::UsingDecl(declaration)) = statement.init else {
            panic!("{source:?}: the initializer is not a using declaration");
        };
        assert_eq!(declaration.is_await, is_await, "{source:?}");
        assert_eq!(declaration.decls.len(), names, "{source:?}");
        assert!(
            declaration.decls.iter().all(|decl| decl.init.is_some()),
            "{source:?}"
        );
    }
}

#[test]
fn using_stays_an_identifier_where_no_declaration_follows_it() {
    for source in [
        "for (using; ;) {}",
        "for (using = 1; ;) {}",
        "for (using.x; ;) {}",
        "for (using(); ;) {}",
        "for (using, x; ;) {}",
    ] {
        let Stmt::For(statement) = first_statement(source) else {
            panic!("{source:?}: not a for statement");
        };
        assert!(
            matches!(statement.init, Some(VarDeclOrExpr::Expr(_))),
            "{source:?}"
        );
    }
    for source in ["for (using of xs) {}", "for (using in o) {}"] {
        assert!(
            matches!(first_statement(source), Stmt::ForOf(_) | Stmt::ForIn(_)),
            "{source:?}"
        );
    }
    let Stmt::ForOf(statement) = first_statement("for (using x of xs) {}") else {
        panic!("not a for-of statement");
    };
    assert!(matches!(statement.left, ForHead::UsingDecl(_)));
}

#[test]
fn a_using_declaration_in_a_for_statement_requires_an_initializer() {
    assert!(parse("for (using q; ;) {}").is_err());
    assert!(parse("for (using {a} = r(); ;) {}").is_err());
}

#[test]
fn a_function_declaration_is_an_if_clause() {
    for source in [
        "if (c) function f() {}",
        "if (c) function f() {} else function g() {}",
        "if (c) {} else function g() {}",
        "if (c) {} else if (d) function g() {}",
    ] {
        let Stmt::If(statement) = first_statement(source) else {
            panic!("{source:?}: not an if statement");
        };
        let clauses = std::iter::once(statement.cons.as_ref()).chain(statement.alt.as_deref());
        assert!(
            clauses.into_iter().any(|clause| match clause {
                Stmt::Decl(Decl::Fn(_)) => true,
                Stmt::If(inner) => matches!(inner.cons.as_ref(), Stmt::Decl(Decl::Fn(_))),
                _ => false,
            }),
            "{source:?}"
        );
    }
    assert!(parse("if (c) function* g() {}").is_err());
    assert!(parse("while (c) function f() {}").is_err());
}
