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
fn keyword_enum_members_are_not_recovery_boundaries() {
    for keyword in ["const", "let", "var", "return", "throw", "export", "else"] {
        for member in [
            format!("{keyword} = 1, second = 2"),
            format!("{keyword}, second"),
        ] {
            let source = format!("enum E {{ {member} }} const later = 42;");
            let (module, recovery, errors) = parse(&source, false, RecoveryMode::Editor);
            assert!(recovery.is_empty(), "{source}: {recovery:?}");
            assert_eq!(errors, 0, "{source}");
            assert_eq!(module.body.len(), 2, "{source}");
        }
    }
}

#[test]
fn incomplete_computed_enum_members_leave_following_statements_to_their_owner() {
    for member in ["[\"a\"", "[`a`", "["] {
        let source = format!("enum E {{ {member}\nconst later = 42;");
        let (module, recovery, _) = parse(&source, false, RecoveryMode::Editor);
        assert_eq!(names(&module), ["later"], "{source}: {recovery:?}");
    }
}

#[test]
fn keyword_type_members_are_not_recovery_boundaries() {
    for keyword in ["const", "let", "var", "return", "throw", "export", "else"] {
        for member in [
            format!("{keyword}; next: number"),
            format!("{keyword}\nnext: number"),
            format!("{keyword}: number; next: number"),
            format!("{keyword}?(): void; next: number"),
        ] {
            for source in [
                format!("type T = {{ {member} }};"),
                format!("interface T {{ {member} }}"),
            ] {
                let (_, recovery, errors) = parse(&source, false, RecoveryMode::Editor);
                assert!(recovery.is_empty(), "{source}: {recovery:?}");
                assert_eq!(errors, 0, "{source}");
            }
        }
    }
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

#[test]
fn skipped_templates_preserve_following_declarations() {
    for template in [
        "`${value}`",
        "`${f({ x: 1 })}text${value}`",
        "`${`nested${value}`}tail`",
        "`${value}const fake = 1;${value}`",
    ] {
        let source = format!("cnst broken = {template}; const later = 42;");
        let (module, _, _) = parse(&source, false, RecoveryMode::Editor);
        assert_eq!(names(&module), ["later"], "{source}");
    }
}

#[test]
fn skipped_type_templates_preserve_following_declarations() {
    for damaged in [
        "const broken: ? `${value}` = 1;",
        "const broken: ? `${`nested${value}`}tail` = 1;",
    ] {
        let source = format!("{damaged} const later = 42;");
        let (module, _, _) = parse(&source, false, RecoveryMode::Editor);
        assert!(
            names(&module).contains(&"later".into()),
            "{source}: {:?}",
            names(&module)
        );
    }
}

/// Parses on a worker so a recovery loop that never ends fails the test
/// instead of hanging the suite.
fn parse_within(
    source: &str,
    tsx: bool,
    seconds: u64,
) -> (swc_ecma_ast::Module, Vec<RecoveryRecord>, usize) {
    let (sender, receiver) = std::sync::mpsc::channel();
    let owned = source.to_string();
    std::thread::spawn(move || {
        let _ = sender.send(parse(&owned, tsx, RecoveryMode::Editor));
    });
    receiver
        .recv_timeout(std::time::Duration::from_secs(seconds))
        .unwrap_or_else(|_| panic!("editor parse did not finish within {seconds}s: {source}"))
}

#[test]
fn keyword_type_members_on_their_own_line_are_members() {
    for source in [
        "interface I {\n  const\n  x: number\n}\nconst later = 1;",
        "type T = {\n  let\n  x: number\n};\nconst later = 1;",
        "interface I {\n  export\n  default: string\n}\nconst later = 1;",
        "interface I {\n  var\n  [k: string]: number\n}\nconst later = 1;",
    ] {
        let (strict, _, strict_errors) = parse(source, false, RecoveryMode::Strict);
        let (module, recovery, errors) = parse_within(source, false, 10);
        assert_eq!(strict_errors, 0, "{source}");
        assert_eq!(errors, 0, "{source}");
        assert!(recovery.is_empty(), "{source}: {recovery:?}");
        assert_eq!(names(&module), names(&strict), "{source}");
        assert_eq!(names(&module), ["later"], "{source}");
    }
}

#[test]
fn a_keyword_member_name_on_the_next_line_is_the_member() {
    // TypeScript's `parseRightSideOfDot` reports a missing name only when
    // the keyword is followed by another name on its own line.
    for source in [
        "x = obj.\nconst\ny = 1;",
        "obj.\nconst [a] = b;",
        "x = obj.\nexport\nconst y = 1;",
        "x = obj?.\nconst\ny = 1;",
    ] {
        let (_, _, strict_errors) = parse(source, false, RecoveryMode::Strict);
        let (_, recovery, errors) = parse_within(source, false, 10);
        assert_eq!(strict_errors, 0, "{source}");
        assert_eq!(errors, 0, "{source}");
        assert!(recovery.is_empty(), "{source}: {recovery:?}");
    }
    let (module, recovery, errors) =
        parse_within("const broken = obj.\nconst later = 1;", false, 10);
    assert_eq!(names(&module), ["broken", "later"]);
    assert_eq!(recovery.len(), 1);
    assert!(errors > 0);
}

#[test]
fn incomplete_superclass_type_arguments_leave_following_statements() {
    for source in [
        "const x = 1; class B extends A<\nconst y = 2;",
        "declare class A<T> {}\nclass B extends A<\nexport const y = 2;",
    ] {
        let (module, recovery, errors) = parse_within(source, false, 10);
        assert!(errors > 0, "{source}");
        assert!(!recovery.is_empty(), "{source}");
        let declared: Vec<_> = module
            .body
            .iter()
            .filter_map(|item| match item {
                ModuleItem::Stmt(Stmt::Decl(Decl::Var(decl))) => Some(&decl.decls[0].name),
                ModuleItem::ModuleDecl(swc_ecma_ast::ModuleDecl::ExportDecl(export)) => {
                    match &export.decl {
                        Decl::Var(decl) => Some(&decl.decls[0].name),
                        _ => None,
                    }
                }
                _ => None,
            })
            .filter_map(|pattern| match pattern {
                Pat::Ident(name) => Some(name.id.sym.to_string()),
                _ => None,
            })
            .collect();
        assert!(
            declared.contains(&"y".to_string()),
            "{source}: {declared:?}"
        );
    }
}

#[test]
fn speculation_rollback_is_independent_of_earlier_recoveries() {
    // Each `<` speculates; a rollback must not copy every earlier record.
    let source = "const a = ;\nf<T>(x);\n".repeat(16000);
    let (_, recovery, _) = parse_within(&source, false, 20);
    assert_eq!(recovery.len(), 16000);
}

#[test]
fn a_skipped_statement_keeps_no_recovery_of_its_parts() {
    // `return(` fails as a statement; its missing operand and `)` belong to
    // no node that remains, so only the skipped statement is recorded.
    let (_, recovery, errors) = parse_within(
        "function f() {\n  { return(return }\n}\nconst later = 1;",
        false,
        10,
    );
    assert!(errors > 0);
    assert!(
        recovery.iter().all(|record| record.replacement.is_none()),
        "{recovery:?}"
    );
}
