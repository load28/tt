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
//! The corpus is the `typescript-go` checkout the typed suites already
//! need — `testdata/tests/cases/` is TypeScript's own conformance corpus,
//! and `internal/bundled/libs/` is the standard library's declarations.
//! Pinning it costs nothing new: CI already fixes that revision, for the
//! same reason (a floating `main` breaks this gate on someone else's
//! commit).
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

use std::path::{Path, PathBuf};

use ttc::{Options, SourceKind};

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

/// Where more TypeScript lives inside a typescript-go checkout: the
/// compiler's own conformance corpus, its bundled library declarations,
/// and the API client's sources. Pinned by the same revision the typed
/// suites use.
const TSGO_DIRS: [&str; 3] = [
    "testdata/tests/cases",
    "internal/bundled/libs",
    "_packages/native-preview/src",
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
/// repository's own TypeScript, plus a typescript-go checkout's when one
/// is resolvable (the same way ttc resolves its toolchain).
fn roots() -> Vec<PathBuf> {
    if let Some(named) = std::env::var_os("TTC_CORPUS").filter(|v| !v.is_empty()) {
        let root = PathBuf::from(named);
        return root.is_dir().then_some(vec![root]).unwrap_or_default();
    }
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tree = match std::env::var_os("TTC_TSGO_ROOT").filter(|v| !v.is_empty()) {
        Some(root) => PathBuf::from(root),
        None => PathBuf::from("../typescript-go"),
    };
    OWN_DIRS
        .iter()
        .map(|dir| here.join(dir))
        .chain(TSGO_DIRS.iter().map(|dir| tree.join(dir)))
        .filter(|dir| dir.is_dir())
        .collect()
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
             (TTC_CORPUS, {OWN_DIRS:?} here, or {TSGO_DIRS:?} under \
             TTC_TSGO_ROOT / ../typescript-go)"
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
