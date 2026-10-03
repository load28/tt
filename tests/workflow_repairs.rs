//! Developer workflow regressions from TASK-383.
use std::{fs, path::Path, process::Command};
mod common;
use common::Workspace;

fn run(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ttc"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}
fn success(output: std::process::Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn outputs_require_ownership_and_preserve_edits_without_a_banner() {
    let root = Workspace::new("output-ownership");
    fs::write(root.join("a.tt"), "export const value = 1;").unwrap();
    fs::write(root.join("a.ts"), "// authored\n").unwrap();
    assert!(!run(&root, &["a.tt"]).status.success());
    assert_eq!(
        fs::read_to_string(root.join("a.ts")).unwrap(),
        "// authored\n"
    );
    fs::remove_file(root.join("a.ts")).unwrap();
    success(run(&root, &["--no-banner", "a.tt"]));
    fs::write(root.join("a.tt"), "export const value = 2;").unwrap();
    success(run(&root, &["--no-banner", "a.tt"]));
    success(run(&root, &["--no-banner", "."]));
    assert_eq!(
        fs::read_to_string(root.join("a.ts")).unwrap(),
        "export const value = 2;"
    );
    fs::write(root.join("a.ts"), "// edited\n").unwrap();
    assert!(!run(&root, &["a.tt"]).status.success());
    assert_eq!(
        fs::read_to_string(root.join("a.ts")).unwrap(),
        "// edited\n"
    );
}

#[test]
fn configured_custom_extension_patterns_admit_type_errors() {
    if !common::toolchain() {
        return;
    }
    let root = Workspace::in_repo("extension-patterns");
    fs::create_dir(root.join("src")).unwrap();
    for extension in ["tt", "ttx"] {
        let file = format!("src/main.{extension}");
        fs::write(root.join(&file), "export const value: number = \"wrong\";").unwrap();
        for key in ["include", "files"] {
            for pattern in [file.clone(), format!("src/**/*.{extension}")] {
                if key == "files" && pattern.contains('*') {
                    continue;
                }
                fs::write(root.join("tsconfig.json"), serde_json::json!({"compilerOptions":{"strict":true,"jsx":"preserve"},(key):[pattern]}).to_string()).unwrap();
                let output = run(&root, &["--check-types", &file]);
                assert!(!output.status.success(), "{key} {pattern}");
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("2322"),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        fs::remove_file(root.join(file)).unwrap();
    }
}

#[test]
fn relative_declaration_roots_preserve_duplicate_basenames() {
    if !common::toolchain() {
        return;
    }
    let root = Workspace::in_repo("declaration-layout");
    for (name, value) in [("a", "1"), ("b", "\"hello\"")] {
        fs::create_dir_all(root.join(format!("src/{name}"))).unwrap();
        fs::write(
            root.join(format!("src/{name}/model.tt")),
            format!("export const value = {value};"),
        )
        .unwrap();
    }
    success(run(&root, &["--types", "src", "-o", "types"]));
    assert!(
        fs::read_to_string(root.join("types/a/model.tt.d.ts"))
            .unwrap()
            .contains("1")
    );
    assert!(
        fs::read_to_string(root.join("types/b/model.tt.d.ts"))
            .unwrap()
            .contains("hello")
    );
}

#[test]
fn jsx_analysis_keeps_all_arms_and_generic_constructors() {
    if !common::toolchain() {
        return;
    }
    for fixture in [
        "jsx-coverage",
        "jsx-generic",
        "jsx-nested-arrow",
        "jsx-nested-captures",
        "jsx-block-callback",
    ] {
        let root = Workspace::in_repo("jsx-workflow");
        fs::write(root.join("tsconfig.json"), r#"{"compilerOptions":{"strict":true,"jsx":"preserve","noEmit":true},"include":["*.ttx"]}"#).unwrap();
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/fixtures/emit/workflow-{fixture}/input.ttx")),
            root.join("main.ttx"),
        )
        .unwrap();
        success(run(&root, &["--check-types", "main.ttx"]));
    }
}

#[test]
fn project_discovers_relative_tt_modules_outside_the_config_root() {
    if !common::toolchain() {
        return;
    }
    let root = Workspace::in_repo("workspace-dependencies");
    fs::create_dir_all(root.join("app/src")).unwrap();
    fs::create_dir_all(root.join("domain")).unwrap();
    fs::write(
        root.join("domain/model.tt"),
        "export variant State { Ready(value: number), Empty }",
    )
    .unwrap();
    fs::write(root.join("app/tsconfig.json"), r#"{"compilerOptions":{"strict":true,"noEmit":true,"module":"preserve","moduleResolution":"bundler","allowImportingTsExtensions":true},"include":["src/**/*.tt"]}"#).unwrap();
    fs::write(root.join("app/src/main.tt"), "import {State} from '../../domain/model.tt'; export const value = match(State.Ready(1)){Ready(value)=>value,Empty=>0};").unwrap();
    success(run(&root.join("app"), &["--check-types", "src"]));
    let output = run(&root.join("app"), &["--dependencies", "src/main.tt"]);
    assert!(output.status.success());
    let dependencies = dependency_files(&output);
    assert!(
        dependencies
            .iter()
            .any(|path| path.ends_with("domain/model.tt"))
    );
}

/// The `files` of what `--dependencies` printed.
fn dependency_files(output: &std::process::Output) -> Vec<std::path::PathBuf> {
    let printed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    serde_json::from_value(printed["files"].clone()).unwrap()
}

#[test]
fn dependencies_list_directories_apart_from_files() {
    if !common::toolchain() {
        return;
    }
    let root = Workspace::in_repo("dependency-directories");
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/main.tt"), "export const value = 1;").unwrap();
    fs::write(root.join("src/extra.ts"), "export const extra = 2;").unwrap();
    fs::write(
        root.join("tsconfig.json"),
        r#"{"compilerOptions":{"strict":true,"noEmit":true},"include":["src"]}"#,
    )
    .unwrap();
    let output = run(&root, &["--dependencies", "src/main.tt"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let printed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let paths = |key: &str| -> Vec<std::path::PathBuf> {
        serde_json::from_value(printed[key].clone()).unwrap_or_else(|_| panic!("{printed}"))
    };
    let files = paths("files");
    let directories = paths("directories");
    // A build integration registers a file and a directory differently, so
    // neither list may hold the other kind.
    assert!(
        files.iter().all(|path| !path.is_dir()),
        "a directory among the files: {files:?}"
    );
    assert!(
        directories.iter().all(|path| path.is_dir()),
        "{directories:?}"
    );
    let src = fs::canonicalize(root.join("src")).unwrap();
    assert!(directories.contains(&src), "{directories:?}");
    assert!(files.contains(&src.join("extra.ts")), "{files:?}");
}

#[test]
fn dependencies_of_a_configured_project_are_only_its_inputs() {
    if !common::toolchain() {
        return;
    }
    let root = Workspace::in_repo("configured-dependencies");
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/main.tt"), "export const value = 1;").unwrap();
    fs::write(
        root.join("tsconfig.json"),
        r#"{"compilerOptions":{"strict":true,"noEmit":true},"include":["src"]}"#,
    )
    .unwrap();
    let output = run(&root, &["--dependencies", "src"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let dependencies = dependency_files(&output);
    assert!(
        dependencies
            .iter()
            .any(|path| path.ends_with("tsconfig.json")),
        "{dependencies:?}"
    );
    // The project and the TypeScript it resolves both live in this
    // repository; a file the compiler wrote for itself does not. The
    // configuration discovery probed and did not find is there too.
    let repository = fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap();
    let src = fs::canonicalize(root.join("src")).unwrap();
    assert!(
        dependencies.contains(&src.join("tsconfig.json")),
        "{dependencies:?}"
    );
    let outside: Vec<_> = dependencies
        .iter()
        .filter(|path| {
            !fs::canonicalize(path)
                .unwrap_or_else(|_| path.to_path_buf())
                .starts_with(&repository)
        })
        .collect();
    assert!(outside.is_empty(), "{outside:?}");
}

struct Watch(std::process::Child);
impl Drop for Watch {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn wait_for(mut ready: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while !ready() {
        assert!(
            std::time::Instant::now() < deadline,
            "watch did not observe the change"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

#[test]
fn typed_watch_invalidates_host_inputs_and_refreshes_emission_membership() {
    if !common::toolchain() {
        return;
    }
    let root = Workspace::in_repo("typed-watch-dependencies");
    fs::create_dir(root.join("src")).unwrap();
    fs::write(
        root.join("src/dep.mts"),
        "export const value: string = 'ok';",
    )
    .unwrap();
    fs::write(
        root.join("src/main.tt"),
        "import {value} from './dep.mjs'; const check: string = value; export const n = check;",
    )
    .unwrap();
    fs::write(root.join("tsconfig.json"), r#"{"compilerOptions":{"strict":true,"module":"preserve","moduleResolution":"bundler"},"include":["src"]}"#).unwrap();
    let log = root.join("watch.log");
    let _watch = Watch(
        Command::new(env!("CARGO_BIN_EXE_ttc"))
            .current_dir(&root)
            .args(["--types", "--watch", "src", "-o", "types"])
            .stdout(std::process::Stdio::null())
            .stderr(fs::File::create(&log).unwrap())
            .spawn()
            .unwrap(),
    );
    wait_for(|| fs::read_to_string(&log).unwrap().contains("Ctrl-C"));
    fs::write(root.join("src/dep.mts"), "export const value: number = 1;").unwrap();
    wait_for(|| fs::read_to_string(&log).unwrap().contains("ts2322"));
    fs::write(root.join("src/new.tt"), "export const added = 42;").unwrap();
    wait_for(|| root.join("types/new.tt.d.ts").exists());
    fs::write(
        root.join("src/main.tt"),
        "export const n = 1; const unused = 2;",
    )
    .unwrap();
    fs::write(root.join("tsconfig.json"), r#"{"compilerOptions":{"noUnusedLocals":true,"module":"preserve","moduleResolution":"bundler"},"include":["src"]}"#).unwrap();
    wait_for(|| fs::read_to_string(&log).unwrap().contains("ts6133"));
}

#[test]
fn extended_config_patterns_preserve_exclusions_and_authored_config() {
    if !common::toolchain() {
        return;
    }
    let root = Workspace::in_repo("extended-config-patterns");
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/main.tt"), "export const value = 1;").unwrap();
    fs::write(
        root.join("src/excluded.tt"),
        "export const value: number = 'wrong';",
    )
    .unwrap();
    let base = r#"{ // JSONC remains authored
        "compilerOptions": {"strict": true},
        "include": ["src/**/*.tt"], "exclude": ["src/excluded.tt"]
    }"#;
    fs::write(root.join("base.json"), base).unwrap();
    fs::write(root.join("tsconfig.json"), r#"{"extends":"./base.json"}"#).unwrap();
    success(run(&root, &["--check-types", "src"]));
    fs::write(
        root.join("src/main.tt"),
        "export const value: number = 'wrong';",
    )
    .unwrap();
    let output = run(&root, &["--check-types", "src"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("2322"));
    assert_eq!(fs::read_to_string(root.join("base.json")).unwrap(), base);
}

/// `ttc --types -o <dir>` writes its sidecars under the project root. With
/// no `include` the configuration's default glob reaches that directory, and
/// `tsc` leaves its own output directory out of that glob; ttc's must stay
/// out of the program too, or every later pass checks the sources against
/// the declarations an earlier one wrote.
#[test]
fn types_output_directory_is_not_a_program_input() {
    if !common::toolchain() {
        return;
    }
    let cases: [(&str, &[(&str, &str)]); 2] = [
        (
            r#"{"compilerOptions":{"strict":true,"module":"nodenext"}}"#,
            &[
                ("src/m.tt", "export variant K { A, B }\n"),
                (
                    "src/u.tt",
                    "import { K } from \"./m.tt\";\nexport const k: K = K.A;\n",
                ),
            ],
        ),
        (
            r#"{"compilerOptions":{"strict":true,"module":"esnext","moduleResolution":"bundler"}}"#,
            &[("src/g.tt", "const globalThing: number = 1;\n")],
        ),
    ];
    for (config, sources) in cases {
        let root = Workspace::in_repo_with_subdir("types-output-directory", "src");
        fs::write(root.join("tsconfig.json"), config).unwrap();
        for (path, text) in sources {
            fs::write(root.join(path), text).unwrap();
        }
        for _ in 0..2 {
            success(run(&root, &["--types", "-o", "types", "src"]));
        }

        let log = root.join("watch.log");
        let _watch = Watch(
            Command::new(env!("CARGO_BIN_EXE_ttc"))
                .current_dir(&root)
                .args(["--types", "--watch", "-o", "types", "src"])
                .stdout(std::process::Stdio::null())
                .stderr(fs::File::create(&log).unwrap())
                .spawn()
                .unwrap(),
        );
        wait_for(|| fs::read_to_string(&log).unwrap().contains("Ctrl-C"));
        let (path, text) = sources[0];
        fs::write(root.join(path), format!("{text}export const edited = 1;\n")).unwrap();
        wait_for(|| {
            fs::read_to_string(&log)
                .unwrap()
                .matches("reported in")
                .count()
                >= 2
        });
        let log = fs::read_to_string(&log).unwrap();
        assert!(!log.contains("error"), "{log}");
    }
}

/// `-o` naming the directory the sources are in — how an editor refreshes
/// the sidecars beside a file — keeps those sources in the program: only
/// what ttc writes there is left out.
#[test]
fn types_written_beside_the_sources_keep_the_sources_as_inputs() {
    if !common::toolchain() {
        return;
    }
    let root = Workspace::in_repo_with_subdir("types-beside-sources", "src");
    fs::write(
        root.join("tsconfig.json"),
        r#"{"compilerOptions":{"strict":true,"module":"esnext","moduleResolution":"bundler"}}"#,
    )
    .unwrap();
    fs::write(root.join("src/m.tt"), "export variant K { A, B }\n").unwrap();
    fs::write(
        root.join("src/u.tt"),
        "import { K } from \"./m.tt\";\nexport const k: K = K.A;\nconst globalThing: number = 1;\n",
    )
    .unwrap();
    for _ in 0..2 {
        success(run(&root, &["--types", "-o", "src", "src"]));
        assert!(root.join("src/u.tt.d.ts").is_file());
        assert!(root.join("src/m.tt.d.ts").is_file());
    }
}

/// TASK-587: the outputs a build published beside its sources, or into an
/// output directory the configuration globs, are ttc's own and never inputs
/// of the TypeScript program — the rule `tsc` applies to its outputs. An
/// output someone edited is theirs again, and an input.
#[test]
fn check_types_leaves_published_outputs_out_of_the_program() {
    if !common::toolchain() {
        return;
    }
    let root = Workspace::in_repo_with_subdir("check-types-owned-outputs", "src");
    fs::write(root.join("src/m.tt"), "export const m: number = \"x\";\n").unwrap();
    fs::write(root.join("src/s.tt"), "let dup = 1;\n").unwrap();
    success(run(&root, &["src"]));
    success(run(&root, &["-o", "build", "src"]));
    assert!(root.join("src/m.ts").is_file() && root.join("build/s.ts").is_file());
    let check = |root: &Path| {
        let output = run(root, &["--check-types", "src"]);
        assert!(!output.status.success());
        String::from_utf8_lossy(&output.stderr).into_owned()
    };
    for config in [None, Some(r#"{"compilerOptions":{"strict":true}}"#)] {
        if let Some(config) = config {
            fs::write(root.join("tsconfig.json"), config).unwrap();
        }
        let stderr = check(&root);
        assert_eq!(stderr.matches("error[").count(), 1, "{stderr}");
        assert!(stderr.contains("src/m.tt:1:14"), "{stderr}");
        assert!(
            !stderr.contains("m.ts") && !stderr.contains("ts2451"),
            "{stderr}"
        );
    }
    fs::write(root.join("src/s.ts"), "let dup = 2;\n").unwrap();
    let stderr = check(&root);
    assert!(stderr.contains("ts2451"), "{stderr}");
}

/// TASK-616: owned outputs are left out of globbing only, as `exclude`
/// leaves files out for `tsc`. An import that names the source reaches its
/// projection once; an import that names the output reaches the output.
#[test]
fn an_import_decides_whether_an_owned_output_joins_the_program() {
    if !common::toolchain() {
        return;
    }
    let root = Workspace::in_repo_with_subdir("owned-output-imports", "src");
    fs::write(root.join("package.json"), r#"{"type":"module"}"#).unwrap();
    fs::write(
        root.join("tsconfig.json"),
        r#"{"compilerOptions":{"strict":true,"module":"nodenext","noEmit":true,"types":[]},"include":["src"]}"#,
    )
    .unwrap();
    fs::write(
        root.join("src/m.tt"),
        "export const m = 1;\nconst bad: string = 3;\n",
    )
    .unwrap();
    success(run(&root, &["src"]));
    let check = |specifier: &str| {
        fs::write(
            root.join("src/h.ts"),
            format!("import {{ m }} from \"{specifier}\";\nexport const h: number = m;\n"),
        )
        .unwrap();
        let output = run(&root, &["--check-types", "src"]);
        assert!(!output.status.success());
        String::from_utf8_lossy(&output.stderr).into_owned()
    };
    let stderr = check("./m.tt");
    assert_eq!(stderr.matches("error[").count(), 1, "{stderr}");
    assert!(stderr.contains("src/m.tt:2:7"), "{stderr}");
    let stderr = check("./m.js");
    assert!(stderr.contains("src/m.tt:2:7"), "{stderr}");
    assert!(stderr.contains("src/m.ts:"), "{stderr}");
}

/// TASK-588: which configuration the inputs belong to is decided by
/// discovery, and a watch reaches the result a fresh run would when a
/// `tsconfig.json` is created or deleted.
#[test]
fn typed_watch_follows_configuration_discovery() {
    if !common::toolchain() {
        return;
    }
    let root = Workspace::in_repo_with_subdir("typed-watch-discovery", "src");
    fs::write(
        root.join("src/a.tt"),
        "export function f(x) { return x; }\n",
    )
    .unwrap();
    let log = root.join("watch.log");
    let _watch = Watch(
        Command::new(env!("CARGO_BIN_EXE_ttc"))
            .current_dir(&root)
            .args(["--check-types", "--watch", "src"])
            .stdout(std::process::Stdio::null())
            .stderr(fs::File::create(&log).unwrap())
            .spawn()
            .unwrap(),
    );
    let passes = |reported: &str| {
        fs::read_to_string(&log)
            .unwrap()
            .matches(&format!("{reported} reported"))
            .count()
    };
    wait_for(|| fs::read_to_string(&log).unwrap().contains("Ctrl-C"));
    assert_eq!(passes("1"), 1);
    fs::write(
        root.join("tsconfig.json"),
        r#"{"compilerOptions":{"strict":false},"include":["src"]}"#,
    )
    .unwrap();
    wait_for(|| passes("0") == 1);
    fs::remove_file(root.join("tsconfig.json")).unwrap();
    wait_for(|| passes("1") == 2);
    let log = fs::read_to_string(&log).unwrap();
    assert_eq!(log.matches("ts7006").count(), 2, "{log}");
    assert!(!log.contains("ts5083"), "{log}");
}

/// TASK-614: an edit made while the configuration cannot be parsed reaches
/// the program once it can be again, whether the fix lands in a later pass
/// or in the same one.
#[test]
fn typed_watch_checks_edits_made_while_the_configuration_was_malformed() {
    if !common::toolchain() {
        return;
    }
    let root = Workspace::in_repo_with_subdir("typed-watch-malformed-config", "src");
    let good = r#"{"compilerOptions":{"strict":true,"noEmit":true,"types":[]},"include":["src"]}"#;
    let broken = r#"{"compilerOptions":{"strict":true,,}"#;
    fs::write(root.join("tsconfig.json"), good).unwrap();
    fs::write(root.join("src/a.tt"), "export const a = 1;\n").unwrap();
    let log = root.join("watch.log");
    let _watch = Watch(
        Command::new(env!("CARGO_BIN_EXE_ttc"))
            .current_dir(&root)
            .args(["--check-types", "--watch", "src"])
            .stdout(std::process::Stdio::null())
            .stderr(fs::File::create(&log).unwrap())
            .spawn()
            .unwrap(),
    );
    let read = || fs::read_to_string(&log).unwrap();
    let passes = || read().matches(" reported in ").count();
    let settled_on = |message: &str| {
        let log = read();
        log.contains(message) && log.trim_end().ends_with("watching")
    };
    let last = || read().trim_end().lines().last().unwrap_or("").to_owned();
    wait_for(|| read().contains("Ctrl-C"));
    fs::write(root.join("tsconfig.json"), broken).unwrap();
    wait_for(|| read().contains("ts1136"));
    let before = passes();
    fs::write(root.join("src/a.tt"), "export const a: string = 1;\n").unwrap();
    wait_for(|| passes() > before);
    fs::write(root.join("tsconfig.json"), good).unwrap();
    wait_for(|| settled_on("to type 'string'"));
    assert!(last().contains(" 1 reported in "), "{}", read());

    fs::write(root.join("tsconfig.json"), broken).unwrap();
    wait_for(|| read().matches("ts1136").count() > 1);
    fs::write(root.join("src/a.tt"), "export const a: number = \"x\";\n").unwrap();
    fs::write(root.join("tsconfig.json"), good).unwrap();
    wait_for(|| settled_on("to type 'number'"));
    assert!(last().contains(" 1 reported in "), "{}", read());
}

/// TASK-588: without a configuration, the program is the walk of the
/// inputs' directory, and discovery read every `tsconfig.json` path it
/// probed; both are what `--dependencies` answers.
#[test]
fn dependencies_of_an_inferred_project_name_discovery_and_the_walk() {
    if !common::toolchain() {
        return;
    }
    let root = Workspace::in_repo_with_subdir("inferred-dependencies", "src/sub");
    fs::write(root.join("src/a.tt"), "export const a = 1;\n").unwrap();
    let output = run(&root, &["--dependencies", "src"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let printed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let paths = |key: &str| -> Vec<std::path::PathBuf> {
        serde_json::from_value(printed[key].clone()).unwrap()
    };
    let src = fs::canonicalize(root.join("src")).unwrap();
    let directories = paths("directories");
    assert!(
        directories.contains(&src) && directories.contains(&src.join("sub")),
        "{directories:?}"
    );
    let files = paths("files");
    for dir in src.ancestors() {
        assert!(files.contains(&dir.join("tsconfig.json")), "{files:?}");
    }
}

/// TASK-589: an unedited output whose recorded input no longer exists
/// belongs to the input that now maps to it (`a.ts` renamed to `a.tt`). An
/// edited output, or one whose input still exists, stays protected.
#[test]
fn a_renamed_input_takes_over_the_output_its_predecessor_left() {
    let root = Workspace::new("output-takeover");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("other")).unwrap();
    fs::write(root.join("src/a.ts"), "export const a = 1;\n").unwrap();
    fs::write(root.join("src/b.ts"), "export const b = 1;\n").unwrap();
    fs::write(root.join("other/c.ts"), "export const c = 1;\n").unwrap();
    success(run(&root, &["-o", "out", "src"]));
    success(run(&root, &["-o", "out", "other"]));
    fs::rename(root.join("src/a.ts"), root.join("src/a.tt")).unwrap();
    fs::write(root.join("src/a.tt"), "export const a: number = 2;\n").unwrap();
    success(run(&root, &["-o", "out", "src"]));
    let out = fs::read_to_string(root.join("out/a.ts")).unwrap();
    assert!(out.contains("export const a: number = 2;"), "{out}");

    fs::write(root.join("out/b.ts"), "// edited\n").unwrap();
    fs::rename(root.join("src/b.ts"), root.join("src/b.tt")).unwrap();
    assert!(!run(&root, &["-o", "out", "src/b.tt"]).status.success());
    assert_eq!(
        fs::read_to_string(root.join("out/b.ts")).unwrap(),
        "// edited\n"
    );

    fs::write(root.join("src/c.tt"), "export const c = 2;\n").unwrap();
    assert!(!run(&root, &["-o", "out", "src/c.tt"]).status.success());
    assert_eq!(
        fs::read_to_string(root.join("out/c.ts")).unwrap(),
        "export const c = 1;\n"
    );
}

/// TASK-589: the same takeover lets a build watch recover from the rename.
#[test]
fn build_watch_recovers_from_a_renamed_input() {
    let root = Workspace::new("output-takeover-watch");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/a.ts"), "export const a = 1;\n").unwrap();
    let log = root.join("watch.log");
    let _watch = Watch(
        Command::new(env!("CARGO_BIN_EXE_ttc"))
            .current_dir(&root)
            .args(["-w", "-o", "out", "src"])
            .stdout(std::process::Stdio::null())
            .stderr(fs::File::create(&log).unwrap())
            .spawn()
            .unwrap(),
    );
    wait_for(|| fs::read_to_string(&log).unwrap().contains("Ctrl-C"));
    fs::rename(root.join("src/a.ts"), root.join("src/a.tt")).unwrap();
    wait_for(|| {
        fs::read_to_string(root.join("out/a.ts")).is_ok_and(|out| out.contains("from a.tt"))
    });
    let log = fs::read_to_string(&log).unwrap();
    assert!(!log.contains("not owned"), "{log}");
}

/// TASK-615: a source whose path is not Unicode cannot be named by an
/// ownership record, nor by TypeScript; the build refuses it before it
/// writes anything, and the typed check reports it rather than failing.
// This traversal fixture requires invalid-byte directory entries, which the
// macOS filesystem rejects. Unix path validation without such entries is
// covered by the ownership unit test and engine_cache's unsaved-overlay test.
#[cfg(target_os = "linux")]
#[test]
fn an_input_path_that_is_not_unicode_is_refused_before_anything_is_written() {
    use std::os::unix::ffi::OsStrExt;
    let root = Workspace::in_repo_with_subdir("non-unicode-input", "src");
    let bad = root
        .join("src")
        .join(std::ffi::OsStr::from_bytes(b"bad\xff.tt"));
    fs::write(&bad, "export const bad = 1;\n").unwrap();
    fs::write(root.join("src/good.tt"), "export const good = 1;\n").unwrap();
    for args in [&["-o", "out", "src"][..], &["src"][..]] {
        let output = run(&root, args);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{stderr}");
        assert!(stderr.contains("input path is not valid UTF-8"), "{stderr}");
        assert!(!root.join("out").exists() && !root.join("src/good.ts").exists());
    }
    if common::toolchain() {
        let output = run(&root, &["--check-types", "src"]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{stderr}");
        assert!(stderr.contains("path is not valid UTF-8"), "{stderr}");
    }
}

/// TASK-615: a publication writes the record first, naming the bytes it
/// replaces, so an output interrupted before its new bytes landed is still
/// ttc's, and the next build completes it.
#[test]
fn an_interrupted_publication_leaves_the_output_owned() {
    let root = Workspace::new("interrupted-publication");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/a.tt"), "export const a = 1;\n").unwrap();
    success(run(&root, &["--no-banner", "-o", "out", "src"]));
    let record_path = root.join("out/.a.ts.ttc-output.json");
    let published = fs::read_to_string(root.join("out/a.ts")).unwrap();
    let mut record: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&record_path).unwrap()).unwrap();
    record["replaced"] = serde_json::Value::from(published.as_str());
    record["content"] = serde_json::Value::from("export const a = 2;\n");
    fs::write(&record_path, record.to_string()).unwrap();
    fs::write(root.join("src/a.tt"), "export const a = 3;\n").unwrap();
    success(run(&root, &["--no-banner", "-o", "out", "src"]));
    assert_eq!(
        fs::read_to_string(root.join("out/a.ts")).unwrap(),
        "export const a = 3;\n"
    );
    let record: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&record_path).unwrap()).unwrap();
    assert!(record["replaced"].is_null(), "{record}");
    assert_eq!(record["content"], "export const a = 3;\n");
}
