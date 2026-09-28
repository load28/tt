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

#[test]
fn a_host_source_is_addressed_as_itself() {
    require_tsgo!();
    let helper = "export function helper(n: number): number { return n; }\n";
    let host = "import { helper } from \"./m.tt\";\n\
                export type Shape = { kind: \"Circle\"; radius: number } | { kind: \"Point\" };\n\
                export function area(s: Shape): number {\n\
                \x20 if (s.kind === \"Circle\") return s.radius + helper(1);\n\
                \x20 return 0;\n\
                }\n";
    let dir = project(&[("src/m.tt", helper), ("src/plain.ts", host)]);
    let plain = dir.join("src/plain.ts").canonicalize().unwrap();
    let module = dir.join("src/m.tt").canonicalize().unwrap();
    let engine = ttc::engine::Engine::new(None);
    let mut project = engine
        .open_project(
            &[dir.join("src").to_string_lossy().into_owned()],
            &ttc::engine::ProjectOptions::default(),
        )
        .expect("the project opens");
    let field = source_location(&plain, host, "radius: number", 0, 6);
    let used = source_location(&plain, host, "radius + helper", 0, 6);
    for open in [true, false] {
        if open {
            project.open_document(plain.clone(), host.to_string());
        } else {
            project.close_document(&plain);
        }
        let position = used.range.start;
        assert_eq!(
            project.definition(&plain, position).expect("definition"),
            vec![field.clone()],
            "open: {open}"
        );
        let references: Vec<_> = project
            .references(&plain, position)
            .expect("references")
            .into_iter()
            .map(|reference| reference.location)
            .collect();
        assert_eq!(references, vec![field.clone(), used.clone()], "open: {open}");
        let edits = project
            .rename(&plain, position)
            .expect("rename")
            .expect("the field renames");
        assert!(
            edits.iter().all(|edit| edit.location.path == plain),
            "open: {open}: {edits:?}"
        );
        assert_eq!(
            apply_rename(host, &edits, "size"),
            host.replace("radius", "size"),
            "open: {open}"
        );
        assert_eq!(
            project
                .definition(&plain, source_position(host, "helper(1)", 0))
                .expect("definition"),
            vec![source_location(&module, helper, "helper", 0, 6)],
            "open: {open}"
        );
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

fn module_project(
    package_type: &str,
    module: &str,
    resolution: &str,
    verbatim: bool,
    files: &[(&str, &str)],
) -> Workspace {
    let dir = tmpdir();
    write(
        &dir,
        "package.json",
        &format!("{{ \"private\": true, \"type\": \"{package_type}\" }}\n"),
    );
    write(
        &dir,
        "tsconfig.json",
        &format!(
            r#"{{
  "compilerOptions": {{
    "target": "es2022",
    "module": "{module}",
    "moduleResolution": "{resolution}",
    "strict": true,
    "skipLibCheck": true,
    "verbatimModuleSyntax": {verbatim},
    "noEmit": true
  }},
  "include": ["src"]
}}
"#
        ),
    );
    for (name, text) in files {
        write(&dir, name, text);
    }
    dir
}

fn error_count(out: &str) -> usize {
    out.lines().filter(|line| line.starts_with("error")).count()
}

#[test]
fn variant_case_and_field_docs_reach_hover_and_signature_help() {
    require_tsgo!();
    let source = "export variant Shape {\n\
                  \x20 /** A circle around the origin. */\n\
                  \x20 Circle(radius: number), // the common case\n\
                  \x20 Rect(\n\
                  \x20   /** Width in pixels. */\n\
                  \x20   width: number,\n\
                  \x20   height: number,\n\
                  \x20 ),\n\
                  }\n\
                  export const c = Shape.Circle(1);\n\
                  declare const s: Shape;\n\
                  export const w = s.kind === \"Rect\" ? s.width : 0;\n\
                  export const r = Shape.Rect(1, 2);\n";
    let dir = project(&[("src/docs.tt", source)]);
    let file = dir.join("src/docs.tt").canonicalize().unwrap();
    let engine = ttc::engine::Engine::new(None);
    let mut project = engine
        .open_project(
            &[file.to_string_lossy().into_owned()],
            &ttc::engine::ProjectOptions::default(),
        )
        .unwrap();
    let on_constructor = project
        .hover(&file, ttc::engine::Position { line: 9, character: 24 })
        .unwrap()
        .expect("the constructor hovers");
    assert_eq!(
        on_constructor.signature,
        "(property) Circle: (radius: number) => Shape"
    );
    assert_eq!(on_constructor.documentation, "A circle around the origin.");
    let on_field = project
        .hover(&file, ttc::engine::Position { line: 11, character: 42 })
        .unwrap()
        .expect("the narrowed field hovers");
    assert_eq!(on_field.signature, "(property) width: number");
    assert_eq!(on_field.documentation, "Width in pixels.");
    let help = project
        .signature_help(&file, ttc::engine::Position { line: 12, character: 28 })
        .unwrap();
    assert!(format!("{help:?}").contains("Width in pixels."), "{help:?}");
}

#[test]
fn bigint_literal_unions_are_checked_for_exhaustiveness() {
    require_tsgo!();
    let dir = project(&[(
        "src/big.tt",
        "export function one(n: 1n | 2n) { return match (n) { 1n => \"a\" }; }\n\
         export function signed(n: -1n | 2n | 0x10n) { return match (n) { 2n => \"a\" }; }\n\
         export function full(n: -1n | 2n) { return match (n) { -1n => \"a\", 2n => \"b\" }; }\n\
         export function mixed(n: 1n | 2) { return match (n) { 2 => \"a\" }; }\n\
         export function narrowed(n: 1n | 2n) { if (n === 1n) return \"a\"; return match (n) { 2n => \"b\" }; }\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("match on literal union is not exhaustive: missing 2n\n"),
        "{out}"
    );
    assert!(out.contains("missing -1n, 16n\n"), "{out}");
    assert!(out.contains("missing 1n\n"), "{out}");
    assert!(out.contains("`, -1n => undefined, 16n => undefined,`"), "{out}");
    assert_eq!(
        out.matches("error[match-not-exhaustive]").count(),
        3,
        "{out}"
    );
}

#[test]
fn hover_documentation_and_jsdoc_tags_are_separate_from_the_signature() {
    require_tsgo!();
    let source = "/**\n\
                  \x20* Adds two numbers.\n\
                  \x20* @param a the first\n\
                  \x20* @returns the sum\n\
                  \x20*/\n\
                  export function add(a: number, b: number): number { return a + b; }\n\
                  export const r = add(1, 2);\n\
                  export const u = r;\n";
    let dir = project(&[("src/add.tt", source)]);
    let file = dir.join("src/add.tt").canonicalize().unwrap();
    let engine = ttc::engine::Engine::new(None);
    let mut project = engine
        .open_project(
            &[file.to_string_lossy().into_owned()],
            &ttc::engine::ProjectOptions::default(),
        )
        .unwrap();
    let on_call = project
        .hover(&file, ttc::engine::Position { line: 6, character: 18 })
        .unwrap()
        .expect("the call hovers");
    assert_eq!(
        on_call.signature,
        "function add(a: number, b: number): number"
    );
    assert_eq!(
        on_call.documentation,
        "Adds two numbers.\n\n*@param* `a` — the first\n\n*@returns* — the sum"
    );
    let undocumented = project
        .hover(&file, ttc::engine::Position { line: 7, character: 17 })
        .unwrap()
        .expect("the reference hovers");
    assert_eq!(undocumented.signature, "const r: number");
    assert_eq!(undocumented.documentation, "");
}

#[test]
fn the_standard_library_resolves_from_either_module_format_in_every_resolution_mode() {
    require_tsgo!();
    for (package_type, module, resolution, verbatim) in [
        ("commonjs", "node16", "node16", false),
        ("module", "node16", "node16", false),
        ("commonjs", "nodenext", "nodenext", false),
        ("module", "nodenext", "nodenext", true),
        ("module", "esnext", "bundler", true),
        ("commonjs", "preserve", "bundler", false),
        ("commonjs", "commonjs", "bundler", false),
    ] {
        let dir = module_project(
            package_type,
            module,
            resolution,
            verbatim,
            &[
                (
                    "src/s.tt",
                    "import type { TOption } from \"@tt/std\";\n\
                     import * as Option from \"@tt/std/option\";\n\
                     import * as Result from \"@tt/std/result\";\n\
                     export const o: TOption<number> = Option.fromNullable(1 as number | null);\n\
                     export const r = Result.Ok(1);\n\
                     export const n = [1, 2] |> ((xs) => xs.length);\n\
                     export const bad: string = n;\n",
                ),
                (
                    "src/u.ts",
                    "import type { TOption } from \"@tt/std\";\n\
                     export const p: TOption<string> | undefined = undefined;\n",
                ),
            ],
        );
        let out = check(&dir);
        let label = format!("{package_type}/{module}/{resolution}/{verbatim}");
        assert!(
            block(&out, "type mismatch: expected `string`").contains("--> src/s.tt"),
            "{label}: {out}"
        );
        assert_eq!(error_count(&out), 1, "{label}: {out}");
    }
}

#[test]
fn a_commonjs_file_requiring_the_standard_library_under_verbatim_module_syntax_is_clean() {
    require_tsgo!();
    for resolution in ["node16", "nodenext"] {
        let dir = module_project(
            "module",
            resolution,
            resolution,
            true,
            &[
                (
                    "src/s.tt",
                    "import * as Option from \"@tt/std/option\";\n\
                     export const o = Option.Some(1);\n\
                     export const n = [1, 2] |> ((xs) => xs.length);\n\
                     export const bad: string = n;\n",
                ),
                (
                    "src/b.cts",
                    "import Option = require(\"@tt/std/option\");\n\
                     import type { TOption } from \"@tt/std\" with { \"resolution-mode\": \"require\" };\n\
                     const legacy: TOption<number> = Option.None;\n\
                     export = { legacy };\n",
                ),
            ],
        );
        let out = check(&dir);
        assert!(
            block(&out, "type mismatch: expected `string`").contains("--> src/s.tt"),
            "{resolution}: {out}"
        );
        assert_eq!(error_count(&out), 1, "{resolution}: {out}");
    }
}

const RELATIVE_SOURCES: &[(&str, &str)] = &[
    (
        "src/a.tt",
        "export variant A { X(n: number), Y }\nexport default 7;\n",
    ),
    (
        "src/c.ts",
        "import seven, { A } from \"./a.tt\";\n\
         export const v: A = A.X(seven);\n\
         export const bad: string = seven;\n",
    ),
    (
        "src/d.tt",
        "import { A } from \"./a.tt\";\n\
         export function size(a: A): number {\n\
         \x20 return match (a) { X(n) => n, Y => 0 };\n\
         }\n",
    ),
    (
        "src/view.ttx",
        "import { A } from \"./a.tt\";\n\
         export const label = (a: A) => match (a) { X(n) => `${n}`, Y => \"y\" };\n",
    ),
];

fn with_tt_content_mapper(dir: &Workspace) {
    let config = dir.join("tsconfig.json");
    let text = fs::read_to_string(&config).unwrap().replacen(
        "\"include\"",
        "\"contentMappers\": [{ \"package\": \"@openload28/tt-lang\", \"extensions\": [\".tt\", \".ttx\"] }],\n  \"include\"",
        1,
    );
    fs::write(config, text).unwrap();
}

#[test]
fn tt_specifiers_resolve_under_node_esm_as_tsc_resolves_them() {
    require_tsgo!();
    for (module, verbatim) in [("nodenext", true), ("node16", false)] {
        let dir = module_project("module", module, module, verbatim, RELATIVE_SOURCES);
        with_tt_content_mapper(&dir);
        let out = check(&dir);
        assert!(
            block(&out, "type mismatch: expected `string`").contains("--> src/c.ts"),
            "{module}: {out}"
        );
        assert_eq!(error_count(&out), 1, "{module}: {out}");
    }
}

#[test]
fn tt_specifiers_keep_resolving_in_commonjs_and_bundler_projects() {
    require_tsgo!();
    for (package_type, module, resolution) in [
        ("commonjs", "node16", "node16"),
        ("commonjs", "nodenext", "nodenext"),
        ("commonjs", "commonjs", "bundler"),
        ("module", "esnext", "bundler"),
        ("module", "preserve", "bundler"),
    ] {
        let dir = module_project(package_type, module, resolution, false, RELATIVE_SOURCES);
        let out = check(&dir);
        let label = format!("{package_type}/{module}/{resolution}");
        assert!(
            block(&out, "type mismatch: expected `string`").contains("--> src/c.ts"),
            "{label}: {out}"
        );
        assert_eq!(error_count(&out), 1, "{label}: {out}");
    }
    let dir = module_project(
        "commonjs",
        "nodenext",
        "nodenext",
        false,
        &[
            (
                "src/legacy.tt",
                "const legacy = { n: 1 };\nexport = legacy;\n",
            ),
            (
                "src/use.ts",
                "import legacy = require(\"./legacy.tt\");\n\
                 export const n: number = legacy.n;\n\
                 export const bad: string = legacy.n;\n",
            ),
        ],
    );
    let out = check(&dir);
    assert!(
        block(&out, "type mismatch: expected `string`").contains("--> src/use.ts"),
        "{out}"
    );
    assert_eq!(error_count(&out), 1, "{out}");
}

#[test]
fn a_project_with_another_content_mapper_runs_no_external_code() {
    require_tsgo!();
    let dir = module_project("commonjs", "nodenext", "nodenext", false, RELATIVE_SOURCES);
    let config = dir.join("tsconfig.json");
    let text = fs::read_to_string(&config).unwrap().replacen(
        "\"include\"",
        "\"contentMappers\": [{ \"package\": \"foo-mapper\", \"extensions\": [\".foo\"] }],\n  \"include\"",
        1,
    );
    fs::write(config, text).unwrap();
    let out = check(&dir);
    assert!(
        block(&out, "type mismatch: expected `string`").contains("--> src/c.ts"),
        "{out}"
    );
    assert!(out.contains("ts100024"), "{out}");
    assert_eq!(error_count(&out), 2, "{out}");
}

#[test]
fn scripts_of_one_program_check_without_colliding_generated_globals() {
    require_tsgo!();
    let script = |suffix: &str| {
        format!(
            "declare const o_{suffix}: {{ kind: \"A\"; n: number }} | {{ kind: \"B\" }};\n\
             declare function step_{suffix}(n: number): number;\n\
             declare function read_{suffix}(): number;\n\
             const size_{suffix} = match (o_{suffix}) {{ A(n) => n, B => 0 }};\n\
             var total_{suffix} = match (o_{suffix}) {{ A(n) => n, B => 0 }};\n\
             function measure_{suffix}() {{ return match (o_{suffix}) {{ A(n) => n, B => 0 }}; }}\n\
             match (o_{suffix}) {{ A => 1, B => 2 }};\n\
             const A(n: first_{suffix}) = o_{suffix} else {{ throw new Error(); }};\n\
             const piped_{suffix} = read_{suffix}() |> step_{suffix};\n\
             const flowed_{suffix} = flow |> step_{suffix} |> step_{suffix};\n"
        )
    };
    let dir = project(&[
        ("src/a.tt", &script("a")),
        ("src/b.ttx", &script("b")),
        (
            "src/use.ts",
            "const sum: number = size_a + size_b + total_a + total_b + measure_a() + first_b + piped_a + flowed_b(1);\n\
             const wrong: string = size_a;\n",
        ),
    ]);
    let out = check(&dir);
    assert!(
        block(&out, "type mismatch: expected `string`").contains("--> src/use.ts"),
        "{out}"
    );
    assert_eq!(error_count(&out), 1, "{out}");
}
