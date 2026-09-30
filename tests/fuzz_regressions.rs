//! Fuzz findings as permanent regressions, and a deterministic mutation
//! pass over the case corpus.
//!
//! Every file under `fuzz/regressions/<target>/` is a crash input a fuzz
//! target once found, replayed here through the same function
//! `cargo fuzz` runs (`fuzz/src/lib.rs`), on the pinned stable toolchain.
//! The inputs that still crash are listed in
//! `fuzz/regressions/expected-failures.txt` with the task that fixes them
//! and the crash they produce, and the replay holds that list exact: an
//! unlisted input must not crash, and a listed one must still crash the
//! same way, so the list shrinks when the fix lands.
//!
//! The mutation pass types each case file prefix by prefix and deletes each
//! of its characters, and runs every mutant through the pipelines the CLI
//! and the editor run ([`ttc_fuzz::every_pipeline`]). A pull request runs a
//! fixed sample; `TT_MUTATIONS=all` runs every mutant, and
//! `TT_MUTATIONS=<count>` and `TT_MUTATION_SEED=<number>` choose another
//! sample.

#[path = "../fuzz/src/lib.rs"]
mod ttc_fuzz;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, Once};

use ttc::SourceKind;
use ttc::engine::Position;

const SAMPLE: usize = 1000;

const GENERATED: [&str; 2] = [
    "tests/cases/conformance/matrix",
    "tests/cases/editor/matrix",
];
const SEED: u64 = 0x7474_6d75_7461_7465;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn regressions() -> PathBuf {
    root().join("fuzz/regressions")
}

std::thread_local! {
    static CAPTURING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static LAST_PANIC: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

fn quiet_panics() {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        let default = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if !CAPTURING.with(std::cell::Cell::get) {
                default(info);
                return;
            }
            let message = info
                .payload()
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| info.payload().downcast_ref::<&str>().copied())
                .unwrap_or("a non-string panic");
            let file = info
                .location()
                .map_or("<unknown>".to_string(), |at| at.file().replace('\\', "/"));
            let signature = format!("{file}: {}", mask_digits(first_line(message)));
            LAST_PANIC.with(|last| *last.borrow_mut() = Some(signature));
        }));
    });
}

fn first_line(message: &str) -> &str {
    message.lines().next().unwrap_or("")
}

fn mask_digits(text: &str) -> String {
    let mut out = String::new();
    let mut in_number = false;
    for c in text.chars() {
        if c.is_ascii_digit() {
            if !in_number {
                out.push('N');
            }
            in_number = true;
        } else {
            in_number = false;
            out.push(c);
        }
    }
    out
}

fn crash_of(run: impl FnOnce()) -> Option<String> {
    quiet_panics();
    LAST_PANIC.with(|last| last.borrow_mut().take());
    CAPTURING.with(|capturing| capturing.set(true));
    let outcome = panic::catch_unwind(AssertUnwindSafe(run));
    CAPTURING.with(|capturing| capturing.set(false));
    match outcome {
        Ok(()) => None,
        Err(_) => Some(
            LAST_PANIC
                .with(|last| last.borrow_mut().take())
                .unwrap_or_else(|| "<unknown>: a panic without a hook record".to_string()),
        ),
    }
}

struct Expected {
    task: String,
    signature: String,
}

fn expected_failures() -> BTreeMap<String, Expected> {
    let path = regressions().join("expected-failures.txt");
    let text = fs::read_to_string(&path).expect("fuzz/regressions/expected-failures.txt");
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.splitn(3, '\t');
        let (Some(input), Some(task), Some(signature)) =
            (fields.next(), fields.next(), fields.next())
        else {
            panic!(
                "{}: `{line}` is not `<target>/<file>\\t<TASK-NNN>\\t<crash>`",
                path.display()
            );
        };
        assert!(
            task.starts_with("TASK-"),
            "{}: `{input}` names no task that fixes it",
            path.display()
        );
        let previous = out.insert(
            input.to_string(),
            Expected {
                task: task.to_string(),
                signature: signature.to_string(),
            },
        );
        assert!(
            previous.is_none(),
            "{}: `{input}` is listed twice",
            path.display()
        );
    }
    out
}

fn target(name: &str) -> Option<fn(&[u8])> {
    ttc_fuzz::TARGETS
        .iter()
        .find(|(target, _)| *target == name)
        .map(|(_, run)| *run)
}

#[test]
fn every_committed_crash_input_replays_as_the_list_says() {
    let expected = expected_failures();
    let mut seen = BTreeSet::new();
    let mut problems = Vec::new();
    let mut entries: Vec<PathBuf> = fs::read_dir(regressions())
        .expect("fuzz/regressions")
        .map(|entry| entry.expect("a readable entry").path())
        .filter(|path| path.is_dir())
        .collect();
    entries.sort();
    for dir in entries {
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let Some(run) = target(&name) else {
            problems.push(format!(
                "fuzz/regressions/{name}/ names no fuzz target in fuzz/src/lib.rs"
            ));
            continue;
        };
        let mut inputs: Vec<PathBuf> = fs::read_dir(&dir)
            .expect("a readable regression directory")
            .map(|entry| entry.expect("a readable entry").path())
            .collect();
        inputs.sort();
        for input in inputs {
            let key = format!("{name}/{}", input.file_name().unwrap().to_string_lossy());
            seen.insert(key.clone());
            let data = fs::read(&input).expect("a readable crash input");
            let crash = crash_of(|| run(&data));
            match (expected.get(&key), crash) {
                (None, None) => {}
                (None, Some(signature)) => problems.push(format!(
                    "{key} crashes: {signature}\n  fix it, or list it in fuzz/regressions/expected-failures.txt with the task that will"
                )),
                (Some(listed), None) => problems.push(format!(
                    "{key} no longer crashes ({} fixed it?): remove it from fuzz/regressions/expected-failures.txt",
                    listed.task
                )),
                (Some(listed), Some(signature)) if signature != listed.signature => {
                    problems.push(format!(
                        "{key} crashes differently than listed:\n  listed: {}\n  now:    {signature}",
                        listed.signature
                    ))
                }
                (Some(_), Some(_)) => {}
            }
        }
    }
    for key in expected.keys().filter(|key| !seen.contains(*key)) {
        problems.push(format!(
            "fuzz/regressions/expected-failures.txt lists {key}, which does not exist"
        ));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n\n"));
}

struct Source {
    path: PathBuf,
    kind: SourceKind,
    text: String,
}

fn tt_kind(name: &str) -> Option<SourceKind> {
    if name.ends_with(".tt") {
        Some(SourceKind::TypeScript)
    } else if name.ends_with(".ttx") {
        Some(SourceKind::Tsx)
    } else {
        None
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("a readable entry").path();
        if GENERATED
            .iter()
            .any(|generated| path == root().join(generated))
        {
            continue;
        }
        if path.is_dir() {
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn filename_directive(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("//")?.trim_start().strip_prefix('@')?;
    let (name, value) = rest.split_once(':')?;
    name.trim()
        .eq_ignore_ascii_case("filename")
        .then(|| value.trim())
}

fn units(path: &Path, text: &str) -> Vec<Source> {
    let mut out = Vec::new();
    let mut current: Option<(PathBuf, Vec<&str>)> = None;
    for line in text.split('\n') {
        if let Some(name) = filename_directive(line) {
            if let Some((unit, lines)) = current.take() {
                out.push((unit, lines.join("\n")));
            }
            current = Some((path.with_file_name(name), Vec::new()));
        } else if let Some((_, lines)) = &mut current {
            lines.push(line);
        }
    }
    match current {
        Some((unit, lines)) => out.push((unit, lines.join("\n"))),
        None => out.push((path.to_path_buf(), text.to_string())),
    }
    out.into_iter()
        .filter_map(|(path, text)| {
            let kind = tt_kind(&path.file_name()?.to_string_lossy())?;
            Some(Source { path, kind, text })
        })
        .collect()
}

fn corpus() -> Vec<Source> {
    let mut files = Vec::new();
    walk(&root().join("tests/cases"), &mut files);
    walk(&root().join("tests/fixtures"), &mut files);
    files.sort();
    let mut out = Vec::new();
    for file in files {
        if tt_kind(&file.file_name().unwrap().to_string_lossy()).is_none() {
            continue;
        }
        let text = fs::read_to_string(&file).expect("a UTF-8 corpus file");
        out.extend(units(&file, &text));
    }
    assert!(
        !out.is_empty(),
        "no .tt or .ttx file under tests/cases or tests/fixtures"
    );
    out
}

#[derive(Clone, Copy)]
enum Edit {
    Prefix(usize),
    Delete(usize),
}

struct Mutant {
    source: usize,
    edit: Edit,
}

fn apply(text: &str, edit: Edit) -> String {
    match edit {
        Edit::Prefix(end) => text[..end].to_string(),
        Edit::Delete(at) => {
            let width = text[at..].chars().next().map_or(0, char::len_utf8);
            format!("{}{}", &text[..at], &text[at + width..])
        }
    }
}

fn mutants(corpus: &[Source]) -> Vec<Mutant> {
    let mut out = Vec::new();
    for (index, source) in corpus.iter().enumerate() {
        let boundaries = source
            .text
            .char_indices()
            .map(|(at, _)| at)
            .chain([source.text.len()]);
        for at in boundaries {
            out.push(Mutant {
                source: index,
                edit: Edit::Prefix(at),
            });
            if at < source.text.len() {
                out.push(Mutant {
                    source: index,
                    edit: Edit::Delete(at),
                });
            }
        }
    }
    out
}

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn selection(all: Vec<Mutant>) -> (Vec<Mutant>, String) {
    let requested = std::env::var("TT_MUTATIONS").unwrap_or_default();
    let seed = std::env::var("TT_MUTATION_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(SEED);
    let total = all.len();
    if requested == "all" {
        return (all, format!("all {total} mutants"));
    }
    let count = requested.parse().unwrap_or(SAMPLE);
    if count >= total {
        return (all, format!("all {total} mutants"));
    }
    let mut state = seed;
    let mut indices: Vec<usize> = (0..total).collect();
    for i in 0..count {
        let j = i + (splitmix(&mut state) % (total - i) as u64) as usize;
        indices.swap(i, j);
    }
    let mut chosen: Vec<usize> = indices[..count].to_vec();
    chosen.sort_unstable();
    let mut slots: Vec<Option<Mutant>> = all.into_iter().map(Some).collect();
    let picked = chosen
        .into_iter()
        .map(|index| slots[index].take().expect("each index once"))
        .collect();
    (
        picked,
        format!("{count} of {total} mutants, seed {seed:#x} (TT_MUTATIONS=all for every one)"),
    )
}

fn run_pipelines(source: &Source, text: &str) -> Option<String> {
    let cursor: Position = ttc_fuzz::end_of(text);
    crash_of(|| ttc_fuzz::every_pipeline(text, source.kind, &source.path, cursor))
}

fn minimize(source: &Source, text: &str, signature: &str) -> String {
    let crashes = |candidate: &str| run_pipelines(source, candidate).as_deref() == Some(signature);
    let mut current = text.to_string();
    let mut chunk = current.len().max(1);
    let mut budget = 4000usize;
    while chunk > 0 && budget > 0 {
        let mut at = 0;
        let mut removed = false;
        while at < current.len() && budget > 0 {
            let start = floor_boundary(&current, at);
            let end = floor_boundary(&current, (start + chunk).min(current.len()));
            if end <= start {
                at = start + 1;
                continue;
            }
            let candidate = format!("{}{}", &current[..start], &current[end..]);
            budget -= 1;
            if crashes(&candidate) {
                current = candidate;
                removed = true;
            } else {
                at = end;
            }
        }
        if !removed {
            chunk /= 2;
        }
    }
    current
}

fn floor_boundary(text: &str, mut at: usize) -> usize {
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[test]
fn the_case_corpus_survives_deletion_and_typing() {
    let corpus = corpus();
    let expected = expected_failures();
    let known: BTreeMap<&str, &str> = expected
        .values()
        .map(|entry| (entry.signature.as_str(), entry.task.as_str()))
        .collect();
    let (selected, description) = selection(mutants(&corpus));
    let started = std::time::Instant::now();
    let next = AtomicUsize::new(0);
    let tolerated: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());
    let found: Mutex<BTreeMap<String, (usize, Edit)>> = Mutex::new(BTreeMap::new());
    let workers = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(4);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some(mutant) = selected.get(index) else {
                        break;
                    };
                    let source = &corpus[mutant.source];
                    let text = apply(&source.text, mutant.edit);
                    let Some(signature) = run_pipelines(source, &text) else {
                        continue;
                    };
                    if known.contains_key(signature.as_str()) {
                        *tolerated.lock().unwrap().entry(signature).or_default() += 1;
                    } else {
                        found
                            .lock()
                            .unwrap()
                            .entry(signature)
                            .or_insert((mutant.source, mutant.edit));
                    }
                }
            });
        }
    });
    println!(
        "mutation pass: {description}, {} source(s), {:.1?}",
        corpus.len(),
        started.elapsed()
    );
    for (signature, count) in tolerated.into_inner().unwrap() {
        println!(
            "  {count} mutant(s) hit the listed crash of {}: {signature}",
            known[signature.as_str()]
        );
    }
    let found = found.into_inner().unwrap();
    let reports: Vec<String> = found
        .into_iter()
        .map(|(signature, (index, edit))| {
            let source = &corpus[index];
            let minimized = minimize(source, &apply(&source.text, edit), &signature);
            let extension = match source.kind {
                SourceKind::TypeScript => "tt",
                SourceKind::Tsx => "ttx",
            };
            format!(
                "{signature}\n  from {}\n  minimized ({} bytes), save as fuzz/regressions/compile_any_bytes/{:016x}.{extension}:\n{minimized}",
                source.path.strip_prefix(root()).unwrap_or(&source.path).display(),
                minimized.len(),
                fnv(&minimized),
            )
        })
        .collect();
    assert!(
        reports.is_empty(),
        "{} crash(es) no list entry covers:\n\n{}",
        reports.len(),
        reports.join("\n\n")
    );
}

#[test]
fn the_generated_target_accepts_a_fixed_sample_of_its_programs() {
    let mut state = SEED;
    let mut rendered = 0usize;
    let mut problems = Vec::new();
    for draw in 0..256 {
        let data: Vec<u8> = (0..96).map(|_| splitmix(&mut state) as u8).collect();
        if <ttc_fuzz::Program as arbitrary::Arbitrary>::arbitrary_take_rest(
            arbitrary::Unstructured::new(&data),
        )
        .ok()
        .and_then(|program| program.render())
        .is_some()
        {
            rendered += 1;
        }
        if let Some(signature) = crash_of(|| ttc_fuzz::generated_tt_compiles(&data)) {
            problems.push(format!("draw {draw}: {signature}"));
        }
    }
    println!("generated target: {rendered} of 256 draws rendered a program");
    assert!(rendered > 0, "no draw rendered a program");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
