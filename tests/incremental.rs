//! An edited project answers as a freshly opened one does.
//!
//! TypeScript checks its incremental parser by parsing the same new text
//! twice, once from scratch and once from the old tree and the change, and
//! requiring the same tree and the same diagnostics
//! (`src/testRunner/unittests/incrementalParser.ts`, `compareTrees`, with
//! `insertCode` and `deleteCode` typing and deleting one character at a
//! time). The engine's incremental state is larger than a tree: projections
//! kept per content version, the semantic cache, the pattern-analysis
//! cache, and a TypeScript language service fed one change at a time. This
//! suite holds all of it to the same rule.
//!
//! Each sample opens a case or fixture project in a [`Workspace`], applies
//! a seeded sequence of edits to one document as an editor sends them,
//! asking the engine a question between some of the edits, and then
//! records the diagnostics, the emitted TypeScript, the service
//! diagnostics, the semantic tokens, and hover, definition and completion
//! at fixed points of the final text. A second workspace opened on the
//! final text must record the same.
//!
//! A pull request runs a fixed-seed sample; `TT_INCREMENTAL=all` runs every
//! sample, `TT_INCREMENTAL=<count>` and `TT_INCREMENTAL_SEED=<number>`
//! choose another sample, and `TT_INCREMENTAL_ONLY=<sample>` runs the one a
//! failure names.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use common::cases::{self, DEFAULT_TSCONFIG, Unit, is_tt};
use common::{Workspace, toolchain};
use ttc::engine::{CheckRequest, Engine, Position, Workspace as EngineWorkspace};

const SEED: u64 = 0x7474_696e_6372_656d;
const SAMPLE: usize = 8;
const SEQUENCES: u64 = 3;
const LONGEST_SPAN: usize = 12;
const HOVER_POINTS: usize = 8;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

struct Input {
    name: String,
    units: Vec<Unit>,
}

fn corpus() -> Vec<Input> {
    let mut out = Vec::new();
    let mut files = Vec::new();
    for suite in ["compiler", "conformance"] {
        cases::files(&root().join("tests/cases").join(suite), &mut files);
    }
    files.sort();
    let generated = root().join("tests/cases/conformance/matrix");
    for path in files {
        if path.starts_with(&generated) {
            continue;
        }
        let text = fs::read_to_string(&path).expect("readable case");
        let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
        let parsed = cases::parse(&text, &file_name, &path);
        out.push(Input {
            name: relative(&path),
            units: parsed.units,
        });
    }
    for suite in ["emit", "diagnostic"] {
        let mut dirs: Vec<PathBuf> = fs::read_dir(root().join("tests/fixtures").join(suite))
            .expect("fixture directory")
            .map(|entry| entry.expect("fixture entry").path())
            .filter(|path| path.is_dir())
            .collect();
        dirs.sort();
        for dir in dirs {
            for name in ["input.tt", "input.ttx"] {
                let path = dir.join(name);
                if let Ok(content) = fs::read_to_string(&path) {
                    out.push(Input {
                        name: relative(&path),
                        units: vec![Unit {
                            name: name.to_string(),
                            content,
                        }],
                    });
                }
            }
        }
    }
    out
}

fn relative(path: &Path) -> String {
    path.strip_prefix(root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[derive(Clone)]
struct Sample {
    input: usize,
    unit: usize,
    seed: u64,
}

fn label(inputs: &[Input], sample: &Sample) -> String {
    let input = &inputs[sample.input];
    format!(
        "{}#{}#{:x}",
        input.name, input.units[sample.unit].name, sample.seed
    )
}

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn samples(inputs: &[Input]) -> (Vec<Sample>, String) {
    let mut all = Vec::new();
    for (input, entry) in inputs.iter().enumerate() {
        for (unit, file) in entry.units.iter().enumerate() {
            if !is_tt(Path::new(&file.name)) || file.content.is_empty() {
                continue;
            }
            for sequence in 0..SEQUENCES {
                let mut state = SEED ^ (input as u64) << 32 ^ (unit as u64) << 16 ^ sequence;
                all.push(Sample {
                    input,
                    unit,
                    seed: splitmix(&mut state),
                });
            }
        }
    }
    if let Some(only) = std::env::var("TT_INCREMENTAL_ONLY")
        .ok()
        .filter(|value| !value.is_empty())
    {
        let chosen: Vec<Sample> = all
            .into_iter()
            .filter(|sample| label(inputs, sample) == only)
            .collect();
        assert!(!chosen.is_empty(), "no sample is named {only}");
        return (chosen, format!("the sample {only}"));
    }
    let requested = std::env::var("TT_INCREMENTAL").unwrap_or_default();
    let total = all.len();
    if requested == "all" {
        return (all, format!("all {total} samples"));
    }
    let count = requested.parse().unwrap_or(SAMPLE).min(total);
    let mut state = std::env::var("TT_INCREMENTAL_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(SEED);
    let mut indices: Vec<usize> = (0..total).collect();
    for i in 0..count {
        let j = i + (splitmix(&mut state) % (total - i) as u64) as usize;
        indices.swap(i, j);
    }
    let mut chosen: Vec<usize> = indices[..count].to_vec();
    chosen.sort_unstable();
    (
        chosen.into_iter().map(|i| all[i].clone()).collect(),
        format!("{count} of {total} samples"),
    )
}

#[derive(Clone, Copy, Debug)]
enum Query {
    Nothing,
    Check,
    ServiceDiagnostics,
    Hover,
    Completion,
    Tokens,
}

struct Step {
    text: String,
    cursor: usize,
    query: Query,
}

struct Script {
    summary: String,
    steps: Vec<Step>,
}

fn boundaries(text: &str) -> Vec<usize> {
    let mut out: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    out.push(text.len());
    out
}

fn span(text: &str, state: &mut u64) -> (usize, usize) {
    let bounds = boundaries(text);
    if bounds.len() < 2 {
        return (0, 0);
    }
    let start = (splitmix(state) % (bounds.len() - 1) as u64) as usize;
    let chars = 1 + (splitmix(state) % LONGEST_SPAN as u64) as usize;
    let end = (start + chars).min(bounds.len() - 1);
    (bounds[start], bounds[end])
}

fn query(state: &mut u64) -> Query {
    match splitmix(state) % 10 {
        0..=3 => Query::Nothing,
        4 | 5 => Query::Check,
        6 => Query::ServiceDiagnostics,
        7 => Query::Hover,
        8 => Query::Completion,
        _ => Query::Tokens,
    }
}

fn script(original: &str, seed: u64) -> Script {
    let mut state = seed;
    let mut steps = Vec::new();
    let mut text = original.to_string();
    let summary;
    match splitmix(&mut state) % 3 {
        0 => {
            let (start, end) = span(&text, &mut state);
            let removed = text[start..end].to_string();
            summary = format!("retype {start}..{end} {removed:?}");
            let mut cursor = end;
            while cursor > start {
                let previous = text[..cursor].char_indices().last().unwrap().0;
                text.replace_range(previous..cursor, "");
                cursor = previous;
                steps.push(Step {
                    text: text.clone(),
                    cursor,
                    query: query(&mut state),
                });
            }
            for c in removed.chars() {
                text.insert(cursor, c);
                cursor += c.len_utf8();
                steps.push(Step {
                    text: text.clone(),
                    cursor,
                    query: query(&mut state),
                });
            }
        }
        1 => {
            let (start, end) = span(&text, &mut state);
            let (from, to) = span(&text, &mut state);
            let donor = text[from..to].to_string();
            summary = format!("replace {start}..{end} with {from}..{to} {donor:?}");
            text.replace_range(start..end, "");
            steps.push(Step {
                text: text.clone(),
                cursor: start,
                query: query(&mut state),
            });
            let mut cursor = start;
            for c in donor.chars() {
                text.insert(cursor, c);
                cursor += c.len_utf8();
                steps.push(Step {
                    text: text.clone(),
                    cursor,
                    query: query(&mut state),
                });
            }
        }
        _ => {
            let lines: Vec<(usize, usize)> = line_spans(&text);
            let (from, to) = lines[(splitmix(&mut state) % lines.len() as u64) as usize];
            let (at, _) = lines[(splitmix(&mut state) % lines.len() as u64) as usize];
            let pasted = format!("{}\n", &text[from..to]);
            text.insert_str(at, &pasted);
            steps.push(Step {
                text: text.clone(),
                cursor: at + pasted.len(),
                query: query(&mut state),
            });
            let (start, end) = span(&text, &mut state);
            summary = format!("paste line {from}..{to} at {at}, then delete {start}..{end}");
            text.replace_range(start..end, "");
            steps.push(Step {
                text: text.clone(),
                cursor: start,
                query: query(&mut state),
            });
        }
    }
    Script { summary, steps }
}

fn line_spans(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            out.push((start, i));
            start = i + 1;
        }
    }
    out.push((start, text.len()));
    out
}

fn position(text: &str, offset: usize) -> Position {
    let before = &text[..offset];
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    Position {
        line: before.matches('\n').count() as u32,
        character: before[line_start..].encode_utf16().count() as u32,
    }
}

fn identifier_starts(text: &str) -> Vec<usize> {
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'$';
    let bytes = text.as_bytes();
    (0..bytes.len())
        .filter(|&i| {
            (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') && (i == 0 || !word(bytes[i - 1]))
        })
        .collect()
}

fn points(text: &str, cursor: usize) -> (Vec<usize>, Vec<usize>) {
    let starts = identifier_starts(text);
    let mut hover: Vec<usize> = if starts.len() <= HOVER_POINTS {
        starts.clone()
    } else {
        (0..HOVER_POINTS)
            .map(|k| starts[k * starts.len() / HOVER_POINTS])
            .collect()
    };
    hover.push(cursor);
    hover.sort_unstable();
    hover.dedup();
    let mut completion = vec![cursor];
    if let Some(dot) = text.find('.') {
        completion.push(dot + 1);
    }
    completion.sort_unstable();
    completion.dedup();
    (hover, completion)
}

fn ask(
    workspace: &mut EngineWorkspace,
    dir: &Path,
    path: &Path,
    text: &str,
    cursor: usize,
    what: Query,
) -> String {
    let at = position(text, cursor);
    match what {
        Query::Nothing => String::new(),
        Query::Check => check(workspace, dir, path),
        Query::ServiceDiagnostics => {
            let project = workspace.project_for(path).expect("a project");
            format!("{:?}", project.service_diagnostics(path))
        }
        Query::Hover => {
            let project = workspace.project_for(path).expect("a project");
            format!("{:?}", project.hover(path, at))
        }
        Query::Completion => completion(workspace, path, at),
        Query::Tokens => {
            let project = workspace.project_for(path).expect("a project");
            format!("{:?}", project.semantic_tokens(path))
        }
    }
}

fn check(workspace: &mut EngineWorkspace, dir: &Path, path: &Path) -> String {
    let project = workspace.project_for(path).expect("a project");
    let mut files = project.scan().expect("a readable project");
    files.push(path.to_path_buf());
    files.sort();
    files.dedup();
    let snapshot = match project.update(&files) {
        Ok(snapshot) => snapshot,
        Err(blocked) => {
            return format!(
                "blocked {}: {}",
                relative_to(&blocked.path, dir),
                blocked.error.message
            );
        }
    };
    let mut out = String::new();
    for file in snapshot.files() {
        out.push_str(&format!(
            "emit {}\n{}\n",
            relative_to(&file.source_path, dir),
            file.code()
        ));
    }
    for file in &files {
        if snapshot.is_blocked(file) {
            out.push_str(&format!("blocked {}\n", relative_to(file, dir)));
        }
    }
    match project.check(
        &snapshot,
        &CheckRequest {
            emit_declarations: true,
            tt_only: false,
        },
    ) {
        Err(error) => out.push_str(&format!("check error {error}\n")),
        Ok(checked) => {
            for diagnostic in &checked.diagnostics {
                out.push_str(&format!("diagnostic {diagnostic:?}\n"));
            }
            for module in &checked.declarations.modules {
                out.push_str(&format!(
                    "declaration {}\n{}\n",
                    relative_to(&module.file.source_path, dir),
                    module.text
                ));
            }
            out.push_str(&format!("backend {:?}\n", checked.backend_error));
        }
    }
    out
}

fn completion(workspace: &mut EngineWorkspace, path: &Path, at: Position) -> String {
    let project = workspace.project_for(path).expect("a project");
    match project.triggered_completion(path, at, false, None) {
        Err(error) => format!("error {error}"),
        Ok(answer) => {
            let mut items: Vec<String> = answer
                .items
                .iter()
                .map(|item| format!("{item:?}"))
                .collect();
            items.sort();
            format!(
                "member {} probe {} items\n{}",
                answer.member,
                answer.probe.is_some(),
                items.join("\n")
            )
        }
    }
}

fn relative_to(path: &Path, dir: &Path) -> String {
    path.strip_prefix(dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn observe(
    workspace: &mut EngineWorkspace,
    dir: &Path,
    edited: &Path,
    units: &[PathBuf],
    text: &str,
    cursor: usize,
) -> String {
    let mut out = check(workspace, dir, edited);
    for unit in units {
        let project = workspace.project_for(unit).expect("a project");
        out.push_str(&format!(
            "service diagnostics {}\n{:?}\n",
            relative_to(unit, dir),
            project.service_diagnostics(unit)
        ));
        out.push_str(&format!(
            "tokens {}\n{:?}\n",
            relative_to(unit, dir),
            project.semantic_tokens(unit)
        ));
    }
    let (hover, completions) = points(text, cursor);
    for offset in hover {
        let at = position(text, offset);
        let project = workspace.project_for(edited).expect("a project");
        out.push_str(&format!(
            "hover {}:{} {:?}\n",
            at.line,
            at.character,
            project.hover(edited, at)
        ));
        out.push_str(&format!(
            "definition {}:{} {:?}\n",
            at.line,
            at.character,
            project.definition(edited, at)
        ));
    }
    for offset in completions {
        let at = position(text, offset);
        out.push_str(&format!(
            "completion {}:{} {}\n",
            at.line,
            at.character,
            completion(workspace, edited, at)
        ));
    }
    out.replace(dir.to_string_lossy().as_ref(), "$DIR")
}

fn run(input: &Input, sample: &Sample) -> Result<(), String> {
    let workspace = Workspace::in_repo("incremental");
    let dir = workspace.path().canonicalize().expect("a workspace");
    for unit in &input.units {
        let path = dir.join(&unit.name);
        fs::create_dir_all(path.parent().unwrap()).expect("writable unit directory");
        fs::write(&path, &unit.content).expect("writable unit");
    }
    if !dir.join("tsconfig.json").exists() {
        fs::write(dir.join("tsconfig.json"), DEFAULT_TSCONFIG).expect("writable tsconfig");
    }
    let edited = dir.join(&input.units[sample.unit].name);
    let units: Vec<PathBuf> = input
        .units
        .iter()
        .filter(|unit| is_tt(Path::new(&unit.name)))
        .map(|unit| dir.join(&unit.name))
        .collect();
    let original = &input.units[sample.unit].content;
    let script = script(original, sample.seed);
    let last = script.steps.last().expect("a script edits");

    let mut incremental = EngineWorkspace::new(Engine::new(None));
    incremental
        .open_document(&edited, original.clone())
        .expect("opens");
    ask(&mut incremental, &dir, &edited, original, 0, Query::Check);
    ask(&mut incremental, &dir, &edited, original, 0, Query::Hover);
    for step in &script.steps {
        incremental
            .open_document(&edited, step.text.clone())
            .expect("updates");
        ask(
            &mut incremental,
            &dir,
            &edited,
            &step.text,
            step.cursor,
            step.query,
        );
    }
    let after_edits = observe(
        &mut incremental,
        &dir,
        &edited,
        &units,
        &last.text,
        last.cursor,
    );
    drop(incremental);

    let mut fresh = EngineWorkspace::new(Engine::new(None));
    fresh
        .open_document(&edited, last.text.clone())
        .expect("opens");
    let from_scratch = observe(&mut fresh, &dir, &edited, &units, &last.text, last.cursor);

    if after_edits == from_scratch {
        return Ok(());
    }
    let queries: Vec<String> = script
        .steps
        .iter()
        .map(|step| format!("{:?}", step.query))
        .collect();
    Err(format!(
        "{} ({}; {} edit(s), queries {})\nthe edited project (-) and a fresh one (+) differ:\n{}",
        edited.file_name().unwrap().to_string_lossy(),
        script.summary,
        script.steps.len(),
        queries.join(","),
        common::baseline::diff(&from_scratch, &after_edits)
    ))
}

#[test]
fn an_edited_project_answers_as_a_fresh_one() {
    if !toolchain() {
        eprintln!("SKIP incremental equivalence: no TypeScript installed — run `npm ci`");
        return;
    }
    let inputs = corpus();
    let (samples, description) = samples(&inputs);
    eprintln!("incremental equivalence: {description}");
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
                    let Some(sample) = samples.get(index) else {
                        break;
                    };
                    let input = &inputs[sample.input];
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        run(input, sample)
                    }));
                    let message = match outcome {
                        Ok(Ok(())) => continue,
                        Ok(Err(message)) => message,
                        Err(payload) => payload
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                            .unwrap_or_else(|| "a non-string panic".to_string()),
                    };
                    failures.lock().unwrap().push(format!(
                        "{}\nrerun with TT_INCREMENTAL_ONLY='{}'\n{message}",
                        label(&inputs, sample),
                        label(&inputs, sample)
                    ));
                }
            });
        }
    });
    let failures = failures.into_inner().unwrap();
    assert!(
        failures.is_empty(),
        "{} sample(s) answered differently after edits than when opened fresh:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
