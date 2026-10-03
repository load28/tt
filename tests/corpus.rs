//! The first design contract, checked against real TypeScript rather than
//! against examples someone thought of.
//!
//! > 모든 유효한 TypeScript 파일은 그대로 유효한 `.tt` 파일입니다.
//!
//! That is a statement about *every* file, and `tests/passthrough.rs`
//! defends it with cases a person wrote — a sample, not a proof. This
//! suite runs the same claim over a corpus of TypeScript nobody wrote for
//! tt, and it needs no oracle: a file with no tt syntax must come back
//! **byte for byte**, so the input is the expected output.
//!
//! The corpus is what is already on the machine: this repository's own
//! TypeScript, plus the standard library declarations that ship inside the
//! installed TypeScript package — over a hundred files of the most
//! scrutinised TypeScript there is. Pinning costs nothing new: the version
//! is the one `package.json` fixes for the typed suites, for the same
//! reason (a floating version breaks this gate on someone else's commit).
//!
//! ```sh
//! cargo test --test corpus                  # sample, as a PR runs it
//! TTC_CORPUS_FULL=1 cargo test --test corpus  # every file
//! TTC_CORPUS=/path/to/tree cargo test --test corpus  # another corpus
//! ```
//!
//! A skip means "no corpus on this machine". Where one is supposed to be
//! there, `TTC_REQUIRE_CORPUS=1` turns the skip into a failure — the same
//! guard `tests/native.rs` uses, for the same reason: a skipped suite is
//! green in every other way.
//!
//! The second test runs TypeScript's own test cases, at the commit
//! `tests/typescript-cases.json` pins (the `gitHead` of the TypeScript
//! `package.json` pins), fetched by `scripts/fetch-typescript-cases`. Each
//! case is split into units by `// @filename` as TypeScript's harness does
//! (`tsc/internal/testrunner/test_case_parser.go`), and every `.ts`/`.tsx`
//! unit that the pinned `tsc` parses without error, and whose full check
//! reports no TS1xxx syntax or grammar error, must pass through `ttc`
//! byte for byte with no diagnostic. A unit that does not is listed in
//! `tests/passthrough-accepted.txt` (intended, with the reason) or
//! `tests/passthrough-triaged.txt` (a bug, with its task), never both; a
//! listed unit that passes through again fails the run, as typescript-go's
//! `submoduleAccepted.txt` and `submoduleTriaged.txt` do.
//!
//! ```sh
//! scripts/fetch-typescript-cases                       # once; about 55 MB
//! cargo test --test corpus typescript_test_cases        # 400 cases, fixed seed
//! TTC_TYPESCRIPT_CASES=all cargo test --test corpus typescript_test_cases
//! ```
//!
//! `TTC_REQUIRE_TYPESCRIPT_CASES=1` turns "not fetched" into a failure.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use ttc::{Options, SourceKind};

mod common;
use common::typescript_cases::{
    Oracle, cases_checkout, cases_required, manifest, seeded_choice, tree,
};

/// How many files a sample run compiles. A PR gets a fixed, spread-out
/// slice of the corpus rather than a random one: a gate that tests
/// something different on every run cannot be bisected.
const SAMPLE: usize = 250;

#[test]
fn oracle_reachability_preserves_the_members_file_spelling() {
    use std::fs;

    if !common::toolchain() {
        return;
    }
    let workspace = common::Workspace::in_repo("oracle-file-identity");
    let dir = workspace.path().join("MixedCase");
    fs::create_dir_all(&dir).unwrap();
    let imported = dir.join("Imported.ts");
    let importer = dir.join("main.ts");
    let config = dir.join("tsconfig.json");
    fs::write(&imported, "export function f() {}\n").unwrap();
    fs::write(&importer, "import { f } from \"./Imported\";\n").unwrap();
    fs::write(
        &config,
        r#"{"compilerOptions":{"module":"preserve","noEmit":true}}"#,
    )
    .unwrap();
    let answer = Oracle::start().ask(&serde_json::json!({
        "check": config,
        "units": [imported, importer],
    }));
    assert_eq!(answer["diagnostics"], serde_json::json!([]));
    assert_eq!(answer["reached"], serde_json::json!([imported]));
}

/// This repository's own TypeScript — hand-written, always present, and
/// under review like everything else here. A corpus that needs no download
/// and no pin, so the differential runs on every machine and every job.
const OWN_DIRS: [&str; 3] = [
    "editors/vscode/server/src",
    "website/scripts",
    "integrations",
];

/// Where more TypeScript lives once the repository has installed one: the
/// TypeScript compiler's own library declarations, which ship in the
/// package and are as real as hand-written TypeScript gets. Pinned by the
/// same `package.json` the typed suites read (`src/typescript/toolchain.rs`).
const INSTALLED_DIRS: [&str; 2] = [
    "node_modules/@typescript/typescript-{platform}/lib",
    "node_modules/@typescript/native-preview-{platform}/lib",
];

/// Directories that hold something other than hand-written sources.
/// `node_modules` is other people's code and would swamp the sample;
/// a build output is a copy of a source already in it.
const SKIP_DIRS: [&str; 4] = ["node_modules", "dist", "out", "target"];

fn required() -> bool {
    std::env::var_os("TTC_REQUIRE_CORPUS").is_some_and(|v| !v.is_empty() && v != "0")
}

fn full() -> bool {
    std::env::var_os("TTC_CORPUS_FULL").is_some_and(|v| !v.is_empty() && v != "0")
}

/// The corpus roots. A named tree replaces them all; otherwise this
/// repository's own TypeScript, plus the installed TypeScript's library
/// declarations.
fn roots() -> Vec<PathBuf> {
    if let Some(named) = std::env::var_os("TTC_CORPUS").filter(|v| !v.is_empty()) {
        let root = PathBuf::from(named);
        return root.is_dir().then_some(vec![root]).unwrap_or_default();
    }
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let platform = format!("{}-{}", os_name(), arch_name());
    OWN_DIRS
        .iter()
        .map(|dir| here.join(dir))
        .chain(
            INSTALLED_DIRS
                .iter()
                .map(|dir| here.join(dir.replace("{platform}", &platform))),
        )
        .filter(|dir| dir.is_dir())
        .collect()
}

fn os_name() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    }
}

fn arch_name() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    }
}

/// Every `.ts`/`.tsx` file under `roots`, in path order — so a sample is
/// the same slice on every machine.
fn corpus() -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack: Vec<PathBuf> = roots();
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if !SKIP_DIRS.contains(&name.as_ref()) {
                    stack.push(path);
                }
            } else if matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("ts" | "tsx")
            ) {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// The files a run compiles: all of them, or a spread-out fixed slice.
fn selection() -> Vec<PathBuf> {
    let all = corpus();
    if full() || all.len() <= SAMPLE {
        return all;
    }
    let stride = all.len().div_ceil(SAMPLE);
    all.into_iter().step_by(stride).collect()
}

/// What one corpus file turned out to be.
enum Verdict {
    /// Byte-identical, and the bytes are TypeScript. The contract holds.
    Unchanged,
    /// Not a case this contract speaks about — the file is not valid
    /// TypeScript, so "every valid TypeScript file" does not reach it.
    NotTypeScript,
    /// The contract broke, with the story of how.
    Broken(String),
}

/// Whether a corpus file survives the transform unchanged.
///
/// The awkward part of a differential over found data is deciding what is
/// *valid* TypeScript, and the answer falls out of the passthrough itself.
/// Compile once with the output check off: if the result is byte-identical
/// to the source, then turning the check on asks swc about **the source's
/// own bytes**, and its verdict is the oracle. A compiler cannot tell
/// "invalid input passed through" from "a lowering bug" — but a *test*
/// that already knows the output equals the input can.
fn check(path: &Path) -> Verdict {
    let Ok(source) = std::fs::read_to_string(path) else {
        return Verdict::NotTypeScript; // not text; not this contract's subject
    };
    let base = Options {
        source_kind: match path.extension().and_then(|e| e.to_str()) {
            Some("tsx") => SourceKind::Tsx,
            _ => SourceKind::TypeScript,
        },
        // The one exception the contract allows is specifier rewriting, and
        // it is a flag — so the differential runs with it off and the
        // claim becomes exactly "input bytes == output bytes".
        rewrite_imports: ttc::ImportRewrite::Off,
        ..Options::default()
    };
    let unchecked = Options {
        verify: false,
        ..base.clone()
    };
    let out = match ttc::compile(&source, &unchecked) {
        Ok(out) => out,
        Err(error) => {
            // A tt rule claimed something in a file that is meant to be
            // ordinary TypeScript. Whether the file is valid is now the
            // question, and nothing here can answer it — so it is reported
            // with what ttc said, for a person to classify.
            return Verdict::Broken(format!("{}: rejected: {error}", path.display()));
        }
    };
    if out != source {
        let at = out
            .bytes()
            .zip(source.bytes())
            .position(|(a, b)| a != b)
            .unwrap_or(source.len().min(out.len()));
        let (line, col) = line_col(&source, at);
        return Verdict::Broken(format!(
            "{}: output differs at {line}:{col}\n  in:  {:?}\n  out: {:?}",
            path.display(),
            window(&source, at),
            window(&out, at),
        ));
    }
    match ttc::compile(&source, &base) {
        Ok(_) => Verdict::Unchanged,
        // The emission is the source, so "the emission does not parse"
        // is "the source does not parse".
        Err(_) => Verdict::NotTypeScript,
    }
}

fn line_col(text: &str, at: usize) -> (usize, usize) {
    let before = &text[..at.min(text.len())];
    (
        before.bytes().filter(|b| *b == b'\n').count() + 1,
        before.len() - before.rfind('\n').map_or(0, |nl| nl + 1) + 1,
    )
}

/// A readable slice of `text` around `at`, on a character boundary.
fn window(text: &str, at: usize) -> String {
    let start = (0..=at.min(text.len()))
        .rev()
        .find(|i| text.is_char_boundary(*i) && at - i >= 20)
        .unwrap_or(0);
    let end = (at.min(text.len())..=text.len())
        .find(|i| text.is_char_boundary(*i) && i - at >= 40)
        .unwrap_or(text.len());
    text[start..end].replace('\n', "\\n")
}

#[test]
fn typescript_the_compiler_never_saw_comes_back_unchanged() {
    let files = selection();
    if files.is_empty() {
        assert!(
            !required(),
            "TTC_REQUIRE_CORPUS is set but no corpus was found \
             (TTC_CORPUS, {OWN_DIRS:?} here, or {INSTALLED_DIRS:?})"
        );
        return;
    }
    let mut unchanged = 0usize;
    let mut skipped = 0usize;
    let mut failures = Vec::new();
    for path in &files {
        match check(path) {
            Verdict::Unchanged => unchanged += 1,
            Verdict::NotTypeScript => skipped += 1,
            Verdict::Broken(story) => failures.push(story),
        }
    }
    // What was actually measured, always — a differential that silently
    // skipped everything would look exactly like one that passed.
    println!(
        "corpus: {unchanged} unchanged, {skipped} not valid TypeScript, \
         {} broken, of {} files",
        failures.len(),
        files.len(),
    );
    assert!(
        unchanged > 0,
        "every file in the corpus was skipped — the corpus is not TypeScript"
    );
    assert!(
        failures.is_empty(),
        "{} of {unchanged} valid TypeScript files did not come back unchanged:\n\n{}",
        failures.len(),
        failures.join("\n\n"),
    );
}

const CASES_SAMPLE: usize = 400;
const CASES_SEED: u64 = 0x7473_6361_7365_7321;
const CASE_SUITES: [&str; 2] = ["compiler", "conformance"];
const ORACLE_BATCH: usize = 4000;

struct TestCase {
    key: String,
    path: PathBuf,
}

fn test_cases(manifest: &serde_json::Value, checkout: &Path) -> Vec<TestCase> {
    let mut out = Vec::new();
    for suite in CASE_SUITES {
        let root = checkout.join(tree(manifest, &format!("/{suite}")));
        assert!(root.is_dir(), "{} is not a directory", root.display());
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("a readable case directory") {
                let path = entry.expect("a readable case entry").path();
                if path.is_dir() {
                    stack.push(path);
                } else if matches!(
                    path.extension().and_then(|e| e.to_str()),
                    Some("ts" | "tsx")
                ) {
                    let relative = path
                        .strip_prefix(&root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    out.push(TestCase {
                        key: format!("{suite}/{relative}"),
                        path,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| a.key.cmp(&b.key));
    out
}

fn case_selection(all: Vec<TestCase>) -> (Vec<TestCase>, bool, String) {
    let requested = std::env::var("TTC_TYPESCRIPT_CASES").unwrap_or_default();
    let total = all.len();
    if requested == "all" {
        return (all, true, format!("all {total} cases"));
    }
    let count = requested.parse().unwrap_or(CASES_SAMPLE).min(total);
    let seed = std::env::var("TTC_TYPESCRIPT_CASES_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(CASES_SEED);
    let chosen = seeded_choice(total, count, seed);
    let picked = all
        .into_iter()
        .enumerate()
        .filter(|(index, _)| chosen.contains(index))
        .map(|(_, case)| case)
        .collect();
    (
        picked,
        false,
        format!(
            "{count} of {total} cases, seed {seed:#x} (TTC_TYPESCRIPT_CASES=all for every one)"
        ),
    )
}

#[derive(Clone)]
struct TsUnit {
    key: String,
    case: usize,
    name: String,
    text: String,
}

fn option_line(line: &str) -> Option<(String, &str)> {
    let rest = line.strip_prefix("//")?.trim_start().strip_prefix('@')?;
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    let (name, tail) = rest.split_at(end);
    let value = tail.trim_start().strip_prefix(':')?;
    Some((name.to_ascii_lowercase(), value.trim()))
}

fn link_line(line: &str) -> bool {
    option_line(line).is_some_and(|(name, value)| name == "link" && value.contains("->"))
}

fn split_units(code: &str, file_name: &str) -> (Vec<(String, String)>, bool) {
    let mut units: Vec<(String, String)> = Vec::new();
    let mut name: Option<String> = None;
    let mut content = String::new();
    for line in code.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if link_line(line) {
            continue;
        }
        if let Some((option, value)) = option_line(line) {
            if option == "filename" {
                if let Some(previous) = name.take() {
                    units.push((previous, std::mem::take(&mut content)));
                } else {
                    content.clear();
                }
                name = Some(value.to_string());
            }
            continue;
        }
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(line);
    }
    let single = units.is_empty() && name.is_none();
    units.push((name.unwrap_or_else(|| file_name.to_string()), content));
    (units, single)
}

fn units_of(case: &TestCase, index: usize, code: &str) -> Vec<TsUnit> {
    let file_name = case.path.file_name().unwrap().to_string_lossy();
    let (units, single) = split_units(code, &file_name);
    units
        .into_iter()
        .filter(|(name, _)| is_typescript_unit(name))
        .map(|(name, text)| TsUnit {
            case: index,
            key: if single {
                case.key.clone()
            } else {
                format!("{}#{name}", case.key)
            },
            name,
            text,
        })
        .collect()
}

fn is_typescript_unit(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".ts", ".tsx", ".mts", ".cts"]
        .iter()
        .any(|extension| lower.ends_with(extension))
}

fn unit_kind(name: &str) -> SourceKind {
    if name.to_ascii_lowercase().ends_with(".tsx") {
        SourceKind::Tsx
    } else {
        SourceKind::TypeScript
    }
}

fn unit_file(index: usize, name: &str) -> String {
    match unit_kind(name) {
        SourceKind::Tsx => format!("u{index}.tsx"),
        SourceKind::TypeScript => format!("u{index}.ts"),
    }
}

fn typescript_accepts(units: &[TsUnit]) -> Vec<bool> {
    let workspace = common::Workspace::new("typescript-cases");
    let dir = workspace.path().to_path_buf();
    let mut valid = vec![true; units.len()];
    for (batch, chunk) in units.chunks(ORACLE_BATCH).enumerate() {
        let base = batch * ORACLE_BATCH;
        let mut files = Vec::new();
        let mut owner: BTreeMap<String, usize> = BTreeMap::new();
        for (offset, unit) in chunk.iter().enumerate() {
            let relative = unit_file(base + offset, &unit.name);
            std::fs::write(dir.join(&relative), &unit.text).expect("a writable unit");
            owner.insert(relative.clone(), base + offset);
            files.push(relative);
        }
        for check in [false, true] {
            let checked: Vec<&String> = files
                .iter()
                .filter(|file| valid[owner[file.as_str()]])
                .collect();
            let config = serde_json::json!({
                "compilerOptions": {
                    "noCheck": !check,
                    "noEmit": true,
                    "noResolve": true,
                    "types": [],
                    "jsx": "preserve",
                    "skipLibCheck": true,
                },
                "files": checked,
            });
            let tsconfig = format!("tsconfig.{batch}.json");
            std::fs::write(dir.join(&tsconfig), config.to_string()).expect("a writable tsconfig");
            let output = common::tsc()
                .args(["-p", &tsconfig, "--pretty", "false"])
                .current_dir(&dir)
                .output()
                .expect("the pinned tsc runs");
            assert!(
                matches!(output.status.code(), Some(0..=2)),
                "the pinned tsc failed on {tsconfig}: {}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let Some(at) = line.find("): error TS") else {
                    continue;
                };
                let code: u32 = line[at + "): error TS".len()..]
                    .split(':')
                    .next()
                    .and_then(|code| code.parse().ok())
                    .unwrap_or(0);
                if check && !(1000..2000).contains(&code) {
                    continue;
                }
                let Some(open) = line[..at].rfind('(') else {
                    continue;
                };
                let file = line[..open].replace('\\', "/");
                if let Some(index) = owner.get(&file) {
                    valid[*index] = false;
                }
            }
        }
        for file in &files {
            let _ = std::fs::remove_file(dir.join(file));
        }
    }
    valid
}

fn passthrough_verdict(unit: &TsUnit) -> Option<(String, String)> {
    let kind = unit_kind(&unit.name);
    let options = Options {
        source_kind: kind,
        rewrite_imports: ttc::ImportRewrite::Off,
        defer_to_checker: true,
        ..Options::default()
    };
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let report = ttc::compile_report(&unit.text, &options);
        let projected = ttc::emit_mapped_with_kind(&unit.text, kind);
        (report, projected)
    }));
    let (report, projected) = match caught {
        Ok(answers) => answers,
        Err(payload) => {
            let message = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            let first = message.lines().next().unwrap_or("").to_string();
            return Some(("crashed".to_string(), first));
        }
    };
    if !report.diagnostics.is_empty() {
        let codes: BTreeSet<&str> = report.diagnostics.iter().map(|d| d.code.as_str()).collect();
        let detail = report
            .diagnostics
            .iter()
            .map(|d| {
                let (line, col) = line_col(&unit.text, d.start.unwrap_or(0));
                format!("{line}:{col} {}: {}", d.code.as_str(), d.message)
            })
            .collect::<Vec<_>>()
            .join("\n    ");
        return Some((
            format!(
                "rejected {}",
                codes.into_iter().collect::<Vec<_>>().join(",")
            ),
            detail,
        ));
    }
    let emitted = report.emit.map(|emit| emit.code).unwrap_or_default();
    if emitted != unit.text {
        return Some(("changed".to_string(), difference(&unit.text, &emitted)));
    }
    if projected.code != unit.text {
        return Some((
            "projection changed".to_string(),
            difference(&unit.text, &projected.code),
        ));
    }
    None
}

fn difference(source: &str, out: &str) -> String {
    let at = out
        .bytes()
        .zip(source.bytes())
        .position(|(a, b)| a != b)
        .unwrap_or(source.len().min(out.len()));
    let (line, col) = line_col(source, at);
    format!(
        "differs at {line}:{col}\n    in:  {:?}\n    out: {:?}",
        window(source, at),
        window(out, at)
    )
}

struct Listed {
    observed: String,
    note: String,
}

fn listed(name: &str, note_is_task: bool) -> BTreeMap<String, Listed> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("tests/{name}"));
    let mut out = BTreeMap::new();
    for line in text.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.splitn(3, '\t').collect();
        let [key, observed, note] = fields[..] else {
            panic!("tests/{name}: `{line}` is not `<case>\\t<observed>\\t<note>`");
        };
        assert!(
            !note.trim().is_empty(),
            "tests/{name}: `{key}` gives no reason"
        );
        assert!(
            !note_is_task || note.starts_with("TASK-"),
            "tests/{name}: `{key}` names no TASK-NNN"
        );
        let previous = out.insert(
            key.to_string(),
            Listed {
                observed: observed.to_string(),
                note: note.to_string(),
            },
        );
        assert!(previous.is_none(), "tests/{name}: `{key}` is listed twice");
    }
    out
}

#[test]
fn typescript_test_cases_come_back_unchanged() {
    let manifest = manifest();
    let accepted = listed("passthrough-accepted.txt", false);
    let triaged = listed("passthrough-triaged.txt", true);
    let both: Vec<&String> = accepted
        .keys()
        .filter(|key| triaged.contains_key(*key))
        .collect();
    assert!(
        both.is_empty(),
        "listed in both tests/passthrough-accepted.txt and tests/passthrough-triaged.txt: {both:?}"
    );
    let pinned: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("package.json"))
            .expect("package.json"),
    )
    .expect("package.json is JSON");
    assert_eq!(
        manifest["typescript"], pinned["devDependencies"]["typescript"],
        "tests/typescript-cases.json pins the cases of another TypeScript than package.json; \
         update its commit to the new package's gitHead and its tree ids"
    );
    let Some(checkout) = cases_checkout(&manifest) else {
        assert!(
            !cases_required(),
            "TTC_REQUIRE_TYPESCRIPT_CASES is set but TypeScript's test cases are not \
             fetched — run scripts/fetch-typescript-cases"
        );
        eprintln!("SKIP TypeScript's test cases: not fetched (scripts/fetch-typescript-cases)");
        return;
    };
    if !common::tsc_available() {
        assert!(
            !cases_required(),
            "TTC_REQUIRE_TYPESCRIPT_CASES is set but the pinned tsc cannot run — run `npm ci`"
        );
        eprintln!("SKIP TypeScript's test cases: no pinned tsc to say which units parse");
        return;
    }
    if let Some(installed) = common::typescript() {
        let package: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(installed.join("package.json")).expect("package.json"),
        )
        .expect("an installed package.json is JSON");
        if let Some(head) = package["gitHead"].as_str() {
            assert_eq!(
                Some(head),
                manifest["commit"].as_str(),
                "the installed TypeScript was built from {head}, not the commit \
                 tests/typescript-cases.json pins"
            );
        }
    }

    let (cases, full, description) = case_selection(test_cases(&manifest, &checkout));
    let mut units = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let bytes = std::fs::read(&case.path).expect("a readable case");
        let Ok(code) = String::from_utf8(bytes) else {
            continue;
        };
        units.extend(units_of(case, index, &code));
    }
    let mut valid = typescript_accepts(&units);
    let parsed: Vec<&TsUnit> = units
        .iter()
        .zip(&valid)
        .filter(|(_, valid)| **valid)
        .map(|(unit, _)| unit)
        .collect();

    let next = AtomicUsize::new(0);
    let verdicts: Mutex<BTreeMap<String, (String, String)>> = Mutex::new(BTreeMap::new());
    let workers = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(4);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some(unit) = parsed.get(index) else {
                        break;
                    };
                    if let Some(verdict) = passthrough_verdict(unit) {
                        verdicts.lock().unwrap().insert(unit.key.clone(), verdict);
                    }
                }
            });
        }
    });
    let mut verdicts = verdicts.into_inner().unwrap();

    let listed_key = |key: &str| accepted.contains_key(key) || triaged.contains_key(key);
    let recheck: BTreeSet<usize> = units
        .iter()
        .zip(&valid)
        .filter(|(unit, valid)| {
            verdicts.contains_key(&unit.key) || (!**valid && listed_key(&unit.key))
        })
        .map(|(unit, _)| unit.case)
        .collect();
    for case in recheck {
        let members: Vec<usize> = (0..units.len())
            .filter(|i| units[*i].case == case)
            .collect();
        let alone: Vec<TsUnit> = members.iter().map(|i| units[*i].clone()).collect();
        for (index, accepted_alone) in members.into_iter().zip(typescript_accepts(&alone)) {
            let unit = &units[index];
            if !accepted_alone {
                verdicts.remove(&unit.key);
            } else if !valid[index]
                && let Some(verdict) = passthrough_verdict(unit)
            {
                verdicts.insert(unit.key.clone(), verdict);
            }
            valid[index] = accepted_alone;
        }
    }
    let parsed: Vec<&TsUnit> = units
        .iter()
        .zip(&valid)
        .filter(|(_, valid)| **valid)
        .map(|(unit, _)| unit)
        .collect();

    let ran: BTreeSet<&str> = parsed.iter().map(|unit| unit.key.as_str()).collect();
    let mut problems = Vec::new();
    for (key, (observed, detail)) in &verdicts {
        let entry = accepted.get(key).or_else(|| triaged.get(key));
        match entry {
            None => problems.push(format!(
                "{key}: {observed}\n    {detail}\n  valid TypeScript that ttc does not pass through: fix it, or list it in \
                 tests/passthrough-triaged.txt (a bug, with its TASK) or tests/passthrough-accepted.txt (intended, with the reason)"
            )),
            Some(listed) if listed.observed != *observed => problems.push(format!(
                "{key}: now {observed}, listed as {}\n    {detail}",
                listed.observed
            )),
            Some(_) => {}
        }
    }
    for (file, entries) in [
        ("passthrough-accepted.txt", &accepted),
        ("passthrough-triaged.txt", &triaged),
    ] {
        for (key, entry) in entries.iter() {
            if verdicts.contains_key(key) {
                continue;
            }
            if ran.contains(key.as_str()) {
                problems.push(format!(
                    "{key} now comes back unchanged: remove it from tests/{file} ({})",
                    entry.note
                ));
            } else if full {
                problems.push(format!(
                    "tests/{file} lists {key}, which is not a TypeScript-valid unit of the pinned cases"
                ));
            }
        }
    }
    println!(
        "typescript cases: {description}; {} TypeScript unit(s), {} parse, {} differ ({} listed)",
        units.len(),
        parsed.len(),
        verdicts.len(),
        verdicts
            .keys()
            .filter(|key| accepted.contains_key(*key) || triaged.contains_key(*key))
            .count()
    );
    assert!(
        !parsed.is_empty(),
        "no unit of the selected cases parses as TypeScript"
    );
    assert!(problems.is_empty(), "{}", problems.join("\n\n"));
}

const TYPED_SAMPLE: usize = 80;
const TYPED_SEED: u64 = 0x7474_7479_7065_6421;
const TYPED_DIFFERENCES: &str = "typed-parity-differences.txt";
const TYPED_WORKERS: usize = 4;
const ORACLE_AUDIT: usize = 40;
const REMOVED_OPTION: [u64; 2] = [5102, 5108];
const HARNESS_ONLY: [&str; 8] = [
    "notypesandsymbols",
    "fullemitpaths",
    "reportdiagnostics",
    "capturesuggestions",
    "typescriptversion",
    "baselinefile",
    "noimplicitreferences",
    "filename",
];
const HARNESS_LAYOUT: [&str; 6] = [
    "currentdirectory",
    "symlink",
    "link",
    "libfiles",
    "includebuiltfile",
    "usecasesensitivefilenames",
];

fn typed_selection(all: Vec<TestCase>) -> (Vec<TestCase>, bool, String) {
    let requested = std::env::var("TTC_TYPED_CASES").unwrap_or_default();
    let total = all.len();
    if let Some(filter) = std::env::var("TTC_TYPED_FILTER")
        .ok()
        .filter(|f| !f.is_empty())
    {
        let patterns: Vec<&str> = filter.split(',').collect();
        let picked: Vec<TestCase> = all
            .into_iter()
            .filter(|case| patterns.iter().any(|p| case.key.contains(p)))
            .collect();
        let description = format!(
            "{} of {total} cases matching TTC_TYPED_FILTER",
            picked.len()
        );
        return (picked, false, description);
    }
    if requested == "all" {
        let shard = std::env::var("TTC_TYPED_SHARD").ok().and_then(|shard| {
            let (index, count) = shard.split_once('/')?;
            Some((index.parse::<usize>().ok()?, count.parse::<usize>().ok()?))
        });
        if let Some((index, count)) = shard.filter(|(index, count)| index < count) {
            let picked: Vec<TestCase> = all
                .into_iter()
                .enumerate()
                .filter(|(position, _)| position % count == index)
                .map(|(_, case)| case)
                .collect();
            let description = format!("shard {index}/{count} of all {total} cases");
            return (picked, false, description);
        }
        return (all, true, format!("all {total} cases"));
    }
    let count = requested.parse().unwrap_or(TYPED_SAMPLE).min(total);
    let seed = std::env::var("TTC_TYPED_CASES_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(TYPED_SEED);
    let chosen = seeded_choice(total, count, seed);
    let picked = all
        .into_iter()
        .enumerate()
        .filter(|(index, _)| chosen.contains(index))
        .map(|(_, case)| case)
        .collect();
    (
        picked,
        false,
        format!("{count} of {total} cases, seed {seed:#x} (TTC_TYPED_CASES=all for every one)"),
    )
}

struct TtServer {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: std::io::BufReader<std::process::ChildStdout>,
}

impl TtServer {
    fn start(dir: &Path) -> TtServer {
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_ttc"))
            .arg("--server")
            .current_dir(dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("ttc --server starts");
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = std::io::BufReader::new(child.stdout.take().expect("piped stdout"));
        TtServer {
            child,
            stdin,
            stdout,
        }
    }

    fn ask(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
        use std::io::{BufRead, Write};
        let line = serde_json::json!({ "id": 1, "method": method, "params": params });
        writeln!(self.stdin, "{line}").expect("the server reads");
        self.stdin.flush().expect("the server reads");
        let mut answer = String::new();
        self.stdout
            .read_line(&mut answer)
            .expect("the server answers");
        serde_json::from_str(&answer)
            .unwrap_or_else(|e| panic!("the server answered no JSON ({e}): {answer}"))
    }
}

impl Drop for TtServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn settings(code: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in code.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if let Some((name, value)) = option_line(line) {
            out.insert(name, value.trim_end_matches(';').trim().to_string());
        }
    }
    out
}

fn mapped_path(name: &str) -> Option<String> {
    if name.contains(':') {
        return None;
    }
    let relative = name.trim_start_matches('/');
    let mut parts = Vec::new();
    for part in relative.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

fn is_renameable(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    (lower.ends_with(".ts") || lower.ends_with(".tsx"))
        && !lower.ends_with(".d.ts")
        && !lower.contains(".d.")
        && !lower.split('/').any(|part| part == "node_modules")
}

fn tt_name(name: &str) -> String {
    if let Some(stem) = name.strip_suffix(".tsx") {
        format!("{stem}.ttx")
    } else if let Some(stem) = name.strip_suffix(".ts") {
        format!("{stem}.tt")
    } else {
        name.to_string()
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Reported {
    file: String,
    range: String,
    code: String,
    message: String,
}

impl Reported {
    fn line(&self) -> String {
        format!(
            "{} {} {}: {}",
            self.file,
            self.range,
            self.code,
            self.message.replace('\n', "\\n")
        )
    }
}

enum TypedOutcome {
    Skipped(String),
    Compared {
        signature: Option<String>,
        detail: String,
        took: std::time::Duration,
    },
}

fn names(list: &serde_json::Value) -> Option<String> {
    let list = list.as_array().filter(|list| !list.is_empty())?;
    Some(
        list.iter()
            .map(|entry| match entry {
                serde_json::Value::String(name) => name.clone(),
                other => other[0].as_str().unwrap_or_default().to_string(),
            })
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn utf16_to_points(text: &str, line: usize, col: usize) -> usize {
    let row = text
        .split('\n')
        .nth(line.saturating_sub(1))
        .unwrap_or_default();
    let mut units = 1usize;
    let mut points = 1usize;
    for ch in row.chars() {
        if units >= col {
            break;
        }
        units += ch.len_utf16();
        points += 1;
    }
    points
}

fn typed_parity(case: &TestCase, dir: &Path, oracle: &mut Oracle, audit: bool) -> TypedOutcome {
    let Ok(code) = std::fs::read_to_string(&case.path) else {
        return TypedOutcome::Skipped("not UTF-8".to_string());
    };
    let file_name = case.path.file_name().unwrap().to_string_lossy();
    let (raw_units, _) = split_units(&code, &file_name);
    let raw_settings = settings(&code);
    for name in HARNESS_LAYOUT {
        if raw_settings.contains_key(name) {
            return TypedOutcome::Skipped(format!("harness file-system option @{name}"));
        }
    }
    let mut units: Vec<(String, String)> = Vec::new();
    for (name, text) in &raw_units {
        let lower = name.to_ascii_lowercase();
        if lower.ends_with("tsconfig.json") || lower.ends_with("jsconfig.json") {
            return TypedOutcome::Skipped("the case brings its own tsconfig.json".to_string());
        }
        let Some(mapped) = mapped_path(name) else {
            return TypedOutcome::Skipped("a unit outside the case's root".to_string());
        };
        if units
            .iter()
            .any(|(other, _)| other.eq_ignore_ascii_case(&mapped))
        {
            return TypedOutcome::Skipped("two units at one path".to_string());
        }
        units.push((mapped, text.clone()));
    }
    if !units.iter().any(|(name, _)| is_renameable(name)) {
        return TypedOutcome::Skipped("no .ts/.tsx unit to rename".to_string());
    }
    let entries: Vec<serde_json::Value> = raw_settings
        .iter()
        .filter(|(name, _)| !HARNESS_ONLY.contains(&name.as_str()))
        .map(|(name, value)| serde_json::json!([name, value]))
        .collect();
    let converted = oracle.ask(&serde_json::json!({ "options": entries }));
    if let Some(unknown) = names(&converted["unknown"]) {
        return TypedOutcome::Skipped(format!(
            "an option the pinned TypeScript does not know: {unknown}"
        ));
    }
    if let Some(varies) = names(&converted["varies"]) {
        return TypedOutcome::Skipped(format!(
            "`*` or an exclusion in an option's values: {varies}"
        ));
    }
    if let Some(invalid) = names(&converted["invalid"]) {
        return TypedOutcome::Skipped(format!(
            "an option value the pinned TypeScript rejects: {invalid}"
        ));
    }
    let configurations = converted["configurations"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let last = &raw_units.last().expect("a unit").1;
    let implicit = raw_settings.contains_key("noimplicitreferences")
        || last.contains("require(")
        || last.lines().any(|line| {
            let line = line.trim_start();
            line.starts_with("///") && line.contains("<reference") && line.contains("path")
        });
    let program_file = |name: &&String| {
        let lower = name.to_ascii_lowercase();
        !lower.ends_with(".json") && !lower.ends_with(".tsbuildinfo")
    };
    let roots: Vec<String> = if implicit {
        vec![units.last().expect("a unit").0.clone()]
    } else {
        units
            .iter()
            .map(|(name, _)| name)
            .filter(program_file)
            .cloned()
            .collect()
    };

    let write =
        |root: &Path, compiler_options: &serde_json::Value, rename: &dyn Fn(&str) -> String| {
            for (name, text) in &units {
                let path = root.join(rename(name));
                std::fs::create_dir_all(path.parent().unwrap()).expect("a writable case directory");
                std::fs::write(&path, text).expect("a writable unit");
            }
            let config = serde_json::json!({
                "compilerOptions": compiler_options,
                "files": roots.iter().map(|name| rename(name)).collect::<Vec<_>>(),
            });
            std::fs::write(root.join("tsconfig.json"), config.to_string())
                .expect("a writable tsconfig");
        };
    let twin = dir.join("ts");
    let twin_units: Vec<String> = units
        .iter()
        .map(|(name, _)| twin.join(name).to_string_lossy().into_owned())
        .collect();
    let started = std::time::Instant::now();
    let mut chosen = None;
    for configuration in &configurations {
        let mut compiler_options = configuration.clone();
        compiler_options["noEmit"] = serde_json::json!(true);
        write(&twin, &compiler_options, &|name| name.to_string());
        let answer = oracle.ask(&serde_json::json!({
            "check": twin.join("tsconfig.json").to_string_lossy(),
            "units": twin_units,
        }));
        let removed = answer["diagnostics"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|d| REMOVED_OPTION.contains(&d["code"].as_u64().unwrap_or(0)));
        if !removed {
            chosen = Some((compiler_options, answer));
            break;
        }
    }
    let Some((compiler_options, answer)) = chosen else {
        return TypedOutcome::Skipped(
            "every configuration sets an option the pinned TypeScript removed (TS5102, TS5108)"
                .to_string(),
        );
    };
    let diagnostics = answer["diagnostics"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if diagnostics.iter().any(|d| {
        d["code"]
            .as_u64()
            .is_some_and(|code| (1000..2000).contains(&code))
            || d["syntactic"].as_bool() == Some(true)
    }) {
        return TypedOutcome::Skipped(
            "TypeScript reports a syntax or grammar error (TS1xxx)".to_string(),
        );
    }
    let reached: BTreeSet<String> = answer["reached"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|path| path.as_str())
        .filter_map(|path| Path::new(path).strip_prefix(&twin).ok())
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .collect();
    let mut reached = reached;
    let candidates: Vec<String> = roots
        .iter()
        .filter(|name| is_renameable(name) && !reached.contains(*name))
        .cloned()
        .collect();
    for candidate in &candidates {
        let others: Vec<&String> = roots.iter().filter(|name| *name != candidate).collect();
        if others.is_empty() {
            continue;
        }
        let probe = twin.join("tsconfig.reach.json");
        std::fs::write(
            &probe,
            serde_json::json!({ "compilerOptions": compiler_options, "files": others }).to_string(),
        )
        .expect("a writable tsconfig");
        let program = oracle.ask(&serde_json::json!({ "files": probe.to_string_lossy() }));
        let _ = std::fs::remove_file(&probe);
        let target = twin.join(candidate).to_string_lossy().into_owned();
        if program["files"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|file| file.as_str() == Some(target.as_str()))
        {
            reached.insert(candidate.clone());
        }
    }
    let renamed: BTreeSet<String> = units
        .iter()
        .map(|(name, _)| name)
        .filter(|name| is_renameable(name) && roots.contains(*name) && !reached.contains(*name))
        .cloned()
        .collect();
    if renamed.is_empty() {
        return TypedOutcome::Skipped(
            "every .ts/.tsx root file is imported by another unit".to_string(),
        );
    }
    for (name, text) in units.iter().filter(|(name, _)| renamed.contains(name)) {
        let unit = TsUnit {
            key: name.clone(),
            case: 0,
            name: name.clone(),
            text: text.clone(),
        };
        if passthrough_verdict(&unit).is_some() {
            return TypedOutcome::Skipped(
                "a unit does not pass through (tests/passthrough-*.txt)".to_string(),
            );
        }
    }
    let rename = |name: &str| {
        if renamed.contains(name) {
            tt_name(name)
        } else {
            name.to_string()
        }
    };
    let tt = dir.join("tt");
    write(&tt, &compiler_options, &rename);

    let ts_dir = twin.to_string_lossy().into_owned();
    let tt_dir = tt.to_string_lossy().into_owned();
    let back: BTreeMap<String, String> = renamed
        .iter()
        .map(|name| (tt_name(name), name.clone()))
        .collect();
    let file_of = |path: &str, root: &str| -> String {
        match path.strip_prefix(root) {
            Some(rest) => {
                let name = rest.trim_start_matches('/').to_string();
                back.get(&name).cloned().unwrap_or(name)
            }
            None => path.to_string(),
        }
    };
    let message_of = |message: &str, root: &str| -> String {
        let mut text = message.to_string();
        for (tt_file, ts_file) in &back {
            text = text.replace(&format!("{root}/{tt_file}"), &format!("$DIR/{ts_file}"));
        }
        text.replace(root, "$DIR")
    };
    let mut expected: Vec<Reported> = diagnostics
        .iter()
        .map(|d| Reported {
            file: d["file"]
                .as_str()
                .map_or("-".to_string(), |path| file_of(path, &ts_dir)),
            range: match (d["start"].as_object(), d["end"].as_object()) {
                (Some(start), Some(end)) => format!(
                    "{}:{}-{}:{}",
                    start["line"].as_u64().unwrap_or(0) + 1,
                    start["character"].as_u64().unwrap_or(0) + 1,
                    end["line"].as_u64().unwrap_or(0) + 1,
                    end["character"].as_u64().unwrap_or(0) + 1
                ),
                _ => "-".to_string(),
            },
            code: format!("TS{}", d["code"]),
            message: message_of(d["message"].as_str().unwrap_or_default(), &ts_dir),
        })
        .collect();
    expected.sort();

    let first = renamed.iter().next().expect("a renamed unit");
    let first_text = units
        .iter()
        .find(|(name, _)| name == first)
        .map(|(_, text)| text.clone())
        .unwrap_or_default();
    let mut server = TtServer::start(&tt);
    let checked = server.ask(
        "typedCheck",
        serde_json::json!({
            "path": tt.join(tt_name(first)).to_string_lossy(),
            "text": first_text,
            "includeTypes": true,
        }),
    );
    drop(server);
    let mut notes = Vec::new();
    if let Some(error) = checked.get("error").filter(|e| !e.is_null()) {
        notes.push(format!("server error: {error}"));
    }
    let result = &checked["result"];
    if result["blocked"].as_bool() == Some(true) {
        notes.push("blocked: the typed check did not run".to_string());
    }
    if let Some(error) = result.get("backendError").filter(|e| !e.is_null()) {
        notes.push(format!("backend error: {}", error["message"]));
    }
    let server_list: Vec<serde_json::Value> = result["diagnostics"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut actual: Vec<Reported> = server_list
        .iter()
        .map(|d| {
            let line = d["line"].as_u64().unwrap_or(0);
            let path = d["path"].as_str().unwrap_or_default();
            Reported {
                file: if path.is_empty() {
                    "-".to_string()
                } else {
                    file_of(path, &tt_dir)
                },
                range: if line == 0 {
                    "-".to_string()
                } else {
                    format!(
                        "{}:{}-{}:{}",
                        line,
                        d["col"].as_u64().unwrap_or(0),
                        d["endLine"].as_u64().unwrap_or(0),
                        d["endCol"].as_u64().unwrap_or(0)
                    )
                },
                code: d["code"]
                    .as_str()
                    .map(|code| match code.strip_prefix("ts") {
                        Some(number) if number.bytes().all(|b| b.is_ascii_digit()) => {
                            format!("TS{number}")
                        }
                        _ => code.to_string(),
                    })
                    .unwrap_or_else(|| "-".to_string()),
                message: message_of(d["message"].as_str().unwrap_or_default(), &tt_dir),
            }
        })
        .collect();
    actual.sort();

    let cli = std::process::Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["--check-types", "--project", "tsconfig.json", "."])
        .current_dir(&tt)
        .env("NO_COLOR", "1")
        .output()
        .expect("ttc --check-types runs");
    let cli_reports = cli_headers(&String::from_utf8_lossy(&cli.stderr));
    let mut server_reports: Vec<(String, String, String)> = server_list
        .iter()
        .map(|d| {
            let path = d["path"].as_str().unwrap_or_default();
            let line = d["line"].as_u64().unwrap_or(0) as usize;
            let place = if line == 0 {
                path.strip_prefix(&tt_dir)
                    .map(|rest| rest.trim_start_matches('/').to_string())
                    .unwrap_or_else(|| "-".to_string())
            } else {
                let text = std::fs::read_to_string(path).unwrap_or_default();
                let col = utf16_to_points(&text, line, d["col"].as_u64().unwrap_or(0) as usize);
                let shown = path
                    .strip_prefix(&tt_dir)
                    .map(|rest| rest.trim_start_matches('/'))
                    .unwrap_or(path);
                format!("{shown}:{line}:{col}")
            };
            (
                d["code"].as_str().unwrap_or_default().to_string(),
                d["message"]
                    .as_str()
                    .unwrap_or_default()
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string(),
                place,
            )
        })
        .collect();
    server_reports.sort();
    if cli_reports != server_reports {
        notes.push(format!(
            "transport: ttc --check-types and the server's typedCheck disagree\n      cli:    {cli_reports:?}\n      server: {server_reports:?}"
        ));
    }
    if audit {
        let tsc = common::tsc()
            .args(["-p", "tsconfig.json", "--pretty", "false"])
            .current_dir(&twin)
            .output()
            .expect("the pinned tsc runs");
        let mut printed: Vec<String> = String::from_utf8_lossy(&tsc.stdout)
            .lines()
            .filter(|line| line.contains("error TS"))
            .map(str::to_string)
            .collect();
        printed.sort();
        let mut answered: Vec<String> = expected
            .iter()
            .map(|d| {
                let head = d.message.lines().next().unwrap_or_default();
                let head = head.replace("$DIR", &ts_dir);
                match d.range.split_once('-') {
                    Some((start, _)) if d.file != "-" => {
                        let (line, col) = start.split_once(':').unwrap_or(("0", "0"));
                        format!("{}({line},{col}): error {}: {head}", d.file, d.code)
                    }
                    _ => format!("error {}: {head}", d.code),
                }
            })
            .collect();
        answered.sort();
        assert_eq!(
            printed, answered,
            "{}: the oracle's diagnostics are not the ones `tsc -p` prints",
            case.key
        );
    }

    let took = started.elapsed();
    let unplaced = |list: Vec<Reported>| -> Vec<Reported> {
        let mut list: Vec<Reported> = list
            .into_iter()
            .map(|mut reported| {
                if reported.range == "-" {
                    reported.file = "-".to_string();
                }
                reported
            })
            .collect();
        list.sort();
        list
    };
    let expected = unplaced(expected);
    let actual = unplaced(actual);
    let mut remaining: Vec<Reported> = expected.clone();
    let mut tt_only = Vec::new();
    for reported in &actual {
        match remaining.iter().position(|other| other == reported) {
            Some(at) => {
                remaining.remove(at);
            }
            None => tt_only.push(reported.clone()),
        }
    }
    let ts_only = remaining;
    if tt_only.is_empty() && ts_only.is_empty() && notes.is_empty() {
        return TypedOutcome::Compared {
            signature: None,
            detail: String::new(),
            took,
        };
    }
    let mut tokens: BTreeSet<String> = BTreeSet::new();
    for reported in &tt_only {
        let candidates: Vec<&Reported> = ts_only
            .iter()
            .filter(|other| counterpart(other, reported))
            .collect();
        let same_place = candidates
            .iter()
            .any(|other| other.range == reported.range && other.file == reported.file);
        let same_words = candidates
            .iter()
            .any(|other| other.message == reported.message);
        tokens.insert(match (candidates.is_empty(), same_place, same_words) {
            (true, _, _) => format!("+{}", reported.code),
            (false, true, _) => format!("~{}@message", reported.code),
            (false, false, true) => format!("~{}@range", reported.code),
            (false, false, false) => format!("~{}", reported.code),
        });
    }
    for reported in &ts_only {
        let counterpart = tt_only.iter().any(|other| counterpart(other, reported));
        if !counterpart {
            tokens.insert(format!("-{}", reported.code));
        }
    }
    for note in &notes {
        tokens.insert(note.split(':').next().unwrap_or(note).to_string());
    }
    let mut detail = String::new();
    for reported in &tt_only {
        detail.push_str(&format!("    tt only: {}\n", reported.line()));
    }
    for reported in &ts_only {
        detail.push_str(&format!("    ts only: {}\n", reported.line()));
    }
    for note in &notes {
        detail.push_str(&format!("    {note}\n"));
    }
    TypedOutcome::Compared {
        signature: Some(tokens.into_iter().collect::<Vec<_>>().join(" ")),
        detail,
        took,
    }
}

fn counterpart(one: &Reported, other: &Reported) -> bool {
    one.code == other.code && (one.file == other.file || one.range == "-" || other.range == "-")
}

fn cli_headers(stderr: &str) -> Vec<(String, String, String)> {
    let lines: Vec<&str> = stderr.lines().collect();
    let mut out = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some((code, message)) = line
            .strip_prefix("error[")
            .and_then(|rest| rest.split_once("]: "))
        else {
            continue;
        };
        let place = lines[index + 1..]
            .iter()
            .take_while(|next| !next.starts_with("error"))
            .find_map(|next| next.trim_start().strip_prefix("--> "))
            .map(|at| at.strip_prefix("./").unwrap_or(at).to_string())
            .unwrap_or_else(|| "-".to_string());
        out.push((code.to_string(), message.to_string(), place));
    }
    out.sort();
    out
}

struct TypedListed {
    observed: String,
    class: String,
}

fn typed_listed() -> BTreeMap<String, TypedListed> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(TYPED_DIFFERENCES);
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut out = BTreeMap::new();
    for line in text.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.splitn(4, '\t').collect();
        let [key, observed, class, reason] = fields[..] else {
            panic!(
                "tests/{TYPED_DIFFERENCES}: `{line}` is not `<case>\\t<observed>\\t<by-design|defect>\\t<reason>`"
            );
        };
        match class {
            "by-design" => assert!(
                reason.contains("docs/"),
                "tests/{TYPED_DIFFERENCES}: `{key}` is by design but cites no document under docs/"
            ),
            "defect" => assert!(
                reason.starts_with("TASK-"),
                "tests/{TYPED_DIFFERENCES}: `{key}` is a defect but names no TASK-NNN"
            ),
            other => panic!(
                "tests/{TYPED_DIFFERENCES}: `{key}` is `{other}`, neither by-design nor defect"
            ),
        }
        let previous = out.insert(
            key.to_string(),
            TypedListed {
                observed: observed.to_string(),
                class: class.to_string(),
            },
        );
        assert!(
            previous.is_none(),
            "tests/{TYPED_DIFFERENCES}: `{key}` is listed twice"
        );
    }
    out
}

#[test]
fn typescript_cases_type_check_as_typescript_does() {
    let manifest = manifest();
    let Some(checkout) = cases_checkout(&manifest) else {
        assert!(
            !cases_required(),
            "TTC_REQUIRE_TYPESCRIPT_CASES is set but TypeScript's test cases are not \
             fetched — run scripts/fetch-typescript-cases"
        );
        eprintln!("SKIP typed parity: TypeScript's test cases are not fetched");
        return;
    };
    if !common::toolchain() || !common::tsc_available() {
        eprintln!("SKIP typed parity: no pinned TypeScript — run `npm ci`");
        return;
    }
    let listed = typed_listed();
    let started = std::time::Instant::now();
    let (cases, full, description) = typed_selection(test_cases(&manifest, &checkout));
    let workspace = common::Workspace::in_repo("typed-parity");
    let next = AtomicUsize::new(0);
    let audits = AtomicUsize::new(0);
    let outcomes: Mutex<BTreeMap<String, TypedOutcome>> = Mutex::new(BTreeMap::new());
    std::thread::scope(|scope| {
        for _ in 0..TYPED_WORKERS {
            scope.spawn(|| {
                let mut oracle = Oracle::start();
                let mut served = 0usize;
                loop {
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some(case) = cases.get(index) else {
                        break;
                    };
                    if served == 200 {
                        oracle = Oracle::start();
                        served = 0;
                    }
                    served += 1;
                    let dir = workspace.path().join(format!("c{index}"));
                    let audit = audits.load(Ordering::SeqCst) < ORACLE_AUDIT;
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        typed_parity(case, &dir, &mut oracle, audit)
                    }));
                    if let Some(keep) = std::env::var_os("TTC_TYPED_KEEP") {
                        let _ = std::fs::rename(&dir, Path::new(&keep).join(format!("c{index}")));
                    }
                    let _ = std::fs::remove_dir_all(&dir);
                    let outcome = match outcome {
                        Ok(outcome) => outcome,
                        Err(payload) => {
                            oracle = Oracle::start();
                            served = 0;
                            let message = payload
                                .downcast_ref::<String>()
                                .cloned()
                                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                                .unwrap_or_default();
                            TypedOutcome::Compared {
                                signature: Some("crashed".to_string()),
                                detail: format!("    {message}\n"),
                                took: std::time::Duration::ZERO,
                            }
                        }
                    };
                    if audit && matches!(outcome, TypedOutcome::Compared { .. }) {
                        audits.fetch_add(1, Ordering::SeqCst);
                    }
                    outcomes.lock().unwrap().insert(case.key.clone(), outcome);
                }
            });
        }
    });
    let outcomes = outcomes.into_inner().unwrap();
    let mut skipped: BTreeMap<String, usize> = BTreeMap::new();
    let mut compared = 0usize;
    let mut differing = 0usize;
    let mut listed_differing = 0usize;
    let mut problems = Vec::new();
    let mut checked_time = std::time::Duration::ZERO;
    for (key, outcome) in &outcomes {
        match outcome {
            TypedOutcome::Skipped(reason) => {
                let class = reason.split(':').next().unwrap_or(reason).to_string();
                *skipped.entry(class).or_default() += 1;
                if listed.contains_key(key) {
                    problems.push(format!(
                        "tests/{TYPED_DIFFERENCES} lists {key}, which is not compared now: {reason}"
                    ));
                }
            }
            TypedOutcome::Compared {
                signature,
                detail,
                took,
            } => {
                compared += 1;
                checked_time += *took;
                match (signature, listed.get(key)) {
                    (None, None) => {}
                    (None, Some(entry)) => problems.push(format!(
                        "{key} now type-checks as TypeScript does: remove it from tests/{TYPED_DIFFERENCES} ({} {})",
                        entry.class, entry.observed
                    )),
                    (Some(signature), None) => {
                        differing += 1;
                        problems.push(format!(
                            "{key}: {signature}\n{detail}  ttc's typed check differs from the pinned tsc: fix it, or list it in \
                             tests/{TYPED_DIFFERENCES} as `{key}<TAB>{signature}<TAB>by-design|defect<TAB><docs/... or TASK-NNN> reason`"
                        ));
                    }
                    (Some(signature), Some(entry)) => {
                        differing += 1;
                        listed_differing += 1;
                        if *signature != entry.observed {
                            problems.push(format!(
                                "{key}: now {signature}, listed as {}\n{detail}",
                                entry.observed
                            ));
                        }
                    }
                }
            }
        }
    }
    if full {
        for key in listed.keys() {
            if !outcomes.contains_key(key) {
                problems.push(format!(
                    "tests/{TYPED_DIFFERENCES} lists {key}, which is not a case of the pinned corpus"
                ));
            }
        }
    }
    let skipped_total: usize = skipped.values().sum();
    println!(
        "typed parity: {description}; {compared} compared, {differing} differ ({listed_differing} listed), \
         {skipped_total} skipped, {:.1}s ({:.1}s comparing)",
        started.elapsed().as_secs_f64(),
        checked_time.as_secs_f64(),
    );
    for (reason, count) in &skipped {
        println!("  skipped {count}: {reason}");
    }
    assert!(compared > 0, "no selected case could be compared");
    assert!(problems.is_empty(), "{}", problems.join("\n\n"));
}
