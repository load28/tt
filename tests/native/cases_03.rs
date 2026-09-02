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
