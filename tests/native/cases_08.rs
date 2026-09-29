fn open_service(file: &Path) -> ttc::engine::Project {
    ttc::engine::Engine::new(None)
        .open_project(
            &[file.to_string_lossy().into_owned()],
            &ttc::engine::ProjectOptions::default(),
        )
        .unwrap()
}

/// `(line, character, code, message)` of every diagnostic, the way an
/// editor lists them.
fn listed(diagnostics: &[ttc::engine::ServiceDiagnostic]) -> Vec<(u32, u32, u32, String)> {
    diagnostics
        .iter()
        .map(|d| {
            (
                d.range.start.line,
                d.range.start.character,
                d.code,
                d.message.clone(),
            )
        })
        .collect()
}

#[test]
fn a_syntax_error_keeps_the_type_errors_a_ts_file_shows() {
    require_tsgo!();
    let edits = [
        "o.",
        "const b = ",
        "Math.max(1,",
        "if (o.k",
        "function f() {",
        "const s = \"abc",
    ];
    let texts: Vec<String> = edits
        .iter()
        .map(|edit| {
            format!("const a: number = \"x\";\nconst o = {{ k: 1 }};\n{edit}\nexport {{}};\n")
        })
        .collect();
    let names: Vec<(String, String)> = (0..edits.len())
        .map(|i| (format!("src/edit{i}.tt"), format!("src/twin{i}.ts")))
        .collect();
    let mut files = Vec::new();
    for ((tt, ts), text) in names.iter().zip(&texts) {
        files.push((tt.as_str(), text.as_str()));
        files.push((ts.as_str(), text.as_str()));
    }
    let dir = project(&files);
    for ((tt, ts), edit) in names.iter().zip(edits) {
        let tt = dir.join(tt).canonicalize().unwrap();
        let ts = dir.join(ts).canonicalize().unwrap();
        let mut project = open_service(&tt);
        let shown = listed(&project.service_diagnostics(&tt).unwrap());
        assert!(
            shown
                .iter()
                .any(|&(line, _, code, _)| line == 0 && code == 2322),
            "{edit}: {shown:?}"
        );
        assert_eq!(
            shown,
            listed(&project.service_diagnostics(&ts).unwrap()),
            "{edit}"
        );
        assert_eq!(
            project.service_restates(&tt).unwrap(),
            vec![ttc::DiagnosticCode::VerifyFailed],
            "{edit}"
        );
    }
}

#[test]
fn a_syntax_error_beside_tt_constructs_keeps_their_type_errors() {
    require_tsgo!();
    let variant = "variant V { A(n: number), B }\n\
declare const v: V;\n\
const a: number = \"x\";\n\
const m = match (v) { A(n) => n, B => 0 };\n\
const u: string = m;\n\
o.\n\
export {};\n";
    let values = "declare function parse(t: string): { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\n\
export function h() {\n\
\x20 const n = try parse(\"1\");\n\
\x20 const k = 1 + try parse(\"2\");\n\
\x20 const r = result { const q = try parse(\"1\"); return q; };\n\
\x20 const w: string = n;\n\
\x20 return { kind: \"Ok\" as const, value: [k, r, w] };\n\
}\n\
const a: number = \"x\";\n\
Math.max(1,\n";
    let dir = project(&[("src/variant.tt", variant), ("src/values.tt", values)]);
    let error = |line, character, code, message: &str| (line, character, code, message.to_string());
    for (name, expected) in [
        (
            "src/variant.tt",
            vec![
                error(6, 7, 1005, "';' expected."),
                error(2, 6, 2322, "Type 'string' is not assignable to type 'number'."),
                error(4, 6, 2322, "Type 'number' is not assignable to type 'string'."),
                error(5, 0, 2304, "Cannot find name 'o'."),
            ],
        ),
        (
            "src/values.tt",
            vec![
                error(10, 0, 1005, "')' expected."),
                error(5, 8, 2322, "Type 'number' is not assignable to type 'string'."),
                error(8, 6, 2322, "Type 'string' is not assignable to type 'number'."),
            ],
        ),
    ] {
        let file = dir.join(name).canonicalize().unwrap();
        let mut project = open_service(&file);
        let errors: Vec<_> = project
            .service_diagnostics(&file)
            .unwrap()
            .into_iter()
            .filter(|d| d.severity == ttc::engine::ServiceSeverity::Error)
            .collect();
        assert_eq!(listed(&errors), expected, "{name}");
        assert_eq!(
            project.service_restates(&file).unwrap(),
            vec![ttc::DiagnosticCode::SourceNotTypeScript],
            "{name}"
        );
    }
}

#[test]
fn an_auto_import_lands_where_the_helpers_of_a_module_stand() {
    require_tsgo!();
    // The module's first emitted line is `function $tt_show`, which no
    // source text owns: TypeScript inserts the import there, at the source
    // point the helpers stand for.
    let source = "variant V { A, B }\n\
declare const v: V;\n\
export const n = match (v) { A => 1, B => 2 };\n\
export const d = help;\n";
    let dir = project(&[
        ("src/util.ts", "export function helperFn(n: number): number { return n; }\n"),
        ("src/main.tt", source),
    ]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut project = open_service(&file);
    let mut at = utf16_position(source, "help;");
    at.character += "help".len() as u32;
    let answer = project.completion(&file, at, false).unwrap();
    assert!(
        answer.items.iter().any(|item| item.label == "helperFn"),
        "helperFn not offered"
    );
    let detail = project
        .completion_resolve(&file, at, "helperFn", answer.probe)
        .unwrap()
        .expect("resolved");
    let start = ttc::engine::Position {
        line: 0,
        character: 0,
    };
    assert_eq!(
        detail.additional_edits,
        vec![ttc::engine::TextEdit {
            range: ttc::engine::Range { start, end: start },
            new_text: "import { helperFn } from \"./util\";\n\n".to_string(),
        }]
    );
}

#[test]
fn tt_text_left_as_written_keeps_its_projection_unread() {
    require_tsgo!();
    // `try g()` without its `;` rolls back to passthrough text: TypeScript
    // would read tt syntax as its own, so neither its diagnostics nor a
    // restatement of ttc's own reach the editor.
    let source = "const a: number = \"x\";\nfunction f() { try g() }\nexport {};\n";
    let dir = project(&[("src/raw.tt", source)]);
    let file = dir.join("src/raw.tt").canonicalize().unwrap();
    let mut project = open_service(&file);
    assert_eq!(listed(&project.service_diagnostics(&file).unwrap()), vec![]);
    assert_eq!(project.service_restates(&file).unwrap(), vec![]);
}

/// The source with `@@` removed, and where it stood.
fn at_cursor(marked: &str) -> (String, ttc::engine::Position) {
    let source = marked.replacen("@@", "", 1);
    let position = utf16_position(marked, "@@");
    (source, position)
}

#[test]
fn an_unfinished_tt_value_answers_signature_help_and_completion() {
    require_tsgo!();
    let parse = "declare function parse(t: string, radix?: number): { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\n";
    let cases = [
        ("value", format!("{parse}export function g() {{\n  const n = try parse(\"1\", @@\n}}\n"), 1),
        ("operand", format!("{parse}export function g() {{\n  const k = 1 + try parse(\"1\", @@\n}}\n"), 1),
        ("name", format!("{parse}export function g() {{\n  const q = try parse(nu@@\n}}\n"), 0),
        ("result", format!("{parse}export const r = result {{ const q = try parse(\"1\", @@\n}};\n"), 1),
        (
            "arm",
            format!(
                "{parse}variant V {{ A(n: number), B }}\ndeclare const v: V;\nexport const m = match (v) {{ B => 0, A(n) => parse(\"x\", @@\n}};\n"
            ),
            1,
        ),
    ];
    let files: Vec<(String, String, ttc::engine::Position, u32)> = cases
        .iter()
        .map(|(name, marked, parameter)| {
            let (source, position) = at_cursor(marked);
            (format!("src/{name}.tt"), source, position, *parameter)
        })
        .collect();
    let dir = project(
        &files
            .iter()
            .map(|(name, source, ..)| (name.as_str(), source.as_str()))
            .collect::<Vec<_>>(),
    );
    for (name, _, position, parameter) in &files {
        let file = dir.join(name).canonicalize().unwrap();
        let mut project = open_service(&file);
        let help = project
            .signature_help(&file, *position)
            .unwrap()
            .unwrap_or_else(|| panic!("{name}: no signature help"));
        assert!(
            help.signatures[help.active_signature as usize]
                .label
                .starts_with("parse(t: string, radix?: number)"),
            "{name}: {help:?}"
        );
        assert_eq!(help.active_parameter, *parameter, "{name}");
        let items = project.completion(&file, *position, false).unwrap().items;
        assert!(
            items.iter().any(|item| item.label == "Math"),
            "{name}: {} items",
            items.len()
        );
    }
}

#[test]
fn an_auto_import_resolves_through_a_probe() {
    require_tsgo!();
    // The last arm's body is copied without the space the cursor stands
    // after, so the answer comes from a probe — and its import edit is
    // mapped back through that probe.
    let (source, position) = at_cursor(
        "variant V { A(n: number), B }\n\
declare const v: V;\n\
declare function parse(t: string, n: number): number;\n\
export const m = match (v) { B => 0, A(n) => parse(\"x\", @@\n};\n",
    );
    let dir = project(&[
        ("src/util.ts", "export function helperFn(n: number): number { return n; }\n"),
        ("src/main.tt", &source),
    ]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut project = open_service(&file);
    let answer = project.completion(&file, position, false).unwrap();
    assert!(answer.probe.is_some(), "the answer came from a probe");
    assert!(
        answer.items.iter().any(|item| item.label == "helperFn"),
        "helperFn not offered"
    );
    let detail = project
        .completion_resolve(&file, position, "helperFn", answer.probe)
        .unwrap()
        .expect("resolved");
    let start = ttc::engine::Position {
        line: 0,
        character: 0,
    };
    assert_eq!(
        detail.additional_edits,
        vec![ttc::engine::TextEdit {
            range: ttc::engine::Range { start, end: start },
            new_text: "import { helperFn } from \"./util\";\n\n".to_string(),
        }]
    );
}
