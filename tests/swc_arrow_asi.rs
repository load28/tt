//! Exercise the vendored parser's automatic semicolon insertion after a
//! block-bodied arrow function directly (`vendor/swc_ecma_parser/TT-PATCH.md`).
use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::{Expr, ModuleItem, Stmt};
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

fn second_statement(source: &str, tsx: bool) -> Expr {
    let module = parse(source, tsx).unwrap_or_else(|error| panic!("{source:?}: {error:?}"));
    assert_eq!(module.body.len(), 2, "{source:?}");
    let ModuleItem::Stmt(Stmt::Expr(statement)) = &module.body[1] else {
        panic!("{source:?}: the second item is not an expression statement");
    };
    *statement.expr.clone()
}

const ARROWS: &[&str] = &[
    "const f = () => {}",
    "const f = async () => {}",
    "const f = (): void => {}",
    "const f = (a: number) => {}",
    "const f = <T,>(a: T) => {}",
    "let f; f = () => {}",
];

#[test]
fn a_line_after_a_block_bodied_arrow_function_starts_a_statement() {
    for arrow in ARROWS {
        let declared = if arrow.starts_with("let") { 2 } else { 1 };
        for tsx in [false, true] {
            for (line, is_expected) in [
                (
                    "/x/g.exec(\"x\")",
                    (|expr: &Expr| matches!(expr, Expr::Call(_))) as fn(&Expr) -> bool,
                ),
                ("/=x/.test(\"=x\")", |expr| matches!(expr, Expr::Call(_))),
                ("+1", |expr| matches!(expr, Expr::Unary(_))),
                ("-1", |expr| matches!(expr, Expr::Unary(_))),
                ("(1)", |expr| matches!(expr, Expr::Paren(_))),
                ("[1]", |expr| matches!(expr, Expr::Array(_))),
                ("`t`", |expr| matches!(expr, Expr::Tpl(_))),
            ] {
                let source = format!("{arrow}\n{line}\n");
                let module =
                    parse(&source, tsx).unwrap_or_else(|error| panic!("{source:?}: {error:?}"));
                assert_eq!(module.body.len(), declared + 1, "{source:?}");
                if declared == 1 {
                    let expr = second_statement(&source, tsx);
                    assert!(is_expected(&expr), "{source:?}: {expr:?}");
                }
            }
        }
    }
}

#[test]
fn an_operator_on_the_arrow_function_line_is_still_an_error() {
    for source in [
        "const f = () => {} / 2\n",
        "const f = () => {} + 1\n",
        "const f = () => {} as any\n",
    ] {
        assert!(parse(source, false).is_err(), "{source:?}");
    }
}

#[test]
fn an_operator_line_inside_arguments_is_still_an_error() {
    assert!(parse("g(() => {}\n/x/g)\n", false).is_err());
}
