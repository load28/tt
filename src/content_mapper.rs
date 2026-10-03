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

#[cfg(test)]
use ttc::content_projection::{FEATURES_NONE, SPAN_ATOM, SPAN_VERBATIM, free_intervals};
use ttc::content_projection::{MapperExchange, mapper_diagnostic, span_mappings};
use ttc::{ImportRewrite, Options, Severity, SourceKind};

/// Everything the mapper keeps between requests.
struct Session {
    exchange: Option<MapperExchange>,
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
    let exchange = match MapperExchange::from_environment() {
        Ok(exchange) => exchange,
        Err(error) => {
            eprintln!("ttc --content-mapper: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut session = Session {
        exchange,
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
    if let Some(exchange) = &session.exchange
        && let Some(record) = exchange
            .lookup(path, &content)
            .map_err(|message| RpcError {
                code: INTERNAL_ERROR,
                message,
            })?
    {
        if (record.response["text"]
            .as_str()
            .is_some_and(|text| text.contains("@tt/"))
            || content.contains("@tt/"))
            && let Some(root) = package_root(path)
        {
            ensure_std_packages(session, &root);
        }
        return Ok(record.response);
    }

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
    let report = ttc::compile_projection_report(&content, &options);

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
            let mappings = span_mappings(&emit.mappings, &emit.anchors, &report.recovered);
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
    for package in ttc::StdPackage::ALL {
        let _ = package.materialize(root);
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
mod tests;
