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
use common::matrix;
use common::{Workspace, toolchain, toolchain_installed};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn reference() -> PathBuf {
    root().join("tests/baselines/reference")
}

fn baseline_dir(path: &Path) -> PathBuf {
    let dir = path.parent().unwrap_or(path);
    for (generated, baselines) in [(MATRIX, "matrix"), (DIAGNOSTICS, "diagnostics")] {
        if let Ok(inside) = dir.strip_prefix(root().join(generated)) {
            return reference().join(baselines).join(inside);
        }
    }
    reference()
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
    diagnostic: Option<Expected>,
    clean: bool,
}

#[derive(Clone)]
struct Expected {
    code: String,
    typed_only: bool,
    ranges: Vec<Place>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Place {
    unit: String,
    start: (usize, usize),
    end: (usize, usize),
}

const KINDS: [&str; 5] = ["ts", "errors.txt", "map.txt", "types", "stdout"];

const PER_FILE: [&str; 8] = [
    "run",
    "twin",
    "expecterrors",
    "expectdiagnostic",
    "typedonly",
    "expectclean",
    "explains",
    "baselines",
];

const MATRIX: &str = "tests/cases/conformance/matrix";

const MATRIX_SAMPLE: usize = 120;

const MATRIX_SEED: u64 = 0x7474_6d61_7472_6978;

const DIAGNOSTICS: &str = "tests/cases/conformance/diagnostics";

const DIAGNOSTICS_SEED: u64 = 0x7474_6469_6167_6e6f;

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
    let mut diagnostics: BTreeMap<(Option<PathBuf>, bool), Vec<matrix::Named>> = BTreeMap::new();
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
        } else if filter.is_none()
            && path.starts_with(root().join(DIAGNOSTICS))
            && !name.contains("_explain")
        {
            let group = (
                path.parent().map(Path::to_path_buf),
                name.ends_with("_fixed"),
            );
            diagnostics.entry(group).or_default().push((name, path));
        } else {
            chosen.push((name, path));
        }
    }
    let matrix::Sample {
        sampled,
        mut unsampled,
        summary,
    } = matrix::sample(matrix, MATRIX_SAMPLE, MATRIX_SEED);
    chosen.extend(sampled);
    let diagnostics = matrix::stratified(diagnostics.into_values().collect(), DIAGNOSTICS_SEED);
    chosen.extend(diagnostics.sampled);
    unsampled.extend(diagnostics.unsampled);
    let summary = match (summary, diagnostics.summary) {
        (Some(matrix), Some(diagnostics)) => {
            Some(format!("{matrix}; diagnostics matrix: {diagnostics}"))
        }
        (matrix, diagnostics) => matrix.or(diagnostics),
    };
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
    let mut parsed = cases::parse(&text, &file_name, path);
    let run = run_entry(&parsed.directives, &parsed.units, path);
    let mut oracle = oracle(&parsed.directives, &parsed.units, run.as_deref(), path);
    oracle.diagnostic = expected_diagnostic(&parsed.directives, &mut parsed.units, path);
    oracle.clean = single(&parsed.directives, "expectclean", path).is_some_and(|value| {
        assert_eq!(
            value,
            "true",
            "{}: @expectClean takes `true`",
            path.display()
        );
        true
    });
    assert!(
        oracle.diagnostic.is_none() && !oracle.clean || run.is_none(),
        "{}: a case that expects a diagnostic or a clean compile is not run, so it takes no @run",
        path.display()
    );
    assert!(
        [
            oracle.twin.is_some(),
            !oracle.errors.is_empty(),
            oracle.diagnostic.is_some(),
            oracle.clean
        ]
        .iter()
        .filter(|set| **set)
        .count()
            <= 1,
        "{}: a case has one oracle: @twin, @expectErrors, @expectDiagnostic, or @expectClean",
        path.display()
    );
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
    Oracle {
        twin,
        errors,
        ..Oracle::default()
    }
}

fn expected_diagnostic(
    directives: &[(String, String)],
    units: &mut [Unit],
    path: &Path,
) -> Option<Expected> {
    let typed_only = single(directives, "typedonly", path).is_some_and(|value| {
        assert_eq!(value, "true", "{}: @typedOnly takes `true`", path.display());
        true
    });
    let Some(code) = single(directives, "expectdiagnostic", path) else {
        assert!(
            !typed_only,
            "{}: @typedOnly qualifies an @expectDiagnostic",
            path.display()
        );
        return None;
    };
    assert!(
        ttc::DiagnosticCode::parse(code).is_some(),
        "{}: @expectDiagnostic names `{code}`, which is no tt diagnostic code (`ttc explain` lists them)",
        path.display()
    );
    let mut ranges = Vec::new();
    for unit in units.iter_mut() {
        if !is_tt(Path::new(&unit.name)) {
            continue;
        }
        let (text, found) = cases::strip_ranges(&unit.content, path);
        for (start, end) in found {
            ranges.push(Place {
                unit: unit.name.clone(),
                start: cases::line_col(&text, start),
                end: cases::line_col(&text, end),
            });
        }
        unit.content = text;
    }
    assert!(
        !ranges.is_empty(),
        "{}: @expectDiagnostic needs the [|range|] the diagnostic covers in a .tt or .ttx unit",
        path.display()
    );
    ranges.sort();
    Some(Expected {
        code: code.to_string(),
        typed_only,
        ranges,
    })
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
                "{}: unknown directive `@{name}`; a case takes @filename, @run, @twin, @expectErrors, @expectDiagnostic, @typedOnly, @expectClean, @explains, @baselines, and the ttc options @rewriteImports and @noVerify",
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
    reported: Vec<Reported>,
}

const SURFACES: [&str; 4] = ["ttc --out-dir", "ttc --check-types", "check", "typedCheck"];

struct Reported {
    surface: &'static str,
    code: String,
    place: Place,
}

fn cli_reports(surface: &'static str, text: &str, out: &mut Vec<Reported>) {
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        let Some(code) = line
            .strip_prefix("error[")
            .or_else(|| line.strip_prefix("warning["))
            .and_then(|rest| rest.split_once(']'))
            .map(|(code, _)| code)
        else {
            continue;
        };
        if ttc::DiagnosticCode::parse(code).is_none() {
            continue;
        }
        let at = lines
            .next()
            .and_then(|next| next.trim_start().strip_prefix("--> "))
            .unwrap_or_default();
        let mut parts = at.rsplitn(3, ':');
        let col = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
        let line = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
        let unit = parts.next().unwrap_or_default();
        out.push(Reported {
            surface,
            code: code.to_string(),
            place: Place {
                unit: unit.strip_prefix("./").unwrap_or(unit).to_string(),
                start: (line, col),
                end: (0, 0),
            },
        });
    }
}

fn server_reports(case: &Case, project: &Path, out: &mut Vec<Reported>) {
    let units: Vec<&Unit> = case
        .units
        .iter()
        .filter(|unit| is_tt(Path::new(&unit.name)))
        .collect();
    let mut requests = String::new();
    for (index, unit) in units.iter().enumerate() {
        let path = project.join(&unit.name).to_string_lossy().into_owned();
        for (offset, request) in [
            serde_json::json!({ "method": "check", "params": { "text": unit.content, "filename": unit.name } }),
            serde_json::json!({ "method": "typedCheck", "params": { "path": path, "text": unit.content } }),
        ]
        .into_iter()
        .enumerate()
        {
            let mut request = request;
            request["id"] = serde_json::json!(index * 2 + offset);
            requests.push_str(&request.to_string());
            requests.push('\n');
        }
    }
    let mut child = ttc_command(project)
        .arg("--server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("ttc --server runs");
    {
        use std::io::Write as _;
        let mut stdin = child.stdin.take().expect("the server's stdin");
        stdin
            .write_all(requests.as_bytes())
            .expect("the server reads its requests");
    }
    let output = child.wait_with_output().expect("ttc --server ends");
    let mut answered = 0;
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Ok(answer) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(id) = answer["id"].as_u64() else {
            continue;
        };
        answered += 1;
        let unit = &units[id as usize / 2];
        let surface = SURFACES[2 + id as usize % 2];
        assert!(
            answer["error"].is_null(),
            "{}: `{surface}` on {} failed: {}",
            case.path.display(),
            unit.name,
            answer["error"]
        );
        for d in answer["result"]["diagnostics"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let Some(code) = d["code"]
                .as_str()
                .filter(|code| ttc::DiagnosticCode::parse(code).is_some())
            else {
                continue;
            };
            let number = |field: &str| d[field].as_u64().unwrap_or(0) as usize;
            let reported_in = d["path"]
                .as_str()
                .map(|path| {
                    Path::new(path)
                        .strip_prefix(project)
                        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_else(|_| path.to_string())
                })
                .unwrap_or_else(|| unit.name.clone());
            out.push(Reported {
                surface,
                code: code.to_string(),
                place: Place {
                    unit: reported_in,
                    start: (number("line"), number("col")),
                    end: (number("endLine"), number("endCol")),
                },
            });
        }
    }
    assert_eq!(
        answered,
        units.len() * 2,
        "{}: ttc --server answered {answered} of {} requests\n{}",
        case.path.display(),
        units.len() * 2,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn shown(places: &[(String, Place)]) -> String {
    if places.is_empty() {
        return "nothing".to_string();
    }
    places
        .iter()
        .map(|(code, place)| {
            let end = if place.end == (0, 0) {
                String::new()
            } else {
                format!("-{}:{}", place.end.0, place.end.1)
            };
            format!(
                "{code} at {}:{}:{}{end}",
                place.unit, place.start.0, place.start.1
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn judge_diagnostic(expected: &Expected, artifacts: &Artifacts) -> Verdict {
    for surface in SURFACES {
        let typed = matches!(surface, "ttc --check-types" | "typedCheck");
        let ranged = matches!(surface, "check" | "typedCheck");
        let mut wanted: Vec<(String, Place)> = if typed || !expected.typed_only {
            expected
                .ranges
                .iter()
                .map(|place| {
                    let mut place = place.clone();
                    if !ranged {
                        place.end = (0, 0);
                    }
                    (expected.code.clone(), place)
                })
                .collect()
        } else {
            Vec::new()
        };
        let mut got: Vec<(String, Place)> = artifacts
            .reported
            .iter()
            .filter(|reported| reported.surface == surface)
            .map(|reported| (reported.code.clone(), reported.place.clone()))
            .collect();
        wanted.sort();
        got.sort();
        let agrees = wanted.len() == got.len()
            && wanted
                .iter()
                .zip(&got)
                .all(|((want_code, want), (got_code, place))| {
                    want_code == got_code
                        && want.unit == place.unit
                        && want.start == place.start
                        && (want.end == place.end || place.end == (0, 0))
                });
        if !agrees {
            return Verdict::Disagrees {
                observed: format!("`{surface}` {}", difference_of(&wanted, &got)),
                detail: format!(
                    "`{surface}` was to report {} and reports {}\n{}",
                    shown(&wanted),
                    shown(&got),
                    artifacts.errors.clone().unwrap_or_default()
                ),
            };
        }
    }
    if artifacts.errors.is_none() {
        return Verdict::Disagrees {
            observed: "compiles cleanly".to_string(),
            detail: format!("expected {}", expected.code),
        };
    }
    Verdict::Agrees
}

fn difference_of(wanted: &[(String, Place)], got: &[(String, Place)]) -> String {
    if got.is_empty() {
        return "reports nothing".to_string();
    }
    let codes = |list: &[(String, Place)]| {
        list.iter()
            .map(|(code, _)| code.as_str())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ")
    };
    if codes(wanted) != codes(got) {
        return format!("reports {}", codes(got));
    }
    if wanted.len() != got.len() {
        return format!(
            "reports {} {} time(s) for {} range(s)",
            codes(got),
            got.len(),
            wanted.len()
        );
    }
    let delta = |want: (usize, usize), place: (usize, usize)| {
        format!(
            "{:+}:{:+}",
            place.0 as i64 - want.0 as i64,
            place.1 as i64 - want.1 as i64
        )
    };
    let moved: BTreeSet<String> = wanted
        .iter()
        .zip(got)
        .map(|((_, want), (_, place))| {
            let mut text = format!("start {}", delta(want.start, place.start));
            if want.end != (0, 0) && place.end != (0, 0) {
                text.push_str(&format!(", end {}", delta(want.end, place.end)));
            }
            text
        })
        .collect();
    format!(
        "reports {} off its range ({})",
        codes(got),
        moved.into_iter().collect::<Vec<_>>().join("; ")
    )
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

    let mut reported = Vec::new();
    if case.oracle.diagnostic.is_some() {
        cli_reports(SURFACES[0], &report(&build), &mut reported);
        cli_reports(SURFACES[1], &report(&check), &mut reported);
        server_reports(case, project, &mut reported);
    }

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
        reported,
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
    if let Some(expected) = &case.oracle.diagnostic {
        return Some(judge_diagnostic(expected, artifacts));
    }
    if case.oracle.clean {
        return Some(match &artifacts.errors {
            Some(errors) => Verdict::Disagrees {
                observed: "does not compile cleanly".to_string(),
                detail: errors.clone(),
            },
            None => Verdict::Agrees,
        });
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
        if unfiltered
            && !selection
                .names
                .iter()
                .any(|case| matches_listed(name, case))
        {
            failures.push(format!(
                "{ORACLE_FAILURES}:{}: `{name}` names no case; remove the line",
                entry.line
            ));
        }
    }
    if !typed {
        return failures;
    }
    let mut patterns: BTreeMap<&str, (bool, bool)> = listed
        .keys()
        .filter(|name| name.contains('*'))
        .map(|name| (name.as_str(), (true, false)))
        .collect();
    for case in &selection.unsampled {
        for (pattern, (complete, _)) in patterns.iter_mut() {
            if matches_listed(pattern, &case.0) {
                *complete = false;
            }
        }
    }
    for (name, verdict) in verdicts {
        let pattern = listed
            .iter()
            .find(|(listed, _)| listed.contains('*') && matches_listed(listed, name));
        if !listed.contains_key(name)
            && let Some((pattern, entry)) = pattern
        {
            match verdict {
                Some(Verdict::Disagrees { observed, .. }) if *observed == entry.observed => {
                    if let Some((_, seen)) = patterns.get_mut(pattern.as_str()) {
                        *seen = true;
                    }
                    continue;
                }
                Some(Verdict::Agrees) | None => continue,
                Some(Verdict::Disagrees { .. }) => {}
            }
        }
        let entry = listed.get(name).or_else(|| pattern.map(|(_, entry)| entry));
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
    for (pattern, (complete, seen)) in patterns {
        if unfiltered && complete && !seen {
            failures.push(format!(
                "{ORACLE_FAILURES}:{}: no case `{pattern}` names observes `{}` now; remove the line",
                listed[pattern].line, listed[pattern].observed
            ));
        }
    }
    failures
}

fn matches_listed(pattern: &str, name: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = name.strip_prefix(first) else {
        return false;
    };
    let parts: Vec<&str> = parts.collect();
    let Some((last, middle)) = parts.split_last() else {
        return rest.is_empty();
    };
    for part in middle {
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    rest.ends_with(last)
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

const WITHOUT_CASES: &str = "tests/diagnostic-codes-without-cases.txt";

struct Claim {
    path: PathBuf,
    expects: Option<String>,
    clean: bool,
    lines: Vec<String>,
}

type Examples = BTreeMap<(String, usize), Vec<Claim>>;

fn claims() -> (BTreeMap<String, usize>, Examples) {
    let mut files = Vec::new();
    for suite in ["compiler", "conformance"] {
        cases::files(&root().join("tests/cases").join(suite), &mut files);
    }
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut examples: Examples = BTreeMap::new();
    for path in files {
        let text = fs::read_to_string(&path).expect("readable case");
        let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
        let parsed = cases::parse(&text, &file_name, &path);
        let expects = single(&parsed.directives, "expectdiagnostic", &path).map(str::to_string);
        if let Some(code) = &expects {
            *counts.entry(code.clone()).or_default() += 1;
        }
        let Some(explains) = single(&parsed.directives, "explains", &path) else {
            continue;
        };
        let (code, number) = explains
            .split_once(' ')
            .and_then(|(code, number)| Some((code.to_string(), number.trim().parse().ok()?)))
            .unwrap_or_else(|| {
                panic!(
                    "{}: @explains takes a code and the number of its example, as `match-duplicate-arm 1`",
                    path.display()
                )
            });
        let lines = parsed
            .units
            .iter()
            .flat_map(|unit| {
                cases::strip_ranges(&unit.content, &path)
                    .0
                    .lines()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && cases::directive(line).is_none())
            .collect();
        let clean = single(&parsed.directives, "expectclean", &path).is_some();
        examples.entry((code, number)).or_default().push(Claim {
            path,
            expects,
            clean,
            lines,
        });
    }
    (counts, examples)
}

fn without_cases() -> BTreeMap<String, usize> {
    let text = fs::read_to_string(root().join(WITHOUT_CASES)).unwrap_or_default();
    let mut out = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [code, _why, task] = fields[..] else {
            panic!(
                "{WITHOUT_CASES}:{}: a line is a code, a tab, why no case can report it, a tab, and the TASK-NNN that records it",
                index + 1
            );
        };
        assert!(
            task.starts_with("TASK-"),
            "{WITHOUT_CASES}:{}: the last field names the TASK-NNN that records it",
            index + 1
        );
        assert!(
            ttc::DiagnosticCode::parse(code).is_some(),
            "{WITHOUT_CASES}:{}: `{code}` is no diagnostic code",
            index + 1
        );
        assert!(
            out.insert(code.to_string(), index + 1).is_none(),
            "{WITHOUT_CASES}:{}: `{code}` is listed twice",
            index + 1
        );
    }
    out
}

#[test]
fn every_diagnostic_code_has_cases() {
    let (counts, _) = claims();
    let listed = without_cases();
    let mut failures = Vec::new();
    for code in ttc::DiagnosticCode::ALL {
        let name = code.as_str();
        match (counts.get(name), listed.get(name)) {
            (None, None) => failures.push(format!(
                "`{name}` has no case that expects it (`// @expectDiagnostic: {name}`); add one to tests/matrix/diagnostics.mjs, or list it in {WITHOUT_CASES} with the reason no program can report it"
            )),
            (Some(count), Some(line)) => failures.push(format!(
                "{WITHOUT_CASES}:{line}: `{name}` has {count} case(s) now; remove the line"
            )),
            _ => {}
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_explanation_example_is_a_case() {
    let (_, mut examples) = claims();
    let listed = without_cases();
    let mut failures = Vec::new();
    for code in ttc::DiagnosticCode::ALL {
        let name = code.as_str();
        let explanation = code.explanation();
        if explanation.trim().is_empty() {
            failures.push(format!("`{name}` has no explanation for `ttc explain`"));
            continue;
        }
        let mut reproduced = false;
        for (index, block) in cases::example_blocks(explanation).iter().enumerate() {
            let number = index + 1;
            let Some(claims) = examples.remove(&(name.to_string(), number)) else {
                failures.push(format!(
                    "example {number} of `ttc explain {name}` is in no case; add it to tests/matrix/diagnostics.mjs:\n{block}"
                ));
                continue;
            };
            let wanted: Vec<String> = block
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect();
            for claim in claims {
                let found = claim
                    .lines
                    .windows(wanted.len())
                    .any(|window| window == wanted.as_slice());
                if !found {
                    failures.push(format!(
                        "{}: does not hold example {number} of `ttc explain {name}` as the explanation writes it:\n{block}",
                        claim.path.display()
                    ));
                }
                match (&claim.expects, claim.clean) {
                    (Some(expects), _) if expects == name => reproduced = true,
                    (None, true) => {}
                    _ => failures.push(format!(
                        "{}: an example of `ttc explain {name}` either reproduces it (`// @expectDiagnostic: {name}`) or compiles cleanly (`// @expectClean: true`)",
                        claim.path.display()
                    )),
                }
            }
        }
        if !reproduced && !listed.contains_key(name) {
            failures.push(format!(
                "`ttc explain {name}` has no example that reproduces it; write one and add its case to tests/matrix/diagnostics.mjs"
            ));
        }
    }
    for ((code, number), claims) in examples {
        for claim in claims {
            failures.push(format!(
                "{}: `ttc explain {code}` has no example {number}",
                claim.path.display()
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} problem(s):\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
