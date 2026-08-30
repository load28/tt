//! `ttc --content-mapper` — a TypeScript content mapper process.
//!
//! TypeScript 7.1 asks an external process to turn a foreign file into
//! TypeScript it can hold *virtually*: no sidecar on disk, no consumer
//! `rootDirs`/`paths` wiring. ttc is that process for `.tt`/`.ttx`. The
//! consumer names the mapper once in `tsconfig.json` —
//!
//! ```jsonc
//! { "contentMappers": [
//!     { "package": "@openload28/tt-lang", "extensions": [".tt", ".ttx"] } ] }
//! ```
//!
//! — and every surface of that TypeScript (CLI `tsc --runExternalCode`,
//! the LSP server, `--build`, `--watch`) resolves `.tt` imports through
//! this mode. The contract is typescript-go PR #4712 ("Content mappers");
//! the wire facts below were measured against the pinned
//! `typescript@7.1.0-dev.20260826.1` (TASK-257).
//!
//! The protocol is JSON-RPC 2.0 over stdio with `Content-Length` framing
//! (the LSP base protocol). TypeScript sends every request; the mapper
//! only answers. Four methods:
//!
//! ```text
//! → initialize   { positionEncodings: ["utf-8", "utf-16"], locale? }
//! ← { positionEncoding: "utf-8", diagnosticSource: "tt" }
//!
//! → openProject  { configFileName, projectHandle, options?, compilerOptions }
//! ← {}
//!
//! → transform    { fileName, content, projectHandle }
//! ← { text, extension, mappings, diagnostics? }
//!
//! → closeProject { projectHandle }
//! ← {}
//! ```
//!
//! Everything in an answer is computed by the same public entry points the
//! CLI runs — [`ttc::compile_report`] for the emission and the tt-level
//! diagnostics, [`ttc::scan_module_with_kind`] +
//! [`ttc::exported_variants_with_kind`] for one-hop exhaustiveness — so
//! `tsc` through the mapper and `ttc --check` agree about the same file.
//!
//! Position encoding is `"utf-8"`: ttc's spans are byte offsets end to
//! end (mappings, anchors, diagnostics), and UTF-8 code units *are* those
//! bytes, so nothing is converted at this boundary.
//!
//! The error layers survive the protocol. tt-level rules are reported by
//! this process as mapper diagnostics (`diagnosticSource: "tt"`); the
//! emitted text is plain TypeScript, and its type errors are TypeScript's
//! own, mapped back through the span map — verbatim chunks to their exact
//! source bytes, compiler-written glue to the construct that wrote it
//! (an [`ttc::EmitAnchor`], carried as an `Atom` span with no language
//! service features, so diagnostics land on the construct while
//! navigation and rename can never resolve into glue).
//!
//! Exit: end of stdin, code 0. A request that fails never ends the
//! session; a stream that stops being JSON-RPC does.

use std::collections::HashSet;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ttc::{Diagnostic, EmitAnchor, EmitMapping, ImportRewrite, Options, Severity, SourceKind};

/// `SpanMapKind.Verbatim` — same length and content in both texts; the
/// only kind edits may be written back through.
const SPAN_VERBATIM: u64 = 0;
/// `SpanMapKind.Atom` — a correspondence with different text, used here
/// for compiler-written glue.
const SPAN_ATOM: u64 = 1;
/// `SpanMapFeature.None` — the span maps diagnostics (which are not
/// feature-gated) and nothing else.
const FEATURES_NONE: u64 = 0;

/// The stable numeric form of a tt diagnostic code on this wire.
///
/// `MapperDiagnostic.code` is a number, [`ttc::DiagnosticCode::as_str`] is
/// a name; this table joins them. It is append-only: a code keeps its
/// number for as long as the mapper exists, and a name this table does not
/// know yet reports as `0` rather than shifting its neighbours.
const CODE_NUMBERS: [&str; 34] = [
    "stray-pipe",
    "malformed-pipeline-postfix",
    "invalid-optional-receiver",
    "stray-if-let",
    "stray-result",
    "malformed-variant",
    "malformed-match",
    "result-missing-keyword",
    "result-nested-binding",
    "flow-first-step-method",
    "try-placement",
    "let-else-placement",
    "let-else-not-diverging",
    "if-let-placement",
    "variant-duplicate-case",
    "variant-invalid-field-type",
    "pattern-duplicate-binding",
    "match-mixed-patterns",
    "match-wildcard-not-last",
    "match-or-literal-kind-mismatch",
    "match-duplicate-arm",
    "match-nested-in-or-pattern",
    "match-or-binding-mismatch",
    "match-tuple-arity",
    "unknown-case",
    "unknown-field",
    "match-not-exhaustive",
    "val-mutation",
    "val-pass",
    "verify-failed",
    "source-not-typescript",
    "other",
    "result-tail-semicolon",
    "lowering-plan-failed",
];

/// Everything the mapper keeps between requests.
struct Session {
    /// Handles TypeScript has opened and not yet closed. The tt transform
    /// needs no per-project state — no options, no compiler options — so
    /// the set exists only to answer `closeProject` honestly.
    open_projects: HashSet<String>,
    /// Roots where `@tt/std`/`@tt/runtime` have already been materialized
    /// this session, so a build over many files stats each root once.
    ensured_roots: HashSet<PathBuf>,
}

/// Runs the mapper until stdin closes.
pub(crate) fn run() -> ExitCode {
    let mut session = Session {
        open_projects: HashSet::new(),
        ensured_roots: HashSet::new(),
    };

    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let stdout = std::io::stdout();
    loop {
        let message = match read_message(&mut reader) {
            Ok(Some(message)) => message,
            // End of stdin: TypeScript is done with this process.
            Ok(None) => return ExitCode::SUCCESS,
            // A stream that stops being JSON-RPC cannot carry answers;
            // TypeScript treats a dead mapper as five failures and says so.
            Err(error) => {
                eprintln!("ttc --content-mapper: {error}");
                return ExitCode::FAILURE;
            }
        };
        // A request without an id would be a notification; the protocol
        // sends none, and an answer to nothing is itself a violation.
        let Some(id) = message.get("id").cloned() else {
            continue;
        };
        // A panic in one request is a bug in the compiler, not the end of
        // the session (the same promise `--server` makes): answer this id
        // with an error and keep reading.
        let response = match ttc::ice::catching(|| respond(&mut session, &message)) {
            Ok(Ok(result)) => serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Ok(Err(rpc)) => serde_json::json!({
                "jsonrpc": "2.0", "id": id,
                "error": { "code": rpc.code, "message": rpc.message },
            }),
            Err(message) => serde_json::json!({
                "jsonrpc": "2.0", "id": id,
                "error": { "code": INTERNAL_ERROR, "message": ttc::ice::bug_message(&message) },
            }),
        };
        let mut out = stdout.lock();
        if write_message(&mut out, &response).is_err() {
            // stdout gone means TypeScript is gone.
            return ExitCode::SUCCESS;
        }
    }
}

/// JSON-RPC "method not found".
const METHOD_NOT_FOUND: i64 = -32601;
/// JSON-RPC "invalid params".
const INVALID_PARAMS: i64 = -32602;
/// JSON-RPC "internal error".
const INTERNAL_ERROR: i64 = -32603;

/// A JSON-RPC error answer: the code the protocol names and one sentence.
#[derive(Debug)]
struct RpcError {
    code: i64,
    message: String,
}

impl RpcError {
    fn invalid_params(message: impl Into<String>) -> Self {
        RpcError {
            code: INVALID_PARAMS,
            message: message.into(),
        }
    }
}

/// Answers one request, or says why it cannot.
fn respond(
    session: &mut Session,
    message: &serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let params = message.get("params").cloned().unwrap_or_default();
    match message.get("method").and_then(|m| m.as_str()) {
        Some("initialize") => initialize(&params),
        Some("openProject") => open_project(session, &params),
        Some("transform") => transform(session, &params),
        Some("closeProject") => close_project(session, &params),
        Some(other) => Err(RpcError {
            code: METHOD_NOT_FOUND,
            message: format!("unknown method `{other}`"),
        }),
        None => Err(RpcError {
            code: METHOD_NOT_FOUND,
            message: "request names no method".to_string(),
        }),
    }
}

/// `initialize` — pick the encoding ttc already speaks.
fn initialize(params: &serde_json::Value) -> Result<serde_json::Value, RpcError> {
    let offered = params["positionEncodings"]
        .as_array()
        .map(|encodings| {
            encodings
                .iter()
                .filter_map(|e| e.as_str())
                .any(|e| e == "utf-8")
        })
        .unwrap_or(false);
    if !offered {
        return Err(RpcError::invalid_params(
            "ttc requires the utf-8 position encoding",
        ));
    }
    Ok(serde_json::json!({
        "positionEncoding": "utf-8",
        "diagnosticSource": "tt",
    }))
}

/// `openProject` — remember the handle; materialize the standard library
/// next to the project so the virtual tree's `@tt/std` imports resolve.
fn open_project(
    session: &mut Session,
    params: &serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let handle = string_param(params, "projectHandle")?;
    session.open_projects.insert(handle);
    // "" is a project without a config file; its root is only knowable
    // from the files themselves, so `transform` handles that case.
    let config = params["configFileName"].as_str().unwrap_or_default();
    if !config.is_empty()
        && let Some(root) = Path::new(config).parent()
    {
        ensure_std_packages(session, root);
    }
    Ok(serde_json::json!({}))
}

/// `closeProject` — forget the handle.
fn close_project(
    session: &mut Session,
    params: &serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let handle = string_param(params, "projectHandle")?;
    session.open_projects.remove(&handle);
    Ok(serde_json::json!({}))
}

/// `transform` — one `.tt`/`.ttx` file into TypeScript text, span
/// mappings, and tt-level diagnostics.
fn transform(
    session: &mut Session,
    params: &serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let file_name = string_param(params, "fileName")?;
    let content = string_param(params, "content")?;

    let path = Path::new(&file_name);
    let source_kind = SourceKind::from_path(path).unwrap_or_default();

    // One-hop exhaustiveness, exactly as the CLI collects it: the file's
    // direct relative `.tt`/`.ttx` imports are read from disk and their
    // exported variants join the check. A specifier that cannot be read is
    // skipped — module resolution is TypeScript's domain (`TS2307`), and
    // an unknown variant simply stays unchecked. Reads are per-request on
    // purpose: this process outlives edits under `--watch`, and a cache
    // with no invalidation would answer from before them.
    let scan = ttc::scan_module_with_kind(&content, source_kind);
    let extern_variants = collect_extern_variants(path, &scan.imports);

    let options = Options {
        filename: Some(&file_name),
        source_kind,
        // The virtual text keeps `.tt`/`.ttx` specifiers: the consumer's
        // `contentMappers.extensions` teach module resolution to look
        // those files up, and each resolves to its own mapped output.
        rewrite_imports: ImportRewrite::Off,
        extern_variants: &extern_variants,
        ..Options::default()
    };
    let report = ttc::compile_report(&content, &options);

    let diagnostics: Vec<serde_json::Value> = report
        .diagnostics
        .iter()
        // The wire has no severity: everything a mapper reports renders as
        // an error, so a tt warning must not travel it.
        .filter(|d| d.severity == Severity::Error)
        .map(mapper_diagnostic)
        .collect();

    let (text, mappings) = match report.emit {
        Some(emit) => {
            let mappings = span_mappings(&emit.mappings, &emit.anchors);
            (emit.code, mappings)
        }
        // A diagnostic blocked projection: there is no TypeScript to
        // serve. An empty module is what TypeScript itself substitutes for
        // a failed mapper file, and the tt diagnostics above still carry
        // the cause at its source position.
        None => (String::new(), Vec::new()),
    };

    // The emission imports the standard library by bare specifier; make it
    // resolvable next to the file for projects that never ran a ttc build
    // (an inferred editor project, a first `tsc` run).
    if (text.contains("@tt/") || content.contains("@tt/"))
        && let Some(root) = package_root(path)
    {
        ensure_std_packages(session, &root);
    }

    Ok(serde_json::json!({
        "text": text,
        // The protocol spells virtual extensions with the dot.
        "extension": format!(".{}", source_kind.output_extension()),
        "mappings": mappings,
        "diagnostics": diagnostics,
    }))
}

/// A required string parameter, or the `invalid params` answer.
fn string_param(params: &serde_json::Value, name: &str) -> Result<String, RpcError> {
    params[name]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| RpcError::invalid_params(format!("`{name}` must be a string")))
}

/// One tt diagnostic in the mapper's wire form.
///
/// Suggestions ride along in the text: the wire has one string, and "what
/// is wrong" without "what to do about it" would strand the half of the
/// diagnostic ttc keeps separate for editors.
fn mapper_diagnostic(diagnostic: &Diagnostic) -> serde_json::Value {
    let start = diagnostic.start.unwrap_or(0);
    let length = diagnostic.end.unwrap_or(start).saturating_sub(start);
    let mut message = diagnostic.message.clone();
    for suggestion in &diagnostic.suggestions {
        message.push_str("\nhelp: ");
        message.push_str(&suggestion.message);
    }
    serde_json::json!({
        "messageText": message,
        "start": start,
        "length": length,
        "code": code_number(diagnostic.code.as_str()),
    })
}

/// The wire number of a tt diagnostic code name (see [`CODE_NUMBERS`]).
fn code_number(name: &str) -> u64 {
    CODE_NUMBERS
        .iter()
        .position(|known| *known == name)
        .map(|index| index as u64 + 1)
        .unwrap_or(0)
}

/// The span map of one emission: verbatim chunks as `Verbatim`, glue as
/// `Atom` spans owned by the construct that wrote it.
///
/// Virtual spans must not overlap, and anchors both nest and contain the
/// verbatim chunks of their construct's copied text (a match's arm
/// bodies), so each anchor contributes only the stretches nothing else
/// claimed — innermost first, the same priority [`ttc::MappedEmit::anchor_at`]
/// gives a consumer. `Atom` glue spans carry `SpanMapFeature.None`:
/// diagnostics are not feature-gated and land on the construct's own
/// source range, while navigation and rename — which must never resolve
/// into glue — stay off.
fn span_mappings(mappings: &[EmitMapping], anchors: &[EmitAnchor]) -> Vec<serde_json::Value> {
    // Occupied intervals of the virtual text, kept sorted by start.
    let mut occupied: Vec<(usize, usize)> =
        mappings.iter().map(|m| (m.out, m.out + m.len)).collect();
    occupied.sort_unstable();

    let mut spans: Vec<(usize, serde_json::Value)> = mappings
        .iter()
        .map(|m| {
            (
                m.out,
                serde_json::json!([m.out, m.len, m.src, m.len, SPAN_VERBATIM]),
            )
        })
        .collect();

    for anchor in anchors {
        let original_start = anchor.src;
        let original_length = anchor.src_end.saturating_sub(anchor.src);
        for (start, end) in free_intervals(anchor.out, anchor.end, &occupied) {
            spans.push((
                start,
                serde_json::json!([
                    start,
                    end - start,
                    original_start,
                    original_length,
                    SPAN_ATOM,
                    FEATURES_NONE,
                ]),
            ));
            let position = occupied.partition_point(|&(s, _)| s < start);
            occupied.insert(position, (start, end));
        }
    }

    spans.sort_by_key(|(start, _)| *start);
    spans.into_iter().map(|(_, span)| span).collect()
}

/// The stretches of `[start, end)` not covered by any `occupied` interval.
fn free_intervals(start: usize, end: usize, occupied: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut free = Vec::new();
    let mut cursor = start;
    for &(taken_start, taken_end) in occupied {
        if taken_end <= cursor {
            continue;
        }
        if taken_start >= end {
            break;
        }
        if taken_start > cursor {
            free.push((cursor, taken_start.min(end)));
        }
        cursor = cursor.max(taken_end);
        if cursor >= end {
            return free;
        }
    }
    if cursor < end {
        free.push((cursor, end));
    }
    free
}

/// Variant declarations from the file's direct relative `.tt`/`.ttx`
/// imports — the CLI's one-hop collection, without its whole-run cache.
fn collect_extern_variants(file: &Path, imports: &[ttc::TtImport]) -> Vec<ttc::ExternVariant> {
    let dir = file.parent().unwrap_or(Path::new("."));
    let mut externs: Vec<ttc::ExternVariant> = Vec::new();
    for import in imports {
        if matches!(import.names, ttc::TtImportNames::None) {
            continue;
        }
        let imported = dir.join(&import.specifier);
        let Ok(source) = std::fs::read_to_string(&imported) else {
            continue;
        };
        let kind = SourceKind::from_path(&imported).unwrap_or_default();
        let decls = ttc::exported_variants_with_kind(&source, kind);
        let from = Some(import.specifier.clone());
        match &import.names {
            ttc::TtImportNames::Namespace(ns) => {
                externs.extend(decls.into_iter().map(|d| ttc::ExternVariant {
                    name: format!("{ns}.{}", d.name),
                    tags: d.tags,
                    from: from.clone(),
                }));
            }
            ttc::TtImportNames::Named(entries) => {
                for (name, alias) in entries {
                    if let Some(d) = decls.iter().find(|d| &d.name == name) {
                        externs.push(ttc::ExternVariant {
                            name: alias.clone().unwrap_or_else(|| name.clone()),
                            tags: d.tags.clone(),
                            from: from.clone(),
                        });
                    }
                }
            }
            ttc::TtImportNames::None => unreachable!("a nameless import was skipped above"),
        }
    }
    externs
}

/// The nearest ancestor of `file` that is a package root — has a
/// `package.json` or a `node_modules` — where `@tt/std` belongs.
fn package_root(file: &Path) -> Option<PathBuf> {
    let mut dir = file.parent()?;
    loop {
        if dir.join("package.json").is_file() || dir.join("node_modules").is_dir() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}

/// Makes every `@tt/std` and `@tt/runtime` entry resolvable in `root`, the
/// way the typed engine does for its language service: the modules are
/// served from memory everywhere ttc itself is the consumer, but
/// TypeScript's module resolution reads the file system, so for it the
/// package has to *be* there. Never over one that already exists — a
/// project that manages its own copy keeps it.
fn ensure_std_packages(session: &mut Session, root: &Path) {
    if !session.ensured_roots.insert(root.to_path_buf()) {
        return;
    }
    let std_pkg = root.join("node_modules/@tt/std");
    if !std_pkg.exists() && std::fs::create_dir_all(&std_pkg).is_ok() {
        for module in ttc::StdModule::STANDARD {
            let source = format!(
                "// @generated by ttc --emit-std — do not edit directly.\n{}",
                module.source()
            );
            let _ = std::fs::write(std_pkg.join(module.file_name()), source);
        }
        let _ = std::fs::write(
            std_pkg.join("package.json"),
            "{\n  \"name\": \"@tt/std\",\n  \"version\": \"0.0.0\",\n  \"types\": \"index.ts\"\n}\n",
        );
    }
    let runtime_pkg = root.join("node_modules/@tt/runtime");
    if !runtime_pkg.exists() && std::fs::create_dir_all(&runtime_pkg).is_ok() {
        let source = format!(
            "// @generated by ttc --emit-std — do not edit directly.\n{}",
            ttc::StdModule::Runtime.source()
        );
        let _ = std::fs::write(runtime_pkg.join("index.ts"), source);
        let _ = std::fs::write(
            runtime_pkg.join("package.json"),
            "{\n  \"name\": \"@tt/runtime\",\n  \"version\": \"0.0.0\",\n  \"types\": \"index.ts\"\n}\n",
        );
    }
}

/// Reads one `Content-Length`-framed JSON message, `None` at end of input.
fn read_message(reader: &mut impl BufRead) -> Result<Option<serde_json::Value>, String> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|e| format!("reading a header: {e}"))?;
        if read == 0 {
            // EOF between messages is the clean end; inside a header block
            // it means the peer died mid-message.
            return match content_length {
                None => Ok(None),
                Some(_) => Err("end of input inside a message header".to_string()),
            };
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(value) = line
            .strip_prefix("Content-Length:")
            .or_else(|| line.strip_prefix("content-length:"))
        {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|e| format!("Content-Length `{}`: {e}", value.trim()))?,
            );
        }
        // Any other header (Content-Type) is tolerated and ignored.
    }
    let length = content_length.ok_or("a message frame without Content-Length")?;
    let mut body = vec![0u8; length];
    reader
        .read_exact(&mut body)
        .map_err(|e| format!("reading a {length}-byte message body: {e}"))?;
    let message = serde_json::from_slice(&body).map_err(|e| format!("parsing a message: {e}"))?;
    Ok(Some(message))
}

/// Writes one `Content-Length`-framed JSON message.
fn write_message(writer: &mut impl Write, message: &serde_json::Value) -> std::io::Result<()> {
    let body = serde_json::to_string(message)?;
    write!(writer, "Content-Length: {}\r\n\r\n{body}", body.len())?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_intervals_carve_around_occupied_stretches() {
        let occupied = [(5, 10), (12, 20)];
        assert_eq!(
            free_intervals(0, 25, &occupied),
            [(0, 5), (10, 12), (20, 25)]
        );
        assert_eq!(free_intervals(6, 9, &occupied), []);
        assert_eq!(free_intervals(10, 12, &occupied), [(10, 12)]);
    }

    #[test]
    fn passthrough_maps_as_one_verbatim_span() {
        let source = "const n: number = 1;\n";
        let emit = ttc::emit_mapped(source);
        let spans = span_mappings(&emit.mappings, &emit.anchors);
        assert_eq!(
            spans,
            [serde_json::json!([
                0,
                source.len(),
                0,
                source.len(),
                SPAN_VERBATIM
            ])]
        );
    }

    #[test]
    fn glue_maps_to_its_construct_without_features() {
        let source =
            "variant E { A(x: number), B }\nconst v = match (E.A(1)) { A(x) => x, B => 0 };\n";
        let emit = ttc::emit_mapped(source);
        let spans = span_mappings(&emit.mappings, &emit.anchors);
        // Spans are sorted and non-overlapping in the virtual text.
        let mut last_end = 0u64;
        for span in &spans {
            let start = span[0].as_u64().unwrap();
            let length = span[1].as_u64().unwrap();
            assert!(start >= last_end, "overlap at {start}");
            last_end = start + length;
        }
        // The lowering wrote glue, and every glue span is Atom + no features.
        let atoms: Vec<_> = spans
            .iter()
            .filter(|s| s[4].as_u64() == Some(SPAN_ATOM))
            .collect();
        assert!(!atoms.is_empty(), "a match lowering has glue");
        for atom in atoms {
            assert_eq!(atom[5].as_u64(), Some(FEATURES_NONE));
        }
    }

    #[test]
    fn code_numbers_are_stable_and_start_at_one() {
        assert_eq!(code_number("stray-pipe"), 1);
        assert_eq!(code_number("match-not-exhaustive"), 27);
        assert_eq!(code_number("never-heard-of-it"), 0);
    }

    /// A scratch directory for one case, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|since| since.as_nanos())
                .unwrap_or_default();
            let path =
                std::env::temp_dir().join(format!("tt-cm-{tag}-{}-{nonce}", std::process::id()));
            std::fs::create_dir_all(&path).expect("a writable temporary directory");
            Scratch(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn session() -> Session {
        Session {
            open_projects: HashSet::new(),
            ensured_roots: HashSet::new(),
        }
    }

    fn request(method: &str, params: serde_json::Value) -> serde_json::Value {
        serde_json::json!({ "jsonrpc": "2.0", "id": "api1", "method": method, "params": params })
    }

    #[test]
    fn framed_messages_round_trip() {
        let mut wire = Vec::new();
        let message = serde_json::json!({ "id": "api1", "method": "initialize" });
        write_message(&mut wire, &message).unwrap();
        assert!(wire.starts_with(b"Content-Length: "));
        let mut reader = std::io::Cursor::new(wire);
        assert_eq!(read_message(&mut reader).unwrap(), Some(message));
        // The stream ends cleanly after the one message.
        assert_eq!(read_message(&mut reader).unwrap(), None);
    }

    #[test]
    fn framing_tolerates_extra_headers_and_case() {
        let body = r#"{"id":1}"#;
        let wire = format!(
            "Content-Type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        );
        let mut reader = std::io::Cursor::new(wire.into_bytes());
        assert_eq!(
            read_message(&mut reader).unwrap(),
            Some(serde_json::json!({ "id": 1 }))
        );
    }

    #[test]
    fn framing_reports_a_broken_stream() {
        // A frame without Content-Length cannot be read past.
        let mut reader = std::io::Cursor::new(b"\r\n{}".to_vec());
        assert!(read_message(&mut reader).is_err());
        // EOF inside a header block is a died-mid-message peer.
        let mut reader = std::io::Cursor::new(b"Content-Length: 5\r\n".to_vec());
        assert!(read_message(&mut reader).is_err());
        // A body shorter than its declared length is the same.
        let mut reader = std::io::Cursor::new(b"Content-Length: 99\r\n\r\n{}".to_vec());
        assert!(read_message(&mut reader).is_err());
    }

    #[test]
    fn initialize_picks_utf8_and_names_the_tt_source() {
        let result = initialize(&serde_json::json!({
            "positionEncodings": ["utf-8", "utf-16"],
        }))
        .unwrap();
        assert_eq!(result["positionEncoding"], "utf-8");
        assert_eq!(result["diagnosticSource"], "tt");
    }

    #[test]
    fn initialize_refuses_a_peer_without_utf8() {
        let error = initialize(&serde_json::json!({ "positionEncodings": ["utf-16"] }))
            .expect_err("utf-8 is required");
        assert_eq!(error.code, INVALID_PARAMS);
    }

    #[test]
    fn unknown_and_missing_methods_answer_method_not_found() {
        let mut session = session();
        let error = respond(&mut session, &request("shutdown", serde_json::json!({})))
            .expect_err("unknown method");
        assert_eq!(error.code, METHOD_NOT_FOUND);
        let error =
            respond(&mut session, &serde_json::json!({ "id": 1 })).expect_err("no method at all");
        assert_eq!(error.code, METHOD_NOT_FOUND);
    }

    #[test]
    fn missing_string_params_answer_invalid_params() {
        let mut session = session();
        for method in ["openProject", "closeProject", "transform"] {
            let error = respond(&mut session, &request(method, serde_json::json!({})))
                .expect_err("params are required");
            assert_eq!(error.code, INVALID_PARAMS, "{method}");
        }
    }

    #[test]
    fn open_project_tracks_the_handle_and_materializes_std() {
        let scratch = Scratch::new("open");
        std::fs::write(scratch.path().join("package.json"), "{}\n").unwrap();
        let config = scratch.path().join("tsconfig.json");
        let mut session = session();

        let result = respond(
            &mut session,
            &request(
                "openProject",
                serde_json::json!({
                    "configFileName": config.to_str().unwrap(),
                    "projectHandle": "p:0",
                }),
            ),
        )
        .unwrap();
        assert_eq!(result, serde_json::json!({}));
        assert!(session.open_projects.contains("p:0"));
        let std_entry = scratch.path().join("node_modules/@tt/std/index.ts");
        assert!(std_entry.exists());
        assert!(
            scratch
                .path()
                .join("node_modules/@tt/runtime/index.ts")
                .exists()
        );

        // Never over one that already exists: a project's own copy stays.
        std::fs::write(&std_entry, "// mine\n").unwrap();
        session.ensured_roots.clear();
        ensure_std_packages(&mut session, scratch.path());
        assert_eq!(std::fs::read_to_string(&std_entry).unwrap(), "// mine\n");

        let result = respond(
            &mut session,
            &request(
                "closeProject",
                serde_json::json!({ "projectHandle": "p:0" }),
            ),
        )
        .unwrap();
        assert_eq!(result, serde_json::json!({}));
        assert!(!session.open_projects.contains("p:0"));
    }

    #[test]
    fn a_project_without_a_config_file_opens_too() {
        let mut session = session();
        let result = respond(
            &mut session,
            &request(
                "openProject",
                serde_json::json!({ "configFileName": "", "projectHandle": "p:1" }),
            ),
        )
        .unwrap();
        assert_eq!(result, serde_json::json!({}));
    }

    #[test]
    fn transform_serves_a_variant_file_as_virtual_typescript() {
        let scratch = Scratch::new("transform");
        std::fs::write(scratch.path().join("package.json"), "{}\n").unwrap();
        let file = scratch.path().join("shape.tt");
        let source = "export variant Shape { Circle(radius: number), Point }\n\
             export const tag = (s: Shape) => match (s) { Circle(radius) => \"c\", Point => \"p\" };\n";
        let mut session = session();

        let result = respond(
            &mut session,
            &request(
                "transform",
                serde_json::json!({
                    "fileName": file.to_str().unwrap(),
                    "content": source,
                    "projectHandle": "p:0",
                }),
            ),
        )
        .unwrap();
        assert_eq!(result["extension"], ".ts");
        assert_eq!(result["diagnostics"], serde_json::json!([]));
        let text = result["text"].as_str().unwrap();
        assert!(text.contains("kind: \"Circle\""));
        let mappings = result["mappings"].as_array().unwrap();
        assert!(
            mappings
                .iter()
                .any(|m| m[4].as_u64() == Some(SPAN_VERBATIM))
        );
        assert!(mappings.iter().any(|m| m[4].as_u64() == Some(SPAN_ATOM)));
    }

    #[test]
    fn transform_reads_one_hop_imports_for_exhaustiveness() {
        let scratch = Scratch::new("extern");
        std::fs::write(
            scratch.path().join("shape.tt"),
            "export variant Shape { Circle(radius: number), Rect(width: number, height: number) }\n",
        )
        .unwrap();
        let file = scratch.path().join("partial.tt");
        let source = "import { Shape } from \"./shape.tt\";\n\
             export const tag = (s: Shape): string => match (s) { Circle(radius) => \"c\" };\n";
        let mut session = session();

        let result = respond(
            &mut session,
            &request(
                "transform",
                serde_json::json!({
                    "fileName": file.to_str().unwrap(),
                    "content": source,
                    "projectHandle": "p:0",
                }),
            ),
        )
        .unwrap();
        let diagnostics = result["diagnostics"].as_array().unwrap();
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0]["code"], code_number("match-not-exhaustive"));
        let message = diagnostics[0]["messageText"].as_str().unwrap();
        assert!(message.contains("not exhaustive"), "{message}");
        assert!(
            message.contains("help:"),
            "suggestions ride along: {message}"
        );
        // The diagnostic sits at the match keyword, as byte offsets.
        let start = diagnostics[0]["start"].as_u64().unwrap() as usize;
        assert_eq!(&source[start..start + 5], "match");
    }

    #[test]
    fn namespace_imports_qualify_their_extern_variants() {
        let scratch = Scratch::new("ns");
        std::fs::write(
            scratch.path().join("dep.tt"),
            "export variant Mode { Fast(), Safe }\n",
        )
        .unwrap();
        let imports = [
            ttc::TtImport {
                specifier: "./dep.tt".to_string(),
                names: ttc::TtImportNames::Namespace("dep".to_string()),
            },
            ttc::TtImport {
                specifier: "./missing.tt".to_string(),
                names: ttc::TtImportNames::Named(vec![("Gone".to_string(), None)]),
            },
            ttc::TtImport {
                specifier: "./dep.tt".to_string(),
                names: ttc::TtImportNames::None,
            },
        ];
        let externs = collect_extern_variants(&scratch.path().join("main.tt"), &imports);
        assert_eq!(externs.len(), 1);
        assert_eq!(externs[0].name, "dep.Mode");
    }

    #[test]
    fn a_blocked_projection_serves_an_empty_module_with_the_cause() {
        let mut session = session();
        let result = respond(
            &mut session,
            &request(
                "transform",
                serde_json::json!({
                    "fileName": "/nonexistent/broken.tt",
                    "content": "export variant Broken { Value(x: number]) }\n",
                    "projectHandle": "p:0",
                }),
            ),
        )
        .unwrap();
        assert_eq!(result["text"], "");
        assert_eq!(result["mappings"], serde_json::json!([]));
        let diagnostics = result["diagnostics"].as_array().unwrap();
        assert!(!diagnostics.is_empty());
        assert_eq!(
            diagnostics[0]["code"],
            code_number("variant-invalid-field-type")
        );
    }

    #[test]
    fn a_ttx_file_serves_as_tsx() {
        let mut session = session();
        let result = respond(
            &mut session,
            &request(
                "transform",
                serde_json::json!({
                    "fileName": "/nonexistent/view.ttx",
                    "content": "export const view = <div>hello</div>;\n",
                    "projectHandle": "p:0",
                }),
            ),
        )
        .unwrap();
        assert_eq!(result["extension"], ".tsx");
        assert_eq!(result["text"], "export const view = <div>hello</div>;\n");
    }

    #[test]
    fn std_imports_materialize_next_to_the_nearest_package_root() {
        let scratch = Scratch::new("std");
        std::fs::write(scratch.path().join("package.json"), "{}\n").unwrap();
        let nested = scratch.path().join("src/deep");
        std::fs::create_dir_all(&nested).unwrap();
        let mut session = session();

        respond(
            &mut session,
            &request(
                "transform",
                serde_json::json!({
                    "fileName": nested.join("opt.tt").to_str().unwrap(),
                    "content": "import * as Option from \"@tt/std/option\";\nexport const some = Option.Some(1);\n",
                    "projectHandle": "p:0",
                }),
            ),
        )
        .unwrap();
        assert!(
            scratch
                .path()
                .join("node_modules/@tt/std/option.ts")
                .exists()
        );
    }

    #[test]
    fn package_root_walks_to_a_marker_or_gives_up() {
        let scratch = Scratch::new("root");
        let nested = scratch.path().join("a/b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(scratch.path().join("package.json"), "{}\n").unwrap();
        assert_eq!(
            package_root(&nested.join("x.tt")),
            Some(scratch.path().to_path_buf())
        );
        assert_eq!(package_root(Path::new("/no/such/tree/x.tt")), None);
    }

    #[test]
    fn positionless_diagnostics_serialize_as_zero_spans() {
        let diagnostic = &ttc::analyze("export variant Dup { A, A }\n", &Options::default())[0];
        let wire = mapper_diagnostic(diagnostic);
        assert_eq!(wire["code"], code_number("variant-duplicate-case"));
        assert!(wire["start"].as_u64().is_some());
        assert!(wire["length"].as_u64().is_some());
    }
}
