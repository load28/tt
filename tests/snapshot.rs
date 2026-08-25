//! Snapshot fixtures: the compiler's whole output, held to the byte.
//!
//! Every other suite here asserts a *property* of an answer — that the
//! emission contains a `switch`, that a diagnostic mentions a missing case.
//! That is the right shape for a rule, and the wrong shape for a
//! deliverable: the emitted TypeScript is code a person reads, and a
//! rendered diagnostic is a picture a person looks at, so their contract is
//! everything about them, not one substring. A `contains` assertion cannot
//! notice a stray statement, a lost blank line, a caret that slipped a
//! column — which is exactly the quality TASK-198 and TASK-213 set out to
//! establish.
//!
//! So each fixture is a directory, and each file in it is the whole answer:
//!
//! ```text
//! tests/fixtures/emit/<name>/input.tt        the program
//! tests/fixtures/emit/<name>/expected.ts     what ttc emits for it
//!                                            (expected.tsx for an .ttx)
//!
//! tests/fixtures/diagnostic/<name>/input.tt        the program
//! tests/fixtures/diagnostic/<name>/expected.stderr what the CLI renders
//! tests/fixtures/diagnostic/<name>/expected.json   what `--server` sends
//! ```
//!
//! A diagnostic fixture pins *both* surfaces on purpose. They come from one
//! model but travel to two consumers, and pinning only the text is how a
//! field silently stops reaching the editor.
//!
//! Regenerate after a deliberate change, then read the diff — the diff is
//! the review:
//!
//! ```sh
//! UPDATE_EXPECT=1 cargo test --test snapshot
//! ```

use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use ttc::{Options, SourceKind, compile_report};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn updating() -> bool {
    std::env::var_os("UPDATE_EXPECT").is_some()
}

/// The fixture directories under `group`, in name order.
fn cases(group: &str) -> Vec<PathBuf> {
    let dir = fixtures().join(group);
    let mut out: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("no fixtures at {}: {e}", dir.display()))
        .map(|entry| entry.expect("readable fixture entry").path())
        .filter(|path| path.is_dir())
        .collect();
    out.sort();
    assert!(!out.is_empty(), "no fixtures under {}", dir.display());
    out
}

/// The `input.tt` or `input.ttx` of a case.
fn input(case: &Path) -> (PathBuf, String) {
    for name in ["input.tt", "input.ttx"] {
        let path = case.join(name);
        if path.exists() {
            let text = fs::read_to_string(&path).expect("readable input");
            return (path, text);
        }
    }
    panic!("{} has no input.tt or input.ttx", case.display());
}

/// What the emission of `input` is called: a `.tt` compiles to TypeScript
/// and a `.ttx` to TSX, and the fixture is named for what it actually is so
/// an editor highlights it the way the user's build output would be.
fn emitted_name(input: &Path) -> &'static str {
    match SourceKind::from_path(input) {
        Some(kind) if kind.is_tsx() => "expected.tsx",
        _ => "expected.ts",
    }
}

fn options(path: &Path) -> Options<'_> {
    Options {
        source_kind: SourceKind::from_path(path).unwrap_or_default(),
        ..Options::default()
    }
}

/// Compares `actual` against the file at `path`, or writes it there when
/// the run was asked to update.
fn expect(path: &Path, actual: &str) {
    if updating() {
        fs::write(path, actual).expect("writable expectation");
        return;
    }
    let expected = fs::read_to_string(path).unwrap_or_else(|_| {
        panic!(
            "{} does not exist yet — run `UPDATE_EXPECT=1 cargo test --test snapshot`",
            path.display()
        )
    });
    if expected == actual {
        return;
    }
    panic!(
        "{} is out of date\n\n{}\n\
         Run `UPDATE_EXPECT=1 cargo test --test snapshot` and read the diff.",
        path.display(),
        diff(&expected, actual),
    );
}

/// The lines that differ, with a little of what surrounds them.
///
/// Not a real diff — no dependency here does that — but comparing line by
/// line from the top is worse than nothing: one inserted line makes the
/// whole rest of the file look changed, and the reader has to find the
/// actual edit by eye. Trimming the matching head and tail first leaves
/// exactly the region that moved, which is what an insertion or a
/// rewritten block really is.
fn diff(expected: &str, actual: &str) -> String {
    const CONTEXT: usize = 3;
    let expected: Vec<&str> = expected.lines().collect();
    let actual: Vec<&str> = actual.lines().collect();

    let head = expected
        .iter()
        .zip(&actual)
        .take_while(|(left, right)| left == right)
        .count();
    // The tail may not reach back into the head on either side.
    let tail = expected[head..]
        .iter()
        .rev()
        .zip(actual[head..].iter().rev())
        .take_while(|(left, right)| left == right)
        .count();

    let mut out = String::new();
    let from = head.saturating_sub(CONTEXT);
    if from > 0 {
        out.push_str(&format!("  ... {from} identical line(s)\n"));
    }
    for line in &expected[from..head] {
        out.push_str(&format!("  {line}\n"));
    }
    for line in &expected[head..expected.len() - tail] {
        out.push_str(&format!("- {line}\n"));
    }
    for line in &actual[head..actual.len() - tail] {
        out.push_str(&format!("+ {line}\n"));
    }
    let after = expected.len() - tail;
    let shown = tail.min(CONTEXT);
    for line in &expected[after..after + shown] {
        out.push_str(&format!("  {line}\n"));
    }
    if tail > shown {
        out.push_str(&format!("  ... {} identical line(s)\n", tail - shown));
    }
    out
}

#[test]
fn emitted_typescript_matches_its_fixture() {
    for case in cases("emit") {
        let (path, source) = input(&case);
        let report = compile_report(&source, &options(&path));
        let emit = report.emit.unwrap_or_else(|| {
            panic!(
                "{} did not compile: {:#?}",
                case.display(),
                report.diagnostics
            )
        });
        expect(&case.join(emitted_name(&path)), &emit.code);
    }
}

#[test]
fn rendered_diagnostics_match_their_fixture() {
    for case in cases("diagnostic") {
        let (path, source) = input(&case);
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let report = compile_report(&source, &options(&path));
        assert!(
            !report.diagnostics.is_empty(),
            "{} is a diagnostic fixture but compiles clean",
            case.display()
        );
        // One blank line between blocks, exactly as the CLI prints them.
        let rendered: Vec<String> = report
            .diagnostics
            .iter()
            .map(|d| ttc::render::diagnostic(d, &source, &name))
            .collect();
        expect(
            &case.join("expected.stderr"),
            &format!("{}\n", rendered.join("\n\n")),
        );
    }
}

/// One `ttc --server` process, driven line by line.
///
/// The wire format is what the editor consumes, so the fixture is taken
/// from the server itself rather than rebuilt from the library — a field
/// the server forgets to send would otherwise still look present here.
struct Server {
    child: Child,
    out: BufReader<std::process::ChildStdout>,
}

impl Server {
    fn start() -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
            .arg("--server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to start the server");
        let out = BufReader::new(child.stdout.take().expect("piped stdout"));
        Server { child, out }
    }

    fn ask(&mut self, request: &serde_json::Value) -> serde_json::Value {
        let stdin = self.child.stdin.as_mut().expect("piped stdin");
        writeln!(stdin, "{request}").expect("the server took the request");
        stdin.flush().expect("flushed");
        let mut line = String::new();
        self.out.read_line(&mut line).expect("the server answered");
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("not JSON: {line}: {e}"))
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        drop(self.child.stdin.take());
        let _ = self.child.wait();
    }
}

#[test]
fn the_wire_format_matches_its_fixture() {
    let mut server = Server::start();
    for case in cases("diagnostic") {
        let (path, source) = input(&case);
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let answer = server.ask(&serde_json::json!({
            "id": 1,
            "method": "check",
            "params": { "text": source, "filename": name },
        }));
        let diagnostics = answer["result"]["diagnostics"].clone();
        assert!(
            diagnostics.is_array(),
            "{} got no diagnostics array: {answer}",
            case.display()
        );
        let pretty = serde_json::to_string_pretty(&diagnostics).expect("serializable");
        expect(&case.join("expected.json"), &format!("{pretty}\n"));
    }
}

#[test]
fn no_fixture_file_is_stale_or_missing() {
    // A fixture directory says what it holds. An expectation nobody reads
    // is a claim nobody checks, and a leftover file from a renamed case
    // reads as coverage that does not exist.
    //
    // Not during an update: the tests of one binary run concurrently, so
    // this would race the runs that are still writing the files it is
    // looking for. The next ordinary run is what checks them.
    if updating() {
        return;
    }
    for group in ["emit", "diagnostic"] {
        for case in cases(group) {
            let (input_path, _) = input(&case);
            let wanted: Vec<&str> = match group {
                "emit" => vec![emitted_name(&input_path)],
                _ => vec!["expected.stderr", "expected.json"],
            };
            let mut expected: BTreeSet<String> = wanted
                .into_iter()
                .map(str::to_string)
                .collect::<BTreeSet<_>>();
            expected.insert(
                input_path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            );
            let present: BTreeSet<String> = fs::read_dir(&case)
                .expect("readable case")
                .map(|entry| {
                    entry
                        .expect("entry")
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();
            assert_eq!(
                present,
                expected,
                "{} holds files no test reads, or is missing one",
                case.display()
            );
        }
    }
}
