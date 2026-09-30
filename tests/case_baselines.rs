//! The case runner: every file under `tests/cases/compiler/` and
//! `tests/cases/conformance/` is a compilation whose artifacts are held to
//! baselines under `tests/baselines/reference/`. Editor cases, under
//! `tests/cases/editor/`, have their own runner (`tests/editor_cases.rs`).
//!
//! A case can also carry an oracle: `// @twin: <unit>` names a TypeScript
//! unit that must print what the `// @run` entry prints, and
//! `// @expectErrors: <code>, ...` names the tt diagnostics the case must
//! report. A case whose oracle disagrees fails unless it is listed in
//! `tests/oracle-failures.txt`, and a listed case whose oracle agrees fails
//! too. The generated matrix under `tests/cases/conformance/matrix/`
//! (`scripts/generate-cases`) runs a fixed-seed sample by default and every
//! case with `TT_MATRIX_CASES=all`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use ttc::SourceKind;
use ttc::engine::{Engine, ProjectOptions};

mod common;
use common::baseline::{compare, compare_absent, diff, filtered_by, not_sampled, updating};
use common::cases::{self, Unit, is_tt};
use common::{Workspace, toolchain, toolchain_installed};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn reference() -> PathBuf {
    root().join("tests/baselines/reference")
}

fn baseline_dir(path: &Path) -> PathBuf {
    match path
        .parent()
        .and_then(|dir| dir.strip_prefix(root().join(MATRIX)).ok())
    {
        Some(inside) => reference().join("matrix").join(inside),
        None => reference(),
    }
}

struct Case {
    name: String,
    path: PathBuf,
    units: Vec<Unit>,
    settings: Settings,
    oracle: Oracle,
    kinds: BTreeSet<&'static str>,
}

#[derive(Clone, Default)]
struct Settings {
    rewrite_imports: Option<String>,
    no_verify: bool,
    run: Option<String>,
}

#[derive(Clone, Default)]
struct Oracle {
    twin: Option<String>,
    errors: Vec<String>,
}

const KINDS: [&str; 5] = ["ts", "errors.txt", "map.txt", "types", "stdout"];

const PER_FILE: [&str; 4] = ["run", "twin", "expecterrors", "baselines"];

const MATRIX: &str = "tests/cases/conformance/matrix";

const MATRIX_SAMPLE: usize = 120;

const MATRIX_SEED: u64 = 0x7474_6d61_7472_6978;

struct Selection {
    cases: Vec<Case>,
    names: BTreeSet<String>,
    unsampled: Vec<(String, PathBuf)>,
    summary: Option<String>,
}

fn cases() -> Selection {
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
    let mut names = BTreeSet::new();
    let mut chosen = Vec::new();
    let mut matrix = Vec::new();
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
        if filter.is_none() && path.starts_with(root().join(MATRIX)) {
            matrix.push((name, path));
        } else {
            chosen.push((name, path));
        }
    }
    let (sampled, unsampled, summary) = sample(matrix);
    chosen.extend(sampled);
    let mut out = Vec::new();
    for (name, path) in &chosen {
        out.extend(expand(name, path, &mut names));
    }
    let mut skipped = Vec::new();
    for (name, path) in &unsampled {
        for case in expand(name, path, &mut names) {
            skipped.push((case.name, case.path));
        }
    }
    Selection {
        cases: out,
        names,
        unsampled: skipped,
        summary,
    }
}

fn expand(name: &str, path: &Path, names: &mut BTreeSet<String>) -> Vec<Case> {
    let text = fs::read_to_string(path).expect("readable case");
    let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
    let parsed = cases::parse(&text, &file_name, path);
    let run = run_entry(&parsed.directives, &parsed.units, path);
    let oracle = oracle(&parsed.directives, &parsed.units, run.as_deref(), path);
    let kinds = baseline_kinds(&parsed.directives, path);
    let mut out = Vec::new();
    for (suffix, mut settings) in configurations(&parsed.directives, path) {
        settings.run = run.clone();
        names.insert(format!("{name}{suffix}"));
        out.push(Case {
            name: format!("{name}{suffix}"),
            path: path.to_path_buf(),
            units: parsed.units.clone(),
            settings,
            oracle: oracle.clone(),
            kinds: kinds.clone(),
        });
    }
    out
}

type Named = (String, PathBuf);

fn sample(matrix: Vec<Named>) -> (Vec<Named>, Vec<Named>, Option<String>) {
    let total = matrix.len();
    if total == 0 {
        return (matrix, Vec::new(), None);
    }
    let requested = std::env::var("TT_MATRIX_CASES").unwrap_or_default();
    if requested == "all" {
        return (
            matrix,
            Vec::new(),
            Some(format!("all {total} matrix cases")),
        );
    }
    let count = if requested.is_empty() {
        MATRIX_SAMPLE
    } else {
        requested
            .parse()
            .unwrap_or_else(|_| panic!("TT_MATRIX_CASES takes a count or `all`, not `{requested}`"))
    }
    .min(total);
    let seed = std::env::var("TT_MATRIX_SEED")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .parse()
                .unwrap_or_else(|_| panic!("TT_MATRIX_SEED takes a number, not `{value}`"))
        })
        .unwrap_or(MATRIX_SEED);
    let mut state = seed;
    let mut indices: Vec<usize> = (0..total).collect();
    for i in 0..count {
        let j = i + (splitmix(&mut state) % (total - i) as u64) as usize;
        indices.swap(i, j);
    }
    let picked: BTreeSet<usize> = indices[..count].iter().copied().collect();
    let mut sampled = Vec::new();
    let mut unsampled = Vec::new();
    for (index, case) in matrix.into_iter().enumerate() {
        if picked.contains(&index) {
            sampled.push(case);
        } else {
            unsampled.push(case);
        }
    }
    (
        sampled,
        unsampled,
        Some(format!(
            "{count} of {total} matrix cases, seed {seed} (TT_MATRIX_CASES=all for every one)"
        )),
    )
}

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn single<'a>(directives: &'a [(String, String)], name: &str, path: &Path) -> Option<&'a str> {
    let mut found = directives.iter().filter(|(n, _)| n == name);
    let (_, value) = found.next()?;
    assert!(
        found.next().is_none(),
        "{}: a case has at most one @{name}",
        path.display()
    );
    Some(value.as_str())
}

fn oracle(
    directives: &[(String, String)],
    units: &[Unit],
    run: Option<&str>,
    path: &Path,
) -> Oracle {
    let twin = single(directives, "twin", path).map(|twin| {
        let entry = run.unwrap_or_else(|| {
            panic!(
                "{}: @twin names the TypeScript program that must print what the @run entry prints, and the case has no @run",
                path.display()
            )
        });
        assert!(
            is_tt(Path::new(entry)),
            "{}: @run names `{entry}`, and a twin is the TypeScript counterpart of a .tt or .ttx entry",
            path.display()
        );
        assert!(
            units.iter().any(|unit| unit.name == twin) && !is_tt(Path::new(twin)),
            "{}: @twin names `{twin}`, which is not one of the case's TypeScript units",
            path.display()
        );
        script_of(twin, path);
        twin.to_string()
    });
    let errors: Vec<String> = single(directives, "expecterrors", path)
        .map(|codes| {
            codes
                .split(',')
                .map(|code| code.trim().to_string())
                .filter(|code| !code.is_empty())
                .collect()
        })
        .unwrap_or_default();
    assert!(
        errors.is_empty() || run.is_none(),
        "{}: a case that expects errors is not run, so it takes no @run",
        path.display()
    );
    Oracle { twin, errors }
}

fn baseline_kinds(directives: &[(String, String)], path: &Path) -> BTreeSet<&'static str> {
    let Some(value) = single(directives, "baselines", path) else {
        return KINDS.into_iter().collect();
    };
    let mut kinds = BTreeSet::new();
    for kind in value.split(',').map(str::trim).filter(|k| !k.is_empty()) {
        let known = KINDS
            .iter()
            .find(|known| **known == kind)
            .unwrap_or_else(|| {
                panic!(
                    "{}: @baselines takes {}, not `{kind}`",
                    path.display(),
                    KINDS.join(", ")
                )
            });
        kinds.insert(*known);
    }
    kinds
}

const OPTIONS: [(&str, &[&str]); 2] = [
    ("noverify", &["true", "false"]),
    ("rewriteimports", &["js", "ts", "off"]),
];

const MAX_VARIATIONS: usize = 25;

const RUN_TIMEOUT: Duration = Duration::from_secs(10);

fn run_entry(directives: &[(String, String)], units: &[Unit], path: &Path) -> Option<String> {
    let entry = single(directives, "run", path)?;
    assert!(
        units.iter().any(|unit| unit.name == *entry),
        "{}: @run names `{entry}`, which is not one of the case's units ({})",
        path.display(),
        units
            .iter()
            .map(|unit| unit.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    script_of(entry, path);
    Some(entry.to_string())
}

fn script_of(entry: &str, path: &Path) -> String {
    let unit = Path::new(entry);
    let extension = match unit.extension().and_then(|e| e.to_str()) {
        Some("tt" | "ts" | "ttx" | "tsx") => "js",
        Some("mts") => "mjs",
        Some("cts") => "cjs",
        _ => panic!(
            "{}: @run and @twin take a .tt, .ttx, .ts, .tsx, .mts, or .cts unit, which the emitted tree compiles to a script node runs; `{entry}` is not one",
            path.display()
        ),
    };
    unit.with_extension(extension)
        .to_string_lossy()
        .replace('\\', "/")
}

fn option_values(option: &str, text: &str, path: &Path) -> (Vec<String>, bool) {
    let allowed = OPTIONS
        .iter()
        .find(|(name, _)| *name == option)
        .map(|(_, values)| *values)
        .expect("a known option");
    let check = |value: &str| {
        assert!(
            allowed.contains(&value),
            "{}: @{option} takes {}, not `{value}`",
            path.display(),
            allowed.join(", ")
        );
    };
    let mut star = false;
    let mut includes: Vec<String> = Vec::new();
    let mut excludes: Vec<String> = Vec::new();
    for entry in text.split(',') {
        let entry = entry.trim().to_ascii_lowercase();
        if entry.is_empty() {
            continue;
        }
        if entry == "*" {
            star = true;
        } else if let Some(excluded) = entry.strip_prefix('-').or_else(|| entry.strip_prefix('!')) {
            check(excluded);
            excludes.push(excluded.to_string());
        } else {
            check(&entry);
            includes.push(entry);
        }
    }
    if includes.len() <= 1 && !star && excludes.is_empty() {
        return (includes, false);
    }
    let mut values: Vec<String> = Vec::new();
    for include in includes {
        if !values.contains(&include) {
            values.push(include);
        }
    }
    if star {
        for value in allowed {
            if !values.iter().any(|known| known == value) {
                values.push(value.to_string());
            }
        }
    }
    values.retain(|value| !excludes.contains(value));
    assert!(
        !values.is_empty(),
        "{}: the variations of @{option} are an empty set",
        path.display()
    );
    (values, true)
}

fn configurations(directives: &[(String, String)], path: &Path) -> Vec<(String, Settings)> {
    let mut chosen: BTreeMap<&str, (Vec<String>, bool)> = BTreeMap::new();
    for (name, value) in directives {
        if PER_FILE.contains(&name.as_str()) {
            continue;
        }
        let Some((option, _)) = OPTIONS.iter().find(|(option, _)| option == name) else {
            panic!(
                "{}: unknown directive `@{name}`; a case takes @filename, @run, @twin, @expectErrors, @baselines, and the ttc options @rewriteImports and @noVerify",
                path.display()
            );
        };
        chosen.insert(option, option_values(option, value, path));
    }
    let count: usize = chosen
        .values()
        .map(|(values, _)| values.len().max(1))
        .product();
    assert!(
        count <= MAX_VARIATIONS,
        "{}: the options' variations make {count} configurations, more than {MAX_VARIATIONS}",
        path.display()
    );
    let mut out = vec![(Vec::<(&str, String)>::new(), Settings::default())];
    for (option, (values, varied)) in &chosen {
        let mut next = Vec::new();
        for (name, settings) in &out {
            for value in values {
                let mut name = name.clone();
                if *varied {
                    name.push((option, value.clone()));
                }
                let mut settings = settings.clone();
                match *option {
                    "noverify" => settings.no_verify = value == "true",
                    _ => settings.rewrite_imports = Some(value.clone()),
                }
                next.push((name, settings));
            }
        }
        if !values.is_empty() {
            out = next;
        }
    }
    out.into_iter()
        .map(|(name, settings)| {
            let suffix = if name.is_empty() {
                String::new()
            } else {
                let parts: Vec<String> = name
                    .iter()
                    .map(|(option, value)| format!("{option}={value}"))
                    .collect();
                format!("({})", parts.join(","))
            };
            (suffix, settings)
        })
        .collect()
}

struct Artifacts {
    types: Option<String>,
    errors: Option<String>,
    emit: String,
    executions: Vec<Execution>,
}

struct Execution {
    stdout: String,
    stderr: Option<String>,
    status: String,
}

enum Verdict {
    Agrees,
    Disagrees { observed: String, detail: String },
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

    let mut entries: Vec<&str> = case.settings.run.iter().map(String::as_str).collect();
    entries.extend(case.oracle.twin.as_deref());
    let executions = execute(case, &entries, dir, &out_dir, errors.is_none());

    Artifacts {
        types: case
            .kinds
            .contains("types")
            .then(|| type_baseline(case, project, dir)),
        errors,
        emit: normalize(&emit, dir),
        executions,
    }
}

fn execute(
    case: &Case,
    entries: &[&str],
    dir: &Path,
    out_dir: &Path,
    clean: bool,
) -> Vec<Execution> {
    if entries.is_empty() {
        return Vec::new();
    }
    let not_run = |stderr: String, status: &str| {
        entries
            .iter()
            .map(|_| Execution {
                stdout: String::new(),
                stderr: Some(stderr.clone()),
                status: status.to_string(),
            })
            .collect()
    };
    if !clean {
        return not_run(
            format!(
                "==== not run: the case does not compile cleanly (see {}.errors.txt) ====\n",
                case.name
            ),
            "not run",
        );
    }
    let run_dir = dir.join("run");
    let emit = common::tsc()
        .args([
            "-p",
            "tsconfig.json",
            "--pretty",
            "false",
            "--noEmit",
            "false",
            "--rewriteRelativeImportExtensions",
            "--rootDir",
            ".",
            "--outDir",
        ])
        .arg(&run_dir)
        .current_dir(out_dir)
        .output()
        .expect("tsc runs");
    if !emit.status.success() {
        return not_run(
            normalize(
                &format!(
                    "==== tsc emitting JavaScript (exit {}) ====\n{}",
                    emit.status.code().unwrap_or(-1),
                    report(&emit)
                ),
                dir,
            ),
            "not emitted",
        );
    }
    fs::write(run_dir.join("package.json"), "{ \"type\": \"module\" }\n")
        .expect("writable package.json");
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| node(case, entry, index, dir, &run_dir))
        .collect()
}

fn node(case: &Case, entry: &str, index: usize, dir: &Path, run_dir: &Path) -> Execution {
    let script = script_of(entry, &case.path);
    let stdout_path = dir.join(format!("run{index}.stdout"));
    let stderr_path = dir.join(format!("run{index}.stderr"));
    let mut command = Command::new("node");
    command.env_clear().env("TZ", "UTC");
    for inherited in ["PATH", "SYSTEMROOT"] {
        if let Some(value) = std::env::var_os(inherited) {
            command.env(inherited, value);
        }
    }
    let mut child = command
        .arg("--permission")
        .arg(format!("--allow-fs-read={}", run_dir.display()))
        .arg(&script)
        .current_dir(run_dir)
        .stdin(Stdio::null())
        .stdout(fs::File::create(&stdout_path).expect("writable stdout capture"))
        .stderr(fs::File::create(&stderr_path).expect("writable stderr capture"))
        .spawn()
        .expect("node runs");
    let deadline = Instant::now() + RUN_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().expect("a waitable node") {
            break Some(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let stdout = normalize(
        &String::from_utf8_lossy(&fs::read(&stdout_path).expect("readable stdout capture")),
        dir,
    );
    let stderr = node_report(
        &String::from_utf8_lossy(&fs::read(&stderr_path).expect("readable stderr capture")),
        dir,
    );
    let (heading, status) = match status {
        None => (
            Some(format!(
                "node {script} timed out after {} s",
                RUN_TIMEOUT.as_secs()
            )),
            "timed out".to_string(),
        ),
        Some(status) => {
            let code = status.code().unwrap_or(-1);
            let heading = (!status.success() || !stderr.is_empty())
                .then(|| format!("node {script} (exit {code})"));
            (heading, format!("exit {code}"))
        }
    };
    Execution {
        stdout,
        stderr: heading.map(|heading| format!("==== {heading} ====\n{stderr}")),
        status,
    }
}

fn judge(case: &Case, artifacts: &Artifacts) -> Option<Verdict> {
    let disagrees = |observed: &str, detail: String| {
        Some(Verdict::Disagrees {
            observed: observed.to_string(),
            detail,
        })
    };
    if let Some(twin) = &case.oracle.twin {
        if let Some(errors) = &artifacts.errors {
            return disagrees("does not compile cleanly", errors.clone());
        }
        let [program, counterpart] = &artifacts.executions[..] else {
            unreachable!("a twin case runs its entry and its twin");
        };
        if counterpart.stdout.is_empty() {
            return disagrees(
                "its twin prints nothing",
                format!("{twin} printed nothing, so it proves nothing"),
            );
        }
        if program.stdout != counterpart.stdout {
            return disagrees(
                "prints what its twin does not",
                format!(
                    "the entry's stdout (+) against {twin}'s (-):\n{}",
                    diff(&counterpart.stdout, &program.stdout)
                ),
            );
        }
        if program.status != counterpart.status {
            return disagrees(
                &format!(
                    "ends with {} where its twin ends with {}",
                    program.status, counterpart.status
                ),
                program.stderr.clone().unwrap_or_default(),
            );
        }
        return Some(Verdict::Agrees);
    }
    if case.oracle.errors.is_empty() {
        return None;
    }
    let Some(errors) = &artifacts.errors else {
        return disagrees(
            "compiles cleanly",
            format!("expected {}", case.oracle.errors.join(", ")),
        );
    };
    let missing: Vec<&str> = case
        .oracle
        .errors
        .iter()
        .filter(|code| !errors.contains(&format!("error[{code}]")))
        .map(String::as_str)
        .collect();
    if missing.is_empty() {
        return Some(Verdict::Agrees);
    }
    disagrees(
        &format!("does not report {}", missing.join(", ")),
        errors.clone(),
    )
}

fn node_report(text: &str, dir: &Path) -> String {
    let mut out = String::new();
    for line in normalize(text, dir).lines() {
        let frame = line.trim_start();
        if (frame.starts_with("at ") && frame.contains("node:internal"))
            || line.starts_with("Node.js v")
        {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn run(case: &Case, typed: bool) -> (Option<Verdict>, Vec<String>) {
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

    let with =
        |extension: &str| baseline_dir(&case.path).join(format!("{}.{extension}", case.name));
    let kept = |kind: &str| case.kinds.contains(kind);
    let mut failures = Vec::new();
    if kept("map.txt") {
        failures.extend(compare(&with("map.txt"), &map_table(case)).err());
    }
    let mut verdict = None;
    if typed {
        let artifacts = run_typed(case, &dir, &project);
        if kept("ts") {
            failures.extend(compare(&with("ts"), &artifacts.emit).err());
        }
        if kept("errors.txt") {
            let errors = match &artifacts.errors {
                Some(errors) => compare(&with("errors.txt"), errors),
                None => compare_absent(&with("errors.txt")),
            };
            failures.extend(errors.err());
        }
        if let Some(types) = &artifacts.types {
            failures.extend(compare(&with("types"), types).err());
        }
        if kept("stdout") {
            let (stdout, stderr) = match artifacts.executions.first() {
                Some(execution) if case.settings.run.is_some() => (
                    compare(&with("stdout"), &execution.stdout),
                    match &execution.stderr {
                        Some(stderr) => compare(&with("stderr"), stderr),
                        None => compare_absent(&with("stderr")),
                    },
                ),
                _ => (
                    compare_absent(&with("stdout")),
                    compare_absent(&with("stderr")),
                ),
            };
            failures.extend(stdout.err());
            failures.extend(stderr.err());
        }
        verdict = judge(case, &artifacts);
    }
    (verdict, failures)
}

const ORACLE_FAILURES: &str = "tests/oracle-failures.txt";

struct Listed {
    observed: String,
    line: usize,
}

fn listed_failures() -> BTreeMap<String, Listed> {
    let path = root().join(ORACLE_FAILURES);
    let text = fs::read_to_string(&path).unwrap_or_default();
    let mut out = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [name, observed, reason] = fields[..] else {
            panic!(
                "{ORACLE_FAILURES}:{}: a line is a case name, a tab, what the run observes, a tab, and the TASK-NNN that tracks it",
                index + 1
            );
        };
        assert!(
            reason.starts_with("TASK-"),
            "{ORACLE_FAILURES}:{}: the reason names the TASK-NNN that tracks the defect",
            index + 1
        );
        let previous = out.insert(
            name.to_string(),
            Listed {
                observed: observed.to_string(),
                line: index + 1,
            },
        );
        assert!(
            previous.is_none(),
            "{ORACLE_FAILURES}:{}: `{name}` is listed twice",
            index + 1
        );
    }
    out
}

fn oracle_failures(
    selection: &Selection,
    verdicts: &BTreeMap<String, Option<Verdict>>,
    typed: bool,
) -> Vec<String> {
    let listed = listed_failures();
    let unfiltered = !filtered_by(
        std::env::args().skip(1),
        std::env::var("TT_CASES").ok().as_deref(),
    );
    let mut failures = Vec::new();
    for (name, entry) in &listed {
        if unfiltered && !selection.names.contains(name) {
            failures.push(format!(
                "{ORACLE_FAILURES}:{}: `{name}` names no case; remove the line",
                entry.line
            ));
        }
    }
    if !typed {
        return failures;
    }
    for (name, verdict) in verdicts {
        let entry = listed.get(name);
        match (verdict, entry) {
            (None, Some(entry)) => failures.push(format!(
                "{ORACLE_FAILURES}:{}: `{name}` has no @twin or @expectErrors oracle to fail",
                entry.line
            )),
            (Some(Verdict::Agrees), Some(entry)) => failures.push(format!(
                "{ORACLE_FAILURES}:{}: `{name}` agrees with its oracle now; remove the line",
                entry.line
            )),
            (Some(Verdict::Disagrees { observed, .. }), Some(entry))
                if *observed != entry.observed =>
            {
                failures.push(format!(
                    "{ORACLE_FAILURES}:{}: `{name}` is listed as `{}`, and the run observes `{observed}`",
                    entry.line, entry.observed
                ))
            }
            (Some(Verdict::Disagrees { observed, detail }), None) => failures.push(format!(
                "{name}: {observed}\n{detail}\nA defect is listed in {ORACLE_FAILURES} with the task that tracks it: `{name}<TAB>{observed}<TAB>TASK-NNN: ...`"
            )),
            _ => {}
        }
    }
    failures
}

fn record_unsampled(cases: &[(String, PathBuf)]) {
    for (name, case) in cases {
        for kind in KINDS.iter().chain(&["stderr"]) {
            let path = baseline_dir(case).join(format!("{name}.{kind}"));
            if path.exists() {
                not_sampled(&path);
            }
        }
    }
}

fn message(payload: &Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "a non-string panic".to_string())
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
    let selection = cases();
    if let Some(summary) = &selection.summary {
        eprintln!("case matrix: {summary}");
    }
    record_unsampled(&selection.unsampled);
    let cases = &selection.cases;
    let next = AtomicUsize::new(0);
    let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let verdicts: Mutex<BTreeMap<String, Option<Verdict>>> = Mutex::new(BTreeMap::new());
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
                    match outcome {
                        Ok((verdict, differences)) => {
                            verdicts.lock().unwrap().insert(case.name.clone(), verdict);
                            if !differences.is_empty() {
                                failures.lock().unwrap().push(format!(
                                    "{}:\n{} baseline(s) differ:\n\n{}",
                                    case.path.display(),
                                    differences.len(),
                                    differences.join("\n\n")
                                ));
                            }
                        }
                        Err(payload) => {
                            failures.lock().unwrap().push(format!(
                                "{}:\n{}",
                                case.path.display(),
                                message(&payload)
                            ));
                        }
                    }
                }
            });
        }
    });
    let mut failures = failures.into_inner().unwrap();
    let verdicts = verdicts.into_inner().unwrap();
    failures.extend(oracle_failures(&selection, &verdicts, typed));
    assert!(
        failures.is_empty(),
        "{} case(s) failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

#[test]
fn the_case_matrix_is_what_its_spec_generates() {
    let output = Command::new("node")
        .arg(root().join("scripts/generate-cases"))
        .arg("--check")
        .current_dir(root())
        .output()
        .expect("node runs scripts/generate-cases");
    assert!(
        output.status.success(),
        "tests/cases/conformance/matrix differs from what tests/matrix generates:\n{}\nRun `node scripts/generate-cases` and review the diff.",
        report(&output)
    );
}
