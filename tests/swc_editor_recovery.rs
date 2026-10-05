//! Partial syntax keeps independent declarations while strict syntax rejects errors.
use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::{Decl, ModuleItem, Pat, Stmt};
use swc_ecma_parser::{Parser, RecoveryMode, RecoveryRecord, StringInput, Syntax, TsSyntax};

fn parse(
    source: &str,
    tsx: bool,
    mode: RecoveryMode,
) -> (swc_ecma_ast::Module, Vec<RecoveryRecord>, usize) {
    let sm: Lrc<SourceMap> = Default::default();
    let file = sm.new_source_file(FileName::Anon.into(), source.to_string());
    let mut parser = Parser::new(
        Syntax::Typescript(TsSyntax {
            tsx,
            ..Default::default()
        }),
        StringInput::from(&*file),
        None,
    );
    parser.set_recovery_mode(mode);
    let program = parser
        .parse_module()
        .unwrap_or_else(|e| panic!("{source}: {e:?}"));
    (
        program,
        parser.take_recoveries(),
        parser.take_errors().len(),
    )
}

fn names(module: &swc_ecma_ast::Module) -> Vec<String> {
    module
        .body
        .iter()
        .filter_map(|item| {
            let ModuleItem::Stmt(Stmt::Decl(Decl::Var(decl))) = item else {
                return None;
            };
            let Pat::Ident(name) = &decl.decls[0].name else {
                return None;
            };
            Some(name.id.sym.to_string())
        })
        .collect()
}

#[test]
fn a_missing_expression_keeps_the_declaration_and_following_statements() {
    for damaged in [
        "const broken = ;",
        "const broken = object.;",
        "const broken = f(;",
    ] {
        let source = format!("{damaged} const later = 42;");
        let (module, recovery, errors) = parse(&source, false, RecoveryMode::Editor);
        assert_eq!(names(&module), ["broken", "later"], "{source}");
        assert!(!recovery.is_empty(), "{source}");
        assert!(errors > 0, "{source}");
    }
    let (_, recovery, _) = parse("const broken = ;", false, RecoveryMode::Editor);
    assert!(recovery.iter().any(|r| r.span.lo == r.span.hi));
}

#[test]
fn complete_speculative_syntax_has_no_recovery() {
    for source in [
        "const f = <T,>(x: T) => x; const later = 42;",
        "const x = <div>{1}</div>; const later = 42;",
        "const x = { const: 1, return() {}, throw: 2, else: 3 }; const later = 42;",
    ] {
        let (module, recovery, errors) = parse(source, true, RecoveryMode::Editor);
        assert_eq!(module.body.len(), 2);
        assert!(recovery.is_empty(), "{source}: {recovery:?}");
        assert_eq!(errors, 0);
    }
}

#[test]
fn lexical_text_does_not_create_declarations() {
    for tail in [
        "`const fake = 1;",
        "\"const fake = 1;",
        "/* const fake = 1;",
        "/[const fake = 1;",
    ] {
        let source = format!("const earlier = 1; const broken = {tail}");
        let (module, _, _) = parse(&source, false, RecoveryMode::Editor);
        assert!(names(&module).contains(&"earlier".into()), "{source}");
        assert!(!names(&module).contains(&"fake".into()), "{source}");
    }
}

#[test]
fn nested_lists_return_to_their_enclosing_statement() {
    for expr in ["f(g(1;", "[f(1;", "({ key: ;", "f(1, ;"] {
        let source = format!("const broken = {expr} const later = 42;");
        let (module, _, _) = parse(&source, false, RecoveryMode::Editor);
        assert!(
            names(&module).contains(&"later".into()),
            "{source}: {module:?}"
        );
    }
}

#[test]
fn a_member_hole_does_not_consume_the_next_declaration_keyword() {
    let (module, _, _) = parse(
        "const broken = object.\nconst later = 42;",
        false,
        RecoveryMode::Editor,
    );
    assert_eq!(names(&module), ["broken", "later"]);
}

#[test]
fn strict_parsing_keeps_rejecting_a_missing_expression() {
    let sm: Lrc<SourceMap> = Default::default();
    let file = sm.new_source_file(FileName::Anon.into(), "const broken = ;".to_string());
    let mut parser = Parser::new(
        Syntax::Typescript(TsSyntax::default()),
        StringInput::from(&*file),
        None,
    );
    assert!(parser.parse_module().is_err());
    assert!(parser.take_recoveries().is_empty());
}

#[test]
fn missing_type_annotations_retain_the_declaration_shape() {
    let (module, _, _) = parse(
        "const broken: = 1; const later = 42;",
        false,
        RecoveryMode::Editor,
    );
    assert_eq!(names(&module), ["broken", "later"]);
}

#[test]
fn complete_const_type_parameters_are_not_statement_boundaries() {
    let (_, recovery, errors) = parse(
        "function f<const T>(x: T) { return x; }",
        false,
        RecoveryMode::Editor,
    );
    assert!(recovery.is_empty());
    assert_eq!(errors, 0);
}

#[test]
fn a_contextual_keyword_argument_does_not_create_a_missing_element() {
    let (_, recovery, _) = parse("const x = f(let);", false, RecoveryMode::Editor);
    assert!(recovery.is_empty());
}

#[test]
fn switch_statement_lists_advance_after_a_stray_delimiter() {
    let (module, recovery, _) = parse(
        "switch (value) { case 0:\n)\n}\nconst later = 42;",
        false,
        RecoveryMode::Editor,
    );
    assert_eq!(names(&module), ["later"]);
    assert!(!recovery.is_empty());
}

#[test]
fn an_unclosed_switch_list_returns_at_end_of_file() {
    let (module, recovery, _) = parse(
        "const earlier = 42; switch (value) { case 0:",
        false,
        RecoveryMode::Editor,
    );
    assert_eq!(names(&module), ["earlier"]);
    assert!(!recovery.is_empty());
}
