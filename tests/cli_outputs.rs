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
