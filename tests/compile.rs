//! Emitted-code and error-reporting tests for the tt → TypeScript transform.

use ttc::{DiagnosticCode, Options, SourceKind, compile};

fn ok(src: &str) -> String {
    compile(src, &Options::default()).expect("compile failed")
}

fn compact(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn err(src: &str) -> ttc::CompileError {
    compile(src, &Options::default()).expect_err("expected a compile error")
}

fn ok_tsx(src: &str) -> String {
    compile(
        src,
        &Options {
            source_kind: SourceKind::Tsx,
            ..Options::default()
        },
    )
    .expect("ttx compile failed")
}

/* ------------------------------------------------------------------ */
/* TASK-311 reported composition regressions                           */
/* ------------------------------------------------------------------ */

#[test]
fn value_region_nesting_matrix_compiles_every_directed_pair() {
    let inners = [
        ("match", "match (flag) { true => 1, false => 2 }"),
        (
            "tuple-match",
            "match (tag, tag) { (A, A) => 1, (_, _) => 2 }",
        ),
        ("pipeline", "1 |> ((value: number) => value + 1)"),
        (
            "result",
            "result { const value = try load(); return value; }",
        ),
        ("try", "try load()"),
        ("flow", "flow |> ((value: number) => value + 1)"),
    ];
    let outers = [
        ("match", "match ({inner}) { _ => 0 }"),
        ("tuple-match", "match ({inner}, true) { (_, _) => 0 }"),
        ("pipeline", "({inner}) |> ((value) => value)"),
        (
            "result",
            "result { const seed = try load(); return {inner}; }",
        ),
        ("try", "try ({inner})"),
        ("flow", "flow |> ((_: unknown) => ({inner}))"),
    ];
    let mut cells = 0;
    for (outer_name, outer) in outers {
        for (inner_name, inner) in inners {
            let expression = outer.replace("{inner}", inner);
            let source = format!(
                "variant E {{ A, B }}\n\
                 declare const flag: boolean; declare const tag: E;\n\
                 declare const load: () => {{ kind: \"Ok\"; value: number }} | {{ kind: \"Err\"; error: string }};\n\
                 function probe() {{ const output = {expression}; return output; }}\n"
            );
            let compiled = std::panic::catch_unwind(|| compile(&source, &Options::default()))
                .unwrap_or_else(|payload| {
                    panic!("{outer_name} <- {inner_name} unwound:\n{source}\n{payload:?}")
                });
            let output = compiled.unwrap_or_else(|error| {
                let unchecked = compile(
                    &source,
                    &Options {
                        verify: false,
                        ..Options::default()
                    },
                )
                .expect("unchecked lowering");
                panic!(
                    "{outer_name} <- {inner_name} failed:\n{source}\n{error:#?}\n--- unchecked ---\n{unchecked}"
                )
            });
            assert!(!output.is_empty(), "{outer_name} <- {inner_name}");
            cells += 1;
        }
    }
    assert_eq!(cells, 36);
}

#[test]
fn every_value_region_crosses_every_host_protocol_class() {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Capability {
        Statements,
        Expression,
        Propagation,
        StructuredPropagation,
        Isolated,
    }

    struct ValueCase {
        name: String,
        expression: String,
        capability: Capability,
        reject_when_discarded: bool,
        ungrouped_safe: bool,
    }

    struct HostCase {
        name: &'static str,
        source_kind: SourceKind,
        source: &'static str,
        owner_takes_statements: bool,
        propagation_boundary: bool,
        repeated: bool,
        unmodeled_conditional: bool,
    }

    let mut values = vec![
        ValueCase {
            name: "match".into(),
            expression: "match (flag) { true => 1, false => 2 }".into(),
            capability: Capability::Statements,
            reject_when_discarded: false,
            ungrouped_safe: true,
        },
        ValueCase {
            name: "tuple-match".into(),
            expression: "match (flag, flag) { (_, _) => 1 }".into(),
            capability: Capability::Statements,
            reject_when_discarded: false,
            ungrouped_safe: true,
        },
        ValueCase {
            name: "pipeline".into(),
            expression: "1 |> ((value: number) => value + 1)".into(),
            capability: Capability::Expression,
            reject_when_discarded: false,
            ungrouped_safe: false,
        },
        ValueCase {
            name: "result".into(),
            expression: "result { const value = try load(); return value; }".into(),
            capability: Capability::Isolated,
            reject_when_discarded: true,
            ungrouped_safe: true,
        },
        ValueCase {
            name: "try".into(),
            expression: "try load()".into(),
            capability: Capability::Propagation,
            reject_when_discarded: false,
            ungrouped_safe: true,
        },
        ValueCase {
            name: "flow".into(),
            expression: "flow |> ((value: number) => value + 1)".into(),
            capability: Capability::Expression,
            reject_when_discarded: false,
            ungrouped_safe: false,
        },
    ];
    let inner_values = [
        (
            "match",
            "match (flag) { true => 1, false => 2 }",
            Capability::Statements,
        ),
        (
            "tuple-match",
            "match (flag, flag) { (_, _) => 1 }",
            Capability::Statements,
        ),
        (
            "pipeline",
            "1 |> ((value: number) => value + 1)",
            Capability::Expression,
        ),
        (
            "result",
            "result { const value = try load(); return value; }",
            Capability::Isolated,
        ),
        ("try", "try load()", Capability::Propagation),
        (
            "flow",
            "flow |> ((value: number) => value + 1)",
            Capability::Expression,
        ),
    ];
    let outer_values = [
        (
            "match",
            "match ({inner}) { _ => 0 }",
            Capability::Statements,
        ),
        (
            "tuple-match",
            "match ({inner}, true) { (_, _) => 0 }",
            Capability::Statements,
        ),
        (
            "pipeline",
            "({inner}) |> ((value) => value)",
            Capability::Expression,
        ),
        (
            "result",
            "result { const seed = try load(); return {inner}; }",
            Capability::Isolated,
        ),
        ("try", "try ({inner})", Capability::Propagation),
        (
            "flow",
            "flow |> ((_: unknown) => ({inner}))",
            Capability::Expression,
        ),
    ];
    for (outer_name, outer, outer_capability) in outer_values {
        for (inner_name, inner, inner_capability) in inner_values {
            let capability = match outer_name {
                "result" => Capability::Isolated,
                "try" => Capability::Propagation,
                "flow" => Capability::Expression,
                "pipeline" => match inner_capability {
                    Capability::Statements => Capability::Statements,
                    Capability::Propagation | Capability::StructuredPropagation => {
                        Capability::Propagation
                    }
                    Capability::Expression | Capability::Isolated => Capability::Expression,
                },
                "match" | "tuple-match" => match inner_capability {
                    Capability::Propagation | Capability::StructuredPropagation => {
                        Capability::StructuredPropagation
                    }
                    _ => outer_capability,
                },
                _ => unreachable!(),
            };
            values.push(ValueCase {
                name: format!("{outer_name} <- {inner_name}"),
                expression: outer.replace("{inner}", inner),
                capability,
                reject_when_discarded: outer_name == "result",
                ungrouped_safe: matches!(outer_name, "match" | "tuple-match" | "result" | "try"),
            });
        }
    }
    let hosts = [
        HostCase {
            name: "eager-binary-left",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = ({expr}) + right(); }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "eager-binary-right",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = left() + ({expr}); }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "eager-array-element",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = [before(), {expr}]; }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "eager-object-evaluation",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = { a: before(), b: {expr} }; }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "eager-assignment-right",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { target = {expr}; }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "eager-sequence-element",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = (before(), {expr}); }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "eager-unary-operand",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = !({expr}); }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "eager-call-argument",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { use(before(), {expr}); }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "eager-construct-argument",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { new Box(before(), {expr}); }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "eager-template-interpolation",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = `${before()}${{expr}}`; }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "eager-jsx-expression",
            source_kind: SourceKind::Tsx,
            source: "function probe() { const x = <main data-x={{expr}} />; }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "eager-jsx-child",
            source_kind: SourceKind::Tsx,
            source: "function probe() { const x = <main>{{expr}}</main>; }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "conditional-and-right",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = ready && ({expr}); }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "conditional-or-right",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = ready || ({expr}); }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "conditional-nullish-right",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = maybe ?? ({expr}); }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "conditional-consequent",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = ready ? ({expr}) : 0; }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "conditional-alternate",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = ready ? 0 : ({expr}); }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "conditional-optional-call-argument",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { fn?.(before(), {expr}); }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "reference-call-callee",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { ({expr})(); }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "reference-optional-call-callee",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { ({expr})?.(); }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "reference-member-object",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = ({expr}).value; }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "reference-member-property",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = object[{expr}]; }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "reference-constructor-callee",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = new ({expr})(); }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "reference-tagged-template-tag",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = ({expr})`text`; }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "suspend-await",
            source_kind: SourceKind::TypeScript,
            source: "async function probe() { return await ({expr}); }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "suspend-yield",
            source_kind: SourceKind::TypeScript,
            source: "function* probe() { yield {expr}; }",
            owner_takes_statements: true,
            propagation_boundary: false,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "suspend-yield-delegate",
            source_kind: SourceKind::TypeScript,
            source: "function* probe() { yield* {expr}; }",
            owner_takes_statements: true,
            propagation_boundary: false,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "loop-test",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { while ({expr}) { break; } }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: true,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "continuation-return",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { return {expr}; }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "continuation-arrow-return",
            source_kind: SourceKind::TypeScript,
            source: "const probe = () => {expr};",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "continuation-initialize",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { const x = {expr}; }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "continuation-for-initialize",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { for (let x = {expr};;) { break; } }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "continuation-discard",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { {expr}; }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "owner-module",
            source_kind: SourceKind::TypeScript,
            source: "const x = {expr};",
            owner_takes_statements: true,
            propagation_boundary: false,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "owner-constructor",
            source_kind: SourceKind::TypeScript,
            source: "class C { constructor() { use({expr}); } }",
            owner_takes_statements: true,
            propagation_boundary: false,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "owner-generator",
            source_kind: SourceKind::TypeScript,
            source: "function* probe() { use({expr}); }",
            owner_takes_statements: true,
            propagation_boundary: false,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "owner-parameter",
            source_kind: SourceKind::TypeScript,
            source: "function probe(x = {expr}) {}",
            owner_takes_statements: false,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "owner-class-field",
            source_kind: SourceKind::TypeScript,
            source: "class C { x = {expr}; }",
            owner_takes_statements: false,
            propagation_boundary: false,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "owner-member-decorator",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { class C { @use({expr}) m() {} } }",
            owner_takes_statements: false,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "owner-computed-member-name",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { class C { [use({expr})]() {} } }",
            owner_takes_statements: false,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "owner-class-decorator",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { @use({expr}) class C {} }",
            owner_takes_statements: false,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "owner-decorated-class-heritage",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { @use class C extends use({expr}) {} }",
            owner_takes_statements: false,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "class-heritage",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { class C extends use({expr}) {} }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "owner-static-block",
            source_kind: SourceKind::TypeScript,
            source: "class C { static { use({expr}); } }",
            owner_takes_statements: true,
            propagation_boundary: false,
            repeated: false,
            unmodeled_conditional: false,
        },
        HostCase {
            name: "reach-unmodeled-conditional",
            source_kind: SourceKind::TypeScript,
            source: "function probe() { switch (value) { case {expr}: break; } }",
            owner_takes_statements: true,
            propagation_boundary: true,
            repeated: false,
            unmodeled_conditional: true,
        },
    ];

    let prelude = "type R<T> = { kind: \"Ok\"; value: T } | { kind: \"Err\"; error: string };\n\
        declare const flag: boolean, ready: boolean, maybe: number | undefined, target: any, fn: any, object: any, value: any;\n\
        declare function load(): R<number>; declare function before(): number; declare function left(): number; declare function right(): number; declare function use(...values: any[]): any;\n\
        declare class Box { constructor(...values: any[]); }\n";
    let mut cells = 0;
    let mut ungrouped_cells = 0;
    for value in &values {
        for host in &hosts {
            let ungrouped = if value.ungrouped_safe {
                match host.name {
                    "eager-binary-left" => Some("function probe() { const x = {expr} + right(); }"),
                    "eager-binary-right" => Some("function probe() { const x = left() + {expr}; }"),
                    "eager-unary-operand" => Some("function probe() { const x = !{expr}; }"),
                    "conditional-and-right" => {
                        Some("function probe() { const x = ready && {expr}; }")
                    }
                    "conditional-or-right" => {
                        Some("function probe() { const x = ready || {expr}; }")
                    }
                    "conditional-nullish-right" => {
                        Some("function probe() { const x = maybe ?? {expr}; }")
                    }
                    "conditional-consequent" => {
                        Some("function probe() { const x = ready ? {expr} : 0; }")
                    }
                    "conditional-alternate" => {
                        Some("function probe() { const x = ready ? 0 : {expr}; }")
                    }
                    "suspend-await" => Some("async function probe() { return await {expr}; }"),
                    _ => None,
                }
            } else {
                None
            };
            for (surface, template) in [("canonical", Some(host.source)), ("ungrouped", ungrouped)]
                .into_iter()
                .filter_map(|(surface, template)| template.map(|template| (surface, template)))
            {
                let cell_host = format!("{}:{surface}", host.name);
                let source = format!("{prelude}{}", template.replace("{expr}", &value.expression));
                let expected_diagnostic =
                    if value.reject_when_discarded && host.name == "continuation-discard" {
                        Some(DiagnosticCode::ResultValueDiscarded)
                    } else {
                        match value.capability {
                            Capability::Expression | Capability::Isolated => None,
                            Capability::Statements => (!host.owner_takes_statements
                                || host.unmodeled_conditional)
                                .then_some(DiagnosticCode::MatchPlacement),
                            Capability::Propagation => (!host.owner_takes_statements
                                || !host.propagation_boundary
                                || host.repeated
                                || host.unmodeled_conditional)
                                .then_some(DiagnosticCode::TryPlacement),
                            Capability::StructuredPropagation => {
                                if !host.owner_takes_statements || host.unmodeled_conditional {
                                    Some(DiagnosticCode::MatchPlacement)
                                } else if !host.propagation_boundary {
                                    Some(DiagnosticCode::TryPlacement)
                                } else {
                                    None
                                }
                            }
                        }
                    };
                let expected_to_compile = expected_diagnostic.is_none();
                let result = std::panic::catch_unwind(|| {
                    compile(
                        &source,
                        &Options {
                            source_kind: host.source_kind,
                            ..Options::default()
                        },
                    )
                })
                .unwrap_or_else(|payload| {
                    panic!(
                        "{} in {} unwound:\n{source}\n{payload:?}",
                        value.name, cell_host
                    )
                });
                match (expected_to_compile, result) {
                    (true, Ok(output)) => assert!(!output.is_empty()),
                    (false, Err(error)) => {
                        assert!(
                            !error
                                .message
                                .contains("generated TypeScript failed to parse"),
                            "{source}\n{error}"
                        );
                        let diagnostics = ttc::analyze(
                            &source,
                            &Options {
                                source_kind: host.source_kind,
                                ..Options::default()
                            },
                        );
                        assert_eq!(
                            diagnostics.first().map(|diagnostic| diagnostic.code),
                            expected_diagnostic,
                            "{} in {} returned the wrong diagnostic:\n{source}\n{diagnostics:#?}",
                            value.name,
                            cell_host
                        );
                    }
                    (true, Err(error)) => {
                        let unchecked = compile(
                            &source,
                            &Options {
                                source_kind: host.source_kind,
                                verify: false,
                                ..Options::default()
                            },
                        );
                        match unchecked {
                            Ok(unchecked) => panic!(
                                "{} in {} should compile:\n{source}\n{error:#?}\n--- unchecked ---\n{unchecked}",
                                value.name, cell_host
                            ),
                            Err(unchecked_error) => panic!(
                                "{} in {} should compile:\n{source}\n{error:#?}\n--- unchecked error ---\n{unchecked_error:#?}",
                                value.name, cell_host
                            ),
                        }
                    }
                    (false, Ok(output)) => panic!(
                        "{} in {} should report a placement diagnostic:\n{source}\n{output}",
                        value.name, cell_host
                    ),
                }
                cells += 1;
                ungrouped_cells += usize::from(surface == "ungrouped");
            }
        }
    }
    assert_eq!(values.len(), 42);
    assert_eq!(hosts.len(), 45);
    assert_eq!(ungrouped_cells, 252);
    assert_eq!(cells, 2_142);
}

include!("compile/cases_01.rs");
include!("compile/cases_02.rs");
include!("compile/cases_03.rs");
include!("compile/cases_04.rs");
include!("compile/cases_05.rs");
include!("compile/cases_06.rs");
include!("compile/cases_07.rs");
include!("compile/cases_08.rs");
include!("compile/cases_09.rs");
include!("compile/cases_10.rs");
include!("compile/cases_11.rs");
include!("compile/cases_12.rs");
include!("compile/cases_13.rs");
include!("compile/cases_14.rs");
#[test]
fn editor_projection_rejects_namespaced_jsx_members_without_panicking() {
    let options = Options {
        source_kind: ttc::SourceKind::Tsx,
        ..Options::default()
    };
    for tail in ["<G:U.m", "<G:U.m />", "<div><G:U.m /></div>"] {
        let source = format!("const good = 1 |> String;\n{tail}");
        let report = ttc::compile_projection_report(&source, &options);
        assert!(!report.diagnostics.is_empty(), "{source}");
        assert!(ttc::compile_report(&source, &options).emit.is_none());
    }
}

#[test]
fn editor_projection_rejects_conflict_markers_without_panicking() {
    for marker in ["=======", "<<<<<<< ours", ">>>>>>> theirs", "||||||| base"] {
        let source = format!("const good = 1 |> String;\n{marker}\n/\u{2}\n");
        let options = Options::default();
        let report = ttc::compile_projection_report(&source, &options);
        assert!(report.emit.is_none(), "{source:?}");
        assert!(!report.diagnostics.is_empty(), "{source:?}");
        assert!(ttc::compile_report(&source, &options).emit.is_none());
    }
}

#[test]
fn editor_projection_recovers_incomplete_computed_enum_members() {
    for member in [
        "[\"a\"",
        "[\"a\" = 1, return = 2 }",
        "[`a`",
        "[",
        "[\"a",
        "[`a",
    ] {
        let source = format!(
            "const good = 1 |> String;\nenum E {{ {member}\nexport const later = match (true) {{ true => 1, false => 2 }};\n"
        );
        let options = Options::default();
        let report = ttc::compile_projection_report(&source, &options);
        assert!(!report.diagnostics.is_empty(), "{source}");
        assert!(
            ttc::compile_report(&source, &options).emit.is_none(),
            "{source}"
        );
    }
}

#[test]
fn editor_projection_preserves_matches_after_a_missing_initializer() {
    let source = "variant Status { Idle, Busy }\n\
                  const s: Status = Status.Idle;\n\
                  const broken = ;\n\
                  const a = match (s) { Idle => 1, Busy => 2 };\n\
                  const b = match (s) { Idle => 3, Busy => 4 };\n\
                  const c = match (s) { Idle => 5, Busy => 6 };\n\
                  const wrong: number = \"wrong\";\n";
    let options = ttc::Options::default();
    let report = ttc::compile_projection_report(source, &options);
    assert!(
        report.emit.is_some(),
        "the current editor projection must retain independent owners: {:?}",
        report.diagnostics
    );
    let emit = report.emit.unwrap();
    for declaration in ["const a", "const b", "const c", "const wrong"] {
        assert!(emit.code.contains(declaration), "{}", emit.code);
    }
    assert!(!report.diagnostics.is_empty());
    assert!(ttc::compile_report(source, &options).emit.is_none());
}

#[test]
fn editor_projection_preserves_siblings_after_a_damaged_jsx_owner() {
    let source = "declare const flag: boolean;\n\
        const view = <div><span {match (flag) { true => 1, false => 2 }}</div>;\n\
        const later = match (flag) { true => 3, false => 4 };\n";
    let options = Options {
        source_kind: ttc::SourceKind::Tsx,
        ..Options::default()
    };
    let report = ttc::compile_projection_report(source, &options);
    let emit = report
        .emit
        .expect("independent owners survive a damaged JSX owner");
    assert!(emit.code.contains("const later"), "{}", emit.code);
    assert!(
        !report.recovered.is_empty(),
        "the malformed owner's lost tt child has parser-owned recovery"
    );
    assert!(!report.diagnostics.is_empty());
    assert!(ttc::compile_report(source, &options).emit.is_none());
}

#[test]
fn editor_delimiter_insertions_preserve_unicode_source_mappings() {
    for extension in [ttc::SourceKind::TypeScript, ttc::SourceKind::Tsx] {
        let source = "const 한글 = '😀';\r\ndeclare const flag: boolean;\r\ndeclare function f(...xs: unknown[]): unknown;\r\nconst broken = f(f(\r\nconst later = match (flag) { true => 한글, false => '다음' };\r\n";
        let options = ttc::Options {
            source_kind: extension,
            ..Default::default()
        };
        let report = ttc::compile_projection_report(source, &options);
        assert!(report.editor_only);
        assert!(!report.syntax_repairs.is_empty());
        let emit = report
            .emit
            .expect("nested missing delimiters retain siblings");
        for mapping in &emit.mappings {
            assert_eq!(
                &source[mapping.src..mapping.src + mapping.len],
                &emit.code[mapping.out..mapping.out + mapping.len]
            );
        }
        assert!(emit.code.contains("const later"));
        assert!(ttc::compile_report(source, &options).emit.is_none());
    }
}
