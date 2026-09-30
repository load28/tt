//! The vendored parser reads five forms TypeScript's parser reads (TASK-641).
use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::{
    Decl, ForHead, JSXAttrOrSpread, JSXAttrValue, ModuleDecl, ModuleExportName, ModuleItem, Stmt,
};
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer};

fn parse(source: &str, tsx: bool) -> Result<swc_ecma_ast::Module, swc_ecma_parser::error::Error> {
    let cm: Lrc<SourceMap> = Default::default();
    let file = cm.new_source_file(Lrc::new(FileName::Anon), source.to_owned());
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx,
            decorators: true,
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

fn items(source: &str, tsx: bool) -> Vec<ModuleItem> {
    parse(source, tsx)
        .unwrap_or_else(|error| panic!("{source:?}: {error:?}"))
        .body
}

#[test]
fn of_is_the_binding_of_an_await_using_declaration_in_a_for_of_head() {
    for (source, index) in [
        ("async function f() { for (await using of of []) {} }", 0),
        (
            "async function f() { for await (await using of of []) {} }",
            0,
        ),
        ("for (await using of of of) {}", 0),
    ] {
        let statement = match &items(source, false)[index] {
            ModuleItem::Stmt(Stmt::Decl(Decl::Fn(function))) => {
                function.function.body.as_ref().unwrap().stmts[0].clone()
            }
            ModuleItem::Stmt(statement) => statement.clone(),
            other => panic!("{source:?}: {other:?}"),
        };
        let Stmt::ForOf(for_of) = statement else {
            panic!("{source:?}: not a for-of statement");
        };
        let ForHead::UsingDecl(declaration) = for_of.left else {
            panic!("{source:?}: the head is not a using declaration");
        };
        assert!(declaration.is_await, "{source:?}");
        assert_eq!(
            declaration.decls[0].name.as_ident().unwrap().sym,
            "of",
            "{source:?}"
        );
    }
    assert!(parse("for (using of of []) {}", false).is_err());
}

#[test]
fn a_modifier_followed_by_as_opens_a_parenthesized_expression() {
    for source in [
        "declare const readonly: unknown;\nexport const a = (readonly as number);\n",
        "declare const override: unknown;\nexport const b = (override as number);\n",
    ] {
        items(source, false);
    }
    items(
        "class C { constructor(readonly x: number) {} }\nconst f = (readonly: number) => readonly;\n",
        false,
    );
}

#[test]
fn a_jsx_attribute_string_after_whitespace_spans_lines() {
    for source in [
        "const a = <div className= \"foo\n\n bar\" />;\n",
        "const c = <div className=\n\"foo\n\n bar\" />;\n",
        "const d = <div className=\t'foo\nbar' />;\n",
    ] {
        let module = items(source, true);
        let ModuleItem::Stmt(Stmt::Decl(Decl::Var(declaration))) = &module[0] else {
            panic!("{source:?}: not a declaration");
        };
        let swc_ecma_ast::Expr::JSXElement(element) =
            &**declaration.decls[0].init.as_ref().unwrap()
        else {
            panic!("{source:?}: not a JSX element");
        };
        let JSXAttrOrSpread::JSXAttr(attribute) = &element.opening.attrs[0] else {
            panic!("{source:?}: not an attribute");
        };
        assert!(
            matches!(attribute.value, Some(JSXAttrValue::Str(ref s)) if s.value.to_string_lossy().contains('\n')),
            "{source:?}"
        );
    }
}

#[test]
fn a_type_only_export_specifier_names_a_string() {
    let module = items("export { type \"x\" as \"c d\" } from \"./m\";\n", false);
    let ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(export)) = &module[0] else {
        panic!("not a named export");
    };
    let swc_ecma_ast::ExportSpecifier::Named(specifier) = &export.specifiers[0] else {
        panic!("not a named specifier");
    };
    assert!(specifier.is_type_only);
    assert!(matches!(specifier.orig, ModuleExportName::Str(_)));
    assert!(matches!(specifier.exported, Some(ModuleExportName::Str(_))));
}

#[test]
fn a_decorator_after_export_decorates_an_abstract_class() {
    let module = items(
        "declare const dec: any;\nexport @dec abstract class C11 {}\n",
        false,
    );
    let ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) = &module[1] else {
        panic!("not an exported declaration");
    };
    let Decl::Class(class) = &export.decl else {
        panic!("not a class");
    };
    assert!(class.class.is_abstract);
    assert_eq!(class.class.decorators.len(), 1);
}

#[test]
fn import_attributes_follow_a_line_break_but_an_assertion_does_not() {
    for source in [
        "import a\n\tfrom \"./a.json\"\n\twith {type: \"json\"}\n",
        "import \"./a.json\"\nwith { type: \"json\" };\n",
        "export { a } from \"./a.json\"\nwith { type: \"json\" };\n",
    ] {
        let module = items(source, false);
        let with = match &module[0] {
            ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => import.with.is_some(),
            ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(export)) => export.with.is_some(),
            other => panic!("{source:?}: {other:?}"),
        };
        assert!(with, "{source:?}");
    }
    let module = items(
        "import a from \"./a.json\"\nassert({ type: \"json\" });\n",
        false,
    );
    let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = &module[0] else {
        panic!("not an import");
    };
    assert!(import.with.is_none());
}
