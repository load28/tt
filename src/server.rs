//! `ttc --server` — the engine behind a pipe, for tools that ask often.
//!
//! An editor asks the compiler the same three questions on every keystroke:
//! "does this buffer pass `--check`?", "what does it emit?" and "what does
//! the typed layer say?". Answering each by spawning a process is fine for
//! the first two and ruinous for the third — a typed check opens a project
//! and starts a TypeScript compiler. This mode keeps one `ttc` process
//! alive and, behind it, one [`ttc::engine::Project`] per project identity,
//! so a typed check after the first reuses the running compiler and every
//! unchanged file's projection.
//!
//! The protocol is one JSON object per line, on stdin and stdout. Every
//! non-blank line is answered by exactly one line, in the order the lines
//! arrived, so an answer whose `id` is `null` answers the oldest line still
//! unanswered:
//!
//! ```text
//! → { "id": 1, "method": "check", "params": { "text", "filename"?, "verify"? } }
//! ← { "id": 1, "result": { "diagnostics":
//!        [{ "line", "col", "endLine", "endCol", "message", "code",
//!           "suggestions": [{ "message", "edit": { "line", "col",
//!             "endLine", "endCol", "replacement" } | null }],
//!           "labels"?: [{ "line", "col", "endLine", "endCol",
//!             "message" }] }] } }
//!
//! → { "id": 2, "method": "emitMap", "params": { "text", "filename"? } }
//! ← { "id": 2, "result": { "code", "mappings": [{ "src", "out", "len" }] } }
//!
//! → { "id": 3, "method": "typedCheck",
//!     "params": { "path", "text", "scope"?, "supersedable"? } }
//! ← { "id": 3, "result": { "blocked", "diagnostics":
//!        [{ "path", "line", "col", "endLine", "endCol", "message", "code",
//!           "suggestions" }] } }
//! `"scope": "file"` checks the buffer as a language service checks the
//! file an editor shows: its own diagnostics, its TypeScript checked
//! whenever its own syntax parses, whatever another file's does.
//! `blocked`: the pass checked none of the buffer's TypeScript — the
//! project could not be read, or the buffer could not be lowered and its
//! diagnostics are its tt-level ones alone.
//!
//! → { "id": 4, "method": "semanticTokens", "params": { "text" } }
//! ← { "id": 4, "result": { "tokens": [{ "range", "kind", "modifiers" }] } }
//!
//! → { "method": "prepareRename", "params": { "path", "position" } }
//! ← { "result": { "range" } | { "range": null, "refusal": string | null } }
//! What the rename at the position replaces, or its refusal, with
//! TypeScript's reason when it gave one.
//!
//! → { "method": "documentSemanticTokens", "params": { "path" } }
//! ← { "result": { "tokens": [{ "range", "type", "modifiers" }] } }
//! TypeScript's classification of the source text the emission copied,
//! with the parser's classification of tt's constructs over it.
//!
//! → { "id": 5, "method": "ttSymbol", "params": { "path", "text", "position" } }
//! ← { "id": 5, "result": { "kind", "range", "name", "variantName",
//!                          "signature", "detail", "definition", "binds" } | null }
//!
//! → { "id": 6, "method": "ttCompletions", "params": { "path", "text", "position" } }
//! ← { "id": 6, "result": { "items": [{ "label", "kind", "detail", "covered", "range" }],
//!                          "member": { "receiver" } | null,
//!                          "keywords": [{ "label", "sortText" }], "pattern" } }
//! `member`: the cursor completes a member name; `receiver` is the path of
//! names before the `.` (`Result`, `ns.Shape`), or null for any other
//! expression. `keywords`: the tt keywords whose construct can be written
//! at the position, with TypeScript's rank for a keyword. `pattern`: the
//! position is a pattern position tt completes.
//!
//! → { "method": "patternCompletions", "params": { "path", "position" } }
//! ← { "result": { "items": [{ "label", "kind", "detail", "covered", "range" }] } | null }
//! The pattern completions at a pattern position with what the scrutinee's
//! type admits, from the project's TypeScript; null elsewhere.
//!
//! → { "id": 7, "method": "ttHints", "params": { "path", "text" } }
//! ← { "id": 7, "result": { "hints": [{ "kind", "range", "message" }] } }
//!
//! → { "id": 8, "method": "declarations", "params": { "path", "text" } }
//! ← { "id": 8, "result": { "variants": [{ "name", "generics", "origin",
//!        "specifier", "nameSpan", "span", "cases" }],
//!        "matches": [{ "keyword", "bodyOpen", "bodyClose" }] } }
//! `ttSymbol`, `ttCompletions`, `ttHints` and `declarations` answer from
//! `text` without a project; the `.tt` files it imports are read as the
//! session holds them open, else from disk.
//!
//! → { "id": 9, "method": "reloadProjects", "params": {} }
//! ← { "id": 9, "result": {} }
//! Project graphs and registered overlays are released. The client must
//! replay its openDocument notifications before subsequent semantic requests.
//!
//! → { "id": 10, "method": "print", "params": { "path", "sourceMap"?,
//!        "rewriteImports"?, "banner"?, "verify"? } }
//! ← { "id": 10, "result": { "code": string | null, "messages": [string] } }
//! `ttc -p` for the file on disk: `code` is what it prints on stdout and is
//! null exactly when it exits unsuccessfully; `messages` is each message it
//! writes to stderr. The options are its flags — `sourceMap` "off"
//! (default) or "inline", `rewriteImports` "js" (default), "ts" or "off",
//! `banner: false` for `--no-banner`, `verify: false` for `--no-verify`.
//!
//! → { "id": 11, "method": "dependencies", "params": { "path" } }
//! ← { "id": 11, "result": { "files": [string], "directories": [string] } }
//! `ttc --dependencies` for the file: the files whose change invalidates
//! its compile, and the directories where a file added or removed does.
//!
//! A `typedCheck` or `tsDiagnostics` request whose params carry
//! `"supersedable": true` describes the documents as they are when it is
//! answered. When a document change (`openDocument`, `updateDocument`,
//! `closeDocument`, `reloadProjects`) has already arrived behind it, the
//! answer would be stale, and the server answers without computing it:
//! ← { "id": N, "superseded": true }
//!
//! ← { "id": N, "error": "sentence" }   // the request failed; the session lives
//! ← { "id": null, "error": "sentence" } // a line with no id the server can read
//! ```
//!
//! Every answer is computed by the same code the one-shot modes run —
//! `check` is [`ttc::compile`] with the caller's text standing alone (its
//! relative imports unresolvable, exactly like the one-shot's temp file),
//! `emitMap` is [`ttc::emit_mapped`], and `typedCheck` is the engine's
//! tt-only pass with the buffer as an overlay — so a consumer that falls
//! back from the server to the one-shot commands sees the same diagnostics
//! either way. A `typedCheck` overlay lasts one request: the answer is
//! stateless, the reuse (projection cache, running compiler) is not.
//!
//! `print` is the command line's own `-p` compile, run in this process, so
//! the TypeScript project that refines the generated storage annotations
//! (`docs/design/contextual-type-materialization.md`) is opened by the
//! first request and reused by every later one; a bundler asking once per
//! module pays for it once per build. `dependencies` checks the file's live
//! project, and checks it again only when one of the project's watch paths
//! has changed since.
//!
//! Exit: end of stdin, code 0. A failed request never ends the session.

mod responses;

use responses::{
    completion_detail_json, completion_json, labels_json, location_json, pattern_items_json,
    range_json, service_diagnostic_json, signature_help_json, suggestions_json, symbol_json,
};
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::SystemTime;
use ttc::lines::ProtocolPositions;

use ttc::engine::{CheckRequest, Engine, Position, Project, SignatureTrigger, Workspace};

/// Runs the server until stdin closes.
pub(crate) fn run(node: Option<PathBuf>) -> ExitCode {
    // One live project per identity, and the documents a consumer holds
    // open in them — what a server exists to keep between requests.
    let mut workspace = Workspace::new(Engine::new(node.clone()));
    let mut checks = Checks::default();

    let stdout = std::io::stdout();
    // Lines are read as they arrive, so a request can see the document
    // changes queued behind it (TypeScript's `changeSeq`).
    let (sender, lines) = std::sync::mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut input = stdin.lock();
        loop {
            let mut bytes = Vec::new();
            match input.read_until(b'\n', &mut bytes) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            if sender.send(bytes).is_err() {
                break;
            }
        }
    });
    let mut queued: VecDeque<Vec<u8>> = VecDeque::new();
    loop {
        let bytes = match queued.pop_front() {
            Some(bytes) => bytes,
            None => match lines.recv() {
                Ok(bytes) => bytes,
                Err(_) => break,
            },
        };
        let line = match std::str::from_utf8(&bytes) {
            Ok(line) => line.trim_end_matches(['\n', '\r']),
            Err(error) => {
                let response = serde_json::json!({
                    "id": request_id(&String::from_utf8_lossy(&bytes)),
                    "error": format!("malformed request: the line is not UTF-8: {error}"),
                });
                let mut out = stdout.lock();
                if writeln!(out, "{response}")
                    .and_then(|_| out.flush())
                    .is_err()
                {
                    break;
                }
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        if supersedable(line) {
            queued.extend(lines.try_iter());
            if superseded(line, &queued) {
                let response = serde_json::json!({ "id": request_id(line), "superseded": true });
                let mut out = stdout.lock();
                if writeln!(out, "{response}")
                    .and_then(|_| out.flush())
                    .is_err()
                {
                    break;
                }
                continue;
            }
        }
        // A panic in one request is a bug in the compiler, not the end of
        // the session: the protocol promises that a failed request never
        // ends the session, and a panic is a failed request. The report is
        // already on stderr; stdout carries the answer, so the consumer
        // sees an error for this id and can ask the next question.
        //
        // Unwind safety: the workspace is kept. A panic aborts the work
        // of one request, and what that work builds — a snapshot — is
        // immutable and installed whole or not at all, so the projects the
        // workspace holds are the ones the last successful request left.
        let response = match ttc::ice::catching(|| {
            respond(&mut workspace, &mut checks, node.as_deref(), line)
        }) {
            Ok(response) => response,
            Err(message) => serde_json::json!({
                "id": request_id(line),
                "error": ttc::ice::bug_message(&message),
            }),
        };
        let mut out = stdout.lock();
        if writeln!(out, "{response}")
            .and_then(|_| out.flush())
            .is_err()
        {
            break; // the consumer is gone
        }
    }
    ExitCode::SUCCESS
}

/// The members of a request line, values unparsed.
fn members(line: &str) -> Option<HashMap<String, Box<serde_json::value::RawValue>>> {
    serde_json::from_str(line).ok()
}

/// A request whose answer describes one document revision and that the
/// consumer marked `"supersedable": true`: a later document change makes
/// the answer stale before it is computed, as a document change aborts
/// TypeScript's pending error check (`session.ts`, `changeSeq`).
fn supersedable(line: &str) -> bool {
    let Some(members) = members(line) else {
        return false;
    };
    let method = members
        .get("method")
        .and_then(|method| serde_json::from_str::<String>(method.get()).ok());
    matches!(method.as_deref(), Some("typedCheck" | "tsDiagnostics"))
        && members
            .get("params")
            .and_then(|params| {
                serde_json::from_str::<HashMap<String, Box<serde_json::value::RawValue>>>(
                    params.get(),
                )
                .ok()
            })
            .and_then(|params| params.get("supersedable").map(|flag| flag.get() == "true"))
            .unwrap_or(false)
}

/// Whether a document change queued behind `line` makes its answer stale.
fn superseded(line: &str, queued: &VecDeque<Vec<u8>>) -> bool {
    supersedable(line) && queued.iter().any(|later| changes_documents(later))
}

/// Whether a queued line changes the documents or projects every answer
/// is computed from.
fn changes_documents(bytes: &[u8]) -> bool {
    let Some(members) = std::str::from_utf8(bytes).ok().and_then(members) else {
        return false;
    };
    members
        .get("method")
        .and_then(|method| serde_json::from_str::<String>(method.get()).ok())
        .is_some_and(|method| {
            matches!(
                method.as_str(),
                "openDocument" | "updateDocument" | "closeDocument" | "reloadProjects"
            )
        })
}

/// The `id` of a request the server could not answer.
///
/// A response has to carry the id it answers or the consumer cannot match
/// it to its question; when the request's id cannot be read, `null` is the
/// protocol's own answer for "no id".
fn request_id(line: &str) -> serde_json::Value {
    serde_json::from_str::<HashMap<String, Box<serde_json::value::RawValue>>>(line)
        .ok()
        .and_then(|members| serde_json::from_str(members.get("id")?.get()).ok())
        .unwrap_or(serde_json::Value::Null)
}

/// One request, one answer — errors included, so the session survives them.
fn respond(
    workspace: &mut Workspace,
    checks: &mut Checks,
    node: Option<&Path>,
    line: &str,
) -> serde_json::Value {
    use serde_json::json;
    ttc::ice::panic_for_test("server");
    let request: serde_json::Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(e) => {
            return json!({ "id": request_id(line), "error": format!("malformed request: {e}") });
        }
    };
    let id = request["id"].clone();
    let params = &request["params"];
    let result = match request["method"].as_str().unwrap_or_default() {
        "check" => check(params),
        "print" => print(params, node),
        "dependencies" => dependencies(workspace, checks, params),
        "emitMap" => emit_map(params),
        "typedCheck" => typed_check(workspace, params),
        "openDocument" | "updateDocument" => {
            *checks = Checks::default();
            open_document(workspace, params)
        }
        "closeDocument" => {
            *checks = Checks::default();
            close_document(workspace, params)
        }
        "reloadProjects" => {
            *checks = Checks::default();
            // Filesystem/configuration topology changed. Clients replay open
            // buffers after this ordered barrier; old snapshots cannot leak
            // into a graph resolved against the new configuration.
            workspace.reload();
            Ok(json!({}))
        }
        "hover" => semantic(workspace, params, |project, path, position| {
            Ok(match project.hover(path, position)? {
                None => serde_json::Value::Null,
                Some(info) => json!({
                    "signature": info.signature,
                    "documentation": info.documentation,
                    "range": range_json(info.range),
                }),
            })
        }),
        "definition" => semantic(workspace, params, |project, path, position| {
            let locations: Vec<_> = project
                .definition(path, position)?
                .into_iter()
                .map(location_json)
                .collect();
            Ok(json!({ "locations": locations }))
        }),
        "references" => spanning(workspace, params, |workspace, path, position| {
            let locations: Vec<_> = workspace
                .references(path, position)?
                .into_iter()
                .map(|reference| {
                    let mut value = location_json(reference.location);
                    value["isDefinition"] = json!(reference.is_definition);
                    value
                })
                .collect();
            Ok(json!({ "locations": locations }))
        }),
        "completion" => semantic(workspace, params, |project, path, position| {
            let member = params["member"].as_bool().unwrap_or(false);
            let trigger = params["triggerCharacter"].as_str();
            let answer = project.triggered_completion(path, position, member, trigger)?;
            Ok(completion_json(answer))
        }),
        "completionResolve" => semantic(workspace, params, |project, path, position| {
            let label = params["label"].as_str().unwrap_or_default();
            let source = params["source"].as_str();
            let probe = params["probe"].as_u64();
            let detail = project.completion_resolve(path, position, label, source, probe)?;
            Ok(completion_detail_json(detail))
        }),
        "rename" => spanning(workspace, params, |workspace, path, position| {
            Ok(match workspace.rename(path, position)? {
                None => json!({ "edits": serde_json::Value::Null }),
                Some(edits) => json!({
                    "edits": edits.into_iter().map(|edit| {
                        let mut value = location_json(edit.location);
                        value["newText"] = match edit.new_text {
                            Some(text) => json!(text),
                            None => serde_json::Value::Null,
                        };
                        value
                    }).collect::<Vec<_>>(),
                }),
            })
        }),
        "prepareRename" => spanning(workspace, params, |workspace, path, position| {
            Ok(match workspace.prepare_rename(path, position)? {
                ttc::engine::PrepareRename::Range(range) => json!({ "range": range_json(range) }),
                ttc::engine::PrepareRename::Refused(reason) => {
                    json!({ "range": null, "refusal": reason })
                }
            })
        }),
        "documentSymbols" => semantic(workspace, params, |project, path, _position| {
            Ok(
                json!({ "symbols": project.document_symbols(path)?.iter().map(symbol_json).collect::<Vec<_>>() }),
            )
        }),
        "signatureHelp" => semantic(workspace, params, |project, path, position| {
            let trigger = match (
                params["triggerKind"].as_u64(),
                params["triggerCharacter"].as_str(),
            ) {
                (Some(2), Some(character)) => SignatureTrigger::Character(character.to_string()),
                (Some(3), _) => SignatureTrigger::ContentChange,
                _ => SignatureTrigger::Invoked,
            };
            let retrigger = params["isRetrigger"].as_bool().unwrap_or(false);
            let help = project.triggered_signature_help(path, position, &trigger, retrigger)?;
            Ok(signature_help_json(help))
        }),
        "semanticTokens" => semantic_tokens(params),
        "patternCompletions" => semantic(workspace, params, |project, path, position| {
            Ok(match project.pattern_completions(path, position)? {
                None => serde_json::Value::Null,
                Some(items) => json!({ "items": pattern_items_json(&items) }),
            })
        }),
        "documentSemanticTokens" => semantic(workspace, params, |project, path, _position| {
            let tokens: Vec<_> = project
                .semantic_tokens(path)?
                .into_iter()
                .map(|token| {
                    json!({
                        "range": range_json(token.range),
                        "type": token.token_type,
                        "modifiers": token.modifiers,
                    })
                })
                .collect();
            Ok(json!({ "tokens": tokens }))
        }),
        "declarations" => declarations(workspace, params),
        "ttSymbol" => tt_symbol(workspace, params),
        "ttCompletions" => tt_completions(workspace, params),
        "ttHints" => tt_hints(workspace, params),
        "tsDiagnostics" => semantic(workspace, params, |project, path, _position| {
            let diagnostics: Vec<_> = project
                .service_diagnostics(path)?
                .into_iter()
                .map(service_diagnostic_json)
                .collect();
            // The tt diagnostics these state in TypeScript's own words: a
            // consumer showing both layers shows the fact once.
            let restates: Vec<_> = project
                .service_restates(path)?
                .into_iter()
                .map(|code| code.as_str())
                .collect();
            let retains: Vec<_> = project.service_retained_syntax(path)?.into_iter()
                .map(|cause| json!({ "code": cause.code.as_str(), "start": { "line": cause.range.start.line, "character": cause.range.start.character }, "range": range_json(cause.range), "message": cause.message }))
                .collect();
            Ok(json!({ "diagnostics": diagnostics, "restates": restates, "retains": retains }))
        }),
        method => Err(format!("unknown method \"{method}\"")),
    };
    match result {
        Ok(result) => json!({ "id": id, "result": result }),
        Err(error) => json!({ "id": id, "error": error }),
    }
}

/// Routes a semantic request to the live project the file belongs to. The
/// position defaults to 0:0 for the requests that do not carry one.
fn semantic(
    workspace: &mut Workspace,
    params: &serde_json::Value,
    handle: impl FnOnce(&mut Project, &Path, Position) -> Result<serde_json::Value, String>,
) -> Result<serde_json::Value, String> {
    spanning(workspace, params, |workspace, path, position| {
        handle(workspace.project_for(path)?, path, position)
    })
}

/// Hands a request whose answer can span projects to the workspace.
fn spanning(
    workspace: &mut Workspace,
    params: &serde_json::Value,
    handle: impl FnOnce(&mut Workspace, &Path, Position) -> Result<serde_json::Value, String>,
) -> Result<serde_json::Value, String> {
    let path = params["path"]
        .as_str()
        .ok_or_else(|| "the request needs a \"path\"".to_string())?;
    let position = Position {
        line: params["position"]["line"].as_u64().unwrap_or(0) as u32,
        character: params["position"]["character"].as_u64().unwrap_or(0) as u32,
    };
    handle(workspace, Path::new(path), position)
}

/// `openDocument` / `updateDocument`: the consumer's buffer stands in for
/// the file, in whichever project it belongs to, until `closeDocument`.
fn open_document(
    workspace: &mut Workspace,
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let path = params["path"]
        .as_str()
        .ok_or_else(|| "the request needs a \"path\"".to_string())?;
    workspace.open_document(Path::new(path), text_param(params)?.to_string())?;
    Ok(serde_json::json!({}))
}

/// `closeDocument`: the file's text is the disk's again.
fn close_document(
    workspace: &mut Workspace,
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let path = params["path"]
        .as_str()
        .ok_or_else(|| "the request needs a \"path\"".to_string())?;
    workspace.close_document(Path::new(path));
    Ok(serde_json::json!({}))
}

/// `--check` for a buffer: tt-level diagnostics from the text alone.
fn check(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let text = text_param(params)?;
    let filename = params["filename"].as_str();
    let options = ttc::Options {
        filename,
        source_kind: filename
            .and_then(|name| ttc::SourceKind::from_path(std::path::Path::new(name)))
            .unwrap_or_default(),
        verify: params["verify"].as_bool().unwrap_or(true),
        ..ttc::Options::default()
    };
    // Every tt-level diagnostic of the buffer, in source order (TASK-120).
    // Wrapped in `working_on` so a panic in here names the buffer it was
    // reading rather than "some file" (TASK-214).
    // `endLine`/`endCol` close the range the diagnostic covers — the
    // construct as written. Zero means "position only": the consumer
    // decides the width. `code` is the rule's stable identity.
    let report = ttc::ice::working_on(Path::new(filename.unwrap_or("<buffer>")), || {
        ttc::check_report(text, &options)
    });
    let positions = ProtocolPositions::new(text);
    let diagnostics: Vec<_> = report
        .diagnostics
        .iter()
        .map(|d| {
            let at = |offset: Option<usize>| offset.map_or((0, 0), |at| positions.of_byte(at));
            let (line, col) = at(d.start);
            let (end_line, end_col) = at(d.end);
            let mut entry = json!({
                "line": line,
                "col": col,
                "endLine": end_line,
                "endCol": end_col,
                "message": d.message,
                "code": d.code.as_str(),
                "suggestions": suggestions_json(&d.suggestions, Some(text)),
            });
            if !d.labels.is_empty() {
                entry["labels"] = d
                    .labels
                    .iter()
                    .map(|label| {
                        let (line, col) = positions.of_byte(label.start);
                        let (end_line, end_col) = positions.of_byte(label.end);
                        json!({
                            "line": line,
                            "col": col,
                            "endLine": end_line,
                            "endCol": end_col,
                            "message": label.message,
                        })
                    })
                    .collect();
            }
            entry
        })
        .collect();
    Ok(json!({ "diagnostics": diagnostics }))
}

/// Semantic tokens for a buffer: the parser's own classification of the
/// ambiguous surface, in the buffer's coordinates. Like `check`, this is
/// stateless and parse-only — it needs no project and no TypeScript
/// toolchain, so the editor's colors stay exact in every environment.
/// The declarations visible in a buffer — the compiler's own variant table
/// (local, imported, built-in, under the compiler's shadowing) plus the
/// buffer's `match` sites. This is the surface that replaces the editor's
/// regex re-implementation of tt semantics (`engine::tt_declarations`).
/// Text-only; `path` resolves the buffer's relative `.tt` imports, read as
/// the session holds them open.
fn declarations(
    workspace: &Workspace,
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let path = params["path"]
        .as_str()
        .ok_or_else(|| "the request needs a \"path\"".to_string())?;
    let text = text_param(params)?;
    let decls = workspace.tt_declarations(Path::new(path), text);
    // The engine measures these in bytes; a consumer addresses the buffer
    // in UTF-16 code units, which is what the protocol counts. One
    // conversion here keeps the two from disagreeing about where a name is
    // (`docs/design/lsp-architecture.md` §C).
    let offsets = ttc::Utf16Offsets::new(text);
    let offset = |byte: usize| offsets.offset(byte);
    let span = |bounds: (usize, usize)| serde_json::json!({ "start": offset(bounds.0), "end": offset(bounds.1) });
    let variants: Vec<_> = decls
        .variants
        .iter()
        .map(|e| {
            let (origin, specifier, name_span, declaration_span) = match &e.origin {
                ttc::engine::TtVariantOrigin::Local { name_span, span } => (
                    "local",
                    serde_json::Value::Null,
                    Some(*name_span),
                    Some(*span),
                ),
                ttc::engine::TtVariantOrigin::Imported { specifier } => (
                    "imported",
                    specifier
                        .clone()
                        .map(serde_json::Value::String)
                        .unwrap_or(serde_json::Value::Null),
                    None,
                    None,
                ),
                ttc::engine::TtVariantOrigin::Builtin => {
                    ("builtin", serde_json::Value::Null, None, None)
                }
            };
            json!({
                "name": e.name,
                "generics": e.generics,
                "origin": origin,
                "specifier": specifier,
                "nameSpan": name_span.map(&span),
                "span": declaration_span.map(&span),
                "cases": e.cases.iter().map(|c| json!({
                    "tag": c.tag,
                    "nameSpan": c.name_span.map(&span),
                    "span": c.span.map(&span),
                    "unit": c.unit,
                    "fields": c.fields.iter().map(|f| json!({
                        "name": f.name,
                        "optional": f.optional,
                        "ty": f.ty,
                    })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let matches: Vec<_> = decls
        .matches
        .iter()
        .map(|m| {
            json!({
                "keyword": offset(m.keyword),
                "bodyOpen": offset(m.body_open),
                "bodyClose": offset(m.body_close),
            })
        })
        .collect();
    Ok(json!({ "variants": variants, "matches": matches }))
}

/// The tt name at a position — a variant, a case tag, a payload field.
///
/// Text-only like `semanticTokens`: the answer needs no project and no
/// toolchain, because these names exist nowhere in the emitted TypeScript
/// and are tt's to answer (`engine::names`). `path` is still required, to
/// resolve the file's relative `.tt` imports, which are read as the session
/// holds them open.
fn tt_symbol(
    workspace: &Workspace,
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let path = params["path"]
        .as_str()
        .ok_or_else(|| "the request needs a \"path\"".to_string())?;
    let position = Position {
        line: params["position"]["line"].as_u64().unwrap_or(0) as u32,
        character: params["position"]["character"].as_u64().unwrap_or(0) as u32,
    };
    let Some(symbol) = workspace.tt_symbol_at(Path::new(path), text_param(params)?, position)
    else {
        return Ok(serde_json::Value::Null);
    };
    Ok(json!({
        "kind": match symbol.kind {
            ttc::engine::TtSymbolKind::Variant => "variant",
            ttc::engine::TtSymbolKind::Case => "case",
            ttc::engine::TtSymbolKind::Field => "field",
        },
        "range": range_json(symbol.range),
        "name": symbol.name,
        "variantName": symbol.variant_name,
        "signature": symbol.signature,
        "detail": symbol.detail,
        "definition": symbol.definition.map(|location| json!({
            "path": location.path.to_string_lossy(),
            "range": range_json(location.range),
        })),
        "binds": symbol.binds,
    }))
}

/// What can be written at a pattern position — case tags, payload field
/// names. Text-only, for the same reason [`tt_symbol`] is.
fn tt_completions(
    workspace: &Workspace,
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let path = params["path"]
        .as_str()
        .ok_or_else(|| "the request needs a \"path\"".to_string())?;
    let position = Position {
        line: params["position"]["line"].as_u64().unwrap_or(0) as u32,
        character: params["position"]["character"].as_u64().unwrap_or(0) as u32,
    };
    let member = ttc::engine::member_access_at(Path::new(path), text_param(params)?, position)
        .map(|access| json!({ "receiver": access.receiver }));
    let items = pattern_items_json(&workspace.tt_completions_at(
        Path::new(path),
        text_param(params)?,
        position,
    ));
    let pattern = workspace.is_pattern_position(Path::new(path), text_param(params)?, position);
    let keywords: Vec<_> =
        ttc::engine::tt_keywords_at(Path::new(path), text_param(params)?, position)
            .into_iter()
            .map(|keyword| json!({ "label": keyword.label(), "sortText": keyword.sort_text() }))
            .collect();
    Ok(json!({ "items": items, "member": member, "keywords": keywords, "pattern": pattern }))
}

/// What tt has to say about a buffer that is not an error — today, the
/// arms an earlier arm already covers. Text-only like [`tt_symbol`], and
/// separate from `check` on purpose: a hint never fails a build, so it
/// never travels in the diagnostics of a compile answer.
fn tt_hints(
    workspace: &Workspace,
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let path = params["path"]
        .as_str()
        .ok_or_else(|| "the request needs a \"path\"".to_string())?;
    let hints: Vec<_> = workspace
        .tt_hints(Path::new(path), text_param(params)?)
        .into_iter()
        .map(|hint| {
            json!({
                "kind": match hint.kind {
                    ttc::engine::TtHintKind::UnreachableArm => "unreachableArm",
                },
                "range": range_json(hint.range),
                "message": hint.message,
            })
        })
        .collect();
    Ok(json!({ "hints": hints }))
}

fn semantic_tokens(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let source_kind = params["filename"]
        .as_str()
        .and_then(|name| ttc::SourceKind::from_path(std::path::Path::new(name)))
        .unwrap_or_default();
    let tokens: Vec<_> = ttc::engine::semantic_tokens_with_kind(text_param(params)?, source_kind)
        .into_iter()
        .map(|token| {
            json!({
                "range": range_json(token.range),
                "kind": token.kind.as_str(),
                "modifiers": token.kind.modifiers(),
            })
        })
        .collect();
    Ok(json!({ "tokens": tokens }))
}

/// `-p <path>`: what the one-shot prints for the file on disk, with the
/// options a bundler passes it. The compile is the command line's own
/// ([`crate::build::print_input`]), so the bytes are the same; what a
/// session adds is that the TypeScript project refining the output's
/// storage annotations stays open between requests.
fn print(params: &serde_json::Value, node: Option<&Path>) -> Result<serde_json::Value, String> {
    let path = params["path"]
        .as_str()
        .ok_or_else(|| "print needs a \"path\"".to_string())?;
    let source_map = match params["sourceMap"].as_str().unwrap_or("off") {
        "off" => crate::build::SourceMapMode::Off,
        "inline" => crate::build::SourceMapMode::Inline,
        other => {
            return Err(format!(
                "print: \"sourceMap\" expects off or inline (got {other})"
            ));
        }
    };
    let rewrite_imports = match params["rewriteImports"].as_str().unwrap_or("js") {
        "js" => ttc::ImportRewrite::Js,
        "ts" => ttc::ImportRewrite::Ts,
        "off" => ttc::ImportRewrite::Off,
        other => {
            return Err(format!(
                "print: \"rewriteImports\" expects js, ts, or off (got {other})"
            ));
        }
    };
    let jsx_preserve = crate::build::project_jsx_preserve(
        rewrite_imports,
        &[std::path::PathBuf::from(path)],
        None,
    )
    .map_err(|error| format!("print: {error}"))?;
    let printed = crate::build::print_input(
        path,
        &crate::build::BuildOptions {
            banner: params["banner"].as_bool().unwrap_or(true),
            print: true,
            check: false,
            verify: params["verify"].as_bool().unwrap_or(true),
            rewrite_imports,
            jsx_preserve,
            source_map,
            out_dir: None,
            jobs: None,
            node: node.map(Path::to_path_buf),
        },
    );
    Ok(serde_json::json!({ "code": printed.code, "messages": printed.messages }))
}

/// The stamps a project's watch paths had when it was last checked for
/// `dependencies`, and the files that check covered, per project root.
///
/// `Project::watch_paths` names every path whose change invalidates a
/// project check; while none of them has changed and the check would cover
/// the same files, its answer stands. This is the rule `--check-types
/// --watch` re-checks by, and what keeps a bundler's request per module
/// from type-checking the whole project once per module.
#[derive(Default)]
struct Checks(HashMap<PathBuf, Checked>);

struct Checked {
    files: Vec<PathBuf>,
    stamps: HashMap<PathBuf, SystemTime>,
}

fn stamps(paths: &[PathBuf]) -> HashMap<PathBuf, SystemTime> {
    paths
        .iter()
        .map(|path| {
            let stamp = std::fs::metadata(path)
                .and_then(|meta| meta.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            (path.clone(), stamp)
        })
        .collect()
}

/// `--dependencies <path>`, answered by [`ttc::engine::Project::dependencies_of`]
/// on the live project the file belongs to.
fn dependencies(
    workspace: &mut Workspace,
    checks: &mut Checks,
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let path = params["path"]
        .as_str()
        .ok_or_else(|| "dependencies needs a \"path\"".to_string())?;
    let inputs = ttc::engine::Inputs::collect(&[path.to_string()])?;
    let project = workspace.project_for(&inputs.files()[0])?;
    let files = project
        .candidates(&inputs)
        .map_err(|error| error.to_string())?;
    let watched = project.watch_paths().map_err(|error| error.to_string())?;
    let current = stamps(&watched);
    // The previous check stands for a file it covered; one it left out is
    // a root by request, and is checked as one.
    if let Some(checked) = checks.0.get(project.root())
        && checked.files == files
        && checked.stamps == current
        && inputs.named().iter().all(|file| project.checked(file))
    {
        return project
            .dependencies_for(&inputs)
            .map(|dependencies| dependencies.to_json())
            .map_err(|error| error.to_string());
    }
    checks.0.remove(project.root());
    let dependencies = project.dependencies_of(&inputs)?;
    let paths = project.watch_paths().map_err(|error| error.to_string())?;
    // Stamps taken before the check stand for the paths that were already
    // watched, so an edit made while it ran still invalidates it; the paths
    // the check discovered start from now.
    let mut recorded = current;
    for (path, stamp) in stamps(&paths) {
        recorded.entry(path).or_insert(stamp);
    }
    checks.0.insert(
        project.root().to_path_buf(),
        Checked {
            files,
            stamps: recorded,
        },
    );
    Ok(dependencies.to_json())
}

/// `--emit-map` for a buffer: the emitted TypeScript and its byte mappings.
fn emit_map(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let source_kind = params["filename"]
        .as_str()
        .and_then(|name| ttc::SourceKind::from_path(std::path::Path::new(name)))
        .unwrap_or_default();
    let emit = ttc::emit_mapped_with_kind(text_param(params)?, source_kind);
    let mappings: Vec<_> = emit
        .mappings
        .iter()
        .map(|m| json!({ "src": m.src, "out": m.out, "len": m.len }))
        .collect();
    Ok(json!({ "code": emit.code, "mappings": mappings }))
}

/// `--check-types --overlay <path>` for a buffer, against the live project
/// it belongs to. `includeTypes` controls whether TypeScript diagnostics are
/// included; typed tt facts are always computed by the same pass.
fn typed_check(
    workspace: &mut Workspace,
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let path = params["path"]
        .as_str()
        .ok_or_else(|| "typedCheck needs a \"path\"".to_string())?
        .to_string();
    let buffer = text_param(params)?;
    let text = buffer.to_string();
    let include_types = params["includeTypes"].as_bool().unwrap_or(false);
    let canonical = ttc::engine::normalize_document_path(Path::new(&path))?;
    // A document the consumer holds open keeps its overlay after the check;
    // a one-off buffer's overlay is scoped to this request, so the answer
    // stays stateless while the projection cache keeps the incremental win.
    let registered = workspace.is_open(&canonical);
    let project = workspace.project_for(&canonical)?;

    project.open_document(canonical.clone(), text);
    let files = {
        let mut scanned = project.scan().map_err(|e| e.to_string())?;
        scanned.push(canonical.clone());
        scanned.sort();
        scanned.dedup();
        scanned
    };
    let scoped = params["scope"].as_str() == Some("file");
    let outcome = project.update_scoped(&files, scoped.then_some(canonical.as_path()));
    let response = match outcome {
        Err(blocked) => {
            let positions = (blocked.path == canonical).then(|| ProtocolPositions::new(buffer));
            let at = |position: (usize, usize)| match &positions {
                Some(positions) => positions.of_position(position),
                None => position,
            };
            let (line, col) = at((blocked.error.line, blocked.error.col));
            let (end_line, end_col) = at((blocked.error.end_line, blocked.error.end_col));
            json!({
                "blocked": true,
                "diagnostics": [{
                    "path": blocked.path,
                    "line": line,
                    "col": col,
                    "endLine": end_line,
                    "endCol": end_col,
                    "message": blocked.error.message,
                }],
            })
        }
        Ok(snapshot) => {
            let request = CheckRequest {
                emit_declarations: false,
                tt_only: !include_types,
            };
            let checked = if scoped {
                project.check_file(&snapshot, &request, &canonical)
            } else {
                project.check(&snapshot, &request)
            };
            match checked {
                Err(e) => {
                    if !registered {
                        project.close_document(&canonical);
                    }
                    return Err(e);
                }
                Ok(checked) => {
                    let diagnostics: Vec<_> = checked
                        .diagnostics
                        .iter()
                        .map(|d| {
                            let positions = snapshot.source_of(&d.path).map(ProtocolPositions::new);
                            let at = |position: Option<(usize, usize)>| match (position, &positions)
                            {
                                (Some(position), Some(positions)) => {
                                    positions.of_position(position)
                                }
                                (Some(position), None) => position,
                                (None, _) => (0, 0),
                            };
                            let (line, col) = at(d.position);
                            let (end_line, end_col) = at(d.end);
                            let mut entry = json!({
                                "path": d.path,
                                "line": line,
                                "col": col,
                                "endLine": end_line,
                                "endCol": end_col,
                                "message": d.message,
                                "code": d.code,
                                "suggestions": suggestions_json(
                                    &d.suggestions,
                                    snapshot.source_of(&d.path),
                                ),
                            });
                            // Labels ride only when there are any, so a
                            // consumer of the existing shape sees no new
                            // field until a diagnostic actually carries one.
                            if !d.labels.is_empty() {
                                entry["labels"] = labels_json(&d.labels, &d.path, &|path| {
                                    snapshot.source_of(path)
                                });
                            }
                            entry
                        })
                        .collect();
                    // `backendError`: the TypeScript layer could not run —
                    // the tt diagnostics above are still complete.
                    let backend_error = checked.backend_error.as_ref().map(|error| {
                        json!({
                            "kind": match error.kind {
                                ttc::engine::BackendErrorKind::Unavailable => "unavailable",
                                ttc::engine::BackendErrorKind::Internal => "internal",
                            },
                            "message": error.message,
                        })
                    });
                    json!({
                        "blocked": snapshot.is_blocked(&canonical),
                        "diagnostics": diagnostics,
                        "backendError": backend_error,
                    })
                }
            }
        }
    };
    if !registered {
        project.close_document(&canonical);
    }
    Ok(response)
}

fn text_param(params: &serde_json::Value) -> Result<&str, String> {
    params["text"]
        .as_str()
        .ok_or_else(|| "the request needs a \"text\"".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue(lines: &[serde_json::Value]) -> VecDeque<Vec<u8>> {
        lines
            .iter()
            .map(|line| line.to_string().into_bytes())
            .collect()
    }

    #[test]
    fn a_marked_diagnostic_request_yields_to_a_later_document_change() {
        use serde_json::json;
        let check = json!({"id": 1, "method": "typedCheck",
            "params": {"path": "/p/a.tt", "text": "x", "supersedable": true}})
        .to_string();
        let service = json!({"id": 2, "method": "tsDiagnostics",
            "params": {"path": "/p/a.tt", "supersedable": true}})
        .to_string();
        for change in [
            "openDocument",
            "updateDocument",
            "closeDocument",
            "reloadProjects",
        ] {
            let later = queue(&[
                json!({"id": 3, "method": change, "params": {"path": "/p/b.tt", "text": ""}}),
            ]);
            assert!(superseded(&check, &later), "{change}");
            assert!(superseded(&service, &later), "{change}");
        }
        let unrelated =
            queue(&[json!({"id": 3, "method": "hover", "params": {"path": "/p/a.tt"}})]);
        assert!(!superseded(&check, &unrelated));
        assert!(!superseded(&check, &VecDeque::new()));
    }

    #[test]
    fn an_unmarked_request_is_always_answered() {
        use serde_json::json;
        let later = queue(&[
            json!({"id": 3, "method": "updateDocument", "params": {"path": "/p/a.tt", "text": ""}}),
        ]);
        for request in [
            json!({"id": 1, "method": "typedCheck", "params": {"path": "/p/a.tt", "text": "x"}}),
            json!({"id": 1, "method": "typedCheck", "params": {"path": "/p/a.tt", "text": "x", "supersedable": false}}),
            json!({"id": 1, "method": "hover", "params": {"path": "/p/a.tt", "supersedable": true}}),
            json!({"id": 1, "method": "check", "params": {"text": "x", "supersedable": true}}),
        ] {
            assert!(!superseded(&request.to_string(), &later), "{request}");
        }
    }
}
