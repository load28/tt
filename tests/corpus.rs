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

/// How many files a sample run compiles. A PR gets a fixed, spread-out
/// slice of the corpus rather than a random one: a gate that tests
/// something different on every run cannot be bisected.
const SAMPLE: usize = 250;

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

fn cases_required() -> bool {
    std::env::var_os("TTC_REQUIRE_TYPESCRIPT_CASES").is_some_and(|v| !v.is_empty() && v != "0")
}

fn manifest() -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/typescript-cases.json");
    let text = std::fs::read_to_string(&path).expect("tests/typescript-cases.json");
    serde_json::from_str(&text).expect("tests/typescript-cases.json is JSON")
}

fn cases_checkout(manifest: &serde_json::Value) -> Option<PathBuf> {
    if let Some(named) = std::env::var_os("TTC_TYPESCRIPT_CASES_DIR").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(named));
    }
    let commit = manifest["commit"]
        .as_str()
        .expect("the manifest names a commit");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/typescript-cases")
        .join(commit);
    dir.join(".git").is_dir().then_some(dir)
}

fn case_root(manifest: &serde_json::Value, suite: &str) -> String {
    manifest["trees"]
        .as_object()
        .expect("the manifest lists its trees")
        .keys()
        .find(|tree| tree.ends_with(&format!("/{suite}")))
        .unwrap_or_else(|| panic!("the manifest has no {suite} tree"))
        .clone()
}

struct TestCase {
    key: String,
    path: PathBuf,
}

fn test_cases(manifest: &serde_json::Value, checkout: &Path) -> Vec<TestCase> {
    let mut out = Vec::new();
    for suite in CASE_SUITES {
        let root = checkout.join(case_root(manifest, suite));
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

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
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
    let mut state = seed;
    let mut indices: Vec<usize> = (0..total).collect();
    for i in 0..count {
        let j = i + (splitmix(&mut state) % (total - i) as u64) as usize;
        indices.swap(i, j);
    }
    let chosen: BTreeSet<usize> = indices[..count].iter().copied().collect();
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

fn units_of(case: &TestCase, index: usize, code: &str) -> Vec<TsUnit> {
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
    let last = name.unwrap_or_else(|| {
        case.path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    });
    units.push((last, content));
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
