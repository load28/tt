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
    let source = "export const a = 1 |> String;\nconst b = 2;\nexport const c = b |> String;\nexport const d = help;\nexport const e = ttHel;\n";
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
        let detail = project
            .completion_resolve(&file, at, label, answer.probe)
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
