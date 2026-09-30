//! The bodies of the fuzz targets, callable from a stable test.
//!
//! `fuzz/fuzz_targets/*.rs` hand libFuzzer's bytes to the functions in
//! [`TARGETS`], and `tests/fuzz_regressions.rs` includes this file to replay
//! every committed crash input through the same functions, so a crasher
//! fixed once stays fixed without a nightly toolchain. This follows
//! typescript-go's `FuzzParser` (`internal/parser/parser_test.go`), whose
//! crash inputs under `testdata/fuzz/FuzzParser/` run on every `go test`.
//!
//! A target panics when it finds a bug; returning is the pass.

use arbitrary::{Arbitrary, Unstructured};
use std::path::Path;
use ttc::engine::Position;
use ttc::{ImportRewrite, Options, SourceKind};

/// A fuzz target: the name of its `fuzz/fuzz_targets/<name>.rs` file and of
/// its `fuzz/regressions/<name>/` directory, and its body.
pub type Target = (&'static str, fn(&[u8]));

/// Every fuzz target.
pub const TARGETS: &[Target] = &[
    ("compile_any_bytes", compile_any_bytes),
    ("generated_tt_compiles", generated_tt_compiles),
];

/// Arbitrary text through every pipeline the CLI and the editor run over a
/// buffer: it may reject, it may not crash. Bytes that are not UTF-8 are
/// not a compiler question, because the CLI and the server read text.
pub fn compile_any_bytes(data: &[u8]) {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    for kind in [SourceKind::TypeScript, SourceKind::Tsx] {
        let path = match kind {
            SourceKind::TypeScript => Path::new("/nonexistent/tt-fuzz/input.tt"),
            SourceKind::Tsx => Path::new("/nonexistent/tt-fuzz/input.ttx"),
        };
        every_pipeline(source, kind, path, end_of(source));
    }
}

/// Runs `source` through each untyped pipeline a consumer reaches:
///
/// - `ttc --check`: [`ttc::check_report`] and [`ttc::analyze`], which make
///   ttc's own exhaustiveness and `val` judgments;
/// - the emission of `ttc` and `ttc --out-dir`: [`ttc::compile`] and
///   [`ttc::compile_report`], with and without the output self-check;
/// - `ttc --emit-map` and the server's `emitMap`: [`ttc::emit_mapped_with_kind`];
/// - the engine's projection of an open document:
///   [`ttc::compile_projection_report`], with the engine's options;
/// - the editor requests answered from the text alone, at `cursor`:
///   semantic tokens, hints, declarations, the tt symbol, tt completions,
///   and tt keywords.
///
/// The emission runs with [`Options::defer_to_checker`], as the typed
/// engine runs it: otherwise [`ttc::compile`] asks the TypeScript of the
/// working directory's project to annotate generated storage, which is the
/// typed layer, not this one, and is neither in-process nor deterministic.
///
/// `path` names the document for the editor requests, whose `.tt` imports
/// are read from disk next to it.
pub fn every_pipeline(source: &str, kind: SourceKind, path: &Path, cursor: Position) {
    let checked = Options {
        source_kind: kind,
        ..Options::default()
    };
    let emitted = Options {
        defer_to_checker: true,
        ..checked.clone()
    };
    let projected = Options {
        rewrite_imports: ImportRewrite::Off,
        ..emitted.clone()
    };
    let unverified = Options {
        verify: false,
        ..projected.clone()
    };
    let _ = std::hint::black_box(ttc::check_report(source, &checked));
    let _ = std::hint::black_box(ttc::analyze(source, &checked));
    let _ = std::hint::black_box(ttc::compile(source, &emitted));
    let _ = std::hint::black_box(ttc::compile_report(source, &emitted));
    let _ = std::hint::black_box(ttc::compile_report(source, &unverified));
    let _ = std::hint::black_box(ttc::emit_mapped_with_kind(source, kind));
    let _ = std::hint::black_box(ttc::compile_projection_report(source, &projected));
    let _ = std::hint::black_box(ttc::engine::semantic_tokens_with_kind(source, kind));
    let _ = std::hint::black_box(ttc::engine::tt_hints(path, source));
    let _ = std::hint::black_box(ttc::engine::tt_declarations(path, source));
    let _ = std::hint::black_box(ttc::engine::tt_symbol_at(path, source, cursor));
    let _ = std::hint::black_box(ttc::engine::tt_completions_at(path, source, cursor));
    let _ = std::hint::black_box(ttc::engine::tt_keywords_at(path, source, cursor));
}

/// The LSP position just after the last character of `source`: where the
/// cursor is while the text is being typed.
pub fn end_of(source: &str) -> Position {
    let line = source.bytes().filter(|b| *b == b'\n').count() as u32;
    let last = source.rfind('\n').map_or(0, |at| at + 1);
    let character = source[last..].encode_utf16().count() as u32;
    Position { line, character }
}

/// Programs that are tt must pass ttc's own checks, compile
/// deterministically, and emit TypeScript that parses (`compile` checks that
/// itself unless `--no-verify`). The emission runs with
/// [`Options::defer_to_checker`] for the reason [`every_pipeline`] gives;
/// [`ttc::analyze`] still makes ttc's own exhaustiveness judgment. The bytes
/// choose a [`Program`] the way libFuzzer's typed `fuzz_target!` does
/// (`Arbitrary::arbitrary_take_rest`), so a crash input saved by
/// `cargo fuzz` replays here unchanged.
pub fn generated_tt_compiles(data: &[u8]) {
    let Ok(program) = Program::arbitrary_take_rest(Unstructured::new(data)) else {
        return;
    };
    let Some((source, source_kind)) = program.render() else {
        return;
    };
    let checked = Options {
        source_kind,
        ..Options::default()
    };
    let errors: Vec<_> = ttc::analyze(&source, &checked)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == ttc::Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "a well-formed tt program was rejected: {errors:?}\n\n{source}"
    );
    let options = Options {
        defer_to_checker: true,
        ..checked
    };
    match ttc::compile(&source, &options) {
        Ok(emitted) => {
            let again = ttc::compile(&source, &options).expect("compiled once already");
            assert_eq!(
                emitted, again,
                "compilation is not deterministic for:\n{source}"
            );
        }
        Err(error) => panic!("a well-formed tt program was rejected: {error}\n\n{source}"),
    }
}

/// What an arm's body evaluates to. Every variant is an expression, so an
/// arm is valid wherever it appears.
#[derive(Arbitrary, Debug)]
enum Body {
    /// The payload the pattern bound, used — which is what makes the
    /// binding's lowering observable.
    Bound,
    /// A number literal.
    Literal(u8),
    /// A pipeline over the bound value: two steps, so the runtime helper is
    /// needed and its import has to be placed.
    Piped,
    /// A nested match on a second variant declaration.
    Nested,
}

/// One case of a generated variant declaration: a tag, and how many payload fields.
#[derive(Arbitrary, Debug)]
struct Case {
    fields: u8,
}

/// One generated program.
#[derive(Arbitrary, Debug)]
pub struct Program {
    /// Two to five cases; more says nothing new and costs the fuzzer time.
    cases: Vec<Case>,
    bodies: Vec<Body>,
    /// Whether the match ends in a wildcard instead of covering every case.
    wildcard: bool,
    /// Whether the match sits inside a `result` block.
    in_result: bool,
    /// Whether the function propagates with `try`.
    with_try: bool,
    /// Whether an `if let` precedes the match.
    with_if_let: bool,
    /// Whether the same tt constructs are hosted by TSX attributes and children.
    with_jsx: bool,
}

/// The tag of case `index` — plain ASCII, so nothing here tests the
/// scanner's UTF-8 handling by accident.
fn tag(index: usize) -> String {
    format!("C{index}")
}

fn field(case: usize, index: usize) -> String {
    format!("f{case}_{index}")
}

impl Program {
    /// The program as tt source, or `None` when the draw is degenerate.
    ///
    /// The parentheses of `if let C0() = e` are part of the pattern: without
    /// them the pattern binds a new name instead of testing the case.
    pub fn render(&self) -> Option<(String, SourceKind)> {
        let cases: Vec<usize> = self
            .cases
            .iter()
            .take(5)
            .map(|case| (case.fields % 3) as usize)
            .collect();
        if cases.len() < 2 {
            return None;
        }
        let mut out = String::new();
        out.push_str("declare function step(n: number): number;\n");
        out.push_str("declare function fallible(n: number): TResult<number, string>;\n");
        out.push_str("import type { TResult } from \"@tt/std\";\n\n");

        out.push_str("export variant E {\n");
        for (index, fields) in cases.iter().enumerate() {
            out.push_str("  ");
            out.push_str(&tag(index));
            if *fields > 0 {
                let list: Vec<String> = (0..*fields)
                    .map(|f| format!("{}: number", field(index, f)))
                    .collect();
                out.push('(');
                out.push_str(&list.join(", "));
                out.push(')');
            }
            out.push_str(",\n");
        }
        out.push_str("}\n\nexport variant F { Yes, No }\n\n");

        out.push_str("export function run(e: E, f: F, n: number): number {\n");
        if self.with_if_let {
            out.push_str("  if let ");
            out.push_str(&tag(0));
            if cases[0] > 0 {
                out.push('(');
                out.push_str(&field(0, 0));
                out.push_str(") = e {\n    n = n + ");
                out.push_str(&field(0, 0));
                out.push_str(";\n  }\n");
            } else {
                out.push_str("() = e {\n    n = n + 1;\n  }\n");
            }
        }
        if self.with_try {
            out.push_str("  const checked = try fallible(n);\n  n = checked;\n");
        }

        let indent = if self.in_result { "    " } else { "  " };
        if self.in_result {
            out.push_str("  const value = result {\n    const first = try fallible(n);\n");
        }
        out.push_str(indent);
        out.push_str("const chosen = match (e) {\n");

        let covered = if self.wildcard { 1 } else { cases.len() };
        for (index, fields) in cases.iter().enumerate().take(covered) {
            out.push_str(indent);
            out.push_str("  ");
            out.push_str(&tag(index));
            if *fields > 0 {
                let list: Vec<String> = (0..*fields).map(|f| field(index, f)).collect();
                out.push('(');
                out.push_str(&list.join(", "));
                out.push(')');
            }
            out.push_str(" => ");
            let bound = (*fields > 0).then(|| field(index, 0));
            out.push_str(&self.body(index, bound.as_deref()));
            out.push_str(",\n");
        }
        if self.wildcard {
            out.push_str(indent);
            out.push_str("  _ => 0,\n");
        }
        out.push_str(indent);
        out.push_str("};\n");

        if self.with_jsx {
            out.push_str(indent);
            out.push_str("const view = <section data-value={chosen}>{match (f) {\n");
            out.push_str(indent);
            out.push_str("  Yes => <strong>{chosen |> step}</strong>,\n");
            out.push_str(indent);
            out.push_str("  No => null,\n");
            out.push_str(indent);
            out.push_str("}}</section>;\n");
            out.push_str(indent);
            out.push_str("void view;\n");
        }

        if self.in_result {
            out.push_str(
                "    return first + chosen;\n  };\n  return value.kind === \"Ok\" ? 0 : 1;\n",
            );
        } else {
            out.push_str("  return chosen + (f.kind === \"Yes\" ? 1 : 0);\n");
        }
        out.push_str("}\n");
        let source_kind = if self.with_jsx {
            SourceKind::Tsx
        } else {
            SourceKind::TypeScript
        };
        Some((out, source_kind))
    }

    /// One arm's body expression.
    fn body(&self, index: usize, bound: Option<&str>) -> String {
        let choice = self.bodies.get(index % self.bodies.len().max(1));
        match (choice, bound) {
            (Some(Body::Bound), Some(name)) => name.to_string(),
            (Some(Body::Piped), Some(name)) => format!("{name} |> step |> step"),
            (Some(Body::Piped), None) => "n |> step |> step".to_string(),
            (Some(Body::Nested), _) => "match (f) { Yes => 1, No => 0 }".to_string(),
            (Some(Body::Literal(v)), _) => format!("{}", v % 100),
            (Some(Body::Bound), None) | (None, _) => "0".to_string(),
        }
    }
}
