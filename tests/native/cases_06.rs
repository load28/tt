fn assert_type_checks(source: &str) {
    let dir = project(&[("src/main.tt", source)]);
    let output = run(&dir, &["--check-types", "src"]);
    assert!(
        output.status.success(),
        "{source}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn each_pipeline_step_holds_the_type_of_its_own_value() {
    require_tsgo!();
    assert_type_checks(
        "declare const flag: boolean;\n\
declare function pick(value: { kind: \"a\" } | { kind: \"b\" }): string;\n\
export const a = match (1) { _ => [1] } |> (p => p.length);\n\
export const b: string = match (flag) { true => [1, 2], false => [3] } |> (p => p.length) |> String;\n\
export const c: number = match (flag) { true => \"xy\", false => \"z\" } |> .length |> (n => [n]) |> .length;\n\
export const d = match (flag) { true => ({ kind: \"a\" }), false => ({ kind: \"b\" }) } |> pick;\n\
export const e: string[] = [match (flag) { true => 1, false => 2 }] |> .map(n => n + 1) |> .map(String);\n\
export function f(): string { return match (flag) { true => [1], false => [] } |> .length |> String; }\n",
    );
}

#[test]
fn a_hoisted_value_in_a_member_step_type_checks() {
    require_tsgo!();
    assert_type_checks(
        "class Box {\n  constructor(readonly n: number) {}\n  add(amount: number): Box { return new Box(this.n + amount); }\n}\n\
declare const flag: boolean;\n\
export const a: number = new Box(1) |> .add(match (flag) { true => 1, false => 2 }) |> .n;\n\
export const b: number = new Box(1) |> .add(1).add(match (flag) { true => 1, false => 2 }).n;\n\
export const c: number = { list: [1, 2] } |> .list[match (flag) { true => 0, false => 1 }];\n\
export const d: Box | undefined = (new Box(1) as Box | undefined) |> ?.add(match (flag) { true => 1, false => 2 });\n",
    );
}

#[test]
fn a_try_in_a_template_in_a_pipeline_type_checks() {
    require_tsgo!();
    assert_type_checks(
        "type R = { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\n\
type S = { kind: \"Ok\"; value: string } | { kind: \"Err\"; error: string };\n\
declare function read(): R;\n\
declare function wrap(value: number): string;\n\
export function head(): S {\n  const value = `${wrap(try read())}!` |> String;\n  return { kind: \"Ok\", value };\n}\n\
export function step(): S {\n  const value = \"v\" |> ((tail: string) => (v: string) => v + tail)(`${wrap(try read())}`);\n  return { kind: \"Ok\", value };\n}\n",
    );
}

#[test]
fn service_suggestions_keep_their_severity_and_tags_on_written_text() {
    require_tsgo!();
    let source = "variant Shape { Circle(radius: number), Point }\n\
/** @deprecated */\n\
declare function old(): void;\n\
export function area(shape: Shape): number {\n\
\x20 const unused = 1;\n\
\x20 old();\n\
\x20 return match (shape) {\n\
\x20   Circle(radius) => 1,\n\
\x20   Point => 0,\n\
\x20 };\n\
}\n\
export const wrong: number = \"x\";\n";
    let dir = project(&[("src/main.tt", source)]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut project = ttc::engine::Engine::new(None)
        .open_project(
            &[file.to_string_lossy().into_owned()],
            &ttc::engine::ProjectOptions::default(),
        )
        .unwrap();
    let diagnostics = project.service_diagnostics(&file).unwrap();
    let seen: Vec<_> = diagnostics
        .iter()
        .map(|d| {
            (
                utf16_slice(source, d.range),
                d.code,
                d.severity,
                d.tags.clone(),
            )
        })
        .collect();
    use ttc::engine::{ServiceSeverity::*, ServiceTag::*};
    assert_eq!(
        seen,
        vec![
            ("wrong", 2322, Error, vec![]),
            ("unused", 6133, Hint, vec![Unnecessary]),
            ("old", 6387, Hint, vec![Deprecated]),
            ("radius", 6133, Hint, vec![Unnecessary]),
        ],
        "{diagnostics:?}"
    );
}

#[test]
fn an_auto_import_completion_carries_its_import_edit_onto_the_source() {
    require_tsgo!();
    let source = "export const a = 1 |> ((n: number) => String(n));\nconst b = 2;\nexport const c = b |> ((n: number) => String(n));\nexport const d = help;\nexport const e = ttHel;\n";
    let dir = project(&[
        ("src/util.ts", "export function helperFn(n: number): number { return n; }\n"),
        ("src/lib.tt", "export function ttHelper(n: number): number { return n; }\n"),
        ("src/main.tt", source),
    ]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut project = ttc::engine::Engine::new(None)
        .open_project(
            &[file.to_string_lossy().into_owned()],
            &ttc::engine::ProjectOptions::default(),
        )
        .unwrap();
    for (typed, label, import) in [
        ("help", "helperFn", "import { helperFn } from \"./util\";\n"),
        ("ttHel", "ttHelper", "import { ttHelper } from \"./lib.tt\";\n"),
    ] {
        let mut at = utf16_position(source, typed);
        at.character += typed.len() as u32;
        let answer = project.completion(&file, at, false).unwrap();
        assert!(
            answer.items.iter().any(|item| item.label == label),
            "{label} not offered"
        );
        let source = answer
            .items
            .iter()
            .find(|item| item.label == label)
            .and_then(|item| item.source.clone());
        let detail = project
            .completion_resolve(&file, at, label, source.as_deref(), answer.probe)
            .unwrap()
            .expect("resolved");
        // The emitted file opens with the runtime import the pipelines
        // need; the import lands before the first line the user wrote.
        let start = ttc::engine::Position { line: 0, character: 0 };
        assert_eq!(
            detail.additional_edits,
            vec![ttc::engine::TextEdit {
                range: ttc::engine::Range { start, end: start },
                new_text: import.to_string(),
            }],
            "{label}"
        );
    }
}

#[test]
fn references_to_a_tt_name_reach_its_declaration_its_patterns_and_its_typescript_uses() {
    require_tsgo!();
    let lib = "export variant Color { Red, Green(level: number) }\n\
export function f(c: Color): number {\n\
\x20 return match (c) { Red => 0, Green(level) => level };\n\
}\n";
    let use_tt = "import { Color } from \"./lib.tt\";\n\
export const g = (c: Color) => match (c) { Green(level: l) => l, _ => 1 };\n\
export const made = Color.Green(3);\n";
    let use_ts = "import { Color } from \"./lib.tt\";\nexport const x: Color = Color.Green(2);\n";
    let dir = project(&[
        ("src/lib.tt", lib),
        ("src/use.tt", use_tt),
        ("src/use.ts", use_ts),
    ]);
    let file = |name: &str| dir.join(name).canonicalize().unwrap();
    let mut project = ttc::engine::Engine::new(None)
        .open_project(
            &[
                file("src/lib.tt").to_string_lossy().into_owned(),
                file("src/use.tt").to_string_lossy().into_owned(),
            ],
            &ttc::engine::ProjectOptions::default(),
        )
        .unwrap();
    let text = |name: &str| match name {
        "src/lib.tt" => lib,
        "src/use.tt" => use_tt,
        _ => use_ts,
    };
    let mut references = |name: &str, needle: &str| {
        let mut at = utf16_position(text(name), needle);
        at.character += 1;
        let mut found: Vec<String> = project
            .references(&file(name), at)
            .unwrap()
            .into_iter()
            .map(|reference| {
                let owner = ["src/lib.tt", "src/use.tt", "src/use.ts"]
                    .into_iter()
                    .find(|owner| file(owner) == reference.location.path)
                    .expect("a project file");
                format!(
                    "{owner}:{}{}",
                    utf16_slice(text(owner), reference.location.range),
                    if reference.is_definition { "*" } else { "" }
                )
            })
            .collect();
        found.sort();
        found
    };
    let green = vec![
        "src/lib.tt:Green",
        "src/lib.tt:Green*",
        "src/use.ts:Green",
        "src/use.tt:Green",
        "src/use.tt:Green",
    ];
    for (name, needle) in [
        ("src/lib.tt", "Green(level: number)"),
        ("src/lib.tt", "Green(level) =>"),
        ("src/use.tt", "Green(level: l)"),
        ("src/use.ts", "Green(2)"),
    ] {
        assert_eq!(references(name, needle), green, "{name} {needle}");
    }
    assert_eq!(
        references("src/use.tt", "level: l"),
        vec!["src/lib.tt:level", "src/lib.tt:level*", "src/use.tt:level"]
    );
    assert_eq!(references("src/lib.tt", "Color {").len(), 8);
}

#[test]
fn the_outline_keeps_the_users_declarations_and_leaves_out_generated_ones() {
    require_tsgo!();
    let source = "export variant Shape { Circle(radius: number), Point }\n\
export function area(s: Shape): number {\n\
\x20 const a = match (s) { Circle(radius) => radius, Point => 0 };\n\
\x20 return a |> String |> ((v: string) => v.length);\n\
}\n";
    let dir = project(&[("src/main.tt", source)]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut project = ttc::engine::Engine::new(None)
        .open_project(
            &[file.to_string_lossy().into_owned()],
            &ttc::engine::ProjectOptions::default(),
        )
        .unwrap();
    fn tree(source: &str, symbols: &[ttc::engine::DocumentSymbol]) -> Vec<String> {
        symbols
            .iter()
            .flat_map(|symbol| {
                let mut lines = vec![format!(
                    "{} [{}]",
                    symbol.name,
                    utf16_slice(source, symbol.selection_range)
                )];
                lines.extend(tree(source, &symbol.children).into_iter().map(|line| format!("  {line}")));
                lines
            })
            .collect()
    }
    let symbols = project.document_symbols(&file).unwrap();
    assert_eq!(
        tree(source, &symbols),
        vec![
            "area [area]",
            "  a [a]",
            "    radius [radius]",
            "  <function> []",
        ],
        "{symbols:?}"
    );
    assert!(utf16_slice(source, symbols[0].range).starts_with("export function area"));
    assert!(utf16_slice(source, symbols[0].range).ends_with("v.length);\n}"));
}

#[test]
fn a_declaration_initialized_by_try_spans_its_initializer_in_the_outline() {
    require_tsgo!();
    let source = "import type { TResult } from \"@tt/std\";\n\
declare function rr(n: number): TResult<number, string>;\n\
export function f(): TResult<number, string> {\n\
\x20 const s = try rr(1);\n\
\x20 const n = s + 1;\n\
\x20 return { kind: \"Ok\", value: n };\n\
}\n";
    let dir = project(&[("src/main.tt", source)]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut project = ttc::engine::Engine::new(None)
        .open_project(
            &[file.to_string_lossy().into_owned()],
            &ttc::engine::ProjectOptions::default(),
        )
        .unwrap();
    let symbols = project.document_symbols(&file).unwrap();
    let function = symbols.iter().find(|symbol| symbol.name == "f").expect("f");
    let ranges: Vec<&str> = function
        .children
        .iter()
        .filter(|symbol| symbol.name == "s" || symbol.name == "n")
        .map(|symbol| utf16_slice(source, symbol.range))
        .collect();
    assert_eq!(ranges, vec!["s = try rr(1)", "n = s + 1"], "{symbols:?}");
}

#[test]
fn an_operand_hoisted_ahead_of_a_later_match_still_answers_at_its_end() {
    require_tsgo!();
    let source = "variant Shape { Circle(radius: number), Point }\n\
declare const s: Shape;\n\
declare function helper(n: number): number;\n\
const obj = { a: s.ki, b: match (s) { Circle(radius) => radius, Point => 0 } };\n\
const sum = s.ki + match (s) { Circle(radius) => radius, Point => 0 };\n\
const arr = [helper, match (s) { Circle(radius) => radius, Point => 0 }];\n\
export { obj, sum, arr };\n";
    let dir = project(&[("src/main.tt", source)]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut project = ttc::engine::Engine::new(None)
        .open_project(
            &[file.to_string_lossy().into_owned()],
            &ttc::engine::ProjectOptions::default(),
        )
        .unwrap();
    let end_of = |needle: &str, nth: usize| {
        let at = source.match_indices(needle).nth(nth).unwrap().0 + needle.len();
        ttc::engine::Position {
            line: source[..at].matches('\n').count() as u32,
            character: (at - source[..at].rfind('\n').map_or(0, |n| n + 1)) as u32,
        }
    };
    for nth in 0..2 {
        let labels: Vec<_> = project
            .completion(&file, end_of("s.ki", nth), false)
            .unwrap()
            .items
            .into_iter()
            .map(|item| item.label)
            .collect();
        assert_eq!(labels, ["kind"], "occurrence {nth}");
    }
    let helper = end_of("[helper", 0);
    let hover = project.hover(&file, helper).unwrap().expect("hover");
    assert!(hover.signature.starts_with("function helper"), "{hover:?}");
    let definition = project.definition(&file, helper).unwrap();
    assert_eq!(definition.len(), 1);
    assert_eq!(utf16_slice(source, definition[0].range), "helper");
}
