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
