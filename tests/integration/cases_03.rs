#[test]
fn cli_build_emits_a_complete_tree_that_runs() {
    require_toolchain!();
    let dir = tmpdir();
    write_consumer_tree(&dir);

    let (ok, err) = run_ttc(&dir, &["-o", "build", "--no-banner", "src"]);
    assert!(ok, "build failed:\n{err}");

    // Hand-written TypeScript rides along byte-for-byte except for its
    // relative `.tt` (and `@tt/std`) specifiers.
    let main_ts = fs::read_to_string(dir.join("build/main.ts")).unwrap();
    assert_eq!(
        main_ts,
        CONSUMER_MAIN_TS
            .replace("./notice.tt", "./notice.js")
            .replace("@tt/std/option", "./tt/option.js")
    );
    for module in ttc::StdModule::STANDARD {
        assert!(dir.join("build/tt").join(module.file_name()).exists());
    }

    // The emitted tree stands on its own: tsc compiles it, node runs it.
    fs::write(dir.join("build/package.json"), "{ \"type\": \"module\" }\n").unwrap();
    let out = common::tsc()
        .current_dir(&dir)
        .args(["build/main.ts", "--outDir", "build"])
        .args(TSC_FLAGS)
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "tsc failed:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out = Command::new("node")
        .current_dir(&dir)
        .arg("build/main.js")
        .output()
        .expect("failed to run node");
    assert!(
        out.status.success(),
        "node failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        ["info: hello", "warn[7]: careful", "true"]
    );
}

#[test]
fn cli_refuses_to_overwrite_a_pass_through_input() {
    let dir = tmpdir();
    fs::write(dir.join("a.tt"), "export const a = 1;\n").unwrap();
    fs::write(dir.join("main.ts"), "import { a } from \"./a.tt\";\nexport const x = a;\n").unwrap();

    // In place, a pass-through `.ts` whose specifiers are rewritten would
    // land on top of itself.
    let (ok, err) = run_ttc(&dir, &["main.ts"]);
    assert!(!ok, "expected failure:\n{err}");
    assert!(err.contains("output would overwrite the input"), "{err}");
    let untouched = fs::read_to_string(dir.join("main.ts")).unwrap();
    assert_eq!(untouched, "import { a } from \"./a.tt\";\nexport const x = a;\n");

    // A separate output tree is fine.
    let (ok, err) = run_ttc(&dir, &["-o", "out", "main.ts"]);
    assert!(ok, "build failed:\n{err}");
}

#[test]
fn cli_in_place_directory_build_keeps_unchanged_pass_through_inputs() {
    let dir = tmpdir();
    fs::create_dir(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.tt"), "export const a = 1;\n").unwrap();
    fs::write(dir.join("src/plain.ts"), "export const p = 2;\n").unwrap();
    fs::write(dir.join("src/imp.ts"), "import { a } from \"./main.tt\";\nexport const q = a;\n").unwrap();

    let (ok, err) = run_ttc(&dir, &["src"]);
    assert!(!ok, "expected failure:\n{err}");
    assert!(err.contains("src/imp.ts: output would overwrite the input"), "{err}");
    assert!(!err.contains("plain.ts"), "{err}");
    assert!(dir.join("src/main.ts").exists(), "{err}");
    assert_eq!(fs::read_to_string(dir.join("src/plain.ts")).unwrap(), "export const p = 2;\n");
    assert_eq!(
        fs::read_to_string(dir.join("src/imp.ts")).unwrap(),
        "import { a } from \"./main.tt\";\nexport const q = a;\n"
    );

    fs::remove_file(dir.join("src/imp.ts")).unwrap();
    let (ok, err) = run_ttc(&dir, &["src"]);
    assert!(ok, "build failed:\n{err}");
    assert_eq!(fs::read_to_string(dir.join("src/plain.ts")).unwrap(), "export const p = 2;\n");
}

#[test]
fn cli_types_leaves_nothing_but_the_sidecars() {
    require_toolchain!();
    require_types_typescript!();
    let dir = project_dir();
    write_consumer_tree(&dir);

    let (ok, err) = run_ttc(&dir, &["--types", "src"]);
    assert!(ok, "--types failed:\n{err}");

    // Declaration emit runs in memory: no cache tree, and above all no
    // copy of the hand-written TypeScript anywhere.
    assert!(!dir.join(".tt-build").exists(), "a cache tree was created");
    let copies: Vec<String> = walk(&dir)
        .into_iter()
        .filter(|path| {
            path.file_name().is_some_and(|name| name == "main.ts")
                && !path.starts_with(dir.join("src"))
        })
        .map(|path| path.display().to_string())
        .collect();
    assert!(
        copies.is_empty(),
        "hand-written source was copied: {copies:?}"
    );

    // What it does leave: one sidecar pair per .tt, plus the std types.
    assert!(dir.join(".tt-types/notice.tt.d.ts").exists());
    assert!(dir.join(".tt-types/notice.tt.d.ts.map").exists());
    assert!(dir.join(".tt-types/level.tt.d.ts").exists());
    for module in ttc::StdModule::STANDARD {
        assert!(
            dir.join(".tt-types/tt")
                .join(module.file_name())
                .with_extension("d.ts")
                .exists()
        );
    }
}

#[test]
fn cli_types_reports_type_errors_but_keeps_the_sidecars_fresh() {
    require_toolchain!();
    require_types_typescript!();
    let dir = project_dir();
    write_consumer_tree(&dir);
    // A type error in the consumer, not a tt-level one: declarations are
    // still emitted, so the sidecars must be written and the run must fail.
    fs::write(
        dir.join("src/main.ts"),
        format!("{CONSUMER_MAIN_TS}\nconst wrong: number = \"text\";\n"),
    )
    .unwrap();

    let (ok, err) = run_ttc(&dir, &["--types", "src"]);
    assert!(!ok, "expected a failing exit code:\n{err}");
    assert!(
        err.contains("main.ts"),
        "diagnostic should name the file: {err}"
    );
    assert!(
        dir.join(".tt-types/notice.tt.d.ts").exists(),
        "sidecars should still be written: {err}"
    );
}

#[test]
fn cli_types_reports_tt_type_errors_at_the_source_position() {
    require_toolchain!();
    require_types_typescript!();
    let dir = project_dir();
    write_consumer_tree(&dir);
    // A type error *inside* tt syntax. The emitted TypeScript is a switch
    // IIFE that moves the offending expression far from where it was
    // written, and the file it lives in is never written to disk — the
    // diagnostic has to name `bad.tt` and the source line/column anyway.
    let bad = "import type { TResult } from \"@tt/std\";\n\
               import * as Result from \"@tt/std/result\";\n\
               \n\
               declare function evaluate(): TResult<number, string>;\n\
               \n\
               export const bad = evaluate() |> Result.mapP((n) => n.length);\n";
    fs::write(dir.join("src/bad.tt"), bad).unwrap();

    let (ok, err) = run_ttc(&dir, &["--types", "src"]);
    assert!(!ok, "expected a failing exit code:\n{err}");

    // `length` sits at column 55 of line 5 of the source. The emitted code
    // puts it elsewhere entirely, and there is no `bad.ts` to open. The
    // message and the position have to belong to the *same* diagnostic, so
    // this reads the rendered block rather than two independent lines.
    let reported = err
        .split("error[")
        .find(|block| block.contains("does not exist on type"))
        .unwrap_or_else(|| panic!("no type error reported:\n{err}"));
    assert!(
        reported.contains("--> src/bad.tt:6:55"),
        "diagnostic should point into the .tt source: {reported}"
    );
    assert!(
        !err.contains("bad.ts"),
        "named a file that does not exist: {err}"
    );
}

#[test]
fn cli_types_without_typescript_says_so() {
    require_toolchain!();
    let dir = tmpdir();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/level.tt"), LEVEL_TT).unwrap();
    // No TypeScript on purpose: a project's TypeScript comes from its own
    // `node_modules` and nowhere else, and a temporary directory has none
    // above it. So this runs everywhere, rather than skipping on any
    // machine that happens to have a compiler installed somewhere.
    let (ok, err) = run_ttc_env(&dir, &["--types", "src"]);
    assert!(!ok, "expected failure:\n{err}");
    assert!(err.contains("no TypeScript compiler found"), "{err}");
}

/// A consumer that type-checks the untouched source tree with plain tsc:
/// the sidecars merge in through `rootDirs`, and `@tt/std` maps to the
/// standard-library declarations beside them.
const SIDECAR_CONSUMER_TSCONFIG: &str = r#"{
  "compilerOptions": {
    "target": "es2022",
    "module": "preserve",
    "moduleResolution": "bundler",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true,
    "rootDirs": ["./src", "./.tt-types"],
    "paths": {
      "@tt/std": ["./.tt-types/tt/index.d.ts"],
      "@tt/std/*": ["./.tt-types/tt/*.d.ts"]
    }
  },
  "include": ["src"]
}
"#;

#[test]
fn cli_types_sidecars_typecheck_the_source_tree() {
    require_toolchain!();
    require_types_typescript!();
    let dir = project_dir();
    write_consumer_tree(&dir);

    let (ok, err) = run_ttc(&dir, &["--types", "src"]);
    assert!(ok, "--types failed:\n{err}");

    // The declarations keep the *source* specifiers — that is what resolves
    // in the consumer's merged view.
    let sidecar = fs::read_to_string(dir.join(".tt-types/notice.tt.d.ts")).unwrap();
    assert!(sidecar.contains("from \"@tt/std\""), "{sidecar}");
    assert!(sidecar.contains("from \"./level.tt\""), "{sidecar}");
    assert!(
        sidecar.contains("export declare function render"),
        "{sidecar}"
    );
    assert!(dir.join(".tt-types/notice.tt.d.ts.map").exists());
    assert!(dir.join(".tt-types/level.tt.d.ts").exists());
    for module in ttc::StdModule::STANDARD {
        assert!(
            dir.join(".tt-types/tt")
                .join(module.file_name())
                .with_extension("d.ts")
                .exists(),
            "std declaration missing: {:?}",
            module
        );
    }

    // Round trip: the untouched source tree typechecks once the sidecars
    // are merged in (`rootDirs`) and `@tt/std` is mapped (`paths`).
    fs::write(dir.join("tsconfig.json"), SIDECAR_CONSUMER_TSCONFIG).unwrap();
    let out = common::tsc()
        .current_dir(&dir)
        .args(["-p", "tsconfig.json"])
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "consumer typecheck failed:\n{}\n---sidecar---\n{sidecar}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn cli_types_under_a_configuration_writes_the_std_declarations_it_maps() {
    require_toolchain!();
    require_types_typescript!();
    let dir = project_dir();
    write_consumer_tree(&dir);
    fs::write(dir.join("tsconfig.json"), SIDECAR_CONSUMER_TSCONFIG).unwrap();

    let (ok, err) = run_ttc(&dir, &["--types", "src"]);
    assert!(ok, "--types failed:\n{err}");
    for module in ttc::StdModule::STANDARD {
        let path = dir
            .join(".tt-types/tt")
            .join(module.file_name())
            .with_extension("d.ts");
        assert_eq!(
            fs::read_to_string(&path).ok().as_deref(),
            Some(module.declaration()),
            "{}:\n{err}",
            path.display()
        );
    }

    let out = common::tsc()
        .current_dir(&dir)
        .args(["-p", "tsconfig.json"])
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "consumer typecheck failed:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/* ------------------------------------------------------------------ */
/* pipeline                                                            */
/* ------------------------------------------------------------------ */

#[test]
fn pipeline_files_import_one_shared_runtime() {
    require_toolchain!();
    let dir = tmpdir();
    let mut files = Vec::new();
    for suffix in ["a", "b"] {
        let source = format!(
            "declare function input_{suffix}(): number;\n\
             declare const step_{suffix}: (value: number) => number;\n\
             const value_{suffix} = input_{suffix}() |> step_{suffix};\n\
             const flow_{suffix} = flow |> step_{suffix} |> step_{suffix};\n\
             export {{}};\n"
        );
        let code =
            compile(&source, &options_with_runtime("./runtime.js")).expect("tt compile failed");
        assert!(!code.lines().any(|line| line.starts_with("function $tt_")));
        assert!(!code.lines().any(|line| line.starts_with("var $tt_")));
        assert!(code.contains("from \"./runtime.js\""));
        let file = dir.join(format!("{suffix}.ts"));
        fs::write(&file, code).unwrap();
        files.push(file);
    }
    write_runtime(&dir);
    files.push(dir.join("runtime.ts"));

    let out = common::tsc()
        .args(&files)
        .arg("--noEmit")
        .args(TSC_FLAGS)
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn scripts_stay_scripts_and_their_generated_globals_never_collide() {
    require_toolchain!();
    let dir = tmpdir();
    let mut files = Vec::new();
    for suffix in ["a", "b"] {
        let source = format!(
            "/// <reference lib=\"es2022\" />\n\
             // @ts-expect-error\n\
             const unchecked_{suffix}: string = 1;\n\
             type Shape_{suffix} = {{ kind: \"A\"; n: number }} | {{ kind: \"B\" }};\n\
             function make_{suffix}(n: number): Shape_{suffix} {{ return n > 0 ? {{ kind: \"A\", n }} : {{ kind: \"B\" }}; }}\n\
             function step_{suffix}(value: number): number {{ return value + 1; }}\n\
             function pick_{suffix}(): (value: number) => number {{ return step_{suffix}; }}\n\
             const size_{suffix} = match (make_{suffix}(2)) {{ A(n) => n, B => 0 }};\n\
             var total_{suffix} = match (make_{suffix}(0)) {{ A(n) => n, B => 10 }};\n\
             function measure_{suffix}(shape: Shape_{suffix}): number {{ return match (shape) {{ A(n) => n * 2, B => -1 }}; }}\n\
             console.log(match (make_{suffix}(1)) {{ A(n) => `{suffix}:${{n}}`, B => \"{suffix}:none\" }});\n\
             const A(n: first_{suffix}) = make_{suffix}(5) else {{ throw new Error(\"{suffix}\"); }};\n\
             const piped_{suffix} = size_{suffix} |> pick_{suffix}();\n\
             const flowed_{suffix} = flow |> step_{suffix} |> step_{suffix};\n\
             const kept_{suffix} = match (make_{suffix}(4)) {{ A(n) => {{ var seen_{suffix} = n; return n; }}, B => 0 }};\n"
        );
        let code = compile(&source, &Options::default()).expect("tt compile failed");
        assert!(!code.contains("import "), "{code}");
        assert!(code.contains("var $tt_ap: "), "{code}");
        assert!(code.contains("var $tt_fl: "), "{code}");
        assert!(code.contains("var $tt_show: "), "{code}");
        assert!(
            code.contains("// @ts-expect-error\nconst unchecked_"),
            "{code}"
        );
        assert!(
            code.contains(&format!("const size_{suffix} = $tt_v0$size_{suffix};")),
            "{code}"
        );
        let file = dir.join(format!("{suffix}.ts"));
        fs::write(&file, code).unwrap();
        files.push(file);
    }
    let consumer = dir.join("c.ts");
    fs::write(
        &consumer,
        "console.log(size_a + size_b, total_a + total_b, measure_a(make_a(3)), first_a + first_b, piped_a, flowed_b(1));\n\
         console.log(kept_a + kept_b, seen_a + seen_b);\n",
    )
    .unwrap();
    files.push(consumer);

    let out = common::tsc()
        .args(&files)
        .arg("--outDir")
        .arg(dir.join("out"))
        .args(TSC_FLAGS)
        .output()
        .expect("failed to run tsc");
    assert!(out.status.success(), "{}", tsc_report(&out));
    let out = Command::new("node")
        .arg("-e")
        .arg(
            "const vm = require('vm'), fs = require('fs');\n\
             for (const file of process.argv.slice(1)) vm.runInThisContext(fs.readFileSync(file, 'utf8'), { filename: file });",
        )
        .args(["a.js", "b.js", "c.js"].map(|name| dir.join("out").join(name)))
        .output()
        .expect("failed to run node");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "a:1\nb:1\n4 20 6 10 3 3\n8 8\n"
    );
}
