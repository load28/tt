//! Standard-library package contracts.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use ttc::{
    Options, RUNTIME_SOURCE, STD_OPTION_SOURCE, STD_RESULT_SOURCE, STD_TYPES_SOURCE, compile,
};

#[test]
fn std_modules_are_plain_typescript_and_pass_through() {
    for source in [
        STD_TYPES_SOURCE,
        STD_OPTION_SOURCE,
        STD_RESULT_SOURCE,
        RUNTIME_SOURCE,
    ] {
        let out = compile(source, &Options::default()).expect("std module failed to compile");
        assert_eq!(out, source);
    }
}

#[test]
fn pipeline_runtime_exports_both_shared_helpers() {
    assert!(RUNTIME_SOURCE.contains("export function $tt_ap"));
    assert!(RUNTIME_SOURCE.contains("export function $tt_fl"));
}

#[test]
fn std_runtime_members_are_independent_esm_exports() {
    for (source, members) in [
        (
            STD_OPTION_SOURCE,
            &["Some", "None", "map", "andThen", "mapP", "collect"][..],
        ),
        (
            STD_RESULT_SOURCE,
            &["Ok", "Err", "map", "andThen", "mapP", "collect"][..],
        ),
    ] {
        for member in members {
            assert!(
                source.contains(&format!("export const {member}")),
                "missing independent export {member}"
            );
        }
        assert!(!source.contains("export const Option ="));
        assert!(!source.contains("export const Result ="));
    }
}

#[test]
fn std_value_shapes_match_builtin_variants() {
    for shape in [
        r#"{ kind: "Some"; value: T }"#,
        r#"{ kind: "None" }"#,
        r#"({ kind: "Some", value })"#,
        r#"{ kind: "Ok"; value: T }"#,
        r#"{ kind: "Err"; error: E }"#,
        r#"({ kind: "Ok", value })"#,
        r#"({ kind: "Err", error })"#,
    ] {
        assert!(
            STD_OPTION_SOURCE.contains(shape) || STD_RESULT_SOURCE.contains(shape),
            "standard-library value shape drifted: {shape}"
        );
    }
}

#[test]
fn std_type_entry_point_is_type_only() {
    assert!(STD_TYPES_SOURCE.contains("export type { TOption }"));
    assert!(STD_TYPES_SOURCE.contains("export type { TErr, TErrorOf, TOk, TResult }"));
    assert!(!STD_TYPES_SOURCE.contains("export const"));
}

#[test]
fn namespace_import_is_pruned_by_a_real_bundler() {
    let Some(rolldown) = rolldown_command() else {
        eprintln!("skipping bundler pruning test: rolldown is not installed");
        return;
    };
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock predates Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("tt-stdlib-tree-shaking-{nonce}"));
    fs::create_dir_all(&root).expect("failed to create bundle fixture directory");
    fs::write(root.join("option.ts"), STD_OPTION_SOURCE).expect("failed to write option module");
    fs::write(root.join("result.ts"), STD_RESULT_SOURCE).expect("failed to write result module");
    fs::write(
        root.join("entry.ts"),
        "import * as Option from './option.ts';\nconsole.log(Option.Some(1));\n",
    )
    .expect("failed to write bundle entry");

    let output_path = root.join("bundle.js");
    let output = Command::new(rolldown)
        .arg(root.join("entry.ts"))
        .args(["--file", output_path.to_str().expect("non-UTF-8 temp path")])
        .arg("--minify")
        .output()
        .expect("failed to run rolldown");
    assert!(
        output.status.success(),
        "rolldown failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bundle = fs::read_to_string(&output_path).expect("failed to read bundle");
    fs::remove_dir_all(&root).expect("failed to remove bundle fixture directory");

    assert!(
        bundle.contains("Some"),
        "used constructor was removed: {bundle}"
    );
    assert!(
        !bundle.contains("None"),
        "unused Option exports remained in the bundle: {bundle}"
    );
}

fn rolldown_command() -> Option<PathBuf> {
    let local = Path::new("website/node_modules/.bin/rolldown");
    if local.is_file() {
        return Some(local.to_path_buf());
    }
    Command::new("rolldown")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| PathBuf::from("rolldown"))
}

fn scratch(tag: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("tt-stdlib-{tag}-{}-{nonce}", std::process::id()))
}

#[test]
fn materialized_packages_are_dual_format_with_an_exports_map() {
    let root = scratch("package");
    for package in ttc::StdPackage::ALL {
        package.materialize(&root).unwrap();
        let directory = package.directory(&root);
        let read = |path: &str| fs::read_to_string(directory.join(path)).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&read("package.json")).unwrap();
        assert_eq!(manifest["name"], package.name());
        assert_eq!(manifest["type"], "module");
        let commonjs: serde_json::Value = serde_json::from_str(&read(&format!(
            "{}/package.json",
            ttc::STD_PACKAGE_COMMONJS_DIR
        )))
        .unwrap();
        assert_eq!(commonjs["type"], "commonjs");
        for module in package.modules() {
            let subpath = format!(".{}", &module.specifier()[package.name().len()..]);
            let entry = &manifest["exports"][subpath.as_str()];
            let file = ttc::StdPackage::file_name(*module);
            let declaration = format!(
                "{}/{}",
                ttc::STD_PACKAGE_COMMONJS_DIR,
                ttc::StdPackage::commonjs_file_name(*module)
            );
            assert!(declaration.ends_with(".d.ts"), "{declaration}");
            let text = read("package.json");
            let import =
                format!("\"import\": {{ \"types\": \"./{file}\", \"default\": \"./{file}\" }}");
            let require = format!(
                "\"require\": {{ \"types\": \"./{declaration}\", \"default\": \"./{file}\" }}"
            );
            assert!(
                text.find(&import).unwrap() < text.find(&require).unwrap(),
                "{text}"
            );
            assert_eq!(entry["import"]["types"], format!("./{file}"), "{subpath}");
            assert_eq!(entry["import"]["default"], format!("./{file}"), "{subpath}");
            assert_eq!(
                entry["require"]["types"],
                format!("./{declaration}"),
                "{subpath}"
            );
            assert_eq!(
                entry["require"]["default"],
                format!("./{file}"),
                "{subpath}"
            );
            assert_eq!(
                read(file),
                format!("{}{}", ttc::GENERATED_BANNER, module.source()),
                "{file}"
            );
            assert_eq!(
                read(&declaration),
                format!("{}{}", ttc::GENERATED_BANNER, module.declaration()),
                "{declaration}"
            );
            assert!(
                !directory
                    .join(ttc::STD_PACKAGE_COMMONJS_DIR)
                    .join(file)
                    .exists()
            );
        }
    }
    fs::remove_dir_all(root).unwrap();
}

fn write_source_copy_layout(root: &Path, package: ttc::StdPackage, edited: Option<&str>) {
    let directory = package.directory(root);
    let cjs = directory.join(ttc::STD_PACKAGE_COMMONJS_DIR);
    fs::create_dir_all(&cjs).unwrap();
    let mut entries = Vec::new();
    for module in package.modules() {
        let file = ttc::StdPackage::file_name(*module);
        let generated = format!("{}{}", ttc::GENERATED_BANNER, module.source());
        fs::write(directory.join(file), &generated).unwrap();
        fs::write(cjs.join(file), edited.unwrap_or(&generated)).unwrap();
        let subpath = &module.specifier()[package.name().len()..];
        entries.push(format!(
            "    \".{subpath}\": {{\n      \"import\": {{ \"types\": \"./{file}\", \"default\": \"./{file}\" }},\n      \"require\": {{ \"types\": \"./cjs/{file}\", \"default\": \"./cjs/{file}\" }}\n    }}"
        ));
    }
    fs::write(cjs.join("package.json"), "{\n  \"type\": \"commonjs\"\n}\n").unwrap();
    fs::write(
        directory.join("package.json"),
        format!(
            "{{\n  \"name\": \"{}\",\n  \"version\": \"0.0.0\",\n  \"type\": \"module\",\n  \"types\": \"./index.ts\",\n  \"exports\": {{\n{}\n  }}\n}}\n",
            package.name(),
            entries.join(",\n")
        ),
    )
    .unwrap();
}

#[test]
fn a_package_with_commonjs_source_copies_gets_declarations_in_their_place() {
    let root = scratch("copies");
    for package in ttc::StdPackage::ALL {
        write_source_copy_layout(&root, package, None);
        package.materialize(&root).unwrap();
        let directory = package.directory(&root);
        let mut found: Vec<_> = fs::read_dir(directory.join(ttc::STD_PACKAGE_COMMONJS_DIR))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        found.sort();
        let mut expected: Vec<_> = package
            .modules()
            .iter()
            .map(|module| ttc::StdPackage::commonjs_file_name(*module))
            .chain(["package.json".to_string()])
            .collect();
        expected.sort();
        assert_eq!(found, expected, "{}", package.name());
        let mut files = package.files();
        files.sort();
        for (name, text) in files {
            assert_eq!(
                fs::read_to_string(directory.join(&name)).unwrap(),
                text,
                "{name}"
            );
        }
    }
    fs::remove_dir_all(&root).unwrap();

    write_source_copy_layout(&root, ttc::StdPackage::Std, Some("// mine\n"));
    let before =
        fs::read_to_string(ttc::StdPackage::Std.directory(&root).join("package.json")).unwrap();
    ttc::StdPackage::Std.materialize(&root).unwrap();
    let directory = ttc::StdPackage::Std.directory(&root);
    assert_eq!(
        fs::read_to_string(directory.join("package.json")).unwrap(),
        before
    );
    assert_eq!(
        fs::read_to_string(directory.join("cjs/option.ts")).unwrap(),
        "// mine\n"
    );
    assert!(!directory.join("cjs/option.d.ts").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_package_ttc_wrote_before_exports_is_upgraded_and_any_other_is_kept() {
    let root = scratch("upgrade");
    let std_dir = ttc::StdPackage::Std.directory(&root);
    fs::create_dir_all(&std_dir).unwrap();
    fs::write(
        std_dir.join("package.json"),
        "{\n  \"name\": \"@tt/std\",\n  \"version\": \"0.0.0\",\n  \"types\": \"index.ts\"\n}\n",
    )
    .unwrap();
    fs::write(std_dir.join("index.ts"), "// mine\n").unwrap();
    ttc::StdPackage::Std.materialize(&root).unwrap();
    assert_eq!(
        fs::read_to_string(std_dir.join("package.json")).unwrap(),
        ttc::StdPackage::Std.manifest()
    );
    for module in ttc::StdPackage::Std.modules() {
        assert!(
            std_dir
                .join(ttc::STD_PACKAGE_COMMONJS_DIR)
                .join(ttc::StdPackage::commonjs_file_name(*module))
                .is_file()
        );
    }
    assert_eq!(
        fs::read_to_string(std_dir.join("index.ts")).unwrap(),
        "// mine\n"
    );

    let runtime_dir = ttc::StdPackage::Runtime.directory(&root);
    fs::create_dir_all(&runtime_dir).unwrap();
    let authored = "{ \"name\": \"@tt/runtime\", \"version\": \"9.9.9\" }\n";
    fs::write(runtime_dir.join("package.json"), authored).unwrap();
    ttc::StdPackage::Runtime.materialize(&root).unwrap();
    assert_eq!(
        fs::read_to_string(runtime_dir.join("package.json")).unwrap(),
        authored
    );
    fs::remove_dir_all(root).unwrap();
}
