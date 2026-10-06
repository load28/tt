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
                 declare const step_{name}: () => (value: number) => number;\n\
                 export const value_{name} = input_{name}() |> step_{name}();\n"
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

/// The runtime is written for an output that imports it, which is what
/// codegen emitted rather than whether the source has a pipeline: a
/// literal-headed pipeline lowers to a direct call, and a script inlines
/// its helpers.
#[test]
fn a_pipeline_that_imports_no_runtime_writes_none() {
    let dir = tmpdir();
    let source = dir.join("src");
    let out_dir = dir.join("out");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("direct.tt"), "export const a = 1 |> String;\n").unwrap();
    fs::write(
        source.join("script.tt"),
        "declare function input(): number;\n\
         declare const step: (value: number) => number;\n\
         const value = input() |> step;\n",
    )
    .unwrap();

    for out in [None, Some(&out_dir)] {
        let mut args = vec!["--no-banner"];
        if let Some(out) = out {
            args.extend(["-o", out.to_str().unwrap()]);
        }
        args.push(source.to_str().unwrap());
        let output = ttc(&args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let root = out.unwrap_or(&source);
        for name in ["direct", "script"] {
            let code = fs::read_to_string(root.join(format!("{name}.ts"))).unwrap();
            assert!(!code.contains("runtime"), "{code}");
        }
        assert!(!root.join("tt").exists(), "{}", root.display());
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
fn separate_input_roots_mirror_under_the_directory_they_share() {
    let dir = tmpdir();
    let left = dir.join("left");
    let right = dir.join("right");
    let out_dir = dir.join("out");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();
    fs::write(left.join("index.tt"), "export const side = \"left\";\n").unwrap();
    fs::write(
        right.join("index.tt"),
        "import { side } from \"../left/index.tt\";\nexport const other = side;\n",
    )
    .unwrap();

    let output = ttc(&[
        "-o",
        out_dir.to_str().unwrap(),
        left.to_str().unwrap(),
        right.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out_dir.join("left/index.ts").is_file());
    let right_out = fs::read_to_string(out_dir.join("right/index.ts")).unwrap();
    assert!(
        right_out.contains("from \"../left/index.js\""),
        "{right_out}"
    );
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
        "declare function input(): number;\n\
         const twice = () => (value: number): number => value * 2;\n\
         export const result = input() |> twice();\n",
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
    #[cfg(unix)]
    std::os::unix::fs::symlink(&out_dir, source.join("alias")).unwrap();

    let output = ttc(&["-o", out_dir.to_str().unwrap(), source.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out_dir.join("main.ts").is_file());
    assert!(out_dir.join("stale.ts").is_file());
    assert!(!out_dir.join("generated/stale.ts").exists());
    assert!(!out_dir.join("alias/stale.ts").exists());
}

/// Only an output root strictly inside a directory input is excluded from
/// it. The input itself, or a directory enclosing it, is where every source
/// lives, and excluding it would leave nothing to compile.
#[test]
fn an_output_directory_that_is_or_encloses_the_input_keeps_its_sources() {
    for (out, input, emitted) in [
        (".", "src", "a.ts"),
        ("src", "src", "src/a.ts"),
        ("gen", "gen/src", "gen/a.ts"),
    ] {
        let dir = tmpdir();
        fs::create_dir_all(dir.join(input)).unwrap();
        fs::write(dir.join(input).join("a.tt"), "export const a = 1;\n").unwrap();

        let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
            .args(["-o", out, input])
            .current_dir(dir.path())
            .output()
            .expect("failed to run ttc");
        assert!(
            output.status.success(),
            "-o {out} {input}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(dir.join(emitted).is_file(), "-o {out} {input}");
    }
}

/// A directory the input reaches both by its own name and through a symlink
/// is mirrored under its own name, wherever the alias sorts. A directory
/// reached only through a symlink is still collected through it.
#[cfg(unix)]
#[test]
fn a_directory_alias_does_not_move_the_real_directory_outputs() {
    let dir = tmpdir();
    let source = dir.join("src");
    let outside = dir.join("shared");
    let out_dir = dir.join("out");
    fs::create_dir_all(source.join("lib")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(source.join("lib/x.tt"), "export const x = 1;\n").unwrap();
    fs::write(outside.join("s.tt"), "export const s = 1;\n").unwrap();
    fs::write(
        source.join("main.tt"),
        "import { x } from \"./lib/x.tt\";\nexport const y = x;\n",
    )
    .unwrap();
    for alias in ["@lib", "zlib"] {
        std::os::unix::fs::symlink("lib", source.join(alias)).unwrap();
    }
    std::os::unix::fs::symlink(&outside, source.join("linked")).unwrap();

    let output = ttc(&["-o", out_dir.to_str().unwrap(), source.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out_dir.join("lib/x.ts").is_file());
    assert!(out_dir.join("linked/s.ts").is_file());
    for alias in ["@lib", "zlib"] {
        assert!(!out_dir.join(alias).exists(), "{alias}");
    }
    let main = fs::read_to_string(out_dir.join("main.ts")).unwrap();
    assert!(main.contains("\"./lib/x.js\""), "{main}");
}

#[test]
fn mixed_source_project_preserves_all_directed_runtime_values() {
    if !common::tsc_available() || !have("bun") {
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
    let output = common::tsc()
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
    for value in ["0", "many"] {
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

/// A flag that takes a value does not take the next option as one.
///
/// `ttc -o --check src` reads as "build into --check"; it created a
/// directory of that name, wrote the tree into it, and exited 0 without
/// running the check the line asked for. Every value-taking flag had the
/// same hole, and each swallowed option also disappeared from the run.
#[test]
fn value_flags_do_not_swallow_the_next_option() {
    let dir = tmpdir();
    let file = dir.join("input.tt");
    fs::write(&file, "export const value = 1;\n").unwrap();
    let path = file.to_str().unwrap();

    for (flag, label) in [
        ("-o", "--out-dir"),
        ("--out-dir", "--out-dir"),
        ("-j", "--jobs"),
        ("--jobs", "--jobs"),
        ("--project", "--project"),
        ("--node", "--node"),
        ("--sidecar", "--sidecar"),
        ("--overlay", "--overlay"),
        ("--source-map", "--source-map"),
        ("--rewrite-imports", "--rewrite-imports"),
        ("--emit-std", "--emit-std"),
    ] {
        let out = ttc(&[flag, "--check", path]);
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            !out.status.success(),
            "{flag} took --check as its value: {stderr}"
        );
        assert!(
            stderr.contains(label) && stderr.contains("--check"),
            "{flag}: {stderr}"
        );
    }

    // Nothing was created for the option that was mistaken for a value.
    assert!(!dir.join("--check").exists());
}

/// The escape hatch stays open: a path that really begins with `-` is
/// spelled the way every other tool spells it.
#[test]
fn a_relative_path_reaches_a_directory_named_like_an_option() {
    let dir = tmpdir();
    let file = dir.join("input.tt");
    fs::write(&file, "export const value = 1;\n").unwrap();
    let _ = &file;

    let out = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .current_dir(dir.path())
        .args(["-o", "./-out", "input.tt"])
        .output()
        .expect("failed to run ttc");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dir.join("-out").join("input.ts").is_file());
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
            vec!["--check", "--project", "tsconfig.json", path],
            "--check does not combine with --project",
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

fn typed_workspace() -> Workspace {
    Workspace::in_repo("cli")
}

/// Runs `ttc --check-types` over a one-file project and returns ttc's
/// stderr. Nothing is written, so a released TypeScript 7 — which cannot
/// emit declarations — answers these just as well as a built one.
fn types_stderr(source: &str) -> String {
    let dir = typed_workspace();
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
    let dir = typed_workspace();
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
        if !have("node") || !common::toolchain() {
            eprintln!("skipping: no node, or no TypeScript for ttc to drive");
            return;
        }
    };
}

#[test]
fn an_overlay_checks_a_buffer_whose_file_is_not_saved_yet() {
    require_types_toolchain!();
    use std::io::Write;
    let dir = typed_workspace();
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();
    let file = src.join("new.tt");
    let path = file.to_str().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["--check-types", "--tt-only", "--overlay", path, path])
        .current_dir(&dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"export variant V { A, B }\nexport const f = (v: V) => match (v) { A => 1 };\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(
        err.contains("match on variant V is not exhaustive: missing \"B\""),
        "{err}"
    );
    assert!(!file.exists());
}

#[test]
fn a_type_error_in_hand_written_typescript_quotes_its_line() {
    require_types_toolchain!();
    let dir = typed_workspace();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.tt"), "export const a = 1;\n").unwrap();
    fs::write(
        dir.join("src/host.ts"),
        "export const b: number = 1;\nexport const c: string = b;\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["--check-types", "src"])
        .current_dir(&dir)
        .output()
        .expect("failed to run ttc");
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(err.contains("--> src/host.ts:2:14"), "{err}");
    assert!(err.contains("export const c: string = b;"), "{err}");
    assert!(err.contains('^'), "{err}");
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

fn types_project_output(tsconfig: &str, files: &[(&str, &str)]) -> (bool, String) {
    let dir = typed_workspace();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("tsconfig.json"), tsconfig).unwrap();
    for (path, text) in files {
        fs::write(dir.join(path), text).unwrap();
    }
    let out = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["--check-types", "src"])
        .current_dir(&dir)
        .output()
        .expect("failed to run ttc");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn types_reports_configuration_parsing_diagnostics() {
    require_types_toolchain!();
    let (ok, err) = types_project_output(
        "{ \"compilerOptions\": { \"strict\": tru }, \"include\": [\"src\"] }\n",
        &[("src/a.tt", "export const ok: number = 1;\n")],
    );
    assert!(!ok, "{err}");
    assert!(err.contains("error[ts5024]"), "{err}");
    assert!(err.contains("--> tsconfig.json:1:"), "{err}");
}

#[test]
fn types_reports_a_missing_extended_configuration_without_a_position() {
    require_types_toolchain!();
    let (ok, err) = types_project_output(
        "{ \"extends\": \"./missing.json\", \"include\": [\"src\"] }\n",
        &[("src/a.tt", "export const ok: number = 1;\n")],
    );
    assert!(!ok, "{err}");
    assert!(err.contains("error[ts5083]"), "{err}");
    assert_eq!(err.matches("error[ts5083]").count(), 1, "{err}");
    assert!(err.contains("--> tsconfig.json\n"), "{err}");
}

#[test]
fn types_reports_program_diagnostics_without_a_file() {
    require_types_toolchain!();
    let (ok, err) = types_project_output(
        "{ \"compilerOptions\": { \"types\": [\"does-not-exist\"] }, \"include\": [\"src\"] }\n",
        &[("src/a.tt", "export const ok: number = 1;\n")],
    );
    assert!(!ok, "{err}");
    assert!(err.contains("error[ts2688]"), "{err}");
    assert_eq!(err.matches("error[ts2688]").count(), 1, "{err}");
}

#[test]
fn types_reports_option_diagnostics_of_a_served_configuration_at_the_option() {
    require_types_toolchain!();
    let (ok, err) = types_project_output(
        "{\n  \"compilerOptions\": { \"target\": \"es5x\" },\n  \"include\": [\"src/**/*.tt\"]\n}\n",
        &[("src/a.tt", "export const ok: number = 1;\n")],
    );
    assert!(!ok, "{err}");
    assert!(err.contains("error[ts6046]"), "{err}");
    assert!(err.contains("--> tsconfig.json:2:34\n"), "{err}");
}

#[test]
fn types_serves_tt_modules_itself_under_the_documented_content_mapper_configuration() {
    require_types_toolchain!();
    let (ok, err) = types_project_output(
        "{\n  \"compilerOptions\": { \"strict\": true, \"noEmit\": true },\n  \"contentMappers\": [{ \"package\": \"@openload28/tt-lang\", \"extensions\": [\".tt\", \".ttx\"] }],\n  \"include\": [\"src\"]\n}\n",
        &[("src/a.tt", "export const a: number = 1;\n")],
    );
    assert!(ok, "{err}");
    assert!(!err.contains("ts100024"), "{err}");
    let (ok, err) = types_project_output(
        "{\n  \"compilerOptions\": { \"strict\": true, \"noEmit\": true },\n  \"contentMappers\": [{ \"package\": \"@openload28/tt-lang\", \"extensions\": [\".tt\", \".ttx\"] }],\n  \"include\": [\"src\"]\n}\n",
        &[("src/a.tt", "export const a: string = 1;\n")],
    );
    assert!(!ok, "{err}");
    assert!(err.contains("error[ts2322]"), "{err}");
}

#[test]
fn types_keeps_reporting_content_mappers_it_does_not_serve() {
    require_types_toolchain!();
    let (ok, err) = types_project_output(
        "{\n  \"compilerOptions\": { \"strict\": true, \"noEmit\": true },\n  \"contentMappers\": [{ \"package\": \"@openload28/tt-lang\", \"extensions\": [\".tt\", \".ttx\"] }, { \"package\": \"other-mapper\", \"extensions\": [\".other\"] }],\n  \"include\": [\"src\"]\n}\n",
        &[("src/a.tt", "export const a: number = 1;\n")],
    );
    assert!(!ok, "{err}");
    assert!(err.contains("error[ts100024]"), "{err}");
}

#[test]
fn types_reports_syntax_errors_in_hand_written_typescript() {
    require_types_toolchain!();
    let (ok, err) = types_project_output(
        "{ \"compilerOptions\": { \"strict\": true }, \"include\": [\"src\"] }\n",
        &[
            ("src/a.tt", "export const ok: number = 1;\n"),
            ("src/h.ts", "export const broken: number = ;\n"),
        ],
    );
    assert!(!ok, "{err}");
    assert!(err.contains("error[ts1109]"), "{err}");
    assert!(err.contains("--> src/h.ts:1:"), "{err}");
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
fn types_type_a_value_with_no_contextual_type_as_at_its_source_position() {
    require_types_toolchain!();
    // TASK-570: `this` in the arm's method is the object literal, as in
    // `n === 1 ? {…} : {…}`, not the `any` of the storage it is written to.
    let err = types_stderr(
        "export function f(n: number) {\n\
         \x20 const b = match (n) { 1 => ({ k: 1, m() { return this; } }), _ => ({ k: 2, m() { return this; } }) };\n\
         \x20 return b.m().zzz;\n\
         }\n",
    );
    assert!(err.contains("error[ts2339]"), "{err}");
    assert!(err.contains("Property 'zzz' does not exist"), "{err}");
    assert!(err.contains("--> src/main.tt:3:16"), "{err}");
}

#[test]
fn types_type_a_recursive_anonymous_join_whole() {
    require_types_toolchain!();
    // TASK-586: the join's annotation would write the cycle one level down
    // as `any` and hide the second call's missing property.
    let err = types_stderr(
        "export function f(n: number) {\n\
         \x20 const b = match (n) { 1 => ({ k: 1, m() { return this; } }), _ => ({ k: 2, m() { return this; } }) };\n\
         \x20 return b.m().m().zzz;\n\
         }\n",
    );
    assert!(err.contains("error[ts2339]"), "{err}");
    assert!(err.contains("--> src/main.tt:3:20"), "{err}");
}

#[test]
fn types_keep_declarations_written_before_a_variant_stopped_parsing() {
    require_types_toolchain!();
    // TASK-590: a malformed variant is reported once, its importers get no
    // follow-on errors from its placeholder, and neither its declarations nor
    // its importers' are replaced by ones emitted against the placeholder.
    let dir = typed_workspace();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("tsconfig.json"),
        r#"{"compilerOptions":{"strict":true,"noEmit":true,"module":"esnext","moduleResolution":"bundler","target":"es2022"},"include":["src"]}"#,
    )
    .unwrap();
    fs::write(dir.join("src/x.tt"), "export variant X { A, B }\n").unwrap();
    fs::write(
        dir.join("src/use.tt"),
        "import { X } from \"./x.tt\";\nexport const a = X.A;\nexport const k: X = a;\n",
    )
    .unwrap();
    fs::write(dir.join("src/ok.tt"), "export const ok = 1;\n").unwrap();
    let types = || {
        Command::new(env!("CARGO_BIN_EXE_ttc"))
            .args(["--types", "--json-report", "src"])
            .current_dir(&dir)
            .output()
            .expect("failed to run ttc")
    };
    let first = types();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let sidecar = |name: &str| fs::read_to_string(dir.join(".tt-types").join(name)).unwrap();
    let (x, uses) = (sidecar("x.tt.d.ts"), sidecar("use.tt.d.ts"));
    fs::write(dir.join("src/x.tt"), "export variant X { A, 1 }\n").unwrap();
    let second = types();
    let err = String::from_utf8_lossy(&second.stderr);
    assert_eq!(err.matches("error[").count(), 1, "{err}");
    assert!(err.contains("error[malformed-variant]"), "{err}");
    assert_eq!(sidecar("x.tt.d.ts"), x);
    assert_eq!(sidecar("use.tt.d.ts"), uses);
    let report: serde_json::Value = serde_json::from_slice(&second.stdout).unwrap();
    let written: Vec<&str> = report["written"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|path| path.as_str())
        .collect();
    assert!(
        written.iter().any(|path| path.ends_with("ok.tt.d.ts"))
            && !written
                .iter()
                .any(|path| path.ends_with("x.tt.d.ts") || path.ends_with("use.tt.d.ts")),
        "{written:?}"
    );
}

#[test]
fn types_reports_nothing_for_storage_inside_a_shadowing_scope() {
    require_types_toolchain!();
    // TASK-575: `T` at the storage is `inner`'s own type parameter, not the
    // `outer` one the arm values have.
    let err = types_stderr(
        "variant K { A, B }\n\
         export function outer<T>(a: T) {\n\
         \x20 function inner<T>(b: T, k: K) {\n\
         \x20   const z = match (k) { A => a, B => a };\n\
         \x20   return [z, b] as const;\n\
         \x20 }\n\
         \x20 return inner(\"s\", K.A);\n\
         }\n",
    );
    assert!(!err.contains("error"), "{err}");
}

#[test]
fn types_join_storage_after_the_storage_its_values_read() {
    require_types_toolchain!();
    // TASK-584: without `noImplicitAny`, storage no round has settled reads
    // as `any`; `g`'s join waits for `f`'s storage instead of keeping
    // `f(): any`, across modules too, and an `any` of the source stays.
    let (ok, err) = types_project_output(
        "{ \"compilerOptions\": { \"strict\": false, \"target\": \"es2022\", \"module\": \"esnext\", \"moduleResolution\": \"bundler\", \"noEmit\": true }, \"include\": [\"src\"] }\n",
        &[
            (
                "src/a.tt",
                "declare const flag: boolean;\n\
                 export const f = () => match(flag) { true => [1], false => [2] };\n\
                 export const g = match(flag) { true => [f()], false => [] };\n\
                 export const bad = g[0]![0]!.toUpperCase();\n",
            ),
            (
                "src/m.tt",
                "declare const flag: boolean;\n\
                 export const p = () => match(flag) { true => [JSON.parse(\"1\")], false => [JSON.parse(\"2\")] };\n",
            ),
            (
                "src/b.tt",
                "import { f } from \"./a.tt\";\n\
                 import { p } from \"./m.tt\";\n\
                 declare const flag: boolean;\n\
                 export const g = match(flag) { true => [f()], false => [] };\n\
                 export const bad = g[0]![0]!.toUpperCase();\n\
                 export const q = match(flag) { true => [p()], false => [] };\n\
                 export const worse = q[0]!.foo;\n",
            ),
        ],
    );
    assert!(!ok, "{err}");
    assert_eq!(err.matches("error[").count(), 3, "{err}");
    assert!(err.contains("--> src/a.tt:4:30"), "{err}");
    assert!(err.contains("--> src/b.tt:5:30"), "{err}");
    assert!(
        err.contains("Property 'foo' does not exist on type 'any[]'"),
        "{err}"
    );
}

#[test]
fn types_reports_plain_typescript_diagnostics_in_typescripts_words() {
    require_types_toolchain!();
    // TASK-585: an arity error, a pipeline step's arity error and a JSX
    // element's missing props keep TypeScript's own sentence; the argument
    // that does not fit its parameter is a mismatch of its own.
    let (ok, err) = types_project_output(
        "{ \"compilerOptions\": { \"strict\": true, \"target\": \"es2022\", \"module\": \"esnext\", \"moduleResolution\": \"bundler\", \"jsx\": \"preserve\", \"noEmit\": true }, \"include\": [\"src\"] }\n",
        &[
            (
                "src/a.tt",
                "function g(a: number): number { return a; }\n\
                 function f(s: string): string { return s; }\n\
                 export const r = f(g());\n\
                 function scale(x: number, by: number): number { return x * by; }\n\
                 declare const x: number;\n\
                 export const y = x |> scale();\n",
            ),
            (
                "src/b.ttx",
                "declare global {\n\
                 \x20 namespace JSX {\n\
                 \x20   interface Element { readonly tag: string }\n\
                 \x20   interface IntrinsicElements { div: {} }\n\
                 \x20 }\n\
                 }\n\
                 function Row(props: { label: string }): JSX.Element { return { tag: props.label }; }\n\
                 export const view = <Row />;\n",
            ),
        ],
    );
    assert!(!ok, "{err}");
    assert!(
        err.contains("error[ts2554]: Expected 1 arguments, but got 0.\n --> src/a.tt:3:20"),
        "{err}"
    );
    assert!(
        err.contains(
            "error[ts2345]: Argument of type 'number' is not assignable to parameter of type 'string'.\n --> src/a.tt:3:20"
        ),
        "{err}"
    );
    assert!(
        err.contains("error[ts2554]: Expected 2 arguments, but got 0.\n --> src/a.tt:6:23"),
        "{err}"
    );
    assert!(
        err.contains("error[ts2741]: Property 'label' is missing in type '{}' but required in type '{ label: string; }'."),
        "{err}"
    );
    assert!(!err.contains("found `(props"), "{err}");
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

/// The flag needs a value, and the path it names needs an existing
/// directory — it stands in for a file of the project, saved or not.
#[test]
fn overlay_reports_a_missing_value_and_a_missing_directory() {
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
    let gone = dir.join("gone").join("a.tt");
    let out = ttc(&[
        "--check-types",
        "--overlay",
        gone.to_str().unwrap(),
        file.to_str().unwrap(),
    ]);
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(!out.status.success(), "{err}");
    assert!(err.contains("--overlay"), "{err}");
    assert!(err.contains("gone"), "{err}");
}

/// A variant the imported module exports through an `export { ... }`
/// specifier is as visible to exhaustiveness as one declared with
/// `export variant`, under the name the specifier gives it (TASK-459).
#[test]
fn check_sees_a_variant_exported_through_a_specifier() {
    let dir = tmpdir();
    fs::write(
        dir.join("shape.tt"),
        "variant Color { Red, Green }\nexport { Color as Hue };\nexport type { Color };\n",
    )
    .unwrap();
    for (name, import, ty) in [
        ("alias.tt", "{ Hue }", "Hue"),
        ("type.tt", "{ Color }", "Color"),
        ("rename.tt", "{ Hue as Shade }", "Shade"),
        ("namespace.tt", "* as shapes", "shapes.Hue"),
    ] {
        let importer = dir.join(name);
        fs::write(
            &importer,
            format!(
                "import {import} from \"./shape.tt\";\n\
                 export function f(h: {ty}) {{ return match (h) {{ Red => \"r\" }}; }}\n"
            ),
        )
        .unwrap();
        let out = ttc(&[
            "--check",
            importer.to_str().unwrap(),
            dir.join("shape.tt").to_str().unwrap(),
        ]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{name}: {stderr}");
        assert!(
            stderr.contains("(imported from \"./shape.tt\") is not exhaustive: missing \"Green\""),
            "{name}: {stderr}"
        );
    }
}

include!("cli/cases_01.rs");
include!("cli/line_breaks.rs");

/// A `#!` line and a byte-order mark are only themselves when they come
/// first, so the generated banner is written after them (TASK-336). A
/// comment above either one turns a runnable script into a parse error and
/// leaves a stray U+FEFF in the middle of the file.
#[test]
fn the_banner_never_displaces_a_shebang_or_a_byte_order_mark() {
    let dir = tmpdir();
    let out_dir = dir.join("out");
    let shebang = dir.join("cli.tt");
    fs::write(&shebang, "#!/usr/bin/env node\nconsole.log(1);\n").unwrap();
    // A hand-written `.ts` passes through byte for byte, banner included:
    // it is not generated, and the only byte ttc may change in one is a
    // relative tt import specifier.
    let passthrough = dir.join("plain.ts");
    let passthrough_source = "#!/usr/bin/env node\nconsole.log(2);\n";
    fs::write(&passthrough, passthrough_source).unwrap();
    let bom = dir.join("bom.tt");
    fs::write(&bom, "\u{feff}export const a = 1;\n").unwrap();
    // A shebang that runs to the end of the file still needs a line break
    // before the banner.
    let bare = dir.join("bare.tt");
    fs::write(&bare, "#!/usr/bin/env node").unwrap();

    let output = ttc(&[
        "-o",
        out_dir.to_str().unwrap(),
        dir.path().to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let emitted = fs::read_to_string(out_dir.join("cli.ts")).unwrap();
    let mut lines = emitted.lines();
    assert_eq!(lines.next(), Some("#!/usr/bin/env node"), "{emitted}");
    assert!(
        lines
            .next()
            .is_some_and(|line| line.starts_with("// @generated")),
        "{emitted}"
    );
    assert_eq!(
        fs::read_to_string(out_dir.join("plain.ts")).unwrap(),
        passthrough_source,
        "a hand-written .ts is not generated and passes through unchanged"
    );
    let bom_emitted = fs::read_to_string(out_dir.join("bom.ts")).unwrap();
    assert!(
        bom_emitted.starts_with("\u{feff}// @generated"),
        "{bom_emitted:?}"
    );
    assert_eq!(
        bom_emitted.matches('\u{feff}').count(),
        1,
        "{bom_emitted:?}"
    );
    let bare_emitted = fs::read_to_string(out_dir.join("bare.ts")).unwrap();
    assert_eq!(
        bare_emitted.lines().next(),
        Some("#!/usr/bin/env node"),
        "{bare_emitted:?}"
    );
    assert!(
        bare_emitted
            .lines()
            .nth(1)
            .is_some_and(|line| line.starts_with("// @generated")),
        "{bare_emitted:?}"
    );
}

/// The banner shifts the lines below it, and only those: a shebang keeps
/// line 1, so a map built against the emission must not shift it (TASK-336).
#[test]
fn a_source_map_follows_the_banner_past_a_shebang() {
    let dir = tmpdir();
    let out_dir = dir.join("out");
    let source = dir.join("trace.tt");
    fs::write(
        &source,
        "#!/usr/bin/env node\nvariant Shape { Circle(radius: number), Square(side: number) }\n\
         export const area = (s: Shape): number => match (s) {\n\
         \x20 Circle(radius) => radius,\n\
         \x20 Square(side) => side,\n\
         };\n",
    )
    .unwrap();
    let output = ttc(&[
        "--source-map",
        "file",
        "-o",
        out_dir.to_str().unwrap(),
        source.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let map = fs::read_to_string(out_dir.join("trace.ts.map")).unwrap();
    let mappings = map
        .split("\"mappings\":\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("a mappings field");
    // The shebang owns generated line 1 in both files, so the first line of
    // the map carries a segment rather than being skipped by the banner's
    // shift.
    assert!(
        !mappings.starts_with(';'),
        "the shebang line lost its mapping: {mappings}"
    );
}

/// A file that is only a shebang, with no line break after it, keeps the
/// shebang on generated line 1: the banner goes on a line of its own after
/// it, and the map's only segment stays on the first line.
#[test]
fn a_source_map_keeps_a_lone_shebang_on_the_first_line() {
    let dir = tmpdir();
    let out_dir = dir.join("out");
    let source = dir.join("only.tt");
    fs::write(&source, "#!/usr/bin/env node").unwrap();
    let output = ttc(&[
        "--source-map",
        "file",
        "-o",
        out_dir.to_str().unwrap(),
        source.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let code = fs::read_to_string(out_dir.join("only.ts")).unwrap();
    assert!(
        code.starts_with("#!/usr/bin/env node\n// @generated"),
        "{code}"
    );
    let map = fs::read_to_string(out_dir.join("only.ts.map")).unwrap();
    let mappings = map
        .split("\"mappings\":\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("a mappings field");
    assert_eq!(mappings, "AAAA", "{map}");
}

/// A reader that stops reading is the reader's decision, not a compiler
/// failure: `ttc --help | head` must end quietly rather than reporting an
/// internal compiler error and exiting 101 (TASK-337).
#[test]
fn a_closed_stdout_ends_the_run_quietly() {
    use std::io::Read;
    use std::process::Stdio;

    let profiles = Workspace::new("closed-stdout-profiles");
    for args in [
        vec!["--help"],
        vec!["-v"],
        vec!["help", "all"],
        vec!["explain"],
        vec!["--emit-std", "option"],
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ttc"));
        command
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        profiles.isolate_unfinalized_child_profile(&mut command);
        let mut child = command.spawn().expect("failed to run ttc");
        // Read one byte, then drop the pipe: the next write has nowhere to go.
        let mut stdout = child.stdout.take().expect("piped stdout");
        let mut first = [0u8; 1];
        let _ = stdout.read(&mut first);
        drop(stdout);
        let output = child.wait_with_output().expect("ttc did not exit");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("internal compiler error"),
            "{args:?} reported a compiler bug for a closed pipe: {stderr}"
        );
        assert!(
            output.status.success(),
            "{args:?} exited with {:?}: {stderr}",
            output.status.code()
        );
    }
}

/// `--project` names the config a check runs against, and its directory
/// becomes the project root. A path that is not there used to root the
/// project somewhere the user never named — or reach the TypeScript backend
/// un-canonicalised — so it is rejected where it is given (TASK-338).
#[test]
fn a_project_path_that_is_not_a_file_is_rejected_by_name() {
    let dir = tmpdir();
    let source = dir.join("a.tt");
    fs::write(&source, "export const a = 1;\n").unwrap();
    fs::write(
        dir.join("tsconfig.json"),
        "{ \"compilerOptions\": { \"noEmit\": true } }\n",
    )
    .unwrap();

    for spelling in ["./tsconfg.json", "tsconfg.json"] {
        let output = ttc(&[
            "--check-types",
            "--project",
            spelling,
            source.to_str().unwrap(),
        ]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("--project") && stderr.contains(spelling),
            "{spelling}: {stderr}"
        );
        assert!(
            !stderr.contains("internal compiler error"),
            "{spelling}: {stderr}"
        );
        assert!(!output.status.success(), "{spelling}: {stderr}");
    }

    let output = ttc(&[
        "--check-types",
        "--project",
        dir.path().to_str().unwrap(),
        source.to_str().unwrap(),
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not a file"), "{stderr}");
    assert!(!output.status.success(), "{stderr}");
}

#[test]
fn overlapping_input_roots_write_each_source_once() {
    let dir = tmpdir();
    fs::create_dir_all(dir.join("src/deep")).unwrap();
    fs::write(dir.join("src/deep/x.tt"), "export const a = 1;\n").unwrap();
    fs::write(dir.join("src/y.tt"), "export const b = 2;\n").unwrap();
    let out_dir = dir.join("out");

    let output = ttc(&[
        "-o",
        out_dir.to_str().unwrap(),
        dir.path().to_str().unwrap(),
        dir.join("src").to_str().unwrap(),
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert_eq!(stderr.matches("y.tt →").count(), 1, "{stderr}");
    assert!(out_dir.join("src/deep/x.ts").is_file());
    assert!(out_dir.join("src/y.ts").is_file());

    let twice = dir.join("twice");
    let output = ttc(&[
        "-o",
        twice.to_str().unwrap(),
        dir.join("src").to_str().unwrap(),
        dir.join("src").to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(twice.join("y.ts").exists());
}

/// A file named on the command line is filtered like any other: the
/// extensions are the contract, not how the file was reached (TASK-338).
#[test]
fn a_named_file_that_is_not_a_source_is_reported() {
    let dir = tmpdir();
    let script = dir.join("app.js");
    fs::write(&script, "variant S { A, B }\n").unwrap();
    let output = ttc(&["-p", script.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not a tt or TypeScript source"), "{stderr}");
    assert!(!output.status.success(), "{stderr}");

    // Every extension the walk takes still works when named directly.
    for (name, body) in [
        ("a.tt", "export const a = 1;\n"),
        ("b.ts", "export const b = 2;\n"),
    ] {
        let file = dir.join(name);
        fs::write(&file, body).unwrap();
        let output = ttc(&["-p", file.to_str().unwrap()]);
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[path = "cli/dynamic_imports.rs"]
mod dynamic_imports;

#[path = "cli/server_print.rs"]
mod server_print;

#[path = "cli/server_response_shapes.rs"]
mod server_response_shapes;

/// `-o` mirrors input paths, and named files are inputs too: two of them
/// under one directory keep the layout that makes a relative import between
/// them resolve in the output tree.
#[test]
fn named_file_inputs_keep_their_layout_under_the_output_directory() {
    let dir = tmpdir();
    let out_dir = dir.join("out");
    fs::create_dir_all(dir.join("src/sub")).unwrap();
    fs::write(
        dir.join("src/shape.tt"),
        "export const area = (n: number) => n;\n",
    )
    .unwrap();
    fs::write(
        dir.join("src/sub/helper.ts"),
        "import { area } from \"../shape.tt\";\nexport const h = area;\n",
    )
    .unwrap();

    let output = ttc(&[
        "-o",
        out_dir.to_str().unwrap(),
        dir.join("src/shape.tt").to_str().unwrap(),
        dir.join("src/sub/helper.ts").to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out_dir.join("shape.ts").is_file());
    assert!(
        out_dir.join("sub/helper.ts").is_file(),
        "a named file kept its depth"
    );
    // The rewritten specifier resolves to the sibling output, which it
    // cannot do when the depth is flattened away.
    let helper = fs::read_to_string(out_dir.join("sub/helper.ts")).unwrap();
    assert!(helper.contains("\"../shape.js\""), "{helper}");
    assert!(out_dir.join("sub").join("../shape.ts").exists(), "{helper}");
}

/// A build that cannot finish writing must leave the previous output where
/// it was. Writing in place cannot do that — the open truncates — so the
/// bytes are staged beside the target and renamed onto it.
#[test]
fn a_failed_write_leaves_the_previous_output_and_no_litter() {
    let dir = tmpdir();
    let out_dir = dir.join("out");
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/a.tt"), "export const a = 1;\n").unwrap();
    fs::create_dir_all(&out_dir).unwrap();

    // A directory where the output file belongs: the rename cannot replace
    // it, which is a write that fails after the source compiled.
    fs::create_dir_all(out_dir.join("a.ts")).unwrap();
    fs::write(out_dir.join("a.ts/keep.txt"), "kept\n").unwrap();

    let output = ttc(&[
        "-o",
        out_dir.to_str().unwrap(),
        dir.join("src").to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert_eq!(
        fs::read_to_string(out_dir.join("a.ts/keep.txt")).unwrap(),
        "kept\n",
        "the existing entry was replaced by a partial write"
    );
    let leftovers: Vec<_> = fs::read_dir(&out_dir)
        .unwrap()
        .filter_map(|entry| entry.ok().map(|entry| entry.file_name()))
        .filter(|name| name.to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "staging files left behind: {leftovers:?}"
    );
}

/// An entry the walk cannot read is named. It used to surface as
/// "no such file or directory" against the directory the user named, which
/// plainly does exist, leaving the actual dangling link unmentioned.
#[test]
fn an_unreadable_entry_is_named_rather_than_the_directory_holding_it() {
    let dir = tmpdir();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/ok.tt"), "export const a = 1;\n").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("/nonexistent/gone.tt", dir.join("src/dangling.tt")).unwrap();
    #[cfg(not(unix))]
    return;

    let output = ttc(&["--check", dir.join("src").to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("dangling.tt"), "{stderr}");
}

#[test]
fn named_inputs_with_parent_components_stay_inside_the_output_tree() {
    let dir = tmpdir();
    let work = dir.join("work");
    fs::create_dir(&work).unwrap();
    fs::write(dir.join("a.tt"), "export const a = 1;\n").unwrap();
    fs::write(work.join("b.tt"), "export const b = 2;\n").unwrap();
    let handwritten = "// handwritten\nexport const untouched = 42;\n";
    fs::write(work.join("a.ts"), handwritten).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .current_dir(&work)
        .args(["-o", "out", "../a.tt", "b.tt", "./b.tt"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(work.join("a.ts")).unwrap(), handwritten);
    assert!(work.join("out/a.ts").exists());
    assert!(work.join("out/work/b.ts").exists());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).matches('→').count(),
        2
    );
}

#[test]
fn watch_reports_input_failure_transitions_and_recovers() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    use std::sync::mpsc;
    use std::time::Duration;
    let dir = tmpdir();
    let input = dir.join("src");
    fs::create_dir(&input).unwrap();
    fs::write(input.join("a.tt"), "export const a = 1;").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_ttc"));
    command
        .current_dir(&dir)
        .args(["--watch", "-o", "out", "src"])
        .stderr(Stdio::piped())
        .stdout(Stdio::null());
    dir.isolate_unfinalized_child_profile(&mut command);
    let mut child = command.spawn().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (send, receive) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let _ = send.send(line.unwrap());
        }
    });
    let result = std::panic::catch_unwind(|| {
        loop {
            if receive
                .recv_timeout(Duration::from_secs(10))
                .unwrap()
                .contains("Ctrl-C")
            {
                break;
            }
        }
        fs::remove_dir_all(&input).unwrap();
        assert!(
            receive
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .contains("no such file")
        );
        assert!(receive.recv_timeout(Duration::from_millis(1000)).is_err());
        fs::create_dir(&input).unwrap();
        fs::write(input.join("a.tt"), "export const a = 2;").unwrap();
        loop {
            if receive
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .contains("file(s) ok")
            {
                break;
            }
        }
        fs::remove_dir_all(&input).unwrap();
        assert!(
            receive
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .contains("no such file")
        );
    });
    let _ = child.kill();
    let _ = child.wait();
    reader.join().unwrap();
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}

/// Support modules belong to the whole watched input set: a round that
/// recompiles one file in a subdirectory writes them where a one-shot build
/// does, and a round whose input set moves the shared root recompiles every
/// output against the new place.
#[test]
fn watch_places_support_modules_by_the_whole_input_set() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    use std::sync::mpsc;
    use std::time::Duration;
    let dir = tmpdir();
    let input = dir.join("src");
    fs::create_dir_all(input.join("sub")).unwrap();
    fs::write(
        input.join("a.tt"),
        "export const x = (n: number) => n |> ((v: number) => String(v));",
    )
    .unwrap();
    fs::write(
        input.join("sub/b.tt"),
        "export const x = (n: number) => n |> ((v: number) => String(v));",
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_ttc"));
    command
        .current_dir(&dir)
        .args(["--watch", "src"])
        .stderr(Stdio::piped())
        .stdout(Stdio::null());
    dir.isolate_unfinalized_child_profile(&mut command);
    let mut child = command.spawn().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (send, receive) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let _ = send.send(line.unwrap());
        }
    });
    let next_round = || {
        loop {
            let line = receive.recv_timeout(Duration::from_secs(10)).unwrap();
            assert!(!line.contains("with errors"), "{line}");
            if line.contains("file(s) ok") {
                break;
            }
        }
    };
    let result = std::panic::catch_unwind(|| {
        next_round();
        assert!(input.join("tt/runtime.ts").exists());
        assert!(
            fs::read_to_string(input.join("sub/b.ts"))
                .unwrap()
                .contains("\"../tt/runtime.js\"")
        );
        fs::write(
            input.join("sub/b.tt"),
            "export const x = (n: number) => n |> ((v: number) => String(v));\n",
        )
        .unwrap();
        next_round();
        assert!(!input.join("sub/tt").exists());
        assert!(
            fs::read_to_string(input.join("sub/b.ts"))
                .unwrap()
                .contains("\"../tt/runtime.js\"")
        );
        fs::remove_file(input.join("a.tt")).unwrap();
        fs::remove_file(input.join("a.ts")).unwrap();
        next_round();
        assert!(input.join("sub/tt/runtime.ts").exists());
        assert!(
            fs::read_to_string(input.join("sub/b.ts"))
                .unwrap()
                .contains("\"./tt/runtime.js\"")
        );
    });
    let _ = child.kill();
    let _ = child.wait();
    reader.join().unwrap();
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}

/// A contextual annotation refines the type of a generated storage slot;
/// the emitted program is correct without one. `--check` is documented as
/// needing no TypeScript, and `-p` is what bundler plugins call, so a
/// toolchain that is not installed has to remove the refinement rather than
/// the compilation.
#[test]
#[cfg(debug_assertions)]
fn a_siblings_compiler_bug_is_its_own_and_the_printed_file_still_compiles() {
    require_types_toolchain!();
    let dir = typed_workspace();
    fs::write(
        dir.join("tsconfig.json"),
        "{ \"compilerOptions\": { \"strict\": true, \"target\": \"esnext\", \"module\": \"esnext\", \"moduleResolution\": \"bundler\", \"noEmit\": true, \"types\": [] } }\n",
    )
    .unwrap();
    fs::write(
        dir.join("a.tt"),
        "import { b } from \"./b.tt\";\n\
         variant O { A(n: number), B }\n\
         declare const o: O;\n\
         export const a = match (o) { A(n) => n + b, B => b };\n",
    )
    .unwrap();
    fs::write(
        dir.join("b.tt"),
        "variant P { C(n: number), D }\n\
         declare const p: P;\n\
         export const b = match (p) { C(n) => n, D => 0 };\n",
    )
    .unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_ttc"))
            .args(args)
            .current_dir(&dir)
            .env("TTC_PANIC_FOR_TEST", "projection:b.tt")
            .env_remove("RUST_BACKTRACE")
            .output()
            .expect("failed to run ttc")
    };
    let printed = run(&["-p", "a.tt"]);
    let stderr = String::from_utf8_lossy(&printed.stderr);
    let stdout = String::from_utf8_lossy(&printed.stdout);
    assert!(printed.status.success(), "{stderr}");
    assert!(stdout.contains("export const a = $tt_v0"), "{stdout}");
    assert!(
        stderr.contains("while compiling: ") && stderr.contains("b.tt"),
        "{stderr}"
    );
    assert!(!stderr.contains("a.tt"), "{stderr}");

    let checked = run(&["--check-types", "."]);
    let stderr = String::from_utf8_lossy(&checked.stderr);
    assert_eq!(checked.status.code(), Some(101), "{stderr}");
    assert!(
        stderr.contains("while compiling: ") && stderr.contains("b.tt") && !stderr.contains("a.tt"),
        "{stderr}"
    );
}

#[test]
fn a_missing_toolchain_does_not_stop_a_tt_level_check_or_print() {
    // Outside the repository: the toolchain is resolved by walking up from
    // the file, and every directory inside this checkout has the
    // repository's own `node_modules` above it.
    let isolated = Workspace::new("no-toolchain");
    let file = isolated.join("shape.tt");
    fs::write(
        &file,
        "variant Shape { Circle(r: number), Point }\n\
         declare const s: Shape;\n\
         export const v = match (s) { Circle(r) => r, Point => 0 };\n",
    )
    .unwrap();

    for mode in ["--check", "-p"] {
        // Run from that directory too: the toolchain is looked up from the
        // process's own location as well as the file's.
        let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
            .args([mode, "shape.tt"])
            .current_dir(&isolated)
            .output()
            .expect("failed to run ttc");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("no TypeScript compiler found"),
            "{mode} demanded a toolchain: {stderr}"
        );
        assert!(output.status.success(), "{mode} failed: {stderr}");
    }
}

/// An installed TypeScript whose host cannot start is as unavailable as one
/// that is not installed: a runtime missing from `PATH`, or one that exits
/// before the compiler answers, removes the refinement and leaves the tt
/// layer, the printed module and the build to succeed.
#[cfg(unix)]
#[test]
fn a_backend_that_cannot_start_does_not_stop_a_check_print_or_build() {
    use std::os::unix::fs::PermissionsExt;
    if !common::toolchain() {
        return;
    }
    let dir = Workspace::in_repo_with_subdir("unstartable-backend", "src");
    fs::write(
        dir.join("src/a.tt"),
        "variant T { A(x: number), B }\n\
         export const f = (t: T) => match (t) { A(x) => x, B => 0 };\n",
    )
    .unwrap();
    let empty = dir.join("no-runtime");
    let dying = dir.join("dying-runtime");
    fs::create_dir_all(&empty).unwrap();
    fs::create_dir_all(&dying).unwrap();
    fs::write(dying.join("node"), "#!/bin/sh\nexit 1\n").unwrap();
    fs::set_permissions(dying.join("node"), fs::Permissions::from_mode(0o755)).unwrap();

    for runtime in [&empty, &dying] {
        for args in [
            &["--check", "src/a.tt"][..],
            &["-p", "src/a.tt"][..],
            &["-o", "out", "src"][..],
        ] {
            let _ = fs::remove_dir_all(dir.join("out"));
            let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
                .args(args)
                .current_dir(&dir)
                .env("PATH", runtime)
                .output()
                .expect("failed to run ttc");
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                output.status.success(),
                "{args:?} with {} failed: {stderr}",
                runtime.display()
            );
            assert!(!stderr.contains("error"), "{args:?}: {stderr}");
        }
        assert!(dir.join("out/a.ts").is_file());

        // The typed modes still report the tt layer and say the TypeScript
        // layer did not run, exactly as with no toolchain installed.
        let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
            .args(["--check-types", "src/a.tt"])
            .current_dir(&dir)
            .env("PATH", runtime)
            .output()
            .expect("failed to run ttc");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{stderr}");
        assert!(
            stderr.contains("the TypeScript layer did not run"),
            "{stderr}"
        );
    }
}

/// `--check` writes nothing, and the TypeScript backend's only effect on a
/// compile is the contextual annotation of the output — so `--check` never
/// starts it, while a compile that prints its output does.
#[cfg(unix)]
#[test]
fn check_does_not_start_the_typescript_backend() {
    use std::os::unix::fs::PermissionsExt;
    if !common::toolchain() {
        return;
    }
    let dir = Workspace::in_repo_with_subdir("check-without-backend", "src");
    fs::write(
        dir.join("src/a.tt"),
        "variant T { A(x: number), B }\n\
         export const f = (t: T) => match (t) { A(x) => x, B => 0 };\n",
    )
    .unwrap();
    let runtime = dir.join("runtime");
    let started = dir.join("started");
    fs::create_dir_all(&runtime).unwrap();
    fs::write(
        runtime.join("node"),
        format!("#!/bin/sh\n: > '{}'\nexit 1\n", started.display()),
    )
    .unwrap();
    fs::set_permissions(runtime.join("node"), fs::Permissions::from_mode(0o755)).unwrap();
    let run = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
            .args(args)
            .current_dir(&dir)
            .env("PATH", &runtime)
            .output()
            .expect("failed to run ttc");
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };

    run(&["--check", "src"]);
    run(&["--check", "src/a.tt"]);
    assert!(!started.exists(), "--check started the TypeScript backend");
    run(&["-p", "src/a.tt"]);
    assert!(started.exists(), "-p did not ask the TypeScript backend");
}

/// The server's `check` is `--check` for a buffer, and starts no backend
/// either: the editor asks it on every keystroke.
#[cfg(unix)]
#[test]
fn the_servers_check_does_not_start_the_typescript_backend() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    if !common::toolchain() {
        return;
    }
    let dir = Workspace::in_repo_with_subdir("server-check-without-backend", "src");
    let file = dir.join("src/a.tt");
    let text = "variant T { A(x: number), B }\n\
                export const f = (t: T) => match (t) { A(x) => x };\n";
    fs::write(&file, text).unwrap();
    let runtime = dir.join("runtime");
    let started = dir.join("started");
    fs::create_dir_all(&runtime).unwrap();
    fs::write(
        runtime.join("node"),
        format!("#!/bin/sh\n: > '{}'\nexit 1\n", started.display()),
    )
    .unwrap();
    fs::set_permissions(runtime.join("node"), fs::Permissions::from_mode(0o755)).unwrap();
    let request = serde_json::json!({
        "id": 1,
        "method": "check",
        "params": { "text": text, "filename": file },
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .arg("--server")
        .current_dir(&dir)
        .env("PATH", &runtime)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to run ttc --server");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("{request}\n").as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let answer: serde_json::Value = serde_json::from_str(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        answer["result"]["diagnostics"][0]["code"], "match-not-exhaustive",
        "{answer}"
    );
    assert!(
        !started.exists(),
        "the server's check started the TypeScript backend"
    );
}

/// The contextual refinement of a build is one TypeScript session per
/// project however many workers compile it: `-j 4` starts exactly the
/// processes `-j 1` does, and writes the same output.
#[cfg(unix)]
#[test]
fn parallel_workers_share_one_typescript_session_per_project() {
    use std::os::unix::fs::PermissionsExt;
    if !common::toolchain() {
        return;
    }
    let Some(node) = std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|dir| dir.join("node"))
            .find(|node| node.is_file())
    }) else {
        assert!(!common::toolchain_required(), "no node on PATH");
        return;
    };
    let dir = Workspace::in_repo_with_subdir("shared-session", "src");
    fs::write(
        dir.join("tsconfig.json"),
        r#"{"compilerOptions":{"strict":true,"module":"nodenext"}}"#,
    )
    .unwrap();
    for index in 0..8 {
        fs::write(
            dir.join(format!("src/m{index}.tt")),
            format!(
                "variant T{index} {{ A(x: number), B }}\n\
                 export const f{index} = (t: T{index}) => match (t) {{ A(x) => x, B => 0 }};\n"
            ),
        )
        .unwrap();
    }
    let runtime = dir.join("runtime");
    let log = dir.join("started");
    fs::create_dir_all(&runtime).unwrap();
    fs::write(
        runtime.join("node"),
        format!(
            "#!/bin/sh\necho started >> '{}'\nexec '{}' \"$@\"\n",
            log.display(),
            node.display()
        ),
    )
    .unwrap();
    fs::set_permissions(runtime.join("node"), fs::Permissions::from_mode(0o755)).unwrap();
    let path = std::env::join_paths(
        std::iter::once(runtime.clone()).chain(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        )),
    )
    .unwrap();

    let mut runs = Vec::new();
    for jobs in ["1", "4"] {
        let _ = fs::remove_file(&log);
        let out = format!("out-{jobs}");
        let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
            .args(["-j", jobs, "-o", &out, "src"])
            .current_dir(&dir)
            .env("PATH", &path)
            .output()
            .expect("failed to run ttc");
        assert!(
            output.status.success(),
            "-j {jobs}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let started = fs::read_to_string(&log).unwrap_or_default().lines().count();
        runs.push((jobs, started));
    }
    assert!(runs[0].1 > 0, "the build never asked TypeScript");
    assert_eq!(runs[0].1, runs[1].1, "processes started per -j: {runs:?}");
    for index in 0..8 {
        let name = format!("m{index}.ts");
        assert_eq!(
            fs::read_to_string(dir.join("out-1").join(&name)).unwrap(),
            fs::read_to_string(dir.join("out-4").join(&name)).unwrap()
        );
    }
}

/// Input read failures must not be mistaken for absent type information.
#[test]
fn an_unreadable_sibling_is_reported_as_a_project_input_failure() {
    if !common::toolchain() {
        return;
    }
    let dir = tmpdir();
    let main = dir.join("main.tt");
    fs::write(&main, "declare const flag: boolean;\nexport const v = match(flag) { true => [1], false => [] };\n").unwrap();
    fs::write(dir.join("sibling.tt"), [0xff, 0xfe]).unwrap();
    let output = ttc(&["-p", main.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("sibling.tt"));
    let source = fs::read_to_string(&main).unwrap();
    let filename = main.to_str().unwrap();
    let error = ttc::compile(
        &source,
        &ttc::Options {
            filename: Some(filename),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(error.message.contains("sibling.tt"), "{error}");
}

/// The standard library is the compiler's own, so typing the storage a
/// `@tt/std` program generates cannot require the package to be installed
/// first. It is served to the checker from the compiler's modules.
#[test]
fn a_std_program_is_typed_without_the_package_on_disk() {
    if !common::toolchain() {
        return;
    }
    let dir = tmpdir();
    let file = dir.join("std.tt");
    fs::write(&file, "import * as Result from '@tt/std/result';\ndeclare const flag: boolean;\nexport const answer = match(flag) { true => Result.Ok(1), false => Result.Ok(2) };\n").unwrap();
    assert!(
        !dir.join("node_modules/@tt/std").exists(),
        "this case is about the package *not* being there"
    );

    let output = ttc(&["-p", file.to_str().unwrap()]);
    let emitted = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        emitted.contains("let $tt_v0: "),
        "missing inferred slot type: {emitted}"
    );
    assert!(
        emitted.contains("<number>"),
        "standard library payload was not inferred: {emitted}"
    );
    assert!(
        !dir.join("node_modules").exists(),
        "the package is served, never written: {emitted}"
    );
}

#[cfg(unix)]
#[test]
fn contextual_input_walk_cannot_silently_stop_before_dependencies() {
    if !common::toolchain() {
        return;
    }
    let dir = tmpdir();
    let file = dir.join("main.tt");
    fs::write(&file, "declare const flag: boolean;\nexport const v = match(flag) { true => [1], false => [] };\n").unwrap();
    std::os::unix::fs::symlink(dir.join("missing"), dir.join("aaa-broken")).unwrap();
    let output = ttc(&["-p", file.to_str().unwrap()]);
    assert!(
        !output.status.success(),
        "partial input scan was accepted: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("aaa-broken"));
}

#[test]
fn contextual_support_respects_ancestor_standard_packages() {
    if !common::toolchain() {
        return;
    }
    let dir = tmpdir();
    let package = dir.join("node_modules/@tt/std");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("package.json"),
        r#"{"name":"@tt/std","types":"index.d.ts"}"#,
    )
    .unwrap();
    fs::write(
        package.join("result.d.ts"),
        "export declare function Ok(value: number): string;\n",
    )
    .unwrap();
    let child = dir.join("child");
    fs::create_dir(&child).unwrap();
    let file = child.join("main.tt");
    fs::write(&file, "import * as Result from '@tt/std/result';\ndeclare const flag: boolean;\nexport const v = match(flag) { true => Result.Ok(1), false => Result.Ok(2) };\n").unwrap();
    // The library preserves the authored module specifier while checking.
    let source = fs::read_to_string(&file).unwrap();
    let filename = file.to_str().unwrap();
    let code = ttc::compile(
        &source,
        &ttc::Options {
            filename: Some(filename),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(code.contains("let $tt_v0: string;"), "{code}");
    assert!(!child.join("node_modules").exists());
}

/* ------------------------------------------------------------------ */
/* TASK-377 boundary contracts                                         */
/* ------------------------------------------------------------------ */

fn server_lines(input: &[u8]) -> (Vec<String>, std::process::ExitStatus) {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .arg("--server")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to run ttc");
    let mut stdin = child.stdin.take().expect("stdin piped");
    let input = input.to_vec();
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let out = child.wait_with_output().expect("failed to run ttc");
    writer.join().unwrap().unwrap();
    let lines = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    (lines, out.status)
}

#[test]
fn a_server_line_that_is_not_utf8_is_answered_and_the_session_continues() {
    let mut input = Vec::new();
    input.extend_from_slice(
        b"{\"id\":1,\"method\":\"check\",\"params\":{\"text\":\"const a = 1;\\n\"}}\n",
    );
    input
        .extend_from_slice(b"{\"id\":2,\"method\":\"check\",\"params\":{\"text\":\"\xff\xfe\"}}\n");
    input.extend_from_slice(
        b"{\"id\":3,\"method\":\"check\",\"params\":{\"text\":\"const a = 1;\\n\"}}\n",
    );
    let (lines, status) = server_lines(&input);
    assert!(status.success());
    assert_eq!(lines.len(), 3, "{lines:#?}");
    assert!(lines[0].contains("\"id\":1"), "{}", lines[0]);
    assert!(
        lines[1].contains("\"id\":2") && lines[1].contains("malformed request"),
        "{}",
        lines[1]
    );
    assert!(lines[2].contains("\"id\":3"), "{}", lines[2]);
}

#[test]
fn a_request_whose_parameters_do_not_decode_is_answered_under_its_id() {
    let (lines, status) = server_lines(
        br#"{"id":7,"method":"print","params":{"path":"/x\udc00.tt"}}
{"method":"check","id":8,"params":{"text":"\ud800"}}
{"id":"\udc00","method":"check"}
not json
{"id":9,"method":"check","params":{"text":"const a = 1;\n"}}
"#,
    );
    assert!(status.success());
    assert_eq!(lines.len(), 5, "{lines:#?}");
    for (line, id) in lines.iter().zip(["7", "8", "null", "null"]) {
        let answer: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(answer["id"].to_string(), id, "{line}");
        assert!(
            answer["error"]
                .as_str()
                .is_some_and(|error| error.starts_with("malformed request")),
            "{line}"
        );
    }
    assert!(lines[4].contains("\"id\":9"), "{}", lines[4]);
}

#[test]
fn a_deeply_nested_match_is_answered_and_the_session_continues() {
    let depth = 10_000;
    let text = format!(
        "export const x = {}1{};\n",
        "match (a) { A => ".repeat(depth),
        " }".repeat(depth)
    );
    let mut input = String::new();
    for (id, method) in [(1, "ttHints"), (2, "semanticTokens"), (3, "declarations")] {
        input.push_str(
            &serde_json::json!({
                "id": id,
                "method": method,
                "params": { "path": "/deep/main.tt", "text": text },
            })
            .to_string(),
        );
        input.push('\n');
    }
    input.push_str("{\"id\":4,\"method\":\"check\",\"params\":{\"text\":\"const a = 1;\\n\"}}\n");
    let (lines, status) = server_lines(input.as_bytes());
    assert!(status.success(), "{status}");
    assert_eq!(lines.len(), 4, "{lines:#?}");
    for (line, id) in lines.iter().zip(1..) {
        let answer: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(answer["id"], id, "{line}");
        assert!(answer["error"].is_null(), "{line}");
    }
}

#[test]
fn a_position_only_diagnostic_keeps_a_zero_end_over_the_protocol() {
    let (lines, _) = server_lines(b"{\"id\":3,\"method\":\"check\",\"params\":{\"text\":\"variant A { X }\\nconst v = match (A.X) { }\\n\",\"filename\":\"x.tt\"}}\n");
    assert_eq!(lines.len(), 1, "{lines:#?}");
    assert!(
        lines[0].contains("\"endLine\":0") && lines[0].contains("\"endCol\":0"),
        "{}",
        lines[0]
    );
}

#[test]
fn emit_map_lowers_a_ttx_buffer_as_tsx() {
    let (lines, _) = server_lines(b"{\"id\":1,\"method\":\"emitMap\",\"params\":{\"text\":\"variant S { A, B }\\nexport const el = (s: S) => <div>{match (s) { A => 1, B => 2 }}</div>;\\n\",\"filename\":\"v.ttx\"}}\n");
    assert_eq!(lines.len(), 1, "{lines:#?}");
    assert!(!lines[0].contains("$tt_recovery"), "{}", lines[0]);
    assert!(lines[0].contains("switch ($tt_m.kind)"), "{}", lines[0]);

    let dir = tmpdir();
    let file = dir.join("v.ttx");
    fs::write(&file, "variant S { A, B }\nexport const el = (s: S) => <div>{match (s) { A => 1, B => 2 }}</div>;\n").unwrap();
    let out = ttc(&["--emit-map", file.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains("$tt_recovery"), "{stdout}");
}

#[test]
fn deeply_nested_typescript_is_checked_rather_than_aborting() {
    let dir = tmpdir();
    let file = dir.join("deep.tt");
    let depth = 20_000;
    fs::write(
        &file,
        format!(
            "variant Q {{ A }}\nconst x = {}1{};\n",
            "(".repeat(depth),
            ")".repeat(depth)
        ),
    )
    .unwrap();
    let out = ttc(&["--check", file.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let mut input = Vec::new();
    input.extend_from_slice(b"{\"id\":1,\"method\":\"check\",\"params\":{\"text\":\"");
    input.extend_from_slice(b"const x = ");
    input.extend(std::iter::repeat_n(b'(', depth));
    input.push(b'1');
    input.extend(std::iter::repeat_n(b')', depth));
    input.extend_from_slice(
        b";\\n\"}}\n{\"id\":2,\"method\":\"check\",\"params\":{\"text\":\"const a = 1;\\n\"}}\n",
    );
    let (lines, status) = server_lines(&input);
    assert!(status.success());
    assert_eq!(lines.len(), 2, "{lines:#?}");
}

#[test]
fn a_crlf_file_is_written_with_crlf_throughout() {
    let dir = tmpdir();
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("crlf.tt"),
        "variant O { Some(value: number), None }\r\nexport const f = (o: O) => match (o) {\r\n  Some(value) => value,\r\n  None => 0,\r\n};\r\n",
    )
    .unwrap();
    let out_dir = dir.join("out");
    let out = ttc(&["-o", out_dir.to_str().unwrap(), src.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let emitted = fs::read_to_string(out_dir.join("crlf.ts")).unwrap();
    for line in emitted.split_inclusive('\n') {
        assert!(
            line.ends_with("\r\n"),
            "line without CRLF: {line:?}\n{emitted}"
        );
    }
}

#[test]
fn deeply_nested_host_expressions_with_tt_keep_the_server_alive() {
    let depth = 100_000;
    let text = format!(
        "variant X {{ A }}\nconst x = {}1{};\n",
        "(".repeat(depth),
        ")".repeat(depth)
    );
    let deep =
        serde_json::json!({"id":1,"method":"check","params":{"text":text,"filename":"deep.tt"}});
    let next = serde_json::json!({"id":2,"method":"check","params":{"text":"const x = 1;","filename":"next.tt"}});
    let (lines, status) = server_lines(format!("{deep}\n{next}\n").as_bytes());
    assert!(status.success(), "{status}: {lines:?}");
    assert_eq!(lines.len(), 2, "{lines:?}");
    for (line, id) in lines.iter().zip([1, 2]) {
        let reply: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(reply["id"], id);
        assert_eq!(
            reply["result"]["diagnostics"],
            serde_json::json!([]),
            "{reply}"
        );
    }
}

#[test]
fn an_invalid_file_reports_the_same_diagnostic_with_typescript_installed() {
    require_types_toolchain!();
    let sources = [
        (
            "type R<T> = { kind: \"Ok\"; value: T } | { kind: \"Err\"; error: string };\nfunction res(b: boolean): R<number> {\n  const q = (try result { if (b) { return 10; } return 1; }) * 2;\n  return { kind: \"Ok\", value: q };\n}\n",
            "error[verify-failed]",
            "main.tt:3:25",
        ),
        (
            "declare const b: boolean;\nconst q = (try result { if (b) { return 10; } return 1; }) * 2;\nexport { q };\n",
            "error[try-placement]",
            "main.tt:2:12",
        ),
    ];
    for (source, code, location) in sources {
        let mut reports = Vec::new();
        for dir in [typed_workspace(), Workspace::new("untyped-invalid")] {
            fs::write(dir.join("main.tt"), source).unwrap();
            let out = Command::new(env!("CARGO_BIN_EXE_ttc"))
                .args(["--check", "main.tt"])
                .current_dir(&dir)
                .output()
                .expect("failed to run ttc");
            assert!(!out.status.success());
            reports.push(String::from_utf8_lossy(&out.stderr).into_owned());
        }
        assert!(reports[0].starts_with(code), "{}", reports[0]);
        assert!(reports[0].contains(location), "{}", reports[0]);
        assert_eq!(reports[0], reports[1]);
    }
}

#[test]
fn json_report_belongs_to_one_types_run() {
    let dir = tmpdir();
    let file = dir.join("a.tt");
    fs::write(&file, "export const n = 1;\n").unwrap();
    let path = file.to_str().unwrap();

    for (args, expected) in [
        (
            vec!["--types", "--json-report", "--watch", path],
            "--json-report does not combine with --watch",
        ),
        (
            vec!["--check-types", "--json-report", path],
            "--check-types does not combine with --json-report",
        ),
        (
            vec!["--json-report", path],
            "build mode does not combine with --json-report",
        ),
    ] {
        let out = ttc(&args);
        let err = String::from_utf8_lossy(&out.stderr).into_owned();
        assert_eq!(out.status.code(), Some(1), "{args:?}:\n{err}");
        assert!(out.stdout.is_empty(), "{args:?} prints no report");
        assert!(err.contains(expected), "{args:?}:\n{err}");
    }
}

const TASK_598_MODULE: &str = "declare const path: { basename(s: string): string };\n\
                               declare const input: string;\n\
                               const f = flow |> ((s: string) => s.trim()) |> path.basename;\n\
                               const g = input |> f;\n\
                               export = { f, g };\n";

/// TASK-598: a module written with CommonJS module syntax (`export =`,
/// `import x = require(...)`) cannot import with ECMAScript syntax under
/// `verbatimModuleSyntax`, so it declares its pipeline helpers itself, as
/// a script does, and imports no runtime.
#[test]
fn a_commonjs_module_declares_its_pipeline_helpers() {
    let dir = tmpdir();
    let source = dir.join("src");
    let out_dir = dir.join("out");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("m.tt"), TASK_598_MODULE).unwrap();
    fs::write(
        source.join("r.tt"),
        "import m = require(\"./m.js\");\n\
         declare const step: () => (value: string) => string;\n\
         export const value = m.g |> step();\n",
    )
    .unwrap();
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
    for name in ["m", "r"] {
        let code = fs::read_to_string(out_dir.join(format!("{name}.ts"))).unwrap();
        assert!(!code.contains("runtime"), "{code}");
        assert!(code.contains("var $tt_"), "{code}");
    }
    assert!(!out_dir.join("tt").exists());
}

#[test]
fn a_commonjs_module_type_checks_under_verbatim_module_syntax() {
    require_types_toolchain!();
    let dir = typed_workspace();
    let source = dir.join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(dir.join("package.json"), "{ \"type\": \"commonjs\" }\n").unwrap();
    fs::write(
        dir.join("tsconfig.json"),
        "{ \"compilerOptions\": { \"strict\": true, \"target\": \"es2022\", \"module\": \"nodenext\", \
         \"verbatimModuleSyntax\": true, \"noEmit\": true, \"types\": [] }, \"include\": [\"src\", \"out\"] }\n",
    )
    .unwrap();
    fs::write(source.join("m.tt"), TASK_598_MODULE).unwrap();
    let checked = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["--check-types", "src"])
        .current_dir(&dir)
        .output()
        .expect("failed to run ttc");
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    let built = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["-o", "out", "src"])
        .current_dir(&dir)
        .output()
        .expect("failed to run ttc");
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    if !common::tsc_available() {
        return;
    }
    let tsc = common::tsc()
        .args(["-p", "tsconfig.json"])
        .current_dir(&dir)
        .output()
        .expect("failed to run tsc");
    assert!(
        tsc.status.success(),
        "{}",
        String::from_utf8_lossy(&tsc.stdout)
    );
}

/// TASK-617: a module written with CommonJS syntax imports the standard
/// library in that syntax too, from `tt/cjs/`, so the materialized modules
/// type-check where it does; a module written with ECMAScript syntax keeps
/// importing `tt/`.
#[test]
fn a_commonjs_module_imports_the_standard_library_in_commonjs_syntax() {
    require_types_toolchain!();
    let dir = typed_workspace();
    let source = dir.join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(dir.join("package.json"), "{ \"type\": \"commonjs\" }\n").unwrap();
    fs::write(
        dir.join("tsconfig.json"),
        "{ \"compilerOptions\": { \"strict\": true, \"target\": \"es2022\", \"module\": \"nodenext\", \
         \"verbatimModuleSyntax\": true, \"noEmit\": true, \"types\": [] }, \"include\": [\"src\", \"out\"] }\n",
    )
    .unwrap();
    fs::write(
        source.join("types.tt"),
        "import type { TOption } from \"@tt/std\";\n\
         const none: TOption<number> | undefined = undefined;\n\
         export = none;\n",
    )
    .unwrap();
    fs::write(
        source.join("values.tt"),
        "import option = require(\"@tt/std/option\");\n\
         import result = require(\"@tt/std/result\");\n\
         import type { TOption, TResult } from \"@tt/std\";\n\
         const some: TOption<number> = option.Some(1);\n\
         const ok: TResult<number, string> = option.okOr(some, \"none\");\n\
         export = [some, ok, result.isOk(ok)];\n",
    )
    .unwrap();
    let checked = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["--check-types", "src"])
        .current_dir(&dir)
        .output()
        .expect("failed to run ttc");
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    let built = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["-o", "out", "src"])
        .current_dir(&dir)
        .output()
        .expect("failed to run ttc");
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let values = fs::read_to_string(dir.join("out/values.ts")).unwrap();
    assert!(
        values.contains("require(\"./tt/cjs/option.js\")")
            && values.contains("from \"./tt/cjs/index.js\""),
        "{values}"
    );
    let option = fs::read_to_string(dir.join("out/tt/cjs/option.ts")).unwrap();
    assert!(option.contains("export = option;"), "{option}");
    assert!(!dir.join("out/tt/option.ts").exists());
    if !common::tsc_available() {
        return;
    }
    let tsc = common::tsc()
        .args(["-p", "tsconfig.json"])
        .current_dir(&dir)
        .output()
        .expect("failed to run tsc");
    assert!(
        tsc.status.success(),
        "{}",
        String::from_utf8_lossy(&tsc.stdout)
    );
}

#[test]
fn a_commonjs_module_annotates_storage_with_the_commonjs_standard_library() {
    require_types_toolchain!();
    let dir = typed_workspace();
    let source = dir.join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(dir.join("package.json"), "{ \"type\": \"commonjs\" }\n").unwrap();
    fs::write(
        dir.join("tsconfig.json"),
        "{ \"compilerOptions\": { \"strict\": true, \"target\": \"es2022\", \"module\": \"nodenext\", \
         \"verbatimModuleSyntax\": true, \"noEmit\": true, \"types\": [] }, \"include\": [\"src\", \"out\"] }\n",
    )
    .unwrap();
    fs::write(
        source.join("dep.tt"),
        "import type { TOption } from \"@tt/std\";\n\
         import option = require(\"@tt/std/option\");\n\
         const api = {\n\
         \x20   take(x: TOption<number>, y: number) { return [x, y]; },\n\
         \x20   some: (n: number) => option.Some(n),\n\
         \x20   none: option.None,\n\
         };\n\
         export = api;\n",
    )
    .unwrap();
    fs::write(
        source.join("m.tt"),
        "import api = require(\"./dep.tt\");\n\
         variant G { E, F }\n\
         declare const g: G;\n\
         function f() {\n\
         \x20   return api.take(match (g) { E => { const a = 1; return api.some(a); }, F => api.none }, 0);\n\
         }\n\
         export = f;\n",
    )
    .unwrap();
    let built = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["-o", "out", "src"])
        .current_dir(&dir)
        .output()
        .expect("failed to run ttc");
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let m = fs::read_to_string(dir.join("out/m.ts")).unwrap();
    assert!(
        m.contains("import(\"./tt/cjs/index.js\").TOption<number>"),
        "{m}"
    );
    if !common::tsc_available() {
        return;
    }
    let tsc = common::tsc()
        .args(["-p", "tsconfig.json"])
        .current_dir(&dir)
        .output()
        .expect("failed to run tsc");
    assert!(
        tsc.status.success(),
        "{}",
        String::from_utf8_lossy(&tsc.stdout)
    );
}

/// TASK-617: a module written with ECMAScript syntax keeps the
/// ECMAScript-syntax standard library.
#[test]
fn an_ecmascript_module_keeps_the_standard_library_in_tt() {
    let dir = tmpdir();
    let source = dir.join("src");
    let out_dir = dir.join("out");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("m.tt"),
        "import { Some } from \"@tt/std/option\";\nexport const m = Some(1);\n",
    )
    .unwrap();
    let output = ttc(&["-o", out_dir.to_str().unwrap(), source.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let code = fs::read_to_string(out_dir.join("m.ts")).unwrap();
    assert!(code.contains("from \"./tt/option.js\""), "{code}");
    assert!(out_dir.join("tt/option.ts").is_file());
    assert!(!out_dir.join("tt/cjs").exists());
}

#[test]
fn a_types_report_without_typescript_counts_only_what_it_printed() {
    let dir = tmpdir();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/a.tt"), "export variant V { A, B }\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["--types", "--json-report", "src"])
        .current_dir(&dir)
        .output()
        .expect("failed to run ttc");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(
        stderr.contains("the TypeScript layer did not run"),
        "{stderr}"
    );
    assert!(!stderr.contains("error["), "{stderr}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["checked"], false, "{report}");
    assert_eq!(report["diagnostics"], 0, "{report}");
}

#[test]
fn an_edited_output_written_in_place_is_one_refusal() {
    let dir = tmpdir();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/a.tt"), "export variant V { A, B }\n").unwrap();
    let build = || {
        Command::new(env!("CARGO_BIN_EXE_ttc"))
            .arg("src")
            .current_dir(&dir)
            .output()
            .expect("failed to run ttc")
    };
    assert!(build().status.success());
    let output = dir.join("src/a.ts");
    let mut edited = fs::read_to_string(&output).unwrap();
    edited.push_str("// edited\n");
    fs::write(&output, &edited).unwrap();
    let second = build();
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert_eq!(second.status.code(), Some(1), "{stderr}");
    assert_eq!(
        stderr.trim_end(),
        "ttc: src/a.ts: output is not owned by this input or has been edited; refusing to overwrite it — choose an empty output directory"
    );
    assert_eq!(fs::read_to_string(&output).unwrap(), edited);
}

#[test]
fn symbols_resolve_an_import_to_a_normalized_path() {
    let dir = tmpdir();
    fs::create_dir_all(dir.join("src/sub")).unwrap();
    fs::write(dir.join("src/sub/b.tt"), "export variant B { X, Y }\n").unwrap();
    fs::write(
        dir.join("src/a.tt"),
        "import { B } from \"./sub/b.tt\";\nimport { C } from \"../src/./sub/../c.tt\";\nexport const b = (v: B) => v;\n",
    )
    .unwrap();
    fs::write(dir.join("src/c.tt"), "export variant C { Z }\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["--symbols", "src/a.tt"])
        .current_dir(&dir)
        .output()
        .expect("failed to run ttc");
    assert!(output.status.success());
    let symbols: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let resolved: Vec<&str> = symbols[0]["imports"]
        .as_array()
        .unwrap()
        .iter()
        .map(|import| import["resolved"].as_str().unwrap())
        .collect();
    assert_eq!(resolved, ["src/sub/b.tt", "src/c.tt"]);
}

#[test]
fn a_passed_through_typescript_file_imports_the_written_standard_library() {
    let dir = tmpdir();
    fs::create_dir_all(dir.join("src")).unwrap();
    let authored = "import { Some } from \"@tt/std/option\";\nexport const v = Some(1);\n";
    fs::write(dir.join("src/p.ts"), authored).unwrap();
    fs::write(dir.join("src/m.tt"), "export const w = 1;\n").unwrap();
    for (args, expected) in [
        (
            vec!["-o", "out", "src"],
            "import { Some } from \"./tt/option.js\";\nexport const v = Some(1);\n",
        ),
        (
            vec!["--rewrite-imports", "off", "-o", "off", "src"],
            authored,
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
            .args(&args)
            .current_dir(&dir)
            .output()
            .expect("failed to run ttc");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let written = fs::read_to_string(dir.join(args[args.len() - 2]).join("p.ts")).unwrap();
        assert_eq!(written, expected, "{args:?}");
    }
}

#[test]
fn symbols_list_an_imports_variants_by_what_its_module_exports() {
    let dir = tmpdir();
    fs::write(
        dir.join("a.tt"),
        "variant Shape { Circle(r: number), Square(s: number) }\n\
         variant Other { X, Y }\n\
         export { Shape, Other as Renamed };\n\
         export namespace NS { export variant Inner { P, Q } }\n",
    )
    .unwrap();
    fs::write(
        dir.join("b.tt"),
        "import { Shape, Renamed, NS } from \"./a.tt\";\nexport type T = [Shape, Renamed, NS.Inner];\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["--symbols", "b.tt"])
        .current_dir(&dir)
        .output()
        .expect("failed to run ttc");
    assert!(output.status.success());
    let symbols: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let variants: Vec<(&str, u64)> = symbols[0]["imports"][0]["variants"]
        .as_array()
        .unwrap()
        .iter()
        .map(|variant| {
            (
                variant["name"].as_str().unwrap(),
                variant["line"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(variants, [("Shape", 1), ("Renamed", 2)]);
}

/// A `.ttx` import names the file tsc writes for the `.tsx` ttc emits:
/// `.jsx` under `"jsx": "preserve"` and `.js` under every other `jsx`
/// value or none (typescript-go `GetOutputExtension`), read from the
/// project's configuration through `extends` (a path, a package, a list
/// whose later entry wins) in JSON with comments, or from `--project`.
/// The emitted tree is then compiled by the pinned `tsc` with the same
/// configuration (except without `jsx`, where tsc refuses a `.tsx` module
/// with TS6142): the specifier names a file it wrote, and the program runs
/// under Node.js where its output is JavaScript.
#[test]
fn a_ttx_import_names_the_output_tsc_writes_under_the_projects_jsx_option() {
    if !common::tsc_available() {
        return;
    }
    let base = r#""module": "nodenext", "target": "es2022", "strict": true, "rootDir": "out", "outDir": "js""#;
    type ProjectFiles = Vec<(&'static str, String)>;
    let cases: [(&str, ProjectFiles, &[&str], &str); 9] = [
        (
            "preserve",
            vec![(
                "tsconfig.json",
                format!(
                    r#"{{"compilerOptions": {{"jsx": "preserve", {base}}}, "include": ["out"]}}"#
                ),
            )],
            &[],
            "jsx",
        ),
        (
            "react",
            vec![(
                "tsconfig.json",
                format!(r#"{{"compilerOptions": {{"jsx": "react", {base}}}, "include": ["out"]}}"#),
            )],
            &[],
            "js",
        ),
        (
            "react-jsx",
            vec![(
                "tsconfig.json",
                format!(
                    r#"{{"compilerOptions": {{"jsx": "react-jsx", {base}}}, "include": ["out"]}}"#
                ),
            )],
            &[],
            "js",
        ),
        (
            "react-jsxdev",
            vec![(
                "tsconfig.json",
                format!(
                    r#"{{"compilerOptions": {{"jsx": "react-jsxdev", {base}}}, "include": ["out"]}}"#
                ),
            )],
            &[],
            "js",
        ),
        (
            "react-native",
            vec![(
                "tsconfig.json",
                format!(
                    r#"{{"compilerOptions": {{"jsx": "react-native", {base}}}, "include": ["out"]}}"#
                ),
            )],
            &[],
            "js",
        ),
        (
            "unset",
            vec![(
                "tsconfig.json",
                format!(r#"{{"compilerOptions": {{{base}}}, "include": ["out"]}}"#),
            )],
            &[],
            "js",
        ),
        (
            "extends a path, with comments",
            vec![
                (
                    "tsconfig.json",
                    format!(
                        "{{\n  // the base sets jsx\n  \"extends\": \"./configs/base\",\n  \"compilerOptions\": {{{base},}},\n  \"include\": [\"out\"],\n}}\n"
                    ),
                ),
                (
                    "configs/base.json",
                    r#"{"compilerOptions": {"jsx": "Preserve"}}"#.to_string(),
                ),
            ],
            &[],
            "jsx",
        ),
        (
            "extends packages, the later one winning",
            vec![
                (
                    "tsconfig.json",
                    format!(
                        r#"{{"extends": ["@cfg/preserve", "@cfg/react/strict.json"], "compilerOptions": {{{base}}}, "include": ["out"]}}"#
                    ),
                ),
                (
                    "node_modules/@cfg/preserve/package.json",
                    r#"{"name": "@cfg/preserve", "tsconfig": "base.json"}"#.to_string(),
                ),
                (
                    "node_modules/@cfg/preserve/base.json",
                    r#"{"compilerOptions": {"jsx": "preserve"}}"#.to_string(),
                ),
                (
                    "node_modules/@cfg/react/strict.json",
                    r#"{"compilerOptions": {"jsx": "react-jsx"}}"#.to_string(),
                ),
            ],
            &[],
            "js",
        ),
        (
            "--project",
            vec![
                (
                    "tsconfig.json",
                    format!(
                        r#"{{"compilerOptions": {{"jsx": "react", {base}}}, "include": ["out"]}}"#
                    ),
                ),
                (
                    "tsconfig.preserve.json",
                    format!(
                        r#"{{"compilerOptions": {{"jsx": "preserve", {base}}}, "include": ["out"]}}"#
                    ),
                ),
            ],
            &["--project", "tsconfig.preserve.json"],
            "jsx",
        ),
    ];
    for (name, files, extra, extension) in cases {
        let dir = tmpdir();
        fs::write(dir.join("package.json"), r#"{"type": "module"}"#).unwrap();
        for (path, text) in &files {
            let path = dir.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(
            dir.join("src/view.ttx"),
            "export const label: string = \"view\";\n",
        )
        .unwrap();
        fs::write(
            dir.join("src/app.tt"),
            "import { label } from \"./view.ttx\";\nconsole.log(label);\n",
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
            .current_dir(&*dir)
            .args(["-o", "out", "src"])
            .args(extra)
            .output()
            .expect("ttc runs");
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let app = fs::read_to_string(dir.join("out/app.ts")).unwrap();
        assert!(
            app.contains(&format!("from \"./view.{extension}\"")),
            "{name}: {app}"
        );
        if name == "unset" {
            continue;
        }
        let config = extra.get(1).copied().unwrap_or("tsconfig.json");
        let output = common::tsc()
            .current_dir(&*dir)
            .args(["-p", config])
            .output()
            .expect("tsc runs");
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(dir.join(format!("js/view.{extension}")).is_file(), "{name}");
        if extension == "js" {
            let output = Command::new("node")
                .arg(dir.join("js/app.js"))
                .output()
                .expect("node runs");
            assert!(
                output.status.success(),
                "{name}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(String::from_utf8_lossy(&output.stdout), "view\n", "{name}");
        }
    }
}

/// A build with a `.ttx` source cannot name that module's output when the
/// project's configuration cannot be read; it says so instead of guessing.
/// A build without one does not need the answer.
#[test]
fn an_unreadable_jsx_option_fails_only_a_build_that_has_a_ttx_source() {
    let dir = tmpdir();
    fs::write(
        dir.join("tsconfig.json"),
        r#"{"extends": "./missing.json"}"#,
    )
    .unwrap();
    fs::write(dir.join("app.tt"), "export const a = 1;\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .current_dir(&*dir)
        .args(["-o", "out", "app.tt"])
        .output()
        .expect("ttc runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(dir.join("view.ttx"), "export const v = 1;\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .current_dir(&*dir)
        .args(["-o", "out", "app.tt", "view.ttx"])
        .output()
        .expect("ttc runs");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot read the project's `jsx` option")
            && stderr.contains("cannot find the configuration it extends, \"./missing.json\""),
        "{stderr}"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .current_dir(&*dir)
        .args(["-o", "out", "--rewrite-imports", "ts", "app.tt", "view.ttx"])
        .output()
        .expect("ttc runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
