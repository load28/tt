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
//! The protocol is one JSON object per line, on stdin and stdout:
//!
//! ```text
//! → { "id": 1, "method": "check", "params": { "text", "filename"?, "verify"? } }
//! ← { "id": 1, "result": { "diagnostics":
//!        [{ "line", "col", "endLine", "endCol", "message", "code",
//!           "suggestions": [{ "message", "edit": { "line", "col",
//!             "endLine", "endCol", "replacement" } | null }] }] } }
//!
//! → { "id": 2, "method": "emitMap", "params": { "text", "filename"? } }
//! ← { "id": 2, "result": { "code", "mappings": [{ "src", "out", "len" }] } }
//!
//! → { "id": 3, "method": "typedCheck", "params": { "path", "text" } }
//! ← { "id": 3, "result": { "blocked", "diagnostics":
//!        [{ "path", "line", "col", "endLine", "endCol", "message", "code",
//!           "suggestions" }] } }
//! `blocked`: the pass checked none of the buffer's TypeScript — the
//! project could not be read, or the buffer could not be lowered and its
//! diagnostics are its tt-level ones alone.
//!
//! → { "id": 4, "method": "semanticTokens", "params": { "text" } }
//! ← { "id": 4, "result": { "tokens": [{ "range", "kind" }] } }
//!
//! → { "id": 5, "method": "ttSymbol", "params": { "path", "text", "position" } }
//! ← { "id": 5, "result": { "kind", "range", "name", "variantName",
//!                          "signature", "detail", "definition", "binds" } | null }
//!
//! → { "id": 6, "method": "ttCompletions", "params": { "path", "text", "position" } }
//! ← { "id": 6, "result": { "items": [{ "label", "kind", "detail", "covered" }] } }
//!
//! → { "id": 7, "method": "ttHints", "params": { "path", "text" } }
//! ← { "id": 7, "result": { "hints": [{ "kind", "range", "message" }] } }
//!
//! → { "id": 8, "method": "declarations", "params": { "path", "text" } }
//! ← { "id": 8, "result": { "variants": [{ "name", "generics", "origin",
//!        "specifier", "nameSpan", "span", "cases" }],
//!        "matches": [{ "keyword", "bodyOpen", "bodyClose" }] } }
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
//! ← { "id": 11, "result": { "paths": [string] } }
//! `ttc --dependencies` for the file: the paths whose change invalidates
//! its compile.
//!
//! ← { "id": N, "error": "sentence" }   // the request failed; the session lives
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

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::SystemTime;
use ttc::lines::ProtocolPositions;

use ttc::engine::{
    CheckRequest, CompletionAnswer, Engine, Location, Position, Project, Range, ServiceSeverity,
    ServiceTag, Workspace,
};

/// Runs the server until stdin closes.
pub(crate) fn run(node: Option<PathBuf>) -> ExitCode {
    // One live project per identity, and the documents a consumer holds
    // open in them — what a server exists to keep between requests.
    let mut workspace = Workspace::new(Engine::new(node));
    let mut checks = Checks::default();

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = stdin.lock();
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        match input.read_until(b'\n', &mut bytes) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let line = match std::str::from_utf8(&bytes) {
            Ok(line) => line.trim_end_matches(['\n', '\r']),
            Err(error) => {
                let response = serde_json::json!({
                    "id": null,
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
        let response = match ttc::ice::catching(|| respond(&mut workspace, &mut checks, line)) {
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

/// One request, one answer — errors included, so the session survives them.
/// The `id` of a request the server could not answer.
///
/// A response has to carry the id it answers or the consumer cannot match
/// it to its question; when the request did not even parse, `null` is the
/// protocol's own answer for "no id".
fn request_id(line: &str) -> serde_json::Value {
    serde_json::from_str::<serde_json::Value>(line)
        .map(|request| request["id"].clone())
        .unwrap_or(serde_json::Value::Null)
}

fn respond(workspace: &mut Workspace, checks: &mut Checks, line: &str) -> serde_json::Value {
    use serde_json::json;
    ttc::ice::panic_for_test("server");
    let request: serde_json::Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(e) => return json!({ "id": null, "error": format!("malformed request: {e}") }),
    };
    let id = request["id"].clone();
    let params = &request["params"];
    let result = match request["method"].as_str().unwrap_or_default() {
        "check" => check(params),
        "print" => print(params),
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
            let CompletionAnswer {
                items,
                member,
                probe,
            } = project.completion(path, position, member)?;
            Ok(json!({
                "items": items.iter().map(|item| json!({
                    "label": item.label,
                    "kind": item.kind,
                    "sortText": item.sort_text,
                    "insertText": item.insert_text,
                    "filterText": item.filter_text,
                    "snippet": item.snippet,
                })).collect::<Vec<_>>(),
                "member": member,
                "probe": probe,
            }))
        }),
        "completionResolve" => semantic(workspace, params, |project, path, position| {
            let label = params["label"].as_str().unwrap_or_default();
            let probe = params["probe"].as_u64();
            Ok(
                match project.completion_resolve(path, position, label, probe)? {
                    None => serde_json::Value::Null,
                    Some(detail) => {
                        let mut answer = json!({
                            "signature": detail.signature,
                            "documentation": detail.documentation,
                        });
                        if !detail.additional_edits.is_empty() {
                            answer["additionalEdits"] = detail
                                .additional_edits
                                .into_iter()
                                .map(|edit| {
                                    json!({
                                        "range": range_json(edit.range),
                                        "newText": edit.new_text,
                                    })
                                })
                                .collect();
                        }
                        answer
                    }
                },
            )
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
        "documentSymbols" => semantic(workspace, params, |project, path, _position| {
            Ok(
                json!({ "symbols": project.document_symbols(path)?.iter().map(symbol_json).collect::<Vec<_>>() }),
            )
        }),
        "signatureHelp" => semantic(workspace, params, |project, path, position| {
            Ok(match project.signature_help(path, position)? {
                None => serde_json::Value::Null,
                Some(help) => json!({
                    "signatures": help.signatures.iter().map(|signature| json!({
                        "label": signature.label,
                        "documentation": signature.documentation,
                        "parameters": signature.parameters.iter().map(|parameter| json!({
                            "label": [parameter.label.0, parameter.label.1],
                            "documentation": parameter.documentation,
                        })).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                    "activeSignature": help.active_signature,
                    "activeParameter": help.active_parameter,
                }),
            })
        }),
        "semanticTokens" => semantic_tokens(params),
        "declarations" => declarations(params),
        "ttSymbol" => tt_symbol(params),
        "ttCompletions" => tt_completions(params),
        "ttHints" => tt_hints(params),
        "tsDiagnostics" => semantic(workspace, params, |project, path, _position| {
            let diagnostics: Vec<_> = project
                .service_diagnostics(path)?
                .into_iter()
                .map(|d| {
                    let mut entry = json!({
                        "range": range_json(d.range),
                        "message": d.message,
                        "code": d.code,
                        "severity": match d.severity {
                            ServiceSeverity::Error => "error",
                            ServiceSeverity::Warning => "warning",
                            ServiceSeverity::Information => "information",
                            ServiceSeverity::Hint => "hint",
                        },
                    });
                    if !d.tags.is_empty() {
                        entry["tags"] = d
                            .tags
                            .iter()
                            .map(|tag| match tag {
                                ServiceTag::Unnecessary => "unnecessary",
                                ServiceTag::Deprecated => "deprecated",
                            })
                            .collect();
                    }
                    // Secondary labeled spans ride only when there are any,
                    // so consumers of the existing shape see no new field
                    // until a diagnostic actually carries one.
                    if !d.related.is_empty() {
                        entry["related"] = d
                            .related
                            .iter()
                            .map(|r| {
                                let mut related = json!({
                                    "range": range_json(r.range),
                                    "message": r.message,
                                });
                                if let Some(path) = &r.path {
                                    related["path"] = json!(path);
                                }
                                related
                            })
                            .collect();
                    }
                    entry
                })
                .collect();
            // The tt diagnostics these state in TypeScript's own words: a
            // consumer showing both layers shows the fact once.
            let restates: Vec<_> = project
                .service_restates(path)?
                .into_iter()
                .map(|code| code.as_str())
                .collect();
            Ok(json!({ "diagnostics": diagnostics, "restates": restates }))
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

/// A diagnostic's suggestions as the JSON the protocol speaks.
///
/// The edit's byte offsets become the same 1-based line/column the
/// diagnostic itself is reported in, so one response never mixes two
/// coordinate spaces. `edit` is null for advice that names no replacement,
/// and for an edit whose source the server cannot resolve — a consumer
/// shows the message either way and only offers a fix when there is one.
fn suggestions_json(suggestions: &[ttc::Suggestion], source: Option<&str>) -> serde_json::Value {
    use serde_json::json;
    let positions = source.map(ProtocolPositions::new);
    let positions = positions.as_ref();
    suggestions
        .iter()
        .map(|suggestion| {
            let edit = suggestion
                .edit
                .as_ref()
                .zip(positions)
                .map(|(edit, positions)| {
                    let (line, col) = positions.of_byte(edit.start);
                    let (end_line, end_col) = positions.of_byte(edit.end);
                    json!({
                        "line": line,
                        "col": col,
                        "endLine": end_line,
                        "endCol": end_col,
                        "replacement": edit.replacement,
                    })
                });
            json!({ "message": suggestion.message, "edit": edit })
        })
        .collect::<Vec<_>>()
        .into()
}

/// A [`Range`] as the JSON the protocol speaks.
fn range_json(range: Range) -> serde_json::Value {
    serde_json::json!({
        "start": { "line": range.start.line, "character": range.start.character },
        "end": { "line": range.end.line, "character": range.end.character },
    })
}

/// A [`Location`] as the JSON the protocol speaks.
fn symbol_json(symbol: &ttc::engine::DocumentSymbol) -> serde_json::Value {
    serde_json::json!({
        "name": symbol.name,
        "detail": symbol.detail,
        "kind": symbol.kind,
        "range": range_json(symbol.range),
        "selectionRange": range_json(symbol.selection_range),
        "children": symbol.children.iter().map(symbol_json).collect::<Vec<_>>(),
    })
}

fn location_json(location: Location) -> serde_json::Value {
    serde_json::json!({
        "path": location.path,
        "range": range_json(location.range),
    })
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
        ttc::compile_report(text, &options)
    });
    let positions = ProtocolPositions::new(text);
    let diagnostics: Vec<_> = report
        .diagnostics
        .iter()
        .map(|d| {
            let at = |offset: Option<usize>| offset.map_or((0, 0), |at| positions.of_byte(at));
            let (line, col) = at(d.start);
            let (end_line, end_col) = at(d.end);
            json!({
                "line": line,
                "col": col,
                "endLine": end_line,
                "endCol": end_col,
                "message": d.message,
                "code": d.code.as_str(),
                "suggestions": suggestions_json(&d.suggestions, Some(text)),
            })
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
/// Text-only; `path` resolves the buffer's relative `.tt` imports.
fn declarations(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let path = params["path"]
        .as_str()
        .ok_or_else(|| "the request needs a \"path\"".to_string())?;
    let text = text_param(params)?;
    let decls = ttc::engine::tt_declarations(Path::new(path), text);
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
/// resolve the file's relative `.tt` imports.
fn tt_symbol(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let path = params["path"]
        .as_str()
        .ok_or_else(|| "the request needs a \"path\"".to_string())?;
    let position = Position {
        line: params["position"]["line"].as_u64().unwrap_or(0) as u32,
        character: params["position"]["character"].as_u64().unwrap_or(0) as u32,
    };
    let Some(symbol) = ttc::engine::tt_symbol_at(Path::new(path), text_param(params)?, position)
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
fn tt_completions(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let path = params["path"]
        .as_str()
        .ok_or_else(|| "the request needs a \"path\"".to_string())?;
    let position = Position {
        line: params["position"]["line"].as_u64().unwrap_or(0) as u32,
        character: params["position"]["character"].as_u64().unwrap_or(0) as u32,
    };
    let items: Vec<_> =
        ttc::engine::tt_completions_at(Path::new(path), text_param(params)?, position)
            .into_iter()
            .map(|item| {
                json!({
                    "label": item.label,
                    "kind": match item.kind {
                        ttc::engine::TtCompletionKind::Case => "case",
                        ttc::engine::TtCompletionKind::Field => "field",
                        ttc::engine::TtCompletionKind::Wildcard => "wildcard",
                    },
                    "detail": item.detail,
                    "covered": item.covered,
                })
            })
            .collect();
    Ok(json!({ "items": items }))
}

/// What tt has to say about a buffer that is not an error — today, the
/// arms an earlier arm already covers. Text-only like [`tt_symbol`], and
/// separate from `check` on purpose: a hint never fails a build, so it
/// never travels in the diagnostics of a compile answer.
fn tt_hints(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let path = params["path"]
        .as_str()
        .ok_or_else(|| "the request needs a \"path\"".to_string())?;
    let hints: Vec<_> = ttc::engine::tt_hints(Path::new(path), text_param(params)?)
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
fn print(params: &serde_json::Value) -> Result<serde_json::Value, String> {
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
    let printed = crate::build::print_input(
        path,
        &crate::build::BuildOptions {
            banner: params["banner"].as_bool().unwrap_or(true),
            print: true,
            check: false,
            verify: params["verify"].as_bool().unwrap_or(true),
            rewrite_imports,
            source_map,
            out_dir: None,
            jobs: None,
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

/// `--dependencies <path>`: every path whose change invalidates the file's
/// compile — the watch paths of the live project it belongs to, after a
/// check of that project with the file among its candidates.
fn dependencies(
    workspace: &mut Workspace,
    checks: &mut Checks,
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let path = params["path"]
        .as_str()
        .ok_or_else(|| "dependencies needs a \"path\"".to_string())?;
    let canonical = ttc::engine::normalize_document_path(Path::new(path))?;
    let project = workspace.project_for(&canonical)?;
    let mut files = project.scan().map_err(|error| error.to_string())?;
    files.push(canonical);
    files.sort();
    files.dedup();
    let watched = project.watch_paths().map_err(|error| error.to_string())?;
    let current = stamps(&watched);
    if let Some(checked) = checks.0.get(project.root())
        && checked.files == files
        && checked.stamps == current
    {
        return Ok(serde_json::json!({ "paths": watched }));
    }
    checks.0.remove(project.root());
    let snapshot = project
        .update(&files)
        .map_err(|blocked| blocked.error.message.clone())?;
    let checked = project.check(&snapshot, &CheckRequest::default())?;
    if let Some(error) = checked.backend_error
        && error.kind == ttc::engine::BackendErrorKind::Internal
    {
        return Err(error.message);
    }
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
    Ok(serde_json::json!({ "paths": paths }))
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
    let outcome = project.update(&files);
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
            let checked = project.check(
                &snapshot,
                &CheckRequest {
                    emit_declarations: false,
                    tt_only: !include_types,
                },
            );
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

/// A diagnostic's secondary labeled spans as the JSON the protocol speaks:
/// 1-based line/column pairs like the diagnostic itself, plus the label's
/// words, and a `path` only when the span is in another file.
fn labels_json<'a>(
    labels: &[ttc::engine::DiagnosticLabel],
    default_path: &Path,
    source_of: &dyn Fn(&Path) -> Option<&'a str>,
) -> serde_json::Value {
    use serde_json::json;
    labels
        .iter()
        .map(|label| {
            // A label carries a path only when it points into another
            // file, so its column is counted against that file's text and
            // otherwise against the diagnostic's own.
            let positions = source_of(label.path.as_deref().unwrap_or(default_path))
                .map(ProtocolPositions::new);
            let at = |position: (usize, usize)| match &positions {
                Some(positions) => positions.of_position(position),
                None => position,
            };
            let (line, col) = at(label.position);
            let (end_line, end_col) = at(label.end);
            let mut entry = json!({
                "line": line,
                "col": col,
                "endLine": end_line,
                "endCol": end_col,
                "message": label.message,
            });
            if let Some(path) = &label.path {
                entry["path"] = json!(path);
            }
            entry
        })
        .collect()
}

fn text_param(params: &serde_json::Value) -> Result<&str, String> {
    params["text"]
        .as_str()
        .ok_or_else(|| "the request needs a \"text\"".to_string())
}
