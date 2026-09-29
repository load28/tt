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
    // repository; a file the compiler wrote for itself does not.
    let repository = fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap();
    let outside: Vec<_> = dependencies
        .iter()
        .filter(|path| !fs::canonicalize(path).is_ok_and(|path| path.starts_with(&repository)))
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
