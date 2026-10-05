//! CLI output contracts: ownership identity, overlays, watch deletions,
//! source map URLs, and sidecar declaration layout.
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

fn success(output: std::process::Output) -> std::process::Output {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn support_module_ownership_does_not_depend_on_the_working_directory() {
    let root = Workspace::new("std-owner-cwd");
    fs::create_dir_all(root.join("p/src")).unwrap();
    fs::write(
        root.join("p/src/a.tt"),
        "import * as Option from \"@tt/std/option\";\nexport const n = Option.unwrapOr(Option.fromNullable(null as number | null), 1);\n",
    )
    .unwrap();
    success(run(&root.join("p"), &["-o", "out", "src"]));
    assert!(root.join("p/out/tt/option.ts").exists());
    success(run(&root, &["-o", "p/out", "p/src"]));
    success(run(&root.join("p/src"), &["-o", "../out", "."]));
}

#[test]
fn support_records_written_before_owner_identities_remain_owned_from_their_directory() {
    let root = Workspace::new("std-owner-legacy");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/a.tt"),
        "import * as Option from \"@tt/std/option\";\nexport const n = Option.unwrapOr(Option.fromNullable(null as number | null), 1);\n",
    )
    .unwrap();
    let cwd = fs::canonicalize(root.path()).unwrap();
    success(run(&cwd, &["-o", "out", "src"]));
    for entry in fs::read_dir(cwd.join("out/tt")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let Some(file) = name
            .strip_prefix('.')
            .and_then(|name| name.strip_suffix(".ttc-output.json"))
        else {
            continue;
        };
        let mut record: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let object = record.as_object_mut().unwrap();
        assert!(object.remove("support").is_some());
        object.insert(
            "source".into(),
            cwd.join("@tt/std").join(file).to_string_lossy().into(),
        );
        fs::write(&path, record.to_string()).unwrap();
    }
    success(run(&cwd, &["-o", "out", "src"]));
}

#[cfg(unix)]
#[test]
fn a_symlinked_source_path_owns_the_outputs_of_its_target() {
    let root = Workspace::new("symlink-owner");
    fs::create_dir(root.join("real")).unwrap();
    fs::write(root.join("real/a.tt"), "export const value = 1;\n").unwrap();
    std::os::unix::fs::symlink("real", root.join("link")).unwrap();
    success(run(&root, &["real"]));
    success(run(&root, &["link"]));
    success(run(&root, &["link/a.tt"]));
    fs::write(root.join("real/a.ts"), "// edited\n").unwrap();
    assert!(!run(&root, &["link"]).status.success());
    assert_eq!(
        fs::read_to_string(root.join("real/a.ts")).unwrap(),
        "// edited\n"
    );
}

fn run_with_stdin(root: &Path, args: &[&str], stdin: &str) -> std::process::Output {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .current_dir(root)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn an_overlay_checks_a_file_that_was_never_saved() {
    if !common::toolchain() {
        return;
    }
    let root = Workspace::in_repo("overlay-unsaved");
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/a.tt"), "export const n = 1;\n").unwrap();
    let buffer = "val const edited = new Map<string, number>();\nedited.delete(\"new\");\n";
    let output = run_with_stdin(
        &root,
        &[
            "--check-types",
            "--tt-only",
            "--overlay",
            "src/new.tt",
            "src",
        ],
        buffer,
    );
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{err}");
    assert!(
        err.contains("cannot call mutating method `delete` through val binding `edited`"),
        "{err}"
    );
    assert!(err.contains("new.tt"), "{err}");
    assert!(!root.join("src/new.tt").exists());
    success(run_with_stdin(
        &root,
        &[
            "--check-types",
            "--tt-only",
            "--overlay",
            "src/new.tt",
            "src",
        ],
        "export const m = 2;\n",
    ));
    let missing = run_with_stdin(
        &root,
        &["--check-types", "--overlay", "gone/new.tt", "src"],
        "export const m = 2;\n",
    );
    let err = String::from_utf8_lossy(&missing.stderr);
    assert!(!missing.status.success(), "{err}");
    assert!(err.contains("--overlay") && err.contains("gone"), "{err}");
}

#[test]
fn watch_rebuilds_importers_of_a_deleted_or_renamed_file() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    use std::sync::mpsc;
    use std::time::Duration;
    let root = Workspace::new("watch-deletion");
    fs::create_dir(root.join("src")).unwrap();
    let variant = "export variant S { A, B, C }\n";
    fs::write(root.join("src/s.tt"), variant).unwrap();
    fs::write(
        root.join("src/u.tt"),
        "import { S } from \"./s.tt\";\nexport const f = (s: S) => match (s) { A => 1, B => 2 };\n",
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_ttc"));
    command
        .current_dir(&root)
        .args(["--watch", "-o", "out", "src"])
        .stderr(Stdio::piped())
        .stdout(Stdio::null());
    root.isolate_unfinalized_child_profile(&mut command);
    let mut child = command.spawn().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (send, receive) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let _ = send.send(line.unwrap());
        }
    });
    let round = || loop {
        let line = receive.recv_timeout(Duration::from_secs(10)).unwrap();
        if line.contains("file(s)") {
            return line;
        }
    };
    let result = std::panic::catch_unwind(|| {
        assert!(round().contains("2 file(s) rebuilt, with errors"));
        assert!(round().contains("Ctrl-C"));
        fs::rename(root.join("src/s.tt"), root.join("src/t.tt")).unwrap();
        assert_eq!(round(), "ttc: 2 file(s) ok — watching");
        fs::write(root.join("src/s.tt"), variant).unwrap();
        assert_eq!(round(), "ttc: 2 file(s) rebuilt, with errors — watching");
        fs::remove_file(root.join("src/s.tt")).unwrap();
        assert_eq!(round(), "ttc: 1 file(s) ok — watching");
    });
    let _ = child.kill();
    let _ = child.wait();
    reader.join().unwrap();
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}

#[test]
fn watch_rebuilds_an_input_when_a_tt_file_it_imports_changes() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    use std::sync::mpsc;
    use std::time::Duration;
    let root = Workspace::new("watch-imported-non-input");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("shared")).unwrap();
    fs::write(root.join("shared/s.tt"), "export variant S { A, B }\n").unwrap();
    fs::write(
        root.join("src/u.tt"),
        "import { S } from \"../shared/s.tt\";\nexport const f = (s: S) => match (s) { A => 1, B => 2 };\n",
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_ttc"));
    command
        .current_dir(&root)
        .args(["--watch", "-o", "out", "src"])
        .stderr(Stdio::piped())
        .stdout(Stdio::null());
    root.isolate_unfinalized_child_profile(&mut command);
    let mut child = command.spawn().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (send, receive) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let _ = send.send(line.unwrap());
        }
    });
    let round = || loop {
        let line = receive.recv_timeout(Duration::from_secs(10)).unwrap();
        if line.contains("file(s)") {
            return line;
        }
    };
    let result = std::panic::catch_unwind(|| {
        assert_eq!(round(), "ttc: 1 file(s) ok — watching");
        assert!(round().contains("Ctrl-C"));
        fs::write(root.join("shared/s.tt"), "export variant S { A, B, C }\n").unwrap();
        assert_eq!(round(), "ttc: 1 file(s) rebuilt, with errors — watching");
        fs::write(root.join("shared/s.tt"), "export variant S { A, B }\n").unwrap();
        assert_eq!(round(), "ttc: 1 file(s) ok — watching");
    });
    let _ = child.kill();
    let _ = child.wait();
    reader.join().unwrap();
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}

fn have_node() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

#[test]
fn source_map_urls_percent_encode_file_names() {
    let root = Workspace::new("source-map-urls");
    fs::create_dir(root.join("my src")).unwrap();
    fs::write(
        root.join("my src/a b#1%.tt"),
        "const f = (n: number) => {\n  throw new Error(\"boom \" + n);\n};\nconst v = 1 |> f;\n",
    )
    .unwrap();
    success(run(
        &root,
        &[
            "--no-banner",
            "--source-map",
            "file",
            "-o",
            "out dir",
            "my src",
        ],
    ));
    let code = fs::read_to_string(root.join("out dir/a b#1%.ts")).unwrap();
    assert!(
        code.ends_with("\n//# sourceMappingURL=a%20b%231%25.ts.map\n"),
        "{code}"
    );
    let map: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("out dir/a b#1%.ts.map")).unwrap())
            .unwrap();
    assert_eq!(
        map["sources"],
        serde_json::json!(["../my%20src/a%20b%231%25.tt"])
    );
    assert_eq!(map["file"], "a b#1%.ts");
    if !have_node() {
        return;
    }
    let run = Command::new("node")
        .current_dir(&root)
        .args(["--enable-source-maps", "--experimental-strip-types"])
        .arg(root.join("out dir/a b#1%.ts"))
        .output()
        .unwrap();
    let trace = String::from_utf8_lossy(&run.stderr);
    assert!(trace.contains("boom 1"), "{trace}");
    assert!(trace.contains("a b#1%.tt:2:"), "{trace}");
}

#[test]
fn source_map_comments_use_the_line_ending_of_the_output() {
    let root = Workspace::new("source-map-crlf");
    fs::write(
        root.join("crlf.tt"),
        "const a = 1 |> String;\r\nexport { a };",
    )
    .unwrap();
    fs::write(
        root.join("bom.tt"),
        "\u{feff}const b = 1 |> String;\r\nexport { b };\r\n",
    )
    .unwrap();
    fs::write(root.join("lf.tt"), "const c = 1 |> String;\nexport { c };").unwrap();
    success(run(&root, &["--source-map", "file", "-o", "out", "."]));
    success(run(&root, &["--source-map", "inline", "-o", "inline", "."]));
    for dir in ["out", "inline"] {
        for (name, ending) in [("crlf", "\r\n"), ("bom", "\r\n"), ("lf", "\n")] {
            let code = fs::read_to_string(root.join(dir).join(format!("{name}.ts"))).unwrap();
            let bare = code.replace("\r\n", "");
            assert!(!bare.contains('\r'), "{dir}/{name}: {code:?}");
            if ending == "\r\n" {
                assert!(!bare.contains('\n'), "{dir}/{name}: {code:?}");
            } else {
                assert!(!code.contains('\r'), "{dir}/{name}: {code:?}");
            }
            let (_, last) = code
                .strip_suffix(ending)
                .unwrap()
                .rsplit_once(ending)
                .unwrap();
            assert!(
                last.starts_with("//# sourceMappingURL="),
                "{dir}/{name}: {code:?}"
            );
        }
    }
}

fn write_all(root: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
}

#[test]
fn sidecars_read_declarations_from_the_layout_tsc_emits() {
    let root = Workspace::new("sidecar-layout");
    write_all(
        &root,
        &[
            ("src/a/x.tt", "export const a = 1;\n"),
            ("src/b/x.tt", "export const b = \"s\";\n"),
            ("decl/a/x.d.ts", "export declare const a = 1;\n"),
            ("decl/b/x.d.ts", "export declare const b = \"s\";\n"),
            ("decl/x.d.ts", "export declare const decoy: 0;\n"),
            ("flat/only.tt", "export const only = 1;\n"),
            ("flat-decl/only.d.ts", "export declare const only = 1;\n"),
            ("mixed/index.ts", "export {};\n"),
            ("mixed/lib/y.tt", "export const y = 2;\n"),
            ("mixed-decl/index.d.ts", "export {};\n"),
            ("mixed-decl/lib/y.d.ts", "export declare const y = 2;\n"),
        ],
    );
    success(run(&root, &["--sidecar", "decl", "src"]));
    let a = fs::read_to_string(root.join("src/a/x.tt.d.ts")).unwrap();
    let b = fs::read_to_string(root.join("src/b/x.tt.d.ts")).unwrap();
    assert!(a.contains("const a = 1") && !a.contains("decoy"), "{a}");
    assert!(b.contains("const b = \"s\"") && !b.contains("decoy"), "{b}");
    success(run(&root, &["--sidecar", "decl", "src"]));
    success(run(&root, &["--sidecar", "flat-decl", "flat"]));
    assert!(
        fs::read_to_string(root.join("flat/only.tt.d.ts"))
            .unwrap()
            .contains("const only = 1")
    );
    success(run(&root, &["--sidecar", "mixed-decl", "mixed"]));
    assert!(
        fs::read_to_string(root.join("mixed/lib/y.tt.d.ts"))
            .unwrap()
            .contains("const y = 2")
    );
}

/// tsc's declarations for ttc's output name a tt module by the file it
/// compiles to; a sidecar is read where the source is, so it names that
/// module as the source does — the spelling `--types` writes. A specifier
/// that names hand-written TypeScript, or a package, stays as tsc wrote it.
#[test]
fn sidecars_name_tt_modules_as_the_source_does() {
    let root = Workspace::new("sidecar-specifiers");
    let declarations = "import { K } from \"./sub/m.js\";\n\
        import type { V } from './sub/v.jsx';\n\
        import type { W } from './sub/w.js';\n\
        import { h } from \"./h.js\";\n\
        export * from \"./sub/m.ts\";\n\
        export { K } from \"pkg/m.js\";\n\
        export declare const k: K;\n\
        export type T = import(\"./sub/m.js\").K | V | W | typeof h;\n";
    write_all(
        &root,
        &[
            ("src/sub/m.tt", "export variant K { A, B }\n"),
            ("src/sub/v.ttx", "export type V = number;\n"),
            ("src/sub/w.ttx", "export type W = string;\n"),
            ("src/h.ts", "export const h = 1;\n"),
            (
                "src/u.tt",
                "import { K } from \"./sub/m.tt\";\nexport const k: K = K.A;\n",
            ),
            ("decl/u.d.ts", declarations),
            ("decl/sub/m.d.ts", "export type K = { kind: \"A\" };\n"),
            ("decl/sub/v.d.ts", "export type V = number;\n"),
            ("decl/sub/w.d.ts", "export type W = string;\n"),
        ],
    );
    let expected = "import { K } from \"./sub/m.tt\";\n\
        import type { V } from './sub/v.ttx';\n\
        import type { W } from './sub/w.ttx';\n\
        import { h } from \"./h.js\";\n\
        export * from \"./sub/m.tt\";\n\
        export { K } from \"pkg/m.js\";\n\
        export declare const k: K;\n\
        export type T = import(\"./sub/m.tt\").K | V | W | typeof h;\n";
    success(run(&root, &["--sidecar", "decl", "src"]));
    success(run(&root, &["--sidecar", "decl", "-o", "types", "src"]));
    for sidecar in ["src/u.tt.d.ts", "types/u.tt.d.ts"] {
        let written = fs::read_to_string(root.join(sidecar)).unwrap();
        assert!(written.contains(expected), "{sidecar}:\n{written}");
    }
}

#[test]
fn tt_only_modes_name_the_file_and_the_extensions_they_accept() {
    let root = Workspace::new("tt-only-inputs");
    fs::write(root.join("x.ts"), "export const x = 1;\n").unwrap();
    fs::write(root.join("app.js"), "export const y = 1;\n").unwrap();
    for mode in [
        &["--symbols"][..],
        &["--emit-map"],
        &["--check-types"],
        &["--sidecar", "decl"],
        &["--dependencies"],
    ] {
        let output = run(&root, &[mode, &["x.ts"]].concat());
        let err = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{mode:?}: {err}");
        assert_eq!(
            err.trim_end(),
            "ttc: x.ts: not a tt source (expected .tt, .ttx)",
            "{mode:?}"
        );
    }
    let output = run(&root, &["app.js"]);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim_end(),
        "ttc: app.js: not a tt or TypeScript source (expected .tt, .ttx, .ts, .tsx, .mts, .cts)"
    );
}

#[test]
fn sidecar_map_urls_percent_encode_file_names() {
    let root = Workspace::new("sidecar-urls");
    write_all(
        &root,
        &[
            ("my src/a b#1%.tt", "export const a = 1;\n"),
            ("decl/a b#1%.d.ts", "export declare const a = 1;\n"),
        ],
    );
    success(run(
        &root,
        &["--sidecar", "decl", "-o", "out dir", "my src"],
    ));
    let declarations = fs::read_to_string(root.join("out dir/a b#1%.tt.d.ts")).unwrap();
    assert!(
        declarations.ends_with("\n//# sourceMappingURL=a%20b%231%25.tt.d.ts.map\n"),
        "{declarations}"
    );
    let map: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("out dir/a b#1%.tt.d.ts.map")).unwrap())
            .unwrap();
    assert_eq!(
        map["sources"],
        serde_json::json!(["../my%20src/a%20b%231%25.tt"])
    );
    assert_eq!(map["file"], "a b#1%.tt.d.ts");
}

#[test]
fn watch_rebuilds_every_output_when_the_jsx_option_changes() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    use std::sync::mpsc;
    use std::time::Duration;
    let root = Workspace::new("watch-jsx-option");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("tsconfig.json"),
        "{\"compilerOptions\":{\"jsx\":\"react-jsx\"}}",
    )
    .unwrap();
    fs::write(root.join("src/c.ttx"), "export const C = () => <div/>;\n").unwrap();
    fs::write(
        root.join("src/m.tt"),
        "import { C } from \"./c.ttx\";\nexport const m = C;\n",
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_ttc"));
    command
        .current_dir(&root)
        .args(["--watch", "-o", "out", "src"])
        .stderr(Stdio::piped())
        .stdout(Stdio::null());
    root.isolate_unfinalized_child_profile(&mut command);
    let mut child = command.spawn().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (send, receive) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let _ = send.send(line.unwrap());
        }
    });
    let round = || loop {
        let line = receive.recv_timeout(Duration::from_secs(10)).unwrap();
        if line.contains("file(s)") {
            return line;
        }
    };
    let result = std::panic::catch_unwind(|| {
        assert_eq!(round(), "ttc: 2 file(s) ok — watching");
        assert!(round().contains("Ctrl-C"));
        assert!(
            fs::read_to_string(root.join("out/m.ts"))
                .unwrap()
                .contains("from \"./c.js\"")
        );
        fs::write(
            root.join("tsconfig.json"),
            "{\"compilerOptions\":{\"jsx\":\"preserve\"}}",
        )
        .unwrap();
        assert_eq!(round(), "ttc: 2 file(s) ok — watching");
        assert!(
            fs::read_to_string(root.join("out/m.ts"))
                .unwrap()
                .contains("from \"./c.jsx\"")
        );
    });
    let _ = child.kill();
    let _ = child.wait();
    reader.join().unwrap();
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}

#[test]
fn print_watch_never_prints_a_second_module() {
    use std::io::{BufRead, BufReader, Read};
    use std::process::Stdio;
    use std::sync::mpsc;
    use std::time::Duration;
    let root = Workspace::new("print-watch-one-source");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/a.tt"), "export const a = 1;\n").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_ttc"));
    command
        .current_dir(&root)
        .args(["--watch", "-p", "--no-banner", "src"])
        .stderr(Stdio::piped())
        .stdout(Stdio::piped());
    root.isolate_unfinalized_child_profile(&mut command);
    let mut child = command.spawn().unwrap();
    let stderr = child.stderr.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let (send, receive) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let _ = send.send(line.unwrap());
        }
    });
    let next = || receive.recv_timeout(Duration::from_secs(10)).unwrap();
    let result = std::panic::catch_unwind(|| {
        assert_eq!(next(), "ttc: 1 file(s) ok — watching");
        assert!(next().contains("Ctrl-C"));
        fs::write(root.join("src/b.tt"), "export const b = 1;\n").unwrap();
        assert_eq!(next(), "ttc: --print requires exactly one source file");
        fs::write(root.join("src/a.tt"), "export const a = 2;\n").unwrap();
        fs::remove_file(root.join("src/b.tt")).unwrap();
        assert_eq!(next(), "ttc: 1 file(s) ok — watching");
    });
    let _ = child.kill();
    let _ = child.wait();
    reader.join().unwrap();
    let mut printed = String::new();
    stdout.read_to_string(&mut printed).unwrap();
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
    assert!(!printed.contains("export const b"), "{printed}");
    assert!(printed.contains("export const a = 2;"), "{printed}");
}
