#[test]
fn an_unfinished_if_let_leaves_the_rest_of_the_file_served() {
    require_tsgo!();
    let decl = "import type { TOption } from \"@tt/std\";\n\
declare function find(id: string): TOption<{ name: string }>;\n";
    for head in [
        "if let Some(value: w) = find(id)",
        "if let Some(",
        "if let Some(v) = find(id.)",
        "if let Some(v) =",
    ] {
        let source = format!(
            "{decl}export function f(id: string) {{\n  {head}\n  const b = find(id);\n  return b.kind;\n}}\n\
export function g(n: number) {{ return n.toFixed(); }}\n"
        );
        let dir = project(&[("src/main.tt", &source)]);
        let file = dir.join("src/main.tt").canonicalize().unwrap();
        let mut project = open_service(&file);

        let later = utf16_position(&source, "toFixed()");
        let completion = project
            .completion(&file, ttc::engine::Position { character: later.character + 3, ..later }, true)
            .unwrap();
        assert!(
            completion.items.iter().any(|item| item.label == "toFixed"),
            "{source}"
        );
        let hover = project
            .hover(&file, utf16_position(&source, "kind;"))
            .unwrap()
            .expect("hover after the unfinished if let");
        assert!(hover.signature.contains("kind"), "{source}");
        let names: Vec<_> = project
            .document_symbols(&file)
            .unwrap()
            .into_iter()
            .map(|symbol| symbol.name)
            .collect();
        assert!(names.contains(&"g".to_string()), "{source}: {names:?}");
        let diagnostics = listed(&project.service_diagnostics(&file).unwrap());
        assert!(
            diagnostics.iter().all(|d| d.3 != "'}' expected."),
            "{source}: {diagnostics:?}"
        );
    }
}

#[test]
fn the_operand_of_an_unfinished_if_let_is_served() {
    require_tsgo!();
    let (source, position) = at_cursor(
        "import type { TOption } from \"@tt/std\";\n\
declare function find(id: string): TOption<{ name: string }>;\n\
export function f(id: string) {\n  if let Some(v) = find(id.@@)\n  return 1;\n}\n",
    );
    let dir = project(&[("src/main.tt", &source)]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut project = open_service(&file);
    let completion = project.completion(&file, position, true).unwrap();
    assert!(
        completion.items.iter().any(|item| item.label == "charAt"),
        "{:?}",
        completion.items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
    let help = project
        .signature_help(&file, position)
        .unwrap()
        .expect("signature help in the operand");
    assert_eq!(
        help.signatures[0].label,
        "find(id: string): TOption<{ name: string; }>"
    );
    assert_eq!(
        listed(&project.service_diagnostics(&file).unwrap()),
        vec![(3, 27, 1003, "Identifier expected.".to_string())]
    );
}

#[test]
fn signature_help_answers_for_the_source_call_around_generated_calls() {
    require_tsgo!();
    let decl = "const half = (n: number) => n / 2;\n\
const obj = { twice(n: number) { return n * 2; } };\n";
    let cases: [(&str, Option<(&str, u32)>); 7] = [
        ("console.log(m |> ha@@lf, m);", Some(("log(...data: any[]): void", 0))),
        ("console.log(m |> half@@);", Some(("log(...data: any[]): void", 0))),
        (
            "Math.max(1, m |> half |> Str@@ing);",
            Some(("max(...values: number[]): number", 0)),
        ),
        ("half(m |> ha@@lf);", Some(("half(n: number): number", 0))),
        ("const x = 4 |> obj.tw@@ice;", None),
        ("const g = flow |> half |> Str@@ing;", None),
        ("const y = m |> (v => v@@ + 1);", None),
    ];
    for (statement, expected) in cases {
        let (source, position) = at_cursor(&format!(
            "{decl}export function f(m: number) {{\n  {statement}\n}}\n"
        ));
        let dir = project(&[("src/main.tt", &source)]);
        let file = dir.join("src/main.tt").canonicalize().unwrap();
        let mut project = open_service(&file);
        let help = project.signature_help(&file, position).unwrap();
        let answer = help.map(|help| {
            (
                help.signatures[help.active_signature as usize].label.clone(),
                help.active_parameter,
            )
        });
        assert_eq!(
            answer,
            expected.map(|(label, parameter)| (label.to_string(), parameter)),
            "{statement}"
        );
    }
}

#[test]
fn signature_help_names_a_stored_callee_as_the_source_call_does() {
    require_tsgo!();
    let head = "class Cls { constructor(a: number, b: string) {} }\n\
declare function two(a: number, b: string): number;\n";
    let arm = "match (s) { A => 1, B(x) => x }";
    for (call, equivalent) in [
        (format!("two({arm}, \"q@@\")"), "two(1, \"q@@\")"),
        (format!("two({arm}, @@)"), "two(1, @@)"),
        (format!("new Cls({arm}, @@)"), "new Cls(1, @@)"),
        (format!("two<number>({arm}, \"q@@\")"), "two<number>(1, \"q@@\")"),
        (format!("(two)({arm}, \"q@@\")"), "(two)(1, \"q@@\")"),
        (format!("`${{two({arm}, \"q@@\")}}`"), "`${two(1, \"q@@\")}`"),
        (format!("new Cls(1, two({arm}, \"q@@\"))"), "new Cls(1, two(1, \"q@@\"))"),
    ] {
        let (tt, at_tt) = at_cursor(&format!(
            "variant S {{ A, B(x: number) }}\n{head}export function f(s: S) {{\n  return {call};\n}}\n"
        ));
        let (ts, at_ts) = at_cursor(&format!(
            "type S = {{ kind: \"A\" }} | {{ kind: \"B\"; x: number }};\n{head}export function f(s: S) {{\n  return {equivalent};\n}}\n"
        ));
        let dir = project(&[("src/main.tt", &tt), ("src/equivalent.ts", &ts)]);
        let tt_file = dir.join("src/main.tt").canonicalize().unwrap();
        let ts_file = dir.join("src/equivalent.ts").canonicalize().unwrap();
        let mut project = open_service(&tt_file);
        project.open_document(ts_file.clone(), ts.clone());
        let answer = |help: Option<ttc::engine::SignatureHelp>| {
            help.map(|help| {
                (
                    help.signatures[help.active_signature as usize].label.clone(),
                    help.active_parameter,
                )
            })
        };
        let expected = answer(project.signature_help(&ts_file, at_ts).unwrap());
        assert!(expected.is_some(), "{equivalent}");
        assert_eq!(
            answer(project.signature_help(&tt_file, at_tt).unwrap()),
            expected,
            "{call}"
        );
    }
}

#[test]
fn signature_help_in_a_try_is_the_same_while_the_file_has_a_syntax_error() {
    require_tsgo!();
    let decl = "import type { TResult } from \"@tt/std\";\n\
declare function getUser(id: string): TResult<{ name: string }, string>;\n";
    let get_user = Some("getUser(id: string): TResult<{ name: string; }, string>".to_string());
    let cases = [
        (
            "export function f(id: string): TResult<number, string> {\n  const q = try getUser(id)@@\n  return { kind: \"Ok\", value: q.name.length };\n}\n",
            None,
        ),
        (
            "export function f(id: string): TResult<number, string> {\n  const q = try getUser(@@id);\n  return { kind: \"Ok\", value: q.name.length };\n}\n",
            get_user.clone(),
        ),
        (
            "export function g(id: string) {\n  return result { const v = try getUser(id)@@; v.name.length };\n}\n",
            None,
        ),
        (
            "export function g(id: string) {\n  return result { const v = try getUser(@@id); v.name.length };\n}\n",
            get_user,
        ),
    ];
    for (body, expected) in cases {
        for broken in ["", "const r3 = (;\n"] {
            let (source, position) = at_cursor(&format!("{decl}{body}{broken}"));
            let dir = project(&[("src/main.tt", &source)]);
            let file = dir.join("src/main.tt").canonicalize().unwrap();
            let mut project = open_service(&file);
            let help = project.signature_help(&file, position).unwrap();
            assert_eq!(
                help.map(|help| help.signatures[help.active_signature as usize].label.clone()),
                expected,
                "{source}"
            );
        }
    }
}

#[test]
fn the_body_of_an_arm_written_up_to_its_arrow_completes_expressions() {
    require_tsgo!();
    let decl = "export variant Shape { Circle(radius: number), Rect(width: number), Point }\n\
const limit = 1;\n";
    for arms in [
        "Circle(radius) => radius,\n    Rect(width) => @@",
        "Circle(radius) => radius,\n    Rect(width) => @@,\n    _ => 0,",
        "Circle(radius) => radius,\n    Rect(width) if width > limit => @@",
    ] {
        let (source, position) = at_cursor(&format!(
            "{decl}export function g(s: Shape) {{\n  return match (s) {{\n    {arms}\n  }};\n}}\n"
        ));
        let dir = project(&[("src/main.tt", &source)]);
        let file = dir.join("src/main.tt").canonicalize().unwrap();
        assert!(
            ttc::engine::tt_completions_at(&file, &source, position).is_empty(),
            "{source}"
        );
        let mut project = open_service(&file);
        let labels: Vec<_> = project
            .completion(&file, position, false)
            .unwrap()
            .items
            .into_iter()
            .map(|item| item.label)
            .collect();
        for name in ["width", "limit", "s", "Math"] {
            assert!(labels.iter().any(|label| label == name), "{source}: {name}");
        }
    }
}

#[test]
fn a_payload_list_whose_bindings_are_all_unused_is_faded_whole() {
    require_tsgo!();
    let source = "export variant S { A, B(x: number, y: string), C(v: S, w: number) }\n\
export class Failure extends Error { code = 1; }\n\
export function f(s: S, e: unknown): number {\n\
\x20 const a = match (s) {\n\
\x20   B(x, y) => 1,\n\
\x20   C(v: B(x, y), w) => w,\n\
\x20   C(v, w) => 2,\n\
\x20   A => 3,\n\
\x20 };\n\
\x20 if let B(x: p, y: q) = s { console.log(1); }\n\
\x20 const b = match (e) { is Failure { code, message } => 1, _ => 2 };\n\
\x20 return a + b;\n\
}\n";
    let dir = project(&[("src/main.tt", source)]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut project = open_service(&file);
    let diagnostics = project.service_diagnostics(&file).unwrap();
    let unused: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == 6198 || d.code == 6133)
        .map(|d| (d.range.start.line, utf16_slice(source, d.range), d.code, d.tags.clone()))
        .collect();
    use ttc::engine::ServiceTag::Unnecessary;
    assert_eq!(
        unused,
        vec![
            (4, "(x, y)", 6198, vec![Unnecessary]),
            (5, "(x, y)", 6198, vec![Unnecessary]),
            (6, "(v, w)", 6198, vec![Unnecessary]),
            (9, "(x: p, y: q)", 6198, vec![Unnecessary]),
            (10, "{ code, message }", 6198, vec![Unnecessary]),
        ],
        "{diagnostics:?}"
    );
}

#[test]
fn the_guard_of_an_arm_with_no_body_is_served() {
    require_tsgo!();
    let decl = "export variant Shape { Circle(radius: number), Rect(width: number), Point }\n\
const limit = 1;\n";
    for arms in [
        "Circle(radius) => radius,\n    Rect(width) if width > li@@",
        "Circle(radius) => radius,\n    Rect(width) if width > li@@,\n    _ => 0,",
        "Circle(radius) => radius,\n    Rect(width) if width > li@@ =>",
    ] {
        let (source, position) = at_cursor(&format!(
            "{decl}export function g(s: Shape) {{\n  return match (s) {{\n    {arms}\n  }};\n}}\n"
        ));
        let dir = project(&[("src/main.tt", &source)]);
        let file = dir.join("src/main.tt").canonicalize().unwrap();
        let mut project = open_service(&file);
        let labels: Vec<_> = project
            .completion(&file, position, false)
            .unwrap()
            .items
            .into_iter()
            .map(|item| item.label)
            .collect();
        for name in ["width", "limit", "s"] {
            assert!(labels.iter().any(|label| label == name), "{source}: {name}");
        }
        let guard = utf16_position(&source, "width >");
        let hover = project
            .hover(&file, guard)
            .unwrap()
            .expect("hover on the guard");
        assert_eq!(hover.signature, "const width: number", "{source}");
        let definitions = project.definition(&file, guard).unwrap();
        assert_eq!(
            definitions
                .iter()
                .map(|location| location.range.start)
                .collect::<Vec<_>>(),
            vec![utf16_position(&source, "width)")],
            "{source}"
        );
        let diagnostics = listed(&project.service_diagnostics(&file).unwrap());
        assert!(
            diagnostics.iter().all(|d| d.2 != 1005 && d.2 != 1128),
            "{source}: {diagnostics:?}"
        );
        let typed = typed_server(&dir, "src/main.tt", &source);
        let codes: Vec<_> = typed["result"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["code"].as_str().unwrap().to_string())
            .collect();
        assert!(
            codes.contains(&"missing-arm-body".to_string())
                && codes.iter().any(|code| code == "ts2304" || code == "ts2552"),
            "{source}: {typed}"
        );
    }
}


fn token_names(
    source: &str,
    tokens: &[ttc::engine::ClassifiedToken],
) -> Vec<(String, String)> {
    let lines: Vec<&str> = source.lines().collect();
    tokens
        .iter()
        .map(|token| {
            let line: Vec<u16> = lines[token.range.start.line as usize].encode_utf16().collect();
            let text = String::from_utf16(
                &line[token.range.start.character as usize..token.range.end.character as usize],
            )
            .unwrap();
            let mut name = token.token_type.clone();
            for modifier in &token.modifiers {
                name.push('.');
                name.push_str(modifier);
            }
            (text, name)
        })
        .collect()
}

#[test]
fn semantic_tokens_classify_the_source_as_typescript_does_with_tt_constructs_over_it() {
    require_tsgo!();
    let source = "variant Shape { Circle(radius: number), Point }\n\
export function area(s: Shape): number {\n\
  const scale = 2;\n\
  return match (s) {\n\
    Circle(radius) => radius * scale,\n\
    Point => Math.PI,\n\
  };\n\
}\n";
    let dir = project(&[("src/main.tt", source)]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut project = open_service(&file);
    let tokens = project.semantic_tokens(&file).unwrap();
    let named = token_names(source, &tokens);
    let expected = [
        ("variant", "keyword.declaration"),
        ("Shape", "enum"),
        ("Circle", "enumMember"),
        ("radius", "property"),
        ("Point", "enumMember"),
        ("area", "function.declaration"),
        ("s", "parameter.declaration"),
        ("Shape", "type.readonly"),
        ("scale", "variable.declaration.readonly.local"),
        ("match", "keyword"),
        ("s", "parameter"),
        ("Circle", "enumMember"),
        ("radius", "variable.declaration.readonly.local"),
        ("radius", "variable.readonly.local"),
        ("scale", "variable.readonly.local"),
        ("Point", "enumMember"),
        ("Math", "variable.defaultLibrary"),
        ("PI", "property.readonly.defaultLibrary"),
    ];
    assert_eq!(
        named,
        expected
            .iter()
            .map(|(text, name)| (text.to_string(), name.to_string()))
            .collect::<Vec<_>>()
    );
}

fn pattern_labels(project: &mut ttc::engine::Project, file: &Path, position: ttc::engine::Position) -> Vec<String> {
    project
        .pattern_completions(file, position)
        .unwrap()
        .expect("a pattern position")
        .into_iter()
        .map(|item| {
            format!(
                "{}{}",
                item.label,
                if item.covered { " (covered)" } else { "" }
            )
        })
        .collect()
}

#[test]
fn pattern_completion_offers_what_the_scrutinee_type_admits() {
    require_tsgo!();
    let head = "type Dir = \"north\" | \"south\";\n\
type K = { kind: \"Alpha\"; x: number } | { kind: \"Beta\" };\n\
variant Shape { Circle(radius: number), Point }\n\
export function f(d: Dir, k: K, n: 1 | 2 | 3, s: Shape, text: string) {\n";
    for (arm, expected) in [
        ("match (d) { \"north\" => 1, @@ }", &["\"north\" (covered)", "\"south\"", "_"][..]),
        ("match (d) { @@ }", &["\"north\"", "\"south\"", "_"][..]),
        ("match (n) { 1 => 1, @@ }", &["1 (covered)", "2", "3", "_"][..]),
        ("match (text) { \"a\" => 1, @@ }", &["_"][..]),
        ("match (k) { Alpha => 1, @@ }", &["Alpha (covered)", "Beta", "_"][..]),
        ("match (k) { Alpha => 1, Be@@ }", &["Alpha (covered)", "Beta", "_"][..]),
        ("match (k) { @@ }", &["Alpha", "Beta", "_"][..]),
        ("match (k) { Alpha if k.x > 0 => 1, @@ }", &["Alpha", "Beta", "_"][..]),
        ("match (s) { Circle(radius) => radius, @@ }", &["Circle (covered)", "Point", "_"][..]),
        ("match (k) { Alpha(@@) => 1, _ => 0 }", &["x"][..]),
        ("match (s) { Circle(@@) => 1, _ => 0 }", &["radius"][..]),
    ] {
        let (source, position) = at_cursor(&format!("{head}  const b = {arm};\n  return b;\n}}\n"));
        let dir = project(&[("src/main.tt", &source)]);
        let file = dir.join("src/main.tt").canonicalize().unwrap();
        let mut project = open_service(&file);
        assert_eq!(pattern_labels(&mut project, &file, position), expected, "{arm}");
    }
}

#[test]
fn completion_never_offers_the_cases_of_a_generated_switch() {
    require_tsgo!();
    let (source, position) = at_cursor(
        "type K = { kind: \"Alpha\"; x: number } | { kind: \"Beta\" };\n\
export function f(k: K) {\n  const b = match (k) { Alpha(x) => { @@ }, _ => 0 };\n  return b;\n}\n\
export function g(k: K) {\n  switch (k.kind) {\n    case \"Alpha\": break;\n    ##\n  }\n}\n",
    );
    let user = utf16_position(&source.replace("@@", ""), "##");
    let source = source.replace("##", "");
    let dir = project(&[("src/main.tt", &source)]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut project = open_service(&file);
    let generated = project.completion(&file, position, false).unwrap();
    assert!(
        generated.items.iter().all(|item| !item.label.starts_with("case ")),
        "{:?}",
        generated.items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
    let written = project.completion(&file, user, false).unwrap();
    assert!(
        written.items.iter().any(|item| item.label == "case \"Beta\": ..."),
        "{:?}",
        written.items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
}

#[test]
fn a_module_specifier_completes_the_sibling_tt_modules_as_tt_imports_them() {
    require_tsgo!();
    for (marked, typed) in [
        ("import { kk } from \"./@@\";\nexport const z = kk;\n", ""),
        ("import { kk } from \"./sh@@\";\nexport const z = kk;\n", "sh"),
        ("export * from \"./@@\";\n", ""),
        ("export const m = import(\"./@@\");\n", ""),
    ] {
        let (source, position) = at_cursor(marked);
        let dir = project(&[
            ("src/main.tt", &source),
            ("src/shapes.tt", "export const kk = 1;\n"),
            ("src/view.ttx", "export const vv = 1;\n"),
            ("src/lib.ts", "export const ll = 1;\n"),
        ]);
        let file = dir.join("src/main.tt").canonicalize().unwrap();
        let mut project = open_service(&file);
        let items = project.completion(&file, position, false).unwrap().items;
        let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
        for expected in ["lib", "shapes.tt", "view.ttx"] {
            assert!(labels.contains(&expected), "{marked}: {labels:?}");
        }
        assert!(!labels.contains(&"main.tt"), "{marked}: {labels:?}");
        let shapes = items.iter().find(|item| item.label == "shapes.tt").unwrap();
        assert_eq!(shapes.kind, Some(ttc::engine::CompletionItemKind::File));
        let start = ttc::engine::Position {
            character: position.character - typed.len() as u32,
            ..position
        };
        assert_eq!(
            shapes.range,
            Some(ttc::engine::Range {
                start,
                end: position
            }),
            "{marked}"
        );
    }

    let (source, position) = at_cursor("import { x } from \"@tt/@@\";\n");
    let dir = project(&[("src/main.tt", &source), ("src/shapes.tt", "export const kk = 1;\n")]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut project = open_service(&file);
    let items = project.completion(&file, position, false).unwrap().items;
    assert!(items.iter().all(|item| !item.label.ends_with(".tt")));
}

#[test]
fn a_builtin_tag_or_field_goes_to_its_declaration_in_the_standard_library() {
    require_tsgo!();
    let source = "import type { TResult, TOption } from \"@tt/std\";\n\
declare const r: TResult<number, string>;\n\
declare const o: TOption<number>;\n\
export const a = match (r) { Ok(value) => value, Err(error) => error.length };\n\
export const b = match (o) { Some(value: x) => x, None => 0 };\n\
export function c() {\n  const Err(error) = r else { return 0; };\n  return error;\n}\n\
export function d() {\n  if let Some(value) = o { return value; }\n  return 0;\n}\n";
    let dir = project(&[("src/main.tt", source)]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut project = open_service(&file);
    let std = dir.join("node_modules/@tt/std");
    let declared = |module: &'static str, needle: &'static str| (module, needle);
    let at = |needle: &str, nth: usize| {
        let at = source.match_indices(needle).nth(nth).unwrap().0;
        let line_start = source[..at].rfind('\n').map_or(0, |newline| newline + 1);
        ttc::engine::Position {
            line: source[..at].matches('\n').count() as u32,
            character: source[line_start..at].encode_utf16().count() as u32 + 1,
        }
    };
    for (position, expected) in [
        (at("Ok(", 0), declared("result.ts", "Ok = ")),
        (at("Err(", 0), declared("result.ts", "Err = ")),
        (at("Some(", 0), declared("option.ts", "Some = ")),
        (at("None =>", 0), declared("option.ts", "None = ")),
        (at("value)", 0), declared("result.ts", "value: T }")),
        (at("error)", 0), declared("result.ts", "error: E }")),
        (at("value: x", 0), declared("option.ts", "value: T }")),
        (at("error)", 1), declared("result.ts", "error: E }")),
        (at("value)", 1), declared("option.ts", "value: T }")),
    ] {
        let found = project.definition(&file, position).unwrap();
        let (module, needle) = expected;
        let text = std::fs::read_to_string(std.join(module)).unwrap();
        let expected = (module.to_string(), utf16_position(&text, needle));
        let targets: Vec<(String, ttc::engine::Position)> = found
            .iter()
            .map(|location| {
                (
                    location.path.file_name().unwrap().to_string_lossy().into_owned(),
                    location.range.start,
                )
            })
            .collect();
        assert_eq!(targets, vec![expected], "{position:?}");
    }
}

#[test]
fn prepare_rename_answers_the_range_a_rename_would_replace_or_refuses_as_it_would() {
    require_tsgo!();
    let source = "variant Shape { Circle(radius: number), Point }\n\
export function f(s: Shape, scale: number) {\n\
  console.log(scale);\n\
  return match (s) { Circle(radius) => radius * scale, Point => 0 };\n\
}\n";
    let dir = project(&[("src/main.tt", source)]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut workspace = ttc::engine::Workspace::new(ttc::engine::Engine::new(None));
    workspace.open_document(&file, source.to_string()).unwrap();
    let inside = |needle: &str, nth: usize| {
        let at = source.match_indices(needle).nth(nth).unwrap().0 + 1;
        let line_start = source[..at].rfind('\n').map_or(0, |newline| newline + 1);
        ttc::engine::Position {
            line: source[..at].matches('\n').count() as u32,
            character: source[line_start..at].encode_utf16().count() as u32,
        }
    };
    let covered = |range: ttc::engine::Range| {
        let line = source.lines().nth(range.start.line as usize).unwrap();
        line[range.start.character as usize..range.end.character as usize].to_string()
    };
    for (needle, nth, expected) in [
        ("scale", 0, Some("scale")),
        ("scale", 1, Some("scale")),
        ("radius)", 0, Some("radius")),
        ("radius *", 0, Some("radius")),
        ("log", 0, None),
    ] {
        let prepared = match workspace.prepare_rename(&file, inside(needle, nth)).unwrap() {
            ttc::engine::PrepareRename::Range(range) => Ok(covered(range)),
            ttc::engine::PrepareRename::Refused(reason) => Err(reason),
        };
        match expected {
            Some(name) => assert_eq!(prepared, Ok(name.to_string()), "{needle} #{nth}"),
            None => assert_eq!(
                prepared,
                Err(Some(
                    "You cannot rename elements that are defined in the standard TypeScript library."
                        .to_string()
                )),
                "{needle} #{nth}"
            ),
        }
        let renamed = workspace.rename(&file, inside(needle, nth)).unwrap();
        assert_eq!(renamed.is_some(), expected.is_some(), "{needle} #{nth}");
    }
}

#[test]
fn an_auto_import_entry_names_the_module_it_imports_from() {
    require_tsgo!();
    let (source, position) = at_cursor("export const z = kkVa@@;\n");
    let dir = project(&[
        ("src/main.tt", &source),
        ("src/shapes.tt", "export const kkValue = 1;\n"),
        ("src/lib.ts", "export const kkValueLib = 1;\n"),
    ]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut project = open_service(&file);
    let completion = project.completion(&file, position, false).unwrap();
    let described = |label: &str| {
        completion
            .items
            .iter()
            .find(|item| item.label == label)
            .and_then(|item| item.description.clone())
    };
    assert_eq!(described("kkValue").as_deref(), Some("./shapes.tt"));
    assert_eq!(described("kkValueLib").as_deref(), Some("./lib"));
    let source = completion
        .items
        .iter()
        .find(|item| item.label == "kkValue")
        .and_then(|item| item.source.clone());
    let detail = project
        .completion_resolve(&file, position, "kkValue", source.as_deref(), completion.probe)
        .unwrap()
        .expect("the entry resolves");
    assert_eq!(
        detail.additional_edits[0].new_text,
        "import { kkValue } from \"./shapes.tt\";\n\n"
    );
}

#[test]
fn entries_of_one_name_from_two_modules_each_import_their_own() {
    require_tsgo!();
    let (source, position) = at_cursor("export const z = kkVa@@;\n");
    let dir = project(&[
        ("src/main.tt", &source),
        ("src/shapes.tt", "export const kkValue = 1;\n"),
        ("src/lib.ts", "export const kkValue = 2;\n"),
    ]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut project = open_service(&file);
    let completion = project.completion(&file, position, false).unwrap();
    let entries: Vec<_> = completion
        .items
        .iter()
        .filter(|item| item.label == "kkValue")
        .cloned()
        .collect();
    let mut modules: Vec<_> = entries
        .iter()
        .map(|item| item.description.clone().unwrap_or_default())
        .collect();
    modules.sort();
    assert_eq!(modules, ["./lib", "./shapes.tt"]);
    for entry in entries.iter().chain(entries.iter().rev()) {
        let detail = project
            .completion_resolve(
                &file,
                position,
                &entry.label,
                entry.source.as_deref(),
                completion.probe,
            )
            .unwrap()
            .expect("the entry resolves");
        let module = entry.description.as_deref().unwrap();
        assert_eq!(
            detail.additional_edits[0].new_text,
            format!("import {{ kkValue }} from \"{module}\";\n\n"),
        );
        assert_eq!(detail.signature, format!("Add import from \"{module}\""));
    }
}

#[test]
fn a_triggered_completion_answers_as_typescript_answers_its_twin() {
    require_tsgo!();
    let body = "import { kkTt } from \"./@@\";\n";
    let rest = "declare global { namespace JSX { interface IntrinsicElements { main: {} } } }\n\
/** @@@ */\n\
export function f(n: number) { return n; }\n\
class K { #secret = 1; read() { return this.#@@; } }\n\
const q = @@1;\n\
const s = '@@';\n\
const t = 2 <@@ 3;\n\
export const v = <main><@@</main>;\n";
    let marked = format!("{body}{rest}");
    let triggers = ["/", "@", "#", " ", "'", "<", "<"];
    let dir = project(&[
        ("src/lib.ts", "export const kkLib = 1;\n"),
        ("src/shapes.tt", "export const kkTt = 2;\n"),
    ]);
    let tt_file = dir.join("src/main.ttx");
    let ts_file = dir.join("src/equivalent.tsx");
    let cleaned = marked.replace("@@", "");
    std::fs::write(&tt_file, &cleaned).unwrap();
    std::fs::write(&ts_file, &cleaned).unwrap();
    let tt_file = tt_file.canonicalize().unwrap();
    let ts_file = ts_file.canonicalize().unwrap();
    let mut project = open_service(&tt_file);
    project.open_document(ts_file.clone(), cleaned.clone());
    let mut rest_of = marked.as_str();
    let mut consumed = 0;
    for trigger in triggers {
        let found = rest_of.find("@@").unwrap();
        let at = consumed + found;
        let before = &cleaned[..at];
        let position = ttc::engine::Position {
            line: before.matches('\n').count() as u32,
            character: before[before.rfind('\n').map_or(0, |n| n + 1)..].encode_utf16().count() as u32,
        };
        consumed = at;
        rest_of = &rest_of[found + 2..];
        let labels = |project: &mut ttc::engine::Project, file: &Path| {
            let mut labels: Vec<String> = project
                .triggered_completion(file, position, false, Some(trigger))
                .unwrap()
                .items
                .into_iter()
                .map(|item| item.label)
                .collect();
            labels.sort();
            labels
        };
        let mut expected = labels(&mut project, &ts_file);
        let mut answered = labels(&mut project, &tt_file);
        if trigger == "/" {
            assert!(answered.contains(&"shapes.tt".to_string()), "{answered:?}");
            assert!(expected.contains(&"lib".to_string()), "{expected:?}");
            let siblings = ["equivalent", "main.ttx", "shapes.tt"];
            answered.retain(|label| !siblings.contains(&label.as_str()));
            expected.retain(|label| !siblings.contains(&label.as_str()));
        }
        assert_eq!(answered, expected, "{trigger:?} at {position:?}");
        match trigger {
            " " | "'" => assert!(answered.is_empty(), "{trigger:?}: {answered:?}"),
            "@" => assert!(answered.contains(&"@param".to_string()), "{answered:?}"),
            "#" => assert!(answered.contains(&"#secret".to_string()), "{answered:?}"),
            _ => {}
        }
    }
}

#[test]
fn a_pipeline_step_being_typed_answers_as_its_typescript_equivalent_does() {
    require_tsgo!();
    let head = "const half = (n: number) => n / 2;\n\
const obj = { twice(n: number) { return n * 2; } };\n";
    for (step, equivalent) in [
        ("4 |> o@@", "o@@(4)"),
        ("4 |> obj@@", "obj@@(4)"),
        ("4 |> obj.tw@@", "obj.tw@@(4)"),
        ("4 |> obj.twice@@", "obj.twice@@(4)"),
    ] {
        let (tt, at_tt) = at_cursor(&format!("{head}export const a = {step}\nexport const z = half(2);\n"));
        let (ts, at_ts) =
            at_cursor(&format!("{head}export const a = {equivalent}\nexport const z = half(2);\n"));
        let dir = project(&[("src/main.tt", &tt), ("src/equivalent.ts", &ts)]);
        let tt_file = dir.join("src/main.tt").canonicalize().unwrap();
        let ts_file = dir.join("src/equivalent.ts").canonicalize().unwrap();
        let mut project = open_service(&tt_file);
        project.open_document(ts_file.clone(), ts.clone());
        let before = |at: ttc::engine::Position| ttc::engine::Position {
            character: at.character - 1,
            ..at
        };
        let hover = |project: &mut ttc::engine::Project, file: &Path, at| {
            project
                .hover(file, before(at))
                .unwrap()
                .map(|info| info.signature)
        };
        assert_eq!(
            hover(&mut project, &tt_file, at_tt),
            hover(&mut project, &ts_file, at_ts),
            "{step}"
        );
        let labels = |project: &mut ttc::engine::Project, file: &Path, at| {
            let mut labels: Vec<String> = project
                .completion(file, at, false)
                .unwrap()
                .items
                .into_iter()
                .map(|item| item.label)
                .collect();
            labels.sort();
            labels
        };
        assert_eq!(
            labels(&mut project, &tt_file, at_tt),
            labels(&mut project, &ts_file, at_ts),
            "{step}"
        );
    }
}

#[test]
fn installed_mapper_keeps_direct_and_declaration_map_targets_in_source_coordinates() {
    require_tsgo!();
    let api = "export variant Shape { Point, Circle(radius: number) }\nexport function work(n: number): number { return n; }\n";
    let main = "import { work } from \"./contract.js\";\nimport { work as original } from \"./api.tt\";\nwork(1);\noriginal(2);\n";
    let dir = project(&[
        ("src/api.tt", api),
        ("src/main.tt", main),
        ("src/contract.d.ts", "export declare function work(n: number): number;\n//# sourceMappingURL=contract.d.ts.map\n"),
        ("src/contract.d.ts.map", r#"{"version":3,"file":"contract.d.ts","sourceRoot":"","sources":["api.tt"],"names":[],"mappings":"wBACgB,IAAI"}"#),
    ]);
    common::installed_mapper::install(&dir);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let target = dir.join("src/api.tt").canonicalize().unwrap();
    let mut service = open_service(&file);
    for call in ["original(2)", "work(1)"] {
        let found = service.definition(&file, utf16_position(main, call)).unwrap();
        assert_eq!(found.len(), 1, "{call}: {found:?}");
        assert_eq!(found[0].path, target);
        assert_eq!(found[0].range.start, utf16_position(api, "work(n"));
        assert_eq!(found[0].range.end.character - found[0].range.start.character, 4);
    }
}

#[test]
fn installed_mapper_preserves_editor_ranges_after_lowering() {
    require_tsgo!();
    let source = "export variant Shape { Point, Circle(radius: number) }\nexport const value = 1;\nvalue.toUpperCase();\nexport function sum(n: number) { return n + value; }\nsum(1);\n";
    let dir = project(&[("src/main.tt", source)]);
    common::installed_mapper::install(&dir);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut service = open_service(&file);
    let called = utf16_position(source, "value.toUpperCase");
    let hover = service.hover(&file, called).unwrap().expect("number hover");
    assert_eq!(hover.range.start, called);
    assert!(hover.signature.contains("value"), "{hover:?}");
    let errors = service.service_diagnostics(&file).unwrap();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].range.start, utf16_position(source, "toUpperCase"));
    let edits = service.rename(&file, called).unwrap().expect("rename number");
    assert_eq!(edits.len(), 3, "{edits:?}");
    for edit in edits { assert_eq!(edit.location.range.end.character - edit.location.range.start.character, 5); }
    let mut call = utf16_position(source, "sum(1)");
    call.character += 4;
    assert!(service.signature_help(&file, call).unwrap().is_some());
    assert_eq!(service.hover(&file, called).unwrap().unwrap().range.start, called);
    let symbols = service.document_symbols(&file).unwrap();
    assert!(symbols.iter().any(|symbol| symbol.name == "sum" && symbol.selection_range.start.line == 3), "{symbols:?}");
    let tokens = service.semantic_tokens(&file).unwrap();
    assert!(tokens.iter().any(|token| token.range.start == called), "{tokens:?}");
}

#[test]
fn installed_mapper_keeps_shared_bindings_as_one_editable_symbol() {
    require_tsgo!();
    let source = "export variant Token { A(value: number), B(value: number), End }\ndeclare const t: Token;\nexport const result = match (t) { A(value: n) | B(value: n) => n, _ => 0 };\n";
    let dir = project(&[("src/main.tt", source)]);
    common::installed_mapper::install(&dir);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut service = open_service(&file);
    let first = utf16_position(source, "n) |");
    let second = utf16_position(source, "n) =>");
    let body = utf16_position(source, "n, _");
    for query in [first, second, body] {
        let definitions = service.definition(&file, query).unwrap();
        let starts: Vec<_> = definitions.iter().map(|place| place.range.start).collect();
        assert!(starts.contains(&first) && starts.contains(&second), "{query:?}: {definitions:?}");
        let references = service.references(&file, query).unwrap();
        for expected in [first, second, body] {
            assert!(references.iter().any(|reference| reference.location.range.start == expected), "{query:?}: {references:?}");
        }
        let edits = service.rename(&file, query).unwrap().expect("shared binding can be renamed");
        assert_eq!(edits.len(), 3, "{edits:?}");
        for expected in [first, second, body] {
            assert!(edits.iter().any(|edit| edit.location.range.start == expected), "{edits:?}");
        }
    }
}

#[test]
fn installed_mapper_refreshes_context_without_changing_the_tt_source() {
    require_tsgo!();
    let source = "import type { Input } from './host';\nexport const read = (x: Input) => match (true) { true => x.value, false => x.value };\n";
    let dir = project(&[("src/main.tt", source), ("src/host.ts", "export type Input = { value: string };\n")]);
    common::installed_mapper::install(&dir);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let host = dir.join("src/host.ts").canonicalize().unwrap();
    let mut service = open_service(&file);
    let at = utf16_position(source, "read =");
    let before_code = service.update(std::slice::from_ref(&file)).unwrap().files()[0].code().to_string();
    let before = service.hover(&file, at).unwrap().expect("contextual string");
    assert!(before.signature.contains("string"), "{before:?}");
    service.open_document(host, "export type Input = { value: number };\n".into());
    let after_code = service.update(std::slice::from_ref(&file)).unwrap().files()[0].code().to_string();
    assert_ne!(after_code, before_code, "the contextual projection must change");
    let after = service.hover(&file, at).unwrap().expect("contextual number");
    assert!(after.signature.contains("number"), "{after:?}");
    assert_eq!(after.range, before.range);
}

#[test]
fn installed_mapper_keeps_ttx_source_roots_and_unsaved_sessions_separate() {
    require_tsgo!();
    let source = "export variant View { Empty, Count(value: number) }\nexport function render(n: number) { return <span>{n}</span>; }\n";
    let main = "import { render } from './contract.js';\nimport { render as direct } from './source files/ui.ttx';\nrender(1);\ndirect(2);\n";
    let dir = project(&[
        ("src/main.tt", main),
        ("src/contract.d.ts", "export declare function render(n: number): unknown;\n//# sourceMappingURL=contract.d.ts.map\n"),
        ("src/contract.d.ts.map", r#"{"version":3,"file":"contract.d.ts","sourceRoot":"source files/","sources":["ui.ttx"],"names":[],"mappings":"wBACgB,MAAM"}"#),
    ]);
    fs::create_dir_all(dir.join("src/source files")).unwrap();
    write(&dir, "src/source files/ui.ttx", source);
    common::installed_mapper::install(&dir);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let target = dir.join("src/source files/ui.ttx").canonicalize().unwrap();
    let mut first = open_service(&file);
    for call in ["render(1)", "direct(2)"] {
        let found = first.definition(&file, utf16_position(main, call)).unwrap();
        assert_eq!(found.len(), 1, "{call}: {found:?}");
        assert_eq!(found[0].path, target);
        assert_eq!(found[0].range.start, utf16_position(source, "render(n"));
    }
    let mut second = open_service(&file);
    let first_source = format!("\n{source}");
    let second_source = format!("\n\n{source}");
    first.open_document(target.clone(), first_source.clone());
    second.open_document(target, second_source.clone());
    for select_first in [true, false, true] {
        let (service, text) = if select_first { (&mut first, &first_source) } else { (&mut second, &second_source) };
        let found = service.definition(&file, utf16_position(main, "direct(2)")).unwrap();
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].range.start, utf16_position(text, "render(n"));
    }
}

#[test]
fn installed_mapper_preserves_the_configured_auto_import_name() {
    require_tsgo!();
    let dir = project(&[("src/fooBar.tt", "export default function () {}\n"), ("src/main.tt", "fooB\n")]);
    common::installed_mapper::install(&dir);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut service = open_service(&file);
    let answer = service.completion(&file, ttc::engine::Position { line: 0, character: 4 }, false).unwrap();
    assert!(answer.items.iter().any(|item| item.label == "fooBar"), "{:?}", answer.items);
    assert!(!answer.items.iter().any(|item| item.label == "fooBarTt"));
}

#[test]
fn installed_mapper_without_exchange_support_reports_an_explicit_error() {
    require_tsgo!();
    let dir = project(&[("src/main.tt", "export const value = 1;\nvalue;\n")]);
    common::installed_mapper::install(&dir);
    let package = dir.join("node_modules/@openload28/tt-lang");
    // Simulate an older mapper that transforms normally but cannot acknowledge
    // contextual projections. Its otherwise valid reply must not be accepted.
    fs::write(package.join("old.cjs"), format!(
        "delete process.env.TTC_SERVICE_PROJECTIONS;\nconst child = require('node:child_process').spawn({}, ['--content-mapper'], {{stdio: 'inherit'}});\nchild.on('exit', code => process.exit(code ?? 1));\n",
        serde_json::to_string(env!("CARGO_BIN_EXE_ttc")).unwrap()
    )).unwrap();
    let manifest = package.join("package.json");
    let mut value: serde_json::Value = serde_json::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();
    value["typescript"]["contentMapper"]["exec"] = serde_json::json!(["node", package.join("old.cjs")]);
    fs::write(manifest, value.to_string()).unwrap();
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut service = open_service(&file);
    let error = service.definition(&file, ttc::engine::Position { line: 1, character: 0 }).unwrap_err();
    assert!(error.contains("did not acknowledge"), "{error}");
}

#[test]
fn installed_mapper_preserves_completion_scope_after_lowering() {
    require_tsgo!();
    let (source, at) = at_cursor("declare const x: number;\nfunction f() { const answer = match (@@x) { _ => Math }; }\n");
    let dir = project(&[("src/main.tt", &source)]);
    common::installed_mapper::install(&dir);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut service = open_service(&file);
    let answer = service.completion(&file, at, false).unwrap();
    for excluded in ["answer", "declare", "namespace"] {
        assert!(!answer.items.iter().any(|item| item.label == excluded), "unexpected {excluded}");
    }
    assert!(answer.items.iter().any(|item| item.label == "x"));
}

#[test]
fn installed_mapper_preserves_diagnostic_provenance() {
    require_tsgo!();
    let pipeline = "const inc = (n: number): number => n + 1;\nconst shout = (s: string): string => s.toUpperCase();\nconst answer = 1 |> inc |> shout |> inc;\n";
    let payloads = include_str!("../cases/editor/unusedPayloadList.tt");
    for (source, expected) in [(pipeline, 2), (payloads, 5)] {
        let dir = project(&[("src/main.tt", source)]);
        let file = dir.join("src/main.tt").canonicalize().unwrap();
        let before = open_service(&file).service_diagnostics(&file).unwrap();
        assert_eq!(before.len(), expected, "{before:?}");
        common::installed_mapper::install(&dir);
        let mut service = open_service(&file);
        let actual = service.service_diagnostics(&file).unwrap();
        assert_eq!(actual, before);
        // Diagnostics must not replace the authored document with a projection.
        let at = utf16_position(source, if expected == 2 { "inc =" } else { "Failure extends" });
        assert_eq!(service.hover(&file, at).unwrap().unwrap().range.start, at);
    }
}

#[test]
fn installed_mapper_completes_pipeline_members_in_scripts_and_modules() {
    require_tsgo!();
    for module in [false, true] {
        let prefix = if module { "export {};\n" } else { "" };
        let (source, at) = at_cursor(&format!("{prefix}const title = \"문자\";\nconst result = \"hello\" |> .@@;\n"));
        let dir = project(&[("src/main.tt", &source)]);
        common::installed_mapper::install(&dir);
        let file = dir.join("src/main.tt").canonicalize().unwrap();
        let mut service = open_service(&file);
        let answer = service.completion(&file, at, true).unwrap();
        assert!(answer.member, "module={module}: not a member completion");
        assert!(answer.items.iter().any(|item| item.label == "toUpperCase"), "module={module}: missing String members");
    }
}

#[test]
fn an_or_pattern_binding_is_a_document_symbol() {
    require_tsgo!();
    let source = "export variant Shape { Sq(width: number), Rect(width: number, height: number), Point }\n\
export function side(s: Shape): number {\n  return match (s) {\n    Sq(width: q) | Rect(width: q) => q,\n    Point => 0,\n  };\n}\n";
    let dir = project(&[("src/main.tt", source)]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut service = open_service(&file);
    let symbols = service.document_symbols(&file).unwrap();
    let side = symbols
        .iter()
        .find(|symbol| symbol.name == "side")
        .unwrap_or_else(|| panic!("{symbols:?}"));
    let q = side
        .children
        .iter()
        .find(|symbol| symbol.name == "q")
        .unwrap_or_else(|| panic!("{symbols:?}"));
    assert_eq!(q.selection_range.start, utf16_position(source, "q) | Rect"), "{q:?}");
}
