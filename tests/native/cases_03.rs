#[test]
fn host_overlays_are_snapshot_values_and_live_language_inputs() {
    require_tsgo!();
    for extension in ["ts", "tsx"] {
        let provider = format!("src/provider.{extension}");
        let source = "import { value } from './provider';\nconst result: string = value;\nvalue.toUpperCase();\n";
        let dir = project(&[
            (&provider, "export const value: string = 'disk';\n"),
            ("src/consumer.tt", source),
        ]);
        let provider = dir.join(provider).canonicalize().unwrap();
        let consumer = dir.join("src/consumer.tt").canonicalize().unwrap();
        let engine = ttc::engine::Engine::new(None);
        let mut project = engine.open_project(
            &[consumer.to_string_lossy().into_owned()],
            &ttc::engine::ProjectOptions::default(),
        ).unwrap();
        let files = project.initial_files();
        project.open_document(provider.clone(), "export const value: number = 42;\n".into());
        let snapshot = project.update(&files).unwrap();
        project.update_document(provider.clone(), "export const value: string = 'new';\n".into());
        let checked = project.check(&snapshot, &ttc::engine::CheckRequest::default()).unwrap();
        assert!(checked.backend_error.is_none(), "{:?}", checked.backend_error);
        assert!(checked.diagnostics.iter().any(|d| d.path == consumer), "{:?}", checked.diagnostics);
        assert_eq!(snapshot.source_of(&provider), Some("export const value: number = 42;\n"));
        let position = ttc::engine::Position { line: 2, character: 6 };
        let completions = project.completion(&consumer, position, true).unwrap();
        assert!(completions.items.iter().any(|item| item.label == "toUpperCase"));
        project.update_document(provider.clone(), "export const value: number = 42;\n".into());
        let completions = project.completion(&consumer, position, true).unwrap();
        assert!(completions.items.iter().any(|item| item.label == "toFixed"));
        assert!(!completions.items.iter().any(|item| item.label == "toUpperCase"));
        project.close_document(&provider);
        let completions = project.completion(&consumer, position, true).unwrap();
        assert!(completions.items.iter().any(|item| item.label == "toUpperCase"));
        let snapshot = project.update(&files).unwrap();
        let checked = project.check(&snapshot, &ttc::engine::CheckRequest::default()).unwrap();
        assert!(checked.backend_error.is_none());
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }
}

#[test]
fn typed_exhaustiveness_still_answers_from_the_narrowed_type() {
    require_tsgo!();
    // The point of asking the checker at all: a case an earlier test
    // removed is not demanded back. `--check`, which knows only the
    // declaration, does report it.
    let dir = project(&[(
        "src/narrow.tt",
        "variant Shape { Circle(radius: number), Point }\n\
         export function f(x: Shape): number {\n\
         \x20 if (x.kind === \"Point\") return 0;\n\
         \x20 return match (x) { Circle(radius) => radius };\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(
        !out.contains("not exhaustive"),
        "Point is already excluded here: {out}"
    );
}

#[test]
fn a_hand_written_payload_union_is_named_by_the_checker() {
    require_tsgo!();
    // The payload's declared type is a hand-written union, so no tt
    // declaration describes it — the one thing the declaration table can
    // never answer. The emitted condition tests that payload at exactly
    // its type, and asking there names the column's alphabet (TASK-109).
    let dir = project(&[(
        "src/opaque.tt",
        "type Inner = { kind: \"Yes\"; n: number } | { kind: \"No\" };\n\
         variant Outer { Wrap(inner: Inner), Bare }\n\
         declare const o: Outer;\n\
         export const a = match (o) { Wrap(inner: Yes(n)) => n, Bare => -1 };\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("match is not exhaustive: missing \"Wrap(inner: No())\""),
        "the checker names the payload's constituents: {out}"
    );
}

#[test]
fn a_hand_written_payload_union_fully_covered_is_exhaustive() {
    require_tsgo!();
    // The other half of the same answer: covering the payload's cases
    // makes the match exhaustive, and nothing is reported. Before the
    // payload question existed this stayed quiet too — but only because tt
    // refused to guess, which is a different thing from knowing.
    let dir = project(&[(
        "src/opaque_full.tt",
        "type Inner = { kind: \"Yes\"; n: number } | { kind: \"No\" };\n\
         variant Outer { Wrap(inner: Inner), Bare }\n\
         declare const o: Outer;\n\
         export const a = match (o) {\n\
         \x20 Wrap(inner: Yes(n)) => n,\n\
         \x20 Wrap(inner: No()) => 0,\n\
         \x20 Bare => -1,\n\
         };\n",
    )]);
    let out = check(&dir);
    assert!(!out.contains("not exhaustive"), "covered: {out}");
}

#[test]
fn typed_exhaustiveness_resolves_a_payload_declared_in_another_module() {
    require_tsgo!();
    // The nested column is resolved from declarations, so the imported
    // ones have to be collected on this path too — the same 1-hop
    // collection the default path does.
    let dir = project(&[
        (
            "src/token.tt",
            "export variant Tok { Num(n: number), Eof }\n",
        ),
        (
            "src/line.tt",
            "import { Tok } from \"./token.tt\";\n\
             variant Line { Head(t: Tok), Blank }\n\
             declare const l: Line;\n\
             export const a = match (l) { Head(t: Num(n)) => n, Blank => 0 };\n",
        ),
    ]);
    let out = check(&dir);
    assert!(
        out.contains("match is not exhaustive: missing \"Head(t: Eof())\""),
        "the imported payload variant is resolved: {out}"
    );
}

#[test]
fn typed_exhaustiveness_resolves_a_payload_exported_through_a_specifier() {
    require_tsgo!();
    let dir = project(&[
        (
            "src/token.tt",
            "variant Tok { Num(n: number), Eof }\nexport { Tok as Token };\n",
        ),
        (
            "src/line.tt",
            "import { Token } from \"./token.tt\";\n\
             variant Line { Head(t: Token), Blank }\n\
             declare const l: Line;\n\
             export const a = match (l) { Head(t: Num(n)) => n, Blank => 0 };\n",
        ),
    ]);
    let out = check(&dir);
    assert!(
        out.contains("match is not exhaustive: missing \"Head(t: Eof())\""),
        "the aliased payload variant is resolved: {out}"
    );
}

#[test]
fn typed_exhaustiveness_covers_tuple_matches_too() {
    require_tsgo!();
    // A tuple match asks one question per position. Before, it asked none:
    // the typed path skipped tuple matches entirely, so the product was
    // checked only by the default path's declaration table (TASK-111).
    let dir = project(&[(
        "src/tuple.tt",
        "variant Dir { North(dx: number), South }\n\
         variant Speed { Fast(v: number), Slow }\n\
         declare const d: Dir;\n\
         declare const s: Speed;\n\
         export const n = match (d, s) { (North(dx), Fast(v)) => dx + v, (South, _) => 0 };\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("match is not exhaustive: missing (North, Slow)"),
        "the missing combination is named: {out}"
    );
}

#[test]
fn a_tuple_position_the_checker_narrowed_is_not_demanded_back() {
    require_tsgo!();
    // The reason to ask at all: `South` is impossible at the match, so the
    // combinations that need it are not missing. The default path, which
    // knows only the declaration, does report them.
    let dir = project(&[(
        "src/narrowed_tuple.tt",
        "variant Dir { North(dx: number), South }\n\
         variant Speed { Fast(v: number), Slow }\n\
         export function f(d: Dir, s: Speed): number {\n\
         \x20 if (d.kind === \"South\") return 0;\n\
         \x20 return match (d, s) { (North(dx), Fast(v)) => dx + v, (North(dx), Slow) => dx };\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(
        !out.contains("not exhaustive"),
        "South is impossible: {out}"
    );
}

/// The editor's hardest question, at the compiler layer: completion at a
/// `.` or `?.` the user has just typed, in a pipeline whose value is a
/// `Result`.
///
/// The buffer does not parse — both tails are incomplete — so nothing about
/// it can be decided by parsing it. The probe mends it, and the mended form
/// emits `$tt_ap`, so `@tt/runtime` has to already be resolvable in the
/// workspace or the whole expression comes back untyped and the answer is
/// empty (TASK-217).
#[test]
fn a_probe_answers_in_a_pipeline_the_buffer_cannot_parse_yet() {
    // The engine runs in-process, resolving the toolchain by the same
    // rules this guard mirrors — so a pass here means the compiler found
    // one, not that the test pointed it at one.
    require_tsgo!();
    for tail in [".", "?."] {
        let source = format!(
            "import type {{ TResult }} from \"@tt/std\";\n\
             import * as Result from \"@tt/std/result\";\n\
             \n\
             declare const r: TResult<number, string>;\n\
             const out = r\n\
             \x20 |> Result.mapP((n) => n + 1)\n\
             \x20 |> {tail}"
        );
        let dir = tmpdir();
        fs::create_dir_all(dir.join("src")).unwrap();
        let file = dir.join("src/probe.tt");
        fs::write(&file, &source).unwrap();

        let engine = ttc::engine::Engine::new(None);
        let mut project = engine
            .open_project(
                &[file.to_string_lossy().to_string()],
                &ttc::engine::ProjectOptions::default(),
            )
            .expect("the project opens");
        let lines: Vec<&str> = source.split('\n').collect();
        let position = ttc::engine::Position {
            line: lines.len() as u32 - 1,
            character: lines[lines.len() - 1].chars().count() as u32,
        };
        let answer = project
            .completion(&file, position, true)
            .expect("the probe answers");
        let labels: Vec<&str> = answer.items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            answer.probe.is_some(),
            "the {tail} members had to come from a probe: {labels:?}"
        );
        assert!(
            labels.contains(&"kind"),
            "the value at the {tail} step is a Result: {labels:?}"
        );
    }
}

#[test]
fn service_requests_answer_over_values_the_plan_cannot_own() {
    require_tsgo!();
    let cases = [
        (
            "src/member.tt",
            "declare function parse(text: string): { value: number } | { error: string };\n\
             export function read() {\n\
             \x20 const value = try parse(\"1\").\n\
             }\n",
            ttc::engine::Position { line: 2, character: 31 },
        ),
        (
            "src/parameter.tt",
            "export function read(value = match (1) { 1 => \"one\", _ => \"other\" }) {\n\
             \x20 return value;\n\
             }\n",
            ttc::engine::Position { line: 1, character: 10 },
        ),
        (
            "src/field.tt",
            "const a = 1;\n\
             export class C { z = match (a) { 1 => \"one\", _ => \"other\" } }\n",
            ttc::engine::Position { line: 1, character: 17 },
        ),
    ];
    let dir = project(&cases.map(|(name, text, _)| (name, text)));
    let engine = ttc::engine::Engine::new(None);
    for (name, _, position) in cases {
        let file = dir.join(name).canonicalize().unwrap();
        let mut project = engine
            .open_project(
                &[file.to_string_lossy().into_owned()],
                &ttc::engine::ProjectOptions::default(),
            )
            .unwrap();
        project.completion(&file, position, true).unwrap();
        project.hover(&file, position).unwrap();
        project.signature_help(&file, position).unwrap();
        project.definition(&file, position).unwrap();
        project.service_diagnostics(&file).unwrap();
    }
}

#[test]
fn generated_bindings_never_surface_as_user_symbols() {
    require_tsgo!();
    let source = "declare function parse(t: string): { value: number } | { error: string };\n\
                  declare const o: 1 | 2;\n\
                  export function f() {\n\
                  \x20 const a = match (o) { 1 => \"one\", _ => \"other\" };\n\
                  \x20 const n = try parse(a);\n\
                  \x20 \n\
                  \x20 return n;\n\
                  }\n";
    let emitted = ttc::emit_mapped(source).code;
    let glue: Vec<&str> = ["$tt_v0", "$tt_t0"]
        .into_iter()
        .filter(|name| emitted.contains(name))
        .collect();
    assert_eq!(glue.len(), 2, "{emitted}");
    let dir = project(&[("src/glue.tt", source)]);
    let file = dir.join("src/glue.tt").canonicalize().unwrap();
    let engine = ttc::engine::Engine::new(None);
    let mut project = engine
        .open_project(
            &[file.to_string_lossy().into_owned()],
            &ttc::engine::ProjectOptions::default(),
        )
        .unwrap();
    let answer = project
        .completion(&file, ttc::engine::Position { line: 5, character: 2 }, false)
        .unwrap();
    let labels: Vec<&str> = answer.items.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&"a") && labels.contains(&"n"), "{labels:?}");
    assert!(glue.iter().all(|name| !labels.contains(name)), "{labels:?}");
    let on_match = project
        .hover(&file, ttc::engine::Position { line: 3, character: 12 })
        .unwrap();
    assert!(on_match.is_none(), "{on_match:?}");
    let on_binding = project
        .hover(&file, ttc::engine::Position { line: 3, character: 8 })
        .unwrap()
        .expect("the user's binding still hovers");
    assert_eq!(on_binding.signature, "const a: string");
    let references = project
        .references(&file, ttc::engine::Position { line: 3, character: 12 })
        .unwrap();
    assert!(references.is_empty(), "{references:?}");
}

#[test]
fn or_pattern_bindings_navigate_and_rename_as_one_binding() {
    require_tsgo!();
    let source = "export variant Shape { Circle(size: number), Square(size: number), Point }\n\
                  export function area(s: Shape): number {\n\
                  if let Circle(size: q) | Square(size: q) = s { return q; }\n\
                  let Circle(size: z) | Square(size: z) = s else { return 0; };\n\
                  const b = match (s) { Circle(size) | Square(size: size) => size, Point => 0 };\n\
                  return b + z;\n\
                  }\n";
    let dir = project(&[("src/a.tt", source)]);
    let a = dir.join("src/a.tt").canonicalize().unwrap();
    let engine = ttc::engine::Engine::new(None);
    let mut project = engine
        .open_project(
            &[dir.join("src").to_string_lossy().into_owned()],
            &ttc::engine::ProjectOptions::default(),
        )
        .expect("the project opens");
    let at = |needle: &str, len: usize| source_location(&a, source, needle, 0, len);
    let cases = [
        (
            vec![at("q) | Square", 1), at("q) = s", 1)],
            at("q; }", 1),
            source.replace(
                "if let Circle(size: q) | Square(size: q) = s { return q; }",
                "if let Circle(size: zz) | Square(size: zz) = s { return zz; }",
            ),
        ),
        (
            vec![at("z) | Square", 1), at("z) = s", 1)],
            at("z;", 1),
            source
                .replace(
                    "Circle(size: z) | Square(size: z)",
                    "Circle(size: zz) | Square(size: zz)",
                )
                .replace("b + z;", "b + zz;"),
        ),
        (
            vec![
                at("size) | Square(size: size)", 4),
                at("size) => size", 4),
            ],
            at("size, Point", 4),
            source.replace(
                "Circle(size) | Square(size: size) => size,",
                "Circle(size: zz) | Square(size: zz) => zz,",
            ),
        ),
    ];
    for (declarations, usage, renamed) in cases {
        let shorthand = declarations
            .iter()
            .any(|place| place.range.end.character - place.range.start.character > 1);
        for place in declarations.iter().chain([&usage]) {
            let position = place.range.start;
            let edits = project
                .rename(&a, position)
                .expect("rename answers")
                .unwrap_or_else(|| panic!("{place:?} renames"));
            assert_eq!(apply_rename(source, &edits, "zz"), renamed, "{place:?}");
            if shorthand && place != &usage {
                continue;
            }
            let found = project
                .definition(&a, position)
                .expect("definition answers");
            assert_eq!(found, declarations, "{place:?}");
            let references = project
                .references(&a, position)
                .expect("references answer");
            let mut declared: Vec<_> = references
                .iter()
                .filter(|reference| reference.is_definition)
                .map(|reference| reference.location.clone())
                .collect();
            declared.sort_by_key(|location| {
                (location.range.start.line, location.range.start.character)
            });
            assert_eq!(declared, declarations, "{place:?}: {references:?}");
            assert!(
                references
                    .iter()
                    .any(|reference| reference.location == usage && !reference.is_definition),
                "{place:?}: {references:?}"
            );
        }
    }
}

fn apply_rename(source: &str, edits: &[ttc::engine::RenameEdit], name: &str) -> String {
    let offset = |position: ttc::engine::Position| {
        source
            .split('\n')
            .take(position.line as usize)
            .map(|line| line.len() + 1)
            .sum::<usize>()
            + position.character as usize
    };
    let mut edits: Vec<_> = edits.iter().collect();
    edits.sort_by_key(|edit| std::cmp::Reverse(offset(edit.location.range.start)));
    let mut out = source.to_string();
    for edit in edits {
        let text = edit.new_text.as_deref().map_or(name.to_string(), |text| {
            text.replace(ttc::engine::RENAME_PLACEHOLDER, name)
        });
        out.replace_range(
            offset(edit.location.range.start)..offset(edit.location.range.end),
            &text,
        );
    }
    out
}

fn source_position(text: &str, needle: &str, delta: usize) -> ttc::engine::Position {
    let offset = text.find(needle).expect("needle") + delta;
    let before = &text[..offset];
    ttc::engine::Position {
        line: before.matches('\n').count() as u32,
        character: before[before.rfind('\n').map_or(0, |n| n + 1)..]
            .encode_utf16()
            .count() as u32,
    }
}

fn source_location(path: &Path, text: &str, needle: &str, delta: usize, len: usize) -> ttc::engine::Location {
    ttc::engine::Location {
        path: path.to_path_buf(),
        range: ttc::engine::Range {
            start: source_position(text, needle, delta),
            end: source_position(text, needle, delta + len),
        },
    }
}

#[test]
fn variant_navigation_lands_on_the_variant_declaration() {
    require_tsgo!();
    let local = "variant V { A(x: number), B }\n\
                 const v: V = V.A(1);\n\
                 const w = V.B;\n\
                 function f(q: V) { return q; }\n";
    let shapes = "export variant Shape { Circle(r: number), Point }\n";
    let user = "import { Shape } from \"./shapes.tt\";\n\
                export function g(s: Shape) { return s; }\n\
                const c = Shape.Circle(1);\n";
    let dir = project(&[
        ("src/a.tt", local),
        ("src/shapes.tt", shapes),
        ("src/use.tt", user),
    ]);
    let a = dir.join("src/a.tt").canonicalize().unwrap();
    let shapes_path = dir.join("src/shapes.tt").canonicalize().unwrap();
    let use_path = dir.join("src/use.tt").canonicalize().unwrap();
    let engine = ttc::engine::Engine::new(None);
    let mut project = engine
        .open_project(
            &[dir.join("src").to_string_lossy().into_owned()],
            &ttc::engine::ProjectOptions::default(),
        )
        .expect("the project opens");
    let variant = source_location(&a, local, "V {", 0, 1);
    let case_a = source_location(&a, local, "A(x", 0, 1);
    let case_b = source_location(&a, local, "B }", 0, 1);
    for (needle, delta, expected) in [
        ("v: V", 3, &variant),
        ("V.A(1)", 0, &variant),
        ("V.A(1)", 2, &case_a),
        ("V.B", 0, &variant),
        ("V.B", 2, &case_b),
        ("q: V", 3, &variant),
    ] {
        let found = project
            .definition(&a, source_position(local, needle, delta))
            .expect("definition answers");
        assert_eq!(found, vec![expected.clone()], "{needle:?}+{delta}");
    }
    let references = project
        .references(&a, source_position(local, "V.A(1)", 2))
        .expect("references answer");
    let declared: Vec<_> = references
        .iter()
        .filter(|reference| reference.is_definition)
        .map(|reference| reference.location.clone())
        .collect();
    assert_eq!(declared, vec![case_a], "{references:?}");
    assert_eq!(references.len(), 2, "{references:?}");

    let shape = source_location(&shapes_path, shapes, "Shape", 0, 5);
    let found = project
        .definition(&use_path, source_position(user, "s: Shape", 3))
        .expect("definition answers");
    assert_eq!(found, vec![shape.clone()]);
    let references = project
        .references(&use_path, source_position(user, "s: Shape", 3))
        .expect("references answer");
    let declared: Vec<_> = references
        .iter()
        .filter(|reference| reference.is_definition)
        .map(|reference| reference.location.clone())
        .collect();
    assert_eq!(declared, vec![shape], "{references:?}");
    let specifier = source_location(&use_path, user, "Shape", 0, 5);
    assert!(
        references
            .iter()
            .any(|reference| reference.location == specifier && !reference.is_definition),
        "{references:?}"
    );
}

#[test]
fn a_byte_order_mark_moves_no_reported_position() {
    require_tsgo!();
    let body = "const a: string = 1;\nconst n: number = \"x\";\n";
    let hand_written = "export const h: string = 2;\n";
    let reports: Vec<String> = ["", "\u{feff}"]
        .into_iter()
        .map(|signature| {
            let dir = project(&[
                ("src/b.tt", &format!("{signature}{body}")),
                ("src/h.ts", &format!("{signature}{hand_written}")),
            ]);
            let output = run(&dir, &["--check-types", "src"]);
            String::from_utf8_lossy(&output.stderr).into_owned()
        })
        .collect();
    let plain = &reports[0];
    assert!(plain.contains("src/b.tt:1:19"), "{plain}");
    assert!(plain.contains("src/b.tt:2:19"), "{plain}");
    assert!(plain.contains("src/h.ts:1:26"), "{plain}");
    assert!(plain.contains("\n  |                   ^^^\n"), "{plain}");
    assert_eq!(reports[1], *plain);
}

#[test]
fn a_requested_file_outside_the_configuration_is_checked_in_its_inferred_project() {
    require_tsgo!();
    use std::io::Write;
    let outside = "const n: number = \"x\";\nval const v = [1];\nv.push(2);\nexport {};\n";
    let inside = "export const a: number = 1;\n";
    let dir = project(&[("src/a.tt", inside)]);
    fs::create_dir_all(dir.join("other")).unwrap();
    write(&dir, "other/x.tt", outside);

    let out = run(&dir, &["--check-types", "other/x.tt"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("error[ts2322]"), "{stderr}");
    assert!(stderr.contains("--> other/x.tt:1:19"), "{stderr}");
    assert!(stderr.contains("error[val-mutation]"), "{stderr}");

    let out = run(&dir, &["--check-types", "src"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(!stderr.contains("other/x.tt"), "{stderr}");

    let a = dir.join("src/a.tt").canonicalize().unwrap();
    let x = dir.join("other/x.tt").canonicalize().unwrap();
    let requests = [
        serde_json::json!({ "id": 1, "method": "openDocument",
            "params": { "path": a, "text": inside } }),
        serde_json::json!({ "id": 2, "method": "typedCheck",
            "params": { "path": a, "text": inside, "includeTypes": true } }),
        serde_json::json!({ "id": 3, "method": "openDocument",
            "params": { "path": x, "text": outside } }),
        serde_json::json!({ "id": 4, "method": "typedCheck",
            "params": { "path": x, "text": outside, "includeTypes": true } }),
        serde_json::json!({ "id": 5, "method": "tsDiagnostics",
            "params": { "path": x, "position": { "line": 0, "character": 0 } } }),
    ];
    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .arg("--server")
        .current_dir(&dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("server starts");
    for request in requests {
        writeln!(child.stdin.as_mut().unwrap(), "{request}").unwrap();
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("server answers");
    let answers: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSON response"))
        .collect();
    let answer = |id: u64| {
        answers
            .iter()
            .find(|answer| answer["id"] == id)
            .unwrap_or_else(|| panic!("no answer {id}: {answers:?}"))
    };
    assert!(
        answer(2)["result"]["diagnostics"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "{answers:?}"
    );
    let typed = answer(4)["result"]["diagnostics"]
        .as_array()
        .expect("typed diagnostics");
    let service = answer(5)["result"]["diagnostics"]
        .as_array()
        .expect("service diagnostics");
    assert!(
        service.iter().any(|d| d["code"] == 2322),
        "{answers:?}"
    );
    assert!(
        typed
            .iter()
            .any(|d| d["code"] == "ts2322" && d["line"] == 1 && d["col"] == 19),
        "{answers:?}"
    );
    assert!(typed.iter().any(|d| d["code"] == "val-mutation"), "{answers:?}");
}
