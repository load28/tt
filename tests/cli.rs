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

#[test]
fn a_mixed_source_stem_collision_is_rejected_before_writing() {
    let dir = tmpdir();
    let source = dir.join("src");
    let out_dir = dir.join("out");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("model.tt"),
        "export variant Model { Tt(value: string) }\n",
    )
    .unwrap();
    fs::write(
        source.join("model.ts"),
        "export const source = \"typescript\";\n",
    )
    .unwrap();
    fs::write(
        source.join("view.ttx"),
        "export const source = <main>ttx</main>;\n",
    )
    .unwrap();
    fs::write(
        source.join("view.tsx"),
        "export const source = <main>tsx</main>;\n",
    )
    .unwrap();

    let output = ttc(&["-o", out_dir.to_str().unwrap(), source.to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("model.ts: multiple inputs claim this output"),
        "{stderr}"
    );
    assert!(stderr.contains("model.tt"), "{stderr}");
    assert!(stderr.contains("model.ts"), "{stderr}");
    assert!(
        stderr.contains("view.tsx: multiple inputs claim this output"),
        "{stderr}"
    );
    assert!(stderr.contains("view.ttx"), "{stderr}");
    assert!(stderr.contains("view.tsx"), "{stderr}");
    assert!(!out_dir.join("model.ts").exists());
    assert!(!out_dir.join("view.tsx").exists());
}

#[test]
fn separate_input_roots_cannot_collapse_to_one_output() {
    let dir = tmpdir();
    let left = dir.join("left");
    let right = dir.join("right");
    let out_dir = dir.join("out");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();
    fs::write(left.join("index.tt"), "export const side = \"left\";\n").unwrap();
    fs::write(right.join("index.tt"), "export const side = \"right\";\n").unwrap();

    let output = ttc(&[
        "-o",
        out_dir.to_str().unwrap(),
        left.to_str().unwrap(),
        right.to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("index.ts: multiple inputs claim this output"),
        "{stderr}"
    );
    assert!(stderr.contains("left/index.tt"), "{stderr}");
    assert!(stderr.contains("right/index.tt"), "{stderr}");
    assert!(!out_dir.join("index.ts").exists());
}

#[test]
fn a_source_cannot_claim_a_compiler_support_module_output() {
    let dir = tmpdir();
    let source = dir.join("src");
    let out_dir = dir.join("out");
    fs::create_dir_all(source.join("tt")).unwrap();
    fs::write(
        source.join("main.tt"),
        "const twice = (value: number): number => value * 2;\n\
         export const result = 1 |> twice;\n",
    )
    .unwrap();
    fs::write(
        source.join("tt/runtime.tt"),
        "export const userOwned = true;\n",
    )
    .unwrap();

    let output = ttc(&["-o", out_dir.to_str().unwrap(), source.to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("tt/runtime.ts"), "{stderr}");
    assert!(stderr.contains("compiler support module"), "{stderr}");
    assert!(stderr.contains("runtime.tt"), "{stderr}");
    assert!(!out_dir.join("main.ts").exists());
    assert!(!out_dir.join("tt/runtime.ts").exists());
}

#[test]
fn an_output_directory_inside_the_input_is_not_recompiled() {
    let dir = tmpdir();
    let source = dir.join("src");
    let out_dir = source.join("generated");
    fs::create_dir_all(&out_dir).unwrap();
    fs::write(
        source.join("main.tt"),
        "export const current = \"source\";\n",
    )
    .unwrap();
    fs::write(
        out_dir.join("stale.ts"),
        "export const stale = \"previous output\";\n",
    )
    .unwrap();

    let output = ttc(&["-o", out_dir.to_str().unwrap(), source.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out_dir.join("main.ts").is_file());
    assert!(out_dir.join("stale.ts").is_file());
    assert!(!out_dir.join("generated/stale.ts").exists());
}

#[test]
fn mixed_source_project_preserves_all_directed_runtime_values() {
    if !have("tsc") || !have("bun") || !have("node") {
        return;
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = root.join("tests/fixtures/mixed-source-runtime");
    let dir = tmpdir();
    let emitted = dir.join("emitted");
    let bundle = dir.join("bundle.js");

    let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["--no-banner", "-o"])
        .arg(&emitted)
        .arg(&source)
        .output()
        .expect("ttc runs");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(
        emitted.join("tsconfig.json"),
        "{\"compilerOptions\":{\"jsx\":\"react\",\"jsxFactory\":\"h\"}}\n",
    )
    .unwrap();
    let mut inputs: Vec<_> = fs::read_dir(&emitted)
        .expect("emitted mixed-source tree")
        .map(|entry| entry.expect("emitted entry").path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("ts" | "tsx")
            )
        })
        .collect();
    inputs.sort();
    let output = Command::new("tsc")
        .args(&inputs)
        .args([
            "--strict",
            "--target",
            "es2022",
            "--module",
            "preserve",
            "--moduleResolution",
            "bundler",
            "--jsx",
            "preserve",
            "--skipLibCheck",
            "--noEmit",
        ])
        .output()
        .expect("tsc runs");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let output = Command::new("bun")
        .args(["build"])
        .arg(emitted.join("main.ts"))
        .args(["--target", "node", "--format", "esm", "--outfile"])
        .arg(&bundle)
        .output()
        .expect("bun build runs");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let output = Command::new("node")
        .arg(&bundle)
        .output()
        .expect("node runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        r#"{"values":["ts<-ts","ts<-tsx","ts<-tt","ts<-ttx","tsx<-ts","tsx<-tsx","tsx<-tt","tsx<-ttx","tt<-ts","tt<-tsx","tt<-tt","tt<-ttx","ttx<-ts","ttx<-tsx","ttx<-tt","ttx<-ttx"],"trace":["ts","tsx","tt","ttx","ts","tsx","tt","ttx","ts","tsx","tt","ttx","ts","tsx","tt","ttx"]}"#
    );
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
fn modes_reject_options_they_would_otherwise_ignore() {
    let dir = tmpdir();
    let file = dir.join("input.tt");
    fs::write(&file, "export const value = 1;\n").unwrap();
    let path = file.to_str().unwrap();

    let cases = [
        (
            vec!["--content-mapper", "--project", "tsconfig.json"],
            "--content-mapper does not combine with --project",
        ),
        (
            vec!["--server", "--jobs", "2"],
            "--server does not combine with --jobs",
        ),
        (
            vec!["--emit-std", "types", "--source-map", "off"],
            "--emit-std does not combine with --source-map",
        ),
        (
            vec!["--symbols", "--no-banner", path],
            "--symbols does not combine with --no-banner",
        ),
        (
            vec!["--emit-map", "--jobs", "2", path],
            "--emit-map does not combine with --jobs",
        ),
        (
            vec!["--sidecar", "declarations", "--no-verify", path],
            "--sidecar does not combine with --no-verify",
        ),
        (
            vec!["--check-types", "--rewrite-imports", "off", path],
            "--check-types does not combine with --rewrite-imports",
        ),
        (
            vec!["--types", "--jobs", "2", path],
            "--types does not combine with --jobs",
        ),
        (
            vec!["--project", "tsconfig.json", path],
            "build mode does not combine with --project",
        ),
        (
            vec!["--symbols", "--emit-map", path],
            "--symbols does not combine with --emit-map",
        ),
    ];

    for (args, message) in cases {
        let out = ttc(&args);
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(!out.status.success(), "{args:?} should fail");
        assert_eq!(stderr.trim(), format!("ttc: {message}"));
        assert!(out.stdout.is_empty(), "{args:?} polluted stdout");
    }
}

#[test]
fn check_rejects_output_options_instead_of_silently_changing_or_ignoring_them() {
    let dir = tmpdir();
    let file = dir.join("input.tt");
    fs::write(&file, "export const value = 1;\n").unwrap();
    let path = file.to_str().unwrap();

    for (option, value) in [
        ("--print", None),
        ("--out-dir", Some("out")),
        ("--source-map", Some("inline")),
        ("--rewrite-imports", Some("off")),
        ("--no-banner", None),
    ] {
        let mut args = vec!["--check", option];
        if let Some(value) = value {
            args.push(value);
        }
        args.push(path);
        let out = ttc(&args);
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(!out.status.success(), "{args:?} should fail");
        assert_eq!(
            stderr.trim(),
            format!("ttc: --check does not combine with {option}")
        );
        assert!(out.stdout.is_empty(), "{args:?} polluted stdout");
    }
}

#[test]
fn print_requires_one_self_contained_stdout_document() {
    let dir = tmpdir();
    let first = dir.join("first.tt");
    let second = dir.join("second.tt");
    fs::write(&first, "export const first = 1;\n").unwrap();
    fs::write(&second, "export const second = 2;\n").unwrap();

    let external_map = ttc(&["--print", "--source-map", "file", first.to_str().unwrap()]);
    assert!(!external_map.status.success());
    assert!(external_map.stdout.is_empty());
    assert_eq!(
        String::from_utf8(external_map.stderr).unwrap().trim(),
        "ttc: --print requires --source-map off or inline; file maps require written output"
    );

    let multiple = ttc(&["--print", first.to_str().unwrap(), second.to_str().unwrap()]);
    assert!(!multiple.status.success());
    assert!(multiple.stdout.is_empty());
    assert_eq!(
        String::from_utf8(multiple.stderr).unwrap().trim(),
        "ttc: --print requires exactly one source file"
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

include!("cli/cases_01.rs");
