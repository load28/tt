//! CLI contract tests: `ttc help` — the embedded language & workflow
//! reference (docs/ai/tt.md served by topic) — and `--jobs`, whose whole
//! contract is that parallelism changes nothing an observer can see.

use std::fs;
use std::path::Path;
use std::process::Command;

fn ttc(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(args)
        .output()
        .expect("failed to run ttc")
}

mod common;
use common::Workspace;

/// A directory for one case, removed when the case ends — and kept, with
/// its path printed, when the case failed (`tests/common/mod.rs`).
fn tmpdir() -> Workspace {
    Workspace::new("cli")
}

#[test]
fn ttx_builds_to_tsx_and_keeps_jsx() {
    let dir = tmpdir();
    let source = dir.join("view.ttx");
    let out_dir = dir.join("out");
    fs::write(
        &source,
        "variant State { Ready(value: string), Empty }\n\
         declare const state: State;\n\
         export const view = <main>{match (state) { Ready(value) => <b>{value}</b>, Empty => null }}</main>;\n",
    )
    .unwrap();
    let output = ttc(&["-o", out_dir.to_str().unwrap(), source.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = fs::read_to_string(out_dir.join("view.tsx")).unwrap();
    assert!(emitted.contains("<main>{"), "{emitted}");
    assert!(emitted.contains("switch ($tt_m.kind)"), "{emitted}");
}

#[test]
fn handwritten_tsx_is_checked_in_tsx_mode_and_keeps_its_extension() {
    let dir = tmpdir();
    let source = dir.join("main.tsx");
    let out_dir = dir.join("out");
    let text = "export const view = <main>plain TSX</main>;\n";
    fs::write(&source, text).unwrap();

    let output = ttc(&["-o", out_dir.to_str().unwrap(), dir.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = fs::read_to_string(out_dir.join("main.tsx")).unwrap();
    assert!(emitted.ends_with(text), "{emitted}");
}

#[test]
fn a_project_writes_one_pipeline_runtime_and_imports_it() {
    let dir = tmpdir();
    let source = dir.join("src");
    let out_dir = dir.join("out");
    fs::create_dir_all(&source).unwrap();
    for name in ["a", "b"] {
        fs::write(
            source.join(format!("{name}.tt")),
            format!(
                "declare function input_{name}(): number;\n\
                 declare const step_{name}: (value: number) => number;\n\
                 export const value_{name} = input_{name}() |> step_{name};\n"
            ),
        )
        .unwrap();
    }

    let output = ttc(&[
        "--no-banner",
        "-o",
        out_dir.to_str().unwrap(),
        source.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out_dir.join("tt/runtime.ts").exists());
    assert!(!out_dir.join("tt/option.ts").exists());
    for name in ["a", "b"] {
        let code = fs::read_to_string(out_dir.join(format!("{name}.ts"))).unwrap();
        assert!(
            code.contains("import { $tt_ap } from \"./tt/runtime.js\";"),
            "{code}"
        );
    }
}

/// A small project: one shared module every other file imports (the shape
/// that exercises the imported-declaration cache), plus a file that fails
/// to compile so diagnostics are part of what must stay ordered.
fn write_project(dir: &Path, files: usize) {
    fs::write(
        dir.join("shared.tt"),
        "export variant Token { Num(value: number), Word(text: string), Eof }\n",
    )
    .unwrap();
    for n in 0..files {
        fs::write(
            dir.join(format!("m{n}.tt")),
            format!(
                "import {{ Token }} from \"./shared.tt\";\n\
                 export const n{n} = {n};\n\
                 export function name{n}(t: Token): string {{\n\
                 \x20 return match (t) {{ Num(value) => `${{value}}`, Word(text) => text, Eof => \"\" }};\n\
                 }}\n"
            ),
        )
        .unwrap();
    }
    // one non-exhaustive match: its error must appear in the same place
    // however many threads ran
    fs::write(
        dir.join("bad.tt"),
        "import { Token } from \"./shared.tt\";\n\
         export const broken = (t: Token) => match (t) { Eof => 0 };\n",
    )
    .unwrap();
}

/// What one `ttc` run leaves behind: the files it wrote (name → content,
/// sorted), its diagnostics, and whether it succeeded.
type RunResult = (Vec<(String, String)>, String, bool);

#[test]
fn jobs_does_not_change_outputs_or_diagnostics() {
    let src = tmpdir();
    write_project(&src, 12);

    let mut baseline: Option<RunResult> = None;
    for jobs in ["1", "2", "3", "8"] {
        let out = tmpdir();
        let result = ttc(&[
            "-j",
            jobs,
            "-o",
            out.to_str().unwrap(),
            src.to_str().unwrap(),
        ]);
        let mut written: Vec<(String, String)> = fs::read_dir(&out)
            .unwrap()
            .map(|e| {
                let path = e.unwrap().path();
                (
                    path.file_name().unwrap().to_string_lossy().into_owned(),
                    fs::read_to_string(&path).unwrap(),
                )
            })
            .collect();
        written.sort();
        let stderr = String::from_utf8(result.stderr)
            .unwrap()
            .replace(out.to_str().unwrap(), "<out>");
        let observed = (written, stderr, result.status.success());
        match &baseline {
            None => baseline = Some(observed),
            Some(expected) => assert_eq!(*expected, observed, "-j {jobs} diverged"),
        }
    }
    // the run really did compile something, and really did report the error
    let (written, stderr, success) = baseline.unwrap();
    assert!(!success, "the non-exhaustive match should fail the run");
    assert!(written.iter().any(|(name, _)| name == "m0.ts"));
    assert!(stderr.contains("not exhaustive"), "{stderr}");
}

#[test]
fn jobs_rejects_zero_and_garbage() {
    for value in ["0", "many", "-1"] {
        let out = ttc(&["-j", value, "--check", "examples"]);
        assert!(!out.status.success(), "--jobs {value} should be rejected");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr.contains("--jobs expects a positive number"),
            "{stderr}"
        );
    }
    let out = ttc(&["--jobs"]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("--jobs requires a value")
    );
}

#[test]
fn help_lists_every_topic() {
    let out = ttc(&["help"]);
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    for topic in [
        "overview",
        "variant",
        "match",
        "try",
        "let-else",
        "if-let",
        "pipe",
        "std",
        "modules",
        "install",
        "setup",
        "workflow",
        "errors",
        "checklist",
    ] {
        assert!(stdout.contains(topic), "topic list missing {topic}");
        let out = ttc(&["help", topic]);
        assert!(out.status.success(), "ttc help {topic} failed");
        assert!(!out.stdout.is_empty(), "ttc help {topic} printed nothing");
    }
}

#[test]
fn help_topic_prints_only_its_section() {
    let out = ttc(&["help", "match"]);
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.starts_with("## match"));
    assert!(stdout.contains("or-pattern"));
    assert!(!stdout.contains("\n## try"), "leaked into the next section");
}

#[test]
fn help_resolves_aliases_case_insensitively() {
    let out = ttc(&["help", "Pipeline"]);
    assert!(out.status.success());
    assert!(String::from_utf8(out.stdout).unwrap().starts_with("## |>"));
}

#[test]
fn help_all_prints_the_whole_guide() {
    let out = ttc(&["help", "all"]);
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout, include_str!("../docs/ai/tt.md"));
}

#[test]
fn help_unknown_topic_fails_with_a_pointer() {
    let out = ttc(&["help", "nosuch"]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty(), "errors must not pollute stdout");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("unknown help topic \"nosuch\""));
    assert!(stderr.contains("ttc help"));
}

#[test]
fn help_only_triggers_as_the_first_argument() {
    // `ttc --check help` must treat "help" as an input path, not a command.
    let out = ttc(&["--check", "help"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("no such file or directory"), "{stderr}");
}

/* ------------------------------------------------------------------ */
/* --check-types: what only the real checker can answer                */
/* ------------------------------------------------------------------ */

fn have(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Whether ttc can resolve a TypeScript to drive. Asked by running the mode
/// itself over a trivial project: the answer is ttc's own resolution, not a
/// guess about the machine.
fn have_typescript() -> bool {
    let dir = tmpdir();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/probe.tt"), "export const n: number = 1;\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["--check-types", "src"])
        .current_dir(&dir)
        .output()
        .expect("failed to run ttc");
    out.status.success()
}

/// Runs `ttc --check-types` over a one-file project and returns ttc's
/// stderr. Nothing is written, so a released TypeScript 7 — which cannot
/// emit declarations — answers these just as well as a built one.
fn types_stderr(source: &str) -> String {
    let dir = tmpdir();
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("main.tt"), source).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["--check-types", "src"])
        .current_dir(&dir)
        .output()
        .expect("failed to run ttc");
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// [`types_stderr`], but the file's text arrives as an editor's unsaved
/// buffer: `saved` is written to disk, `buffer` goes on stdin under
/// `--overlay`, and the check is what an editor would run.
fn types_stderr_overlay(saved: &str, buffer: &str, tt_only: bool) -> String {
    use std::io::Write;
    let dir = tmpdir();
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();
    let file = src.join("main.tt");
    fs::write(&file, saved).unwrap();

    let mut args = vec!["--check-types".to_string()];
    if tt_only {
        args.push("--tt-only".to_string());
    }
    args.push("--overlay".to_string());
    args.push(file.to_str().unwrap().to_string());
    args.push("src".to_string());

    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(&args)
        .current_dir(&dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to run ttc");
    child
        .stdin
        .take()
        .expect("stdin piped")
        .write_all(buffer.as_bytes())
        .unwrap();
    let out = child.wait_with_output().expect("failed to run ttc");
    String::from_utf8_lossy(&out.stderr).into_owned()
}

macro_rules! require_types_toolchain {
    () => {
        if !have("node") || !have_typescript() {
            eprintln!("skipping: no node, or no TypeScript for ttc to drive");
            return;
        }
    };
}

#[test]
fn types_reports_a_missing_literal_of_a_finite_union() {
    require_types_toolchain!();
    let err = types_stderr(
        "type Direction = \"north\" | \"south\";\n\
         export function short(dir: Direction) {\n\
         \x20 return match (dir) { \"north\" => \"N\" };\n\
         }\n",
    );
    assert!(
        err.contains("match on literal union is not exhaustive: missing \"south\""),
        "{err}"
    );
    // reported at the `match` keyword of the .tt source, not in the
    // generated TypeScript
    assert!(err.contains("--> src/main.tt:3:10"), "{err}");
}

#[test]
fn types_is_silent_when_the_literal_match_is_exhaustive() {
    require_types_toolchain!();
    let err = types_stderr(
        "type Direction = \"north\" | \"south\";\n\
         export function short(dir: Direction) {\n\
         \x20 return match (dir) { \"north\" => \"N\", \"south\" => \"S\" };\n\
         }\n",
    );
    assert!(!err.contains("not exhaustive"), "{err}");
}

#[test]
fn types_does_not_guess_when_the_scrutinee_type_is_open() {
    require_types_toolchain!();
    // string / number / unknown / any / a type parameter / a widened union
    // are not finite literal sets — no diagnostic, by design.
    let err = types_stderr(
        "export const a = (x: string) => match (x) { \"a\" => 1, \"b\" => 2 };\n\
         export const b = (x: number) => match (x) { 1 => 1, 2 => 2 };\n\
         export const c = (x: unknown) => match (x) { \"a\" => 1 };\n\
         export const d = (x: any) => match (x) { \"a\" => 1 };\n\
         export const e = <T extends string>(x: T) => match (x) { \"a\" => 1 };\n\
         export const f = (x: \"a\" | string) => match (x) { \"a\" => 1 };\n\
         export const g = (x: string | number) => match (x) { \"a\" => 1 };\n",
    );
    assert!(!err.contains("not exhaustive"), "{err}");
}

#[test]
fn types_checks_a_union_derived_from_as_const() {
    require_types_toolchain!();
    // The kind of type ttc could never resolve on its own — the checker can.
    let err = types_stderr(
        "const values = [\"north\", \"south\"] as const;\n\
         type D = (typeof values)[number];\n\
         export const pick = (x: D) => match (x) { \"north\" => 1 };\n",
    );
    assert!(err.contains("not exhaustive: missing \"south\""), "{err}");
}

#[test]
fn types_skips_a_literal_match_with_a_wildcard() {
    require_types_toolchain!();
    let err = types_stderr(
        "type D = \"north\" | \"south\";\n\
         export const pick = (x: D) => match (x) { \"north\" => 1, _ => 0 };\n",
    );
    assert!(!err.contains("not exhaustive"), "{err}");
}

#[test]
fn types_does_not_count_a_guarded_arm_as_covering() {
    require_types_toolchain!();
    let err = types_stderr(
        "export const pick = (x: \"a\" | \"b\", ok: boolean) =>\n\
         \x20 match (x) { \"a\" if ok => 1, \"b\" => 2 };\n",
    );
    assert!(err.contains("not exhaustive: missing \"a\""), "{err}");
}

#[test]
fn types_checks_boolean_and_number_unions() {
    require_types_toolchain!();
    let err = types_stderr(
        "export const b = (x: boolean) => match (x) { true => 1 };\n\
         export const n = (x: 200 | 404) => match (x) { 200 => 1 };\n",
    );
    assert!(err.contains("not exhaustive: missing false"), "{err}");
    assert!(err.contains("not exhaustive: missing 404"), "{err}");
}

#[test]
fn types_maps_a_bad_case_literal_back_to_the_tt_source() {
    require_types_toolchain!();
    // The `case` label is copied from the source, so tsc's complaint about
    // it lands on the literal the user wrote.
    let err = types_stderr(
        "export const pick = (x: \"a\" | \"b\") => match (x) { \"a\" => 1, \"c\" => 2, \"b\" => 3 };\n",
    );
    assert!(err.contains("--> src/main.tt:1:61"), "{err}");
    assert!(err.contains("is not comparable to type"), "{err}");
}

/* ------------------------------------------------------------------ */
/* --types: typed mutation for `val` (TASK-071)                        */
/* ------------------------------------------------------------------ */

#[test]
fn types_reports_a_mutating_method_of_a_built_in() {
    require_types_toolchain!();
    let err = types_stderr(
        "val const map = new Map<string, number>();\n\
         map.set(\"a\", 1);\n",
    );
    assert!(
        err.contains("cannot call mutating method `set` through val binding `map`"),
        "{err}"
    );
    // reported at the path's root in the .tt source
    assert!(err.contains("--> src/main.tt:2:1"), "{err}");
}

#[test]
fn types_reports_set_add_and_array_push() {
    require_types_toolchain!();
    let err = types_stderr(
        "val const set = new Set<number>();\n\
         set.add(1);\n\
         val const items: number[] = [];\n\
         items.push(1);\n\
         val const state = { tags: [] as string[] };\n\
         state.tags.push(\"tt\");\n",
    );
    assert!(
        err.contains("mutating method `add` through val binding `set`"),
        "{err}"
    );
    assert!(
        err.contains("mutating method `push` through val binding `items`"),
        "{err}"
    );
    // ... and at any depth of the access path
    assert!(
        err.contains("mutating method `push` through val binding `state`"),
        "{err}"
    );
}

#[test]
fn types_leaves_a_user_defined_method_of_the_same_name_alone() {
    require_types_toolchain!();
    // The whole point: `set`/`add`/`push` on a user-defined type are not
    // mutations, and ttc must not guess otherwise from the name.
    let err = types_stderr(
        "class Query {\n\
         \x20 set(key: string): Query {\n\
         \x20   return new Query();\n\
         \x20 }\n\
         }\n\
         class Collection {\n\
         \x20 add(value: number): Collection {\n\
         \x20   return new Collection();\n\
         \x20 }\n\
         \x20 push(value: number): Collection {\n\
         \x20   return new Collection();\n\
         \x20 }\n\
         }\n\
         val const query = new Query();\n\
         query.set(\"name\");\n\
         val const collection = new Collection();\n\
         collection.add(1);\n\
         collection.push(2);\n",
    );
    assert!(!err.contains("mutating method"), "{err}");
}

#[test]
fn types_does_not_guess_when_the_receiver_is_unknown() {
    require_types_toolchain!();
    // No resolvable receiver — `any`, a type parameter, an unresolved
    // import — is left alone: a false positive costs more than a miss.
    let err = types_stderr(
        "declare function getSomething(): any;\n\
         val const value = getSomething();\n\
         value.set(\"x\");\n\
         export function shift<T extends { push(v: number): void }>(val box: T) {\n\
         \x20 box.push(1);\n\
         }\n",
    );
    assert!(!err.contains("mutating method"), "{err}");
}

#[test]
fn types_keeps_reporting_syntactic_mutation_without_the_checker() {
    require_types_toolchain!();
    // The syntactic half is ttc's own and fires before the host runs.
    let err = types_stderr(
        "val const user = {\n\
         \x20 profile: {\n\
         \x20   name: \"A\",\n\
         \x20 },\n\
         };\n\
         user.profile.name = \"B\";\n",
    );
    assert!(
        err.contains("cannot mutate through val binding `user`"),
        "{err}"
    );
    assert!(err.contains("--> src/main.tt:6:1"), "{err}");
}

/* ------------------------------------------------------------------ */
/* --overlay / --tt-only: the editor's entry into the typed check      */
/* ------------------------------------------------------------------ */

/// Both flags only make sense while `--check-types` is reporting. Every
/// rejection names the mode that does accept them, so the message is a fix
/// rather than a complaint.
#[test]
fn overlay_and_tt_only_require_check_types() {
    let dir = tmpdir();
    let file = dir.join("a.tt");
    fs::write(&file, "export const n = 1;\n").unwrap();
    let path = file.to_str().unwrap();

    for args in [
        vec!["--overlay", path, path],
        vec!["--tt-only", path],
        vec!["--check", "--tt-only", path],
    ] {
        let out = ttc(&args);
        let err = String::from_utf8_lossy(&out.stderr).into_owned();
        assert!(!out.status.success(), "{args:?} should fail:\n{err}");
        assert!(
            err.contains("--overlay and --tt-only require --check-types"),
            "{args:?}:\n{err}"
        );
    }
}

/// `--types` writes sidecars. Unsaved text must not reach one, and a mode
/// that writes is not one that hides half of what it found.
#[test]
fn overlay_and_tt_only_are_rejected_by_the_writing_mode() {
    let dir = tmpdir();
    let file = dir.join("a.tt");
    fs::write(&file, "export const n = 1;\n").unwrap();
    let path = file.to_str().unwrap();

    for args in [
        vec!["--types", "--overlay", path, path],
        vec!["--types", "--tt-only", path],
    ] {
        let out = ttc(&args);
        let err = String::from_utf8_lossy(&out.stderr).into_owned();
        assert!(!out.status.success(), "{args:?} should fail:\n{err}");
        assert!(
            err.contains("--overlay and --tt-only work with --check-types, not --types"),
            "{args:?}:\n{err}"
        );
    }
}

/// A watch re-reads what it watches; text pinned on stdin would stay the
/// same forever, so the pair has no coherent meaning.
#[test]
fn overlay_does_not_combine_with_watch() {
    let dir = tmpdir();
    let file = dir.join("a.tt");
    fs::write(&file, "export const n = 1;\n").unwrap();
    let path = file.to_str().unwrap();

    let out = ttc(&["--check-types", "--overlay", path, "--watch", path]);
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(!out.status.success(), "{err}");
    assert!(
        err.contains("--overlay does not combine with --watch"),
        "{err}"
    );
}

/// The flag needs a value, and the path it names has to exist — it stands
/// in for a file of the project, so there has to be one.
#[test]
fn overlay_reports_a_missing_value_and_a_missing_file() {
    let out = ttc(&["--check-types", "--overlay"]);
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(!out.status.success(), "{err}");
    assert!(
        err.contains("--overlay requires the path the buffer belongs to"),
        "{err}"
    );

    let dir = tmpdir();
    let file = dir.join("a.tt");
    fs::write(&file, "export const n = 1;\n").unwrap();
    let gone = dir.join("gone.tt");
    let out = ttc(&[
        "--check-types",
        "--overlay",
        gone.to_str().unwrap(),
        file.to_str().unwrap(),
    ]);
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(!out.status.success(), "{err}");
    assert!(err.contains("--overlay"), "{err}");
    assert!(err.contains("gone.tt"), "{err}");
}

/// The overlay is what gets checked: a mutation that exists only in the
/// buffer is reported, and one that exists only on disk is not. This is the
/// whole point — an editor asks about the text it is showing.
#[test]
fn overlay_checks_the_buffer_rather_than_the_saved_file() {
    require_types_toolchain!();
    let saved = "val const saved = new Map<string, number>();\n\
                 saved.set(\"gone\", 1);\n\
                 export const n = saved.size;\n";
    let buffer = "val const edited = new Map<string, number>();\n\
                  edited.delete(\"new\");\n\
                  export const n = edited.size;\n";

    let err = types_stderr_overlay(saved, buffer, true);
    assert!(
        err.contains("cannot call mutating method `delete` through val binding `edited`"),
        "the buffer's mutation should be reported:\n{err}"
    );
    assert!(
        !err.contains("saved"),
        "the saved file's text should not be checked:\n{err}"
    );
    // The position is the buffer's, and the file is named as the user knows
    // it — not as a temporary.
    assert!(err.contains("--> src/main.tt:2:1"), "{err}");
}

/// `--tt-only` drops TypeScript's layer and keeps tt's. The editor uses this
/// form when its type-diagnostic setting is disabled.
#[test]
fn tt_only_keeps_the_tt_layer_and_drops_the_type_layer() {
    require_types_toolchain!();
    let source = "val const scores = new Map<string, number>();\n\
                  scores.set(\"a\", 1);\n\
                  const wrong: number = \"not a number\";\n\
                  export const n = scores.size + wrong;\n";

    let full = types_stderr_overlay(source, source, false);
    assert!(
        full.contains("type mismatch: expected `number`, found `\"not a number\"`"),
        "{full}"
    );
    assert!(full.contains("cannot call mutating method `set`"), "{full}");

    let tt_only = types_stderr_overlay(source, source, true);
    assert!(
        !tt_only.contains("type mismatch:"),
        "no type error should survive --tt-only:\n{tt_only}"
    );
    assert!(
        tt_only.contains("cannot call mutating method `set`"),
        "{tt_only}"
    );
}

/// A `val` mutation is judged by what the receiver *is*, and the overlay
/// keeps the buffer in its own project — so a type that comes from another
/// module of the project still resolves.
#[test]
fn overlay_keeps_the_buffer_in_its_project() {
    require_types_toolchain!();
    let dir = tmpdir();
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("store.ts"),
        "export const scores = new Map<string, number>();\n\
         export class Query { set(k: string): Query { return this; } }\n",
    )
    .unwrap();
    let file = src.join("main.tt");
    fs::write(&file, "export const nothing = 0;\n").unwrap();

    let buffer = "import { scores, Query } from \"./store\";\n\
                  val const shared = scores;\n\
                  shared.set(\"a\", 1);\n\
                  val const query = new Query();\n\
                  query.set(\"b\");\n\
                  export const n = shared.size;\n";

    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args([
            "--check-types",
            "--tt-only",
            "--overlay",
            file.to_str().unwrap(),
            "src",
        ])
        .current_dir(&dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to run ttc");
    child
        .stdin
        .take()
        .expect("stdin piped")
        .write_all(buffer.as_bytes())
        .unwrap();
    let out = child.wait_with_output().expect("failed to run ttc");
    let err = String::from_utf8_lossy(&out.stderr).into_owned();

    // `Map#set` through the imported binding is a built-in mutation …
    assert!(
        err.contains("cannot call mutating method `set` through val binding `shared`"),
        "{err}"
    );
    // … and `Query#set`, which only shares the name, is not.
    assert!(!err.contains("`query`"), "{err}");
}

/// `ttc --server` answers `ttSymbol` without a project or a toolchain: the
/// names it resolves exist only in `.tt` source, so nothing else can.
#[test]
fn the_server_resolves_tt_names_without_a_toolchain() {
    use std::io::Write;
    let dir = tmpdir();
    let file = dir.join("shape.tt");
    let source = "variant Shape { Circle(radius: number), Point }\n\
                  const a = match (s) { Circle(radius) => radius, Point => 0 };\n";
    fs::write(&file, source).unwrap();

    let request = serde_json::json!({
        "id": 1,
        "method": "ttSymbol",
        "params": {
            "path": file.to_string_lossy(),
            "text": source,
            // line 1, on the `Circle` of the match arm
            "position": { "line": 1, "character": 22 },
        },
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .arg("--server")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("server starts");
    writeln!(child.stdin.as_mut().unwrap(), "{request}").unwrap();
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("server answers");
    let answer: serde_json::Value =
        serde_json::from_slice(String::from_utf8_lossy(&out.stdout).trim().as_bytes())
            .expect("one JSON line");
    assert_eq!(answer["result"]["kind"], "case");
    assert_eq!(answer["result"]["variantName"], "Shape");
    assert!(answer["result"].get("enumName").is_none());
    assert_eq!(
        answer["result"]["signature"],
        "Shape.Circle(radius: number)"
    );
    // ...and points at the declaration on line 0.
    assert_eq!(answer["result"]["definition"]["range"]["start"]["line"], 0);
}

/* ------------------------------------------------------------------ */
/* typed check without a backend (TASK-124)                            */
/* ------------------------------------------------------------------ */

#[test]
fn a_missing_backend_still_reports_tt_diagnostics() {
    // The TypeScript layer failing to run removes the typed facts, not
    // the pass: tt's own diagnostics are still reported in full, the
    // failure is named, and the exit code stays "could not check" (2).
    let dir = tmpdir();
    fs::write(
        dir.join("a.tt"),
        "variant E { A(x: number), B }\n\
         const v = match (E.A(1)) { A(x) => x, A(x) => 0, B => 1 };\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["--check-types", dir.to_str().unwrap()])
        // Point the toolchain at nothing: the backend cannot run.
        .env("TTC_TSGO_API", dir.join("nonexistent-api.js"))
        .env_remove("TTC_TSGO_ROOT")
        .output()
        .expect("failed to run ttc");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("duplicate arm"), "{stderr}");
    assert!(
        stderr.contains("only tt-level diagnostics are shown"),
        "{stderr}"
    );
}

#[test]
fn a_build_writes_no_source_map_unless_it_is_asked_for() {
    // TASK-200: emitting a map appends a `sourceMappingURL` line, and a
    // hand-written `.ts` passes through byte for byte by contract — so the
    // default has to be off.
    let dir = tmpdir();
    let source = dir.join("a.tt");
    let out_dir = dir.join("out");
    fs::write(
        &source,
        "variant E { A(v: number), B }\nexport const n = match (E.B) { A(v) => v, B => 0 };\n",
    )
    .unwrap();
    let out = ttc(&[
        "-o",
        out_dir.to_str().unwrap(),
        "--no-banner",
        source.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "{out:?}");
    let code = fs::read_to_string(out_dir.join("a.ts")).unwrap();
    assert!(!code.contains("sourceMappingURL"), "{code}");
    assert!(!out_dir.join("a.ts.map").exists());
}

#[test]
fn a_source_map_file_lands_beside_its_output_and_names_the_tt_source() {
    let dir = tmpdir();
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let source = src_dir.join("a.tt");
    let out_dir = dir.join("out");
    fs::write(
        &source,
        "variant E { A(v: number), B }\nexport const n = match (E.B) { A(v) => v, B => 0 };\n",
    )
    .unwrap();
    let out = ttc(&[
        "-o",
        out_dir.to_str().unwrap(),
        "--source-map",
        "file",
        "--no-banner",
        source.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "{out:?}");
    let code = fs::read_to_string(out_dir.join("a.ts")).unwrap();
    assert!(code.ends_with("//# sourceMappingURL=a.ts.map\n"), "{code}");
    let map = fs::read_to_string(out_dir.join("a.ts.map")).unwrap();
    assert!(map.contains("\"version\":3"), "{map}");
    assert!(map.contains("\"file\":\"a.ts\""), "{map}");
    // The map sits in `out/`, the source in `src/`; the name it records has
    // to resolve from the map's own directory even on a first build, when
    // `out/` did not exist while the map was being built.
    assert!(map.contains("\"sources\":[\"../src/a.tt\"]"), "{map}");
    assert!(map.contains("\"sourcesContent\""), "{map}");
}

#[test]
fn an_inline_source_map_travels_with_printed_output() {
    let dir = tmpdir();
    let source = dir.join("a.tt");
    fs::write(
        &source,
        "variant E { A(v: number), B }\nexport const n = match (E.B) { A(v) => v, B => 0 };\n",
    )
    .unwrap();
    let out = ttc(&[
        "-p",
        "--no-banner",
        "--source-map",
        "inline",
        source.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "{out:?}");
    let code = String::from_utf8(out.stdout).unwrap();
    let marker = "//# sourceMappingURL=data:application/json;charset=utf-8;base64,";
    let at = code.find(marker).expect("inline map");
    let encoded = code[at + marker.len()..].trim_end();
    let json = String::from_utf8(base64_decode(encoded)).expect("utf-8 map");
    assert!(json.contains("\"version\":3"), "{json}");
    assert!(json.contains("a.tt"), "{json}");
}

/// Minimal Base64 decoder for the inline-map test — the test decodes what
/// the compiler encoded rather than comparing encoded text.
fn base64_decode(text: &str) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut bits = 0u32;
    let mut count = 0u32;
    let mut out = Vec::new();
    for byte in text.bytes().filter(|b| *b != b'=') {
        let value = ALPHABET
            .iter()
            .position(|c| *c == byte)
            .unwrap_or_else(|| panic!("not base64: {byte}")) as u32;
        bits = (bits << 6) | value;
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
        }
    }
    out
}

#[test]
fn a_pass_through_file_keeps_its_bytes_even_when_maps_are_on() {
    // Invariant 1: a valid `.ts` is copied byte for byte. There is no
    // translation for a map to describe, so none is written.
    let dir = tmpdir();
    let source = dir.join("plain.ts");
    let out_dir = dir.join("out");
    let text = "export const x: number = 1;\n";
    fs::write(&source, text).unwrap();
    let out = ttc(&[
        "-o",
        out_dir.to_str().unwrap(),
        "--source-map",
        "file",
        "--no-banner",
        source.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(fs::read_to_string(out_dir.join("plain.ts")).unwrap(), text);
    assert!(!out_dir.join("plain.ts.map").exists());
}

#[test]
fn a_banner_shifts_the_map_so_positions_still_line_up() {
    let dir = tmpdir();
    let source = dir.join("a.tt");
    let out_dir = dir.join("out");
    fs::write(
        &source,
        "variant E { A(v: number), B }\nexport const n = match (E.B) { A(v) => v, B => 0 };\n",
    )
    .unwrap();
    let with_banner = ttc(&[
        "-o",
        out_dir.to_str().unwrap(),
        "--source-map",
        "file",
        source.to_str().unwrap(),
    ]);
    assert!(with_banner.status.success(), "{with_banner:?}");
    let map = fs::read_to_string(out_dir.join("a.ts.map")).unwrap();
    let mappings = map
        .split("\"mappings\":\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("mappings");
    // The banner is one generated line with nothing behind it.
    assert!(mappings.starts_with(';'), "{mappings}");
    assert!(!mappings.starts_with(";;"), "{mappings}");
}

/* ------------------------------------------------------------------ */
/* ttc explain                                                        */
/* ------------------------------------------------------------------ */

#[test]
fn explain_prints_the_rule_behind_a_code() {
    let out = ttc(&["explain", "match-not-exhaustive"]);
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("error[match-not-exhaustive]"), "{text}");
    assert!(text.contains("does not cover every case"), "{text}");
    // Longer than the message it explains — that is the point of it.
    assert!(text.lines().count() > 4, "{text}");
}

#[test]
fn explain_accepts_a_code_pasted_from_a_build_log() {
    // What a reader copies is `error[val-mutation]`, brackets and all.
    let out = ttc(&["explain", "error[val-mutation]"]);
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("error[val-mutation]"), "{text}");
}

#[test]
fn explain_with_no_code_lists_every_rule() {
    let out = ttc(&["explain"]);
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    for code in ttc::DiagnosticCode::ALL {
        assert!(
            text.contains(code.as_str()),
            "{} missing:\n{text}",
            code.as_str()
        );
    }
}

#[test]
fn explain_names_the_list_when_the_code_is_unknown() {
    let out = ttc(&["explain", "no-such-rule"]);
    assert!(!out.status.success(), "{out:?}");
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("unknown diagnostic code"), "{err}");
    assert!(err.contains("ttc explain"), "{err}");
}

/* ------------------------------------------------------------------ */
/* rendered diagnostics                                               */
/* ------------------------------------------------------------------ */

#[test]
fn a_diagnostic_is_rendered_with_its_rule_position_snippet_and_fix() {
    // The whole user-visible contract of a tt error, in one place: the
    // rule's code names it, `-->` places it, the snippet quotes the line
    // the reader has to change, the carets cover the construct as written,
    // and `= help:` says what to write instead.
    let dir = tmpdir();
    let source = dir.join("shapes.tt");
    fs::write(
        &source,
        "variant Shape { Circle(radius: number), Empty }\nconst a = match (s) { Circel(radius) => radius, Empty => 0 };\n",
    )
    .unwrap();
    let out = ttc(&["--check", source.to_str().unwrap()]);
    assert!(!out.status.success(), "expected a failing exit code");
    let err = String::from_utf8(out.stderr).unwrap();
    let rendered: Vec<&str> = err.lines().map(|line| line.trim_end()).collect();

    assert_eq!(
        rendered[0],
        "error[unknown-case]: variant Shape has no case `Circel`",
    );
    assert!(rendered[1].ends_with("shapes.tt:2:23"), "{err}");
    assert!(rendered[1].trim_start().starts_with("-->"), "{err}");
    assert_eq!(rendered[2], "  |");
    assert_eq!(
        rendered[3],
        "2 | const a = match (s) { Circel(radius) => radius, Empty => 0 };",
    );
    assert_eq!(
        rendered[4], "  |                       ^^^^^^",
        "the carets cover the tag as written\n{err}",
    );
    assert_eq!(rendered[5], "  |");
    assert_eq!(
        rendered[6],
        "  = help: a case with a similar name exists: `Circle`",
    );
}

#[test]
fn the_rendered_code_is_the_one_explain_answers_to() {
    // A reader's path out of a diagnostic: read the code off the header,
    // paste it into `ttc explain`. That only works if they are the same
    // string, so this pins the round trip rather than each half.
    let dir = tmpdir();
    let source = dir.join("holes.tt");
    fs::write(
        &source,
        "variant Shape { Circle(r: number), Empty }\nconst a = match (s) { Circle(r) => r };\n",
    )
    .unwrap();
    let out = ttc(&["--check", source.to_str().unwrap()]);
    let err = String::from_utf8(out.stderr).unwrap();
    let code = err
        .split_once("error[")
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(code, _)| code.to_string())
        .unwrap_or_else(|| panic!("no rendered code:\n{err}"));
    assert_eq!(code, "match-not-exhaustive");

    let explained = ttc(&["explain", &code]);
    assert!(explained.status.success(), "`ttc explain {code}` failed");
}

/* ------------------------------------------------------------------ */
/* the panic safety net (TASK-214)                                    */
/* ------------------------------------------------------------------ */

/// `ttc` with the environment a test needs. `TTC_PANIC_FOR_TEST` makes a
/// debug build fail at a named point, which is the only way to observe
/// what the compiler does when the compiler itself is wrong.
fn ttc_failing_at(point: &str, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(args)
        .env("TTC_PANIC_FOR_TEST", point)
        // The report offers a backtrace; a test reads the report.
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("failed to run ttc")
}

#[test]
fn a_compiler_bug_is_reported_as_a_bug_and_names_the_file() {
    let dir = tmpdir();
    let source = dir.join("main.tt");
    fs::write(&source, "const a = 1;\n").unwrap();
    let out = ttc_failing_at("compile", &["--check", source.to_str().unwrap()]);

    // 101 is what a Rust panic exits with; keeping it means a caller that
    // already distinguishes "crashed" from "your code is wrong" still can.
    assert_eq!(out.status.code(), Some(101), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.starts_with("error: internal compiler error:"),
        "the report has to lead, not a backtrace: {err}"
    );
    assert!(
        err.contains(&format!("while compiling: {}", source.display())),
        "the report names the file it was working on: {err}"
    );
    assert!(
        err.contains("This is a bug in ttc, not in the code it was given"),
        "a reader must not think their own file is at fault: {err}"
    );
    assert!(err.contains("github.com/load28/tt/issues"), "{err}");
    assert!(
        err.contains("RUST_BACKTRACE=1"),
        "and must be told how to attach a backtrace: {err}"
    );
}

#[test]
fn the_server_answers_a_failed_request_and_keeps_the_session() {
    // The protocol promises a failed request never ends the session. A
    // panic is a failed request, so it may not be the exception.
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .arg("--server")
        .env("TTC_PANIC_FOR_TEST", "server")
        .env_remove("RUST_BACKTRACE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn the server");
    let requests = "{\"id\":1,\"method\":\"check\",\"params\":{\"text\":\"const a = 1;\\n\"}}\n\
                    {\"id\":2,\"method\":\"check\",\"params\":{\"text\":\"const b = 2;\\n\"}}\n";
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(requests.as_bytes())
        .unwrap();
    drop(child.stdin.take());
    let out = child.wait_with_output().unwrap();

    // Both questions were answered, in order, each with its own id — the
    // second one is the whole point: the session outlived the first panic.
    let stdout = String::from_utf8_lossy(&out.stdout);
    let answers: Vec<&str> = stdout.lines().collect();
    assert_eq!(answers.len(), 2, "both requests answered: {stdout}");
    for (index, answer) in answers.iter().enumerate() {
        let value: serde_json::Value = serde_json::from_str(answer).unwrap();
        assert_eq!(value["id"], serde_json::json!(index + 1), "{answer}");
        assert!(
            value["error"]
                .as_str()
                .unwrap_or_default()
                .contains("internal compiler error"),
            "{answer}"
        );
    }
    // Stdin closing is the only thing that ends the session.
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    // And the bug still reached a human, on the stream that is not the
    // protocol's.
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        err.matches("This is a bug in ttc").count(),
        2,
        "one report per panic: {err}"
    );
}
