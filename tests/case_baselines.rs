//! The case runner: every file under `tests/cases/compiler/` and
//! `tests/cases/conformance/` is a compilation whose artifacts are held to
//! baselines under `tests/baselines/reference/`. Editor cases, under
//! `tests/cases/editor/`, have their own runner (`tests/editor_cases.rs`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use ttc::SourceKind;
use ttc::engine::{Engine, ProjectOptions};

mod common;
use common::baseline::{expect, expect_absent, updating};
use common::cases::{self, Unit, is_tt};
use common::{Workspace, toolchain, toolchain_installed};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn reference() -> PathBuf {
    root().join("tests/baselines/reference")
}

struct Case {
    name: String,
    path: PathBuf,
    units: Vec<Unit>,
    settings: Settings,
}

#[derive(Default)]
struct Settings {
    rewrite_imports: Option<String>,
    no_verify: bool,
}

fn cases() -> Vec<Case> {
    let mut files = Vec::new();
    for suite in ["compiler", "conformance"] {
        cases::files(&root().join("tests/cases").join(suite), &mut files);
    }
    files.sort();
    assert!(
        !files.is_empty(),
        "no case files under tests/cases/compiler or tests/cases/conformance"
    );
    let filter = std::env::var("TT_CASES").ok().filter(|f| !f.is_empty());
    let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut out = Vec::new();
    for path in files {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("a UTF-8 case name")
            .to_string();
        if let Some(other) = seen.insert(name.clone(), path.clone()) {
            panic!(
                "case names must be distinct, because baselines are named by them: {} and {}",
                other.display(),
                path.display()
            );
        }
        if filter.as_deref().is_some_and(|f| !name.contains(f)) {
            continue;
        }
        let text = fs::read_to_string(&path).expect("readable case");
        let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
        let parsed = cases::parse(&text, &file_name, &path);
        let settings = settings(&parsed.directives, &path);
        let units = parsed.units;
        out.push(Case {
            name,
            path,
            units,
            settings,
        });
    }
    out
}

fn settings(directives: &[(String, String)], path: &Path) -> Settings {
    let mut settings = Settings::default();
    for (name, value) in directives {
        match name.as_str() {
            "rewriteimports" => {
                assert!(
                    matches!(value.as_str(), "js" | "ts" | "off"),
                    "{}: @rewriteImports takes js, ts, or off",
                    path.display()
                );
                settings.rewrite_imports = Some(value.clone());
            }
            "noverify" => settings.no_verify = value == "true",
            other => panic!(
                "{}: unknown directive `@{other}`; a case takes @filename and the ttc options @rewriteImports and @noVerify",
                path.display()
            ),
        }
    }
    settings
}

struct Artifacts {
    types: String,
    errors: Option<String>,
    emit: String,
}

fn normalize(text: &str, dir: &Path) -> String {
    let dir = dir.to_string_lossy();
    text.replace("\r\n", "\n")
        .replace(dir.as_ref(), "$DIR")
        .replace('\\', "/")
}

fn line_col(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset];
    let line = before.matches('\n').count() + 1;
    let col = offset - before.rfind('\n').map_or(0, |i| i + 1) + 1;
    (line, col)
}

fn map_table(case: &Case) -> String {
    let mut out = String::new();
    for unit in case.units.iter().filter(|u| is_tt(Path::new(&u.name))) {
        let kind = SourceKind::from_path(Path::new(&unit.name)).unwrap_or_default();
        let mapped = ttc::emit_mapped_with_kind(&unit.content, kind);
        out.push_str(&format!(
            "//// [{}] {} mapping(s)\n",
            unit.name,
            mapped.mappings.len()
        ));
        let mut mappings = mapped.mappings.clone();
        mappings.sort_by_key(|m| (m.src, m.out));
        for m in mappings {
            let (src_line, src_col) = line_col(&unit.content, m.src);
            let (out_line, out_col) = line_col(&mapped.code, m.out);
            let text = &unit.content[m.src..m.src + m.len];
            let shown: String = text.chars().take(48).collect();
            let ellipsis = if shown.len() < text.len() { "..." } else { "" };
            out.push_str(&format!(
                "{src_line}:{src_col} -> {out_line}:{out_col} len {}  {:?}{ellipsis}\n",
                m.len, shown
            ));
        }
        out.push('\n');
    }
    out
}

fn ttc_command(project: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ttc"));
    command.current_dir(project);
    command
}

fn report(output: &std::process::Output) -> String {
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    text
}

fn emitted_files(dir: &Path, base: &Path, out: &mut Vec<(String, serde_json::Value, String)>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .expect("readable output directory")
        .map(|e| e.expect("readable output entry").path())
        .collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.is_dir() {
            if name != "node_modules" {
                emitted_files(&path, base, out);
            }
            continue;
        }
        let manifest = path.with_file_name(format!(".{name}.ttc-output.json"));
        let Ok(manifest) = fs::read_to_string(&manifest) else {
            continue;
        };
        let manifest: serde_json::Value =
            serde_json::from_str(&manifest).expect("an output manifest is JSON");
        let relative = path
            .strip_prefix(base)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let content = fs::read_to_string(&path).expect("readable output");
        out.push((relative, manifest, content));
    }
}

fn type_baseline(case: &Case, project: &Path, dir: &Path) -> String {
    let engine = Engine::new(None);
    let mut opened = engine
        .open_project(
            &[project.to_string_lossy().into_owned()],
            &ProjectOptions {
                tsconfig: Some(project.join("tsconfig.json")),
                out_dir: None,
            },
        )
        .unwrap_or_else(|e| panic!("{}: the engine did not open: {e}", case.path.display()));
    let mut out = String::new();
    for unit in case.units.iter().filter(|u| is_tt(Path::new(&u.name))) {
        let file = project
            .join(&unit.name)
            .canonicalize()
            .expect("written unit");
        out.push_str(&format!("=== {} ===\n", unit.name));
        let tokens = opened
            .semantic_tokens(&file)
            .unwrap_or_else(|e| panic!("{}: semantic tokens: {e}", case.path.display()));
        let lines: Vec<&str> = unit.content.split('\n').collect();
        let mut by_line: BTreeMap<u32, Vec<String>> = BTreeMap::new();
        let mut seen = std::collections::BTreeSet::new();
        for token in tokens {
            let start = token.range.start;
            if !seen.insert((start.line, start.character)) {
                continue;
            }
            let Some(hover) = opened
                .hover(&file, start)
                .unwrap_or_else(|e| panic!("{}: hover: {e}", case.path.display()))
            else {
                continue;
            };
            let line = lines.get(start.line as usize).copied().unwrap_or("");
            let name = utf16_slice(line, start.character, token.range.end.character);
            let signature = normalize(&hover.signature, dir).replace('\n', "\n>   ");
            by_line
                .entry(start.line)
                .or_default()
                .push(format!(">{name} : {signature}"));
        }
        for (index, line) in lines.iter().enumerate() {
            let Some(entries) = by_line.get(&(index as u32)) else {
                continue;
            };
            out.push_str(line);
            out.push('\n');
            for entry in entries {
                out.push_str(entry);
                out.push('\n');
            }
        }
        out.push('\n');
    }
    out
}

fn utf16_slice(line: &str, from: u32, to: u32) -> String {
    let mut units = 0u32;
    let mut out = String::new();
    for c in line.chars() {
        if units >= from && units < to {
            out.push(c);
        }
        units += c.len_utf16() as u32;
    }
    out
}

fn run_typed(case: &Case, dir: &Path, project: &Path) -> Artifacts {
    let out_dir = dir.join("out");
    let mut build = ttc_command(project);
    build
        .args(["--no-banner", "--jobs", "1", "--out-dir"])
        .arg(&out_dir);
    if let Some(mode) = &case.settings.rewrite_imports {
        build.args(["--rewrite-imports", mode]);
    }
    if case.settings.no_verify {
        build.arg("--no-verify");
    }
    let build = build.arg(".").output().expect("ttc runs");

    let mut emit = String::new();
    for unit in &case.units {
        emit.push_str(&format!("//// [{}] ////\n{}", unit.name, unit.content));
        if !unit.content.ends_with('\n') {
            emit.push('\n');
        }
        emit.push('\n');
    }
    let mut emitted = Vec::new();
    if out_dir.is_dir() {
        emitted_files(&out_dir, &out_dir, &mut emitted);
    }
    for (relative, manifest, _) in &emitted {
        if let Some(support) = manifest["support"].as_str() {
            emit.push_str(&format!("//// [{relative}] support module {support}\n"));
        }
    }
    for (relative, manifest, content) in &emitted {
        if manifest["support"].is_string() {
            continue;
        }
        emit.push_str(&format!("\n//// [{relative}]\n{content}"));
        if !content.ends_with('\n') {
            emit.push('\n');
        }
    }

    let check = ttc_command(project)
        .args(["--check-types", "."])
        .output()
        .expect("ttc runs");

    let tsc = if out_dir.is_dir() {
        fs::copy(project.join("tsconfig.json"), out_dir.join("tsconfig.json"))
            .expect("copyable tsconfig");
        let output = common::tsc()
            .args(["-p", "tsconfig.json", "--pretty", "false"])
            .current_dir(&out_dir)
            .output()
            .expect("tsc runs");
        Some(output)
    } else {
        None
    };

    let failed = !build.status.success()
        || !check.status.success()
        || tsc.as_ref().is_some_and(|o| !o.status.success());
    let errors = failed.then(|| {
        let mut text = format!(
            "==== ttc --out-dir (exit {}) ====\n{}\n==== ttc --check-types (exit {}) ====\n{}\n",
            build.status.code().unwrap_or(-1),
            report(&build),
            check.status.code().unwrap_or(-1),
            report(&check),
        );
        match &tsc {
            Some(output) => text.push_str(&format!(
                "==== tsc on the emitted TypeScript (exit {}) ====\n{}",
                output.status.code().unwrap_or(-1),
                report(output)
            )),
            None => text.push_str("==== tsc on the emitted TypeScript ====\nnothing was emitted\n"),
        }
        normalize(&text, dir)
    });

    Artifacts {
        types: type_baseline(case, project, dir),
        errors,
        emit: normalize(&emit, dir),
    }
}

fn run(case: &Case, typed: bool) {
    let workspace = Workspace::in_repo("cases");
    let dir = workspace.path().canonicalize().expect("a workspace");
    let project = dir.join("project");
    for unit in &case.units {
        let path = project.join(&unit.name);
        fs::create_dir_all(path.parent().unwrap()).expect("writable unit directory");
        fs::write(&path, &unit.content).expect("writable unit");
    }
    if !project.join("tsconfig.json").exists() {
        fs::write(project.join("tsconfig.json"), cases::DEFAULT_TSCONFIG)
            .expect("writable tsconfig");
    }

    let base = reference().join(&case.name);
    let with = |extension: &str| base.with_file_name(format!("{}.{extension}", case.name));
    expect(&with("map.txt"), &map_table(case));
    if !typed {
        return;
    }
    let artifacts = run_typed(case, &dir, &project);
    expect(&with("ts"), &artifacts.emit);
    match &artifacts.errors {
        Some(errors) => expect(&with("errors.txt"), errors),
        None => expect_absent(&with("errors.txt")),
    }
    expect(&with("types"), &artifacts.types);
}

#[test]
fn every_case_matches_its_baselines() {
    assert!(
        !(updating() && !toolchain_installed()),
        "UPDATE_EXPECT would rewrite the case baselines without a TypeScript \
         to annotate, check, and hover them — run `npm ci` at the repository root first"
    );
    let typed = toolchain();
    if !typed {
        eprintln!(
            "SKIP the typed case baselines (.ts, .errors.txt, .types): no TypeScript \
             installed — run `npm ci`"
        );
    }
    let cases = cases();
    let next = AtomicUsize::new(0);
    let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(4);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some(case) = cases.get(index) else {
                        break;
                    };
                    let outcome =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(case, typed)));
                    if let Err(payload) = outcome {
                        let message = payload
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                            .unwrap_or_else(|| "a non-string panic".to_string());
                        failures
                            .lock()
                            .unwrap()
                            .push(format!("{}:\n{message}", case.path.display()));
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().unwrap();
    assert!(
        failures.is_empty(),
        "{} case(s) failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
