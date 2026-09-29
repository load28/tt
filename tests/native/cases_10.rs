#[test]
fn completion_never_offers_the_pipeline_runtime_helpers() {
    require_tsgo!();
    // Each file imports one runtime helper; the other is an export of the
    // runtime TypeScript would offer to import.
    for marked in [
        "const trimmed = flow |> String |> .trim();\nconst t = @@;\n",
        "function cat(a: string, b: string): string { return a + b; }\n\
const v = 1 |> String |> cat(@@",
    ] {
        let (source, position) = at_cursor(marked);
        let dir = project(&[("src/main.tt", &source)]);
        let file = dir.join("src/main.tt").canonicalize().unwrap();
        let mut project = open_service(&file);
        let answer = project.completion(&file, position, false).unwrap();
        let generated: Vec<_> = answer
            .items
            .iter()
            .filter(|item| item.label.starts_with("$tt_"))
            .map(|item| item.label.as_str())
            .collect();
        assert_eq!(generated, Vec::<&str>::new(), "{source}");
        assert!(
            answer.items.iter().any(|item| item.label == "String"),
            "{source}"
        );
    }
}

#[test]
fn an_unfinished_pipeline_step_leaves_the_rest_of_the_function_checked() {
    require_tsgo!();
    // TypeScript on `const n = xs.length +` reports only the missing
    // operand; the head, the statement after it, and the function's
    // return stay what they are.
    for next in ["return n;", "const m = n + 1; return m;"] {
        let source = format!(
            "export function run(xs: number[]): number {{\n  const n = xs |> .length |> \n  {next}\n}}\n"
        );
        let dir = project(&[("src/main.tt", &source)]);
        let file = dir.join("src/main.tt").canonicalize().unwrap();
        let mut project = open_service(&file);
        assert_eq!(
            listed(&project.service_diagnostics(&file).unwrap()),
            vec![],
            "{source}"
        );
        let hover = project
            .hover(&file, utf16_position(&source, "xs |>"))
            .unwrap()
            .expect("hover on the head");
        assert_eq!(hover.signature, "(parameter) xs: number[]");
    }
}

#[test]
fn a_syntax_error_in_a_match_arm_is_reported_where_typescript_puts_it() {
    require_tsgo!();
    // In the `.ts` twin TypeScript reports at the token after the arm body,
    // the `,`; the glue `;` stands where that token was.
    let decl = "variant Shape { Circle(radius: number), Square(side: number) }\n\
declare const s: Shape;\n";
    for (name, body, code, message) in [
        ("main.tt", "radius.", 1003, "Identifier expected."),
        ("main.tt", "radius *", 1109, "Expression expected."),
        ("main.ttx", "radius.", 1003, "Identifier expected."),
    ] {
        let source = format!(
            "{decl}export const a = match (s) {{\n  Circle(radius) => {body},\n  Square(side) => side,\n}};\n"
        );
        let dir = project(&[(&format!("src/{name}"), &source)]);
        let file = dir.join("src").join(name).canonicalize().unwrap();
        let mut project = open_service(&file);
        let comma = utf16_position(&source, &format!("{body},")).character + body.len() as u32;
        assert_eq!(
            listed(&project.service_diagnostics(&file).unwrap()),
            vec![(3, comma, code, message.to_string())],
            "{source}"
        );
    }
}

#[test]
fn an_unfinished_pipeline_call_step_answers_signature_help() {
    require_tsgo!();
    let cat = "function cat(a: string, b: string): string { return a + b; }\n";
    let add = "function add(a: number, b: number): number { return a + b; }\n";
    for (marked, label) in [
        (format!("{cat}const v = 1 |> String |> cat(\"x\", @@"), "cat(a: string, b: string)"),
        (format!("{add}const v = 1 |> add(2, @@"), "add(a: number, b: number)"),
        (format!("{add}const v = 1 |> add(2, @@\nconst w = 1;\n"), "add(a: number, b: number)"),
        (
            format!("{add}export function f() {{\n  const v = 1 |> add(2, @@\n}}\n"),
            "add(a: number, b: number)",
        ),
    ] {
        let (source, position) = at_cursor(&marked);
        let dir = project(&[("src/main.tt", &source)]);
        let file = dir.join("src/main.tt").canonicalize().unwrap();
        let mut project = open_service(&file);
        let help = project
            .signature_help(&file, position)
            .unwrap()
            .unwrap_or_else(|| panic!("no signature help: {source}"));
        assert!(
            help.signatures[help.active_signature as usize]
                .label
                .starts_with(label),
            "{source}: {help:?}"
        );
        assert_eq!(help.active_parameter, 1, "{source}");
    }
}

#[test]
fn a_type_error_keeps_its_rendering_while_an_open_document_does_not_parse() {
    require_tsgo!();
    // The typed layer is the one an editor shows once it answers; while
    // line 2 does not parse, it still checks the buffer and says the same
    // thing about line 1.
    let clean = "export const bad: number = \"x\";\nexport const y = 1 + 2;\n";
    let edited = "export const bad: number = \"x\";\nexport const y = 1 + ;\n";
    let dir = project(&[("src/main.tt", clean)]);
    let mismatch = |answer: &serde_json::Value| {
        answer["result"]["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("{answer}"))
            .iter()
            .filter(|d| d["code"] == "ts2322")
            .map(|d| (d["line"].clone(), d["col"].clone(), d["message"].clone()))
            .collect::<Vec<_>>()
    };
    let before = typed_server(&dir, "src/main.tt", clean);
    let during = typed_server(&dir, "src/main.tt", edited);
    assert_eq!(during["result"]["blocked"], false, "{during}");
    assert_eq!(mismatch(&during), mismatch(&before), "{during}");
    assert_eq!(
        mismatch(&before),
        vec![(
            serde_json::json!(1),
            serde_json::json!(28),
            serde_json::json!("type mismatch: expected `number`, found `\"x\"`"),
        )]
    );
    assert!(
        during["result"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "ts1109" && d["line"] == 2),
        "{during}"
    );

    // A file read from disk is checked as `tsc` checks it: blocked while
    // it does not parse.
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    fs::write(&file, edited).unwrap();
    let mut project = ttc::engine::Engine::new(None)
        .open_project(
            &[file.to_string_lossy().into_owned()],
            &ttc::engine::ProjectOptions::default(),
        )
        .unwrap();
    let snapshot = project.update(std::slice::from_ref(&file)).unwrap();
    assert!(snapshot.is_blocked(&file));
    project.open_document(file.clone(), edited.to_string());
    let snapshot = project.update(std::slice::from_ref(&file)).unwrap();
    assert!(!snapshot.is_blocked(&file));
}
