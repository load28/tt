//! Editor cases: each file under `tests/cases/editor/` is a small project
//! whose editor answers at named markers are held to a baseline under
//! `tests/baselines/reference/editor/`, the way TypeScript's fourslash tests
//! ask the language service at `/*marker*/` positions.
//!
//! Every question is asked twice, through the engine API
//! (`ttc::engine::Workspace`, what an embedding reads) and through the
//! `ttc --server` transport (what the VS Code extension reads), and the two
//! answers must be identical. `diagnostics` and `completions` are also asked
//! of the VS Code adapter over LSP: what it publishes after merging its
//! diagnostic layers, and its completion items with their LSP kinds and
//! tags. A case with a TypeScript twin (the same stem
//! with `.ts` or `.tsx`) is also asked of `tsgo --lsp` directly at the same
//! markers, and every answer that differs from TypeScript's, after the
//! normalizations `parity_view` documents, is listed in
//! `tests/baselines/reference/editor/failingParity.txt`, which the run keeps
//! exact.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use ttc::engine::{
    CompletionAnswer, Engine, Location, Position, PrepareRename, Range, ServiceSeverity,
    ServiceTag, SignatureTrigger, TtCompletion, TtCompletionKind, TtSymbolKind,
};

mod common;
use common::baseline::{expect, expect_absent, updating};
use common::{Workspace, toolchain, toolchain_installed};

const TSCONFIG: &str = r#"{
  "compilerOptions": {
    "target": "es2022",
    "module": "preserve",
    "moduleResolution": "bundler",
    "jsx": "preserve",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true
  }
}
"#;

const GLOBALS_OR_KEYWORDS: &str = "15";

const TOKEN_TYPES: [&str; 23] = [
    "namespace",
    "type",
    "class",
    "enum",
    "interface",
    "struct",
    "typeParameter",
    "parameter",
    "variable",
    "property",
    "enumMember",
    "event",
    "function",
    "method",
    "macro",
    "keyword",
    "modifier",
    "comment",
    "string",
    "number",
    "regexp",
    "operator",
    "decorator",
];

const TOKEN_MODIFIERS: [&str; 11] = [
    "declaration",
    "definition",
    "readonly",
    "static",
    "deprecated",
    "abstract",
    "async",
    "modification",
    "documentation",
    "defaultLibrary",
    "local",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn reference() -> PathBuf {
    root().join("tests/baselines/reference/editor")
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Verb {
    Hover,
    Completions,
    Definition,
    References,
    Rename,
    SignatureHelp,
    SemanticTokens,
    Diagnostics,
}

impl Verb {
    fn parse(name: &str) -> Option<Verb> {
        Some(match name {
            "hover" => Verb::Hover,
            "completions" => Verb::Completions,
            "definition" => Verb::Definition,
            "references" => Verb::References,
            "rename" => Verb::Rename,
            "signaturehelp" => Verb::SignatureHelp,
            "semantictokens" => Verb::SemanticTokens,
            "diagnostics" => Verb::Diagnostics,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            Verb::Hover => "hover",
            Verb::Completions => "completions",
            Verb::Definition => "definition",
            Verb::References => "references",
            Verb::Rename => "rename",
            Verb::SignatureHelp => "signatureHelp",
            Verb::SemanticTokens => "semanticTokens",
            Verb::Diagnostics => "diagnostics",
        }
    }

    fn per_file(self) -> bool {
        matches!(self, Verb::SemanticTokens | Verb::Diagnostics)
    }
}

struct Unit {
    name: String,
    text: String,
    markers: Vec<(String, usize)>,
    ranges: Vec<(usize, usize)>,
}

struct Case {
    name: String,
    path: PathBuf,
    units: Vec<Unit>,
    verbs: Vec<(Verb, Vec<String>)>,
    twin: Option<Vec<Unit>>,
}

fn directive(line: &str) -> Option<(String, &str)> {
    let rest = line.strip_prefix("//")?.trim_start().strip_prefix('@')?;
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    let (name, tail) = rest.split_at(end);
    let value = tail.trim_start().strip_prefix(':')?;
    Some((name.to_ascii_lowercase(), value.trim()))
}

fn is_tt(name: &str) -> bool {
    name.ends_with(".tt") || name.ends_with(".ttx")
}

fn stem(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

fn strip_markers(raw: &str, path: &Path) -> Unit {
    let mut text = String::new();
    let mut markers = Vec::new();
    let mut ranges = Vec::new();
    let mut open = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        if raw[i..].starts_with("[|") {
            open.push(text.len());
            i += 2;
            continue;
        }
        if raw[i..].starts_with("|]") {
            let start = open
                .pop()
                .unwrap_or_else(|| panic!("{}: `|]` without `[|`", path.display()));
            ranges.push((start, text.len()));
            i += 2;
            continue;
        }
        if raw[i..].starts_with("/*") {
            let body = &raw[i + 2..];
            let length = body
                .bytes()
                .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
                .count();
            if length > 0 && body[length..].starts_with("*/") {
                markers.push((body[..length].to_string(), text.len()));
                i += 2 + length + 2;
                continue;
            }
        }
        let width = raw[i..].chars().next().map_or(1, char::len_utf8);
        text.push_str(&raw[i..i + width]);
        i += width;
    }
    assert!(open.is_empty(), "{}: `[|` without `|]`", path.display());
    ranges.sort_unstable();
    Unit {
        name: String::new(),
        text,
        markers,
        ranges,
    }
}

fn parse_units(
    text: &str,
    default_name: &str,
    path: &Path,
    verbs: Option<&mut Vec<(Verb, Vec<String>)>>,
) -> Vec<Unit> {
    let mut verbs = verbs;
    let mut units: Vec<(String, Vec<&str>)> = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;
    let mut preamble: Vec<&str> = Vec::new();
    for line in text.split('\n') {
        let Some((name, value)) = directive(line) else {
            match &mut current {
                Some((_, lines)) => lines.push(line),
                None => preamble.push(line),
            }
            continue;
        };
        if name == "filename" {
            if let Some(unit) = current.take() {
                units.push(unit);
            } else {
                assert!(
                    preamble.iter().all(|l| {
                        let l = l.trim();
                        l.is_empty() || l.starts_with("//")
                    }),
                    "{}: non-comment content appears before the first `// @filename`",
                    path.display()
                );
            }
            current = Some((value.to_string(), Vec::new()));
            continue;
        }
        let verb = Verb::parse(&name).unwrap_or_else(|| {
            panic!(
                "{}: unknown directive `@{name}`; an editor case takes @filename and the verbs \
                 hover, completions, definition, references, rename, signatureHelp, \
                 semanticTokens, diagnostics",
                path.display()
            )
        });
        let Some(verbs) = verbs.as_deref_mut() else {
            panic!("{}: a TypeScript twin takes no verbs", path.display());
        };
        let targets = value
            .split(',')
            .map(str::trim)
            .filter(|target| !target.is_empty())
            .map(String::from)
            .collect();
        verbs.push((verb, targets));
    }
    match current {
        Some(unit) => units.push(unit),
        None => units.push((default_name.to_string(), preamble)),
    }
    units
        .into_iter()
        .map(|(name, lines)| {
            let mut unit = strip_markers(&lines.join("\n"), path);
            unit.name = name;
            unit
        })
        .collect()
}

fn twin_name(name: &str) -> String {
    if let Some(stem) = name.strip_suffix(".ttx") {
        format!("{stem}.tsx")
    } else if let Some(stem) = name.strip_suffix(".tt") {
        format!("{stem}.ts")
    } else {
        name.to_string()
    }
}

fn cases() -> Vec<Case> {
    let dir = root().join("tests/cases/editor");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("tests/cases/editor")
        .map(|entry| entry.expect("a readable case entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "tt" || e == "ttx"))
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "no editor cases under tests/cases/editor"
    );
    let filter = std::env::var("TT_CASES").ok().filter(|f| !f.is_empty());
    let mut out = Vec::new();
    for path in files {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        if filter.as_deref().is_some_and(|f| !name.contains(f)) {
            continue;
        }
        let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
        let text = fs::read_to_string(&path).expect("a readable editor case");
        let mut verbs = Vec::new();
        let units = parse_units(&text, &file_name, &path, Some(&mut verbs));
        assert!(
            !verbs.is_empty(),
            "{}: the case asks nothing",
            path.display()
        );
        let twin = ["ts", "tsx"]
            .iter()
            .map(|extension| path.with_extension(extension))
            .find(|twin| twin.exists())
            .map(|twin| {
                let text = fs::read_to_string(&twin).expect("a readable twin");
                let twin_file = twin.file_name().unwrap().to_string_lossy().into_owned();
                parse_units(&text, &twin_file, &twin, None)
            });
        if let Some(twin) = &twin {
            let expected: BTreeSet<String> = units.iter().map(|u| twin_name(&u.name)).collect();
            let actual: BTreeSet<String> = twin.iter().map(|u| u.name.clone()).collect();
            assert_eq!(
                expected,
                actual,
                "{}: the twin's units must be the case's, with .ts/.tsx for .tt/.ttx",
                path.display()
            );
        }
        for (verb, targets) in &verbs {
            for target in targets {
                let known = if verb.per_file() {
                    target == "*" || units.iter().any(|u| u.name == *target && is_tt(&u.name))
                } else {
                    units
                        .iter()
                        .any(|u| u.markers.iter().any(|(marker, _)| marker == target))
                };
                assert!(
                    known,
                    "{}: @{} names `{target}`, which is not {}",
                    path.display(),
                    verb.name(),
                    if verb.per_file() {
                        "a .tt/.ttx unit or *"
                    } else {
                        "a marker"
                    }
                );
            }
        }
        out.push(Case {
            name,
            path,
            units,
            verbs,
            twin,
        });
    }
    out
}

fn lsp_position(text: &str, offset: usize) -> Position {
    let before = &text[..offset];
    let line = before.matches('\n').count() as u32;
    let start = before.rfind('\n').map_or(0, |at| at + 1);
    Position {
        line,
        character: text[start..offset].encode_utf16().count() as u32,
    }
}

fn offset_of(text: &str, line: u64, character: u64) -> Option<usize> {
    let mut start = 0usize;
    for _ in 0..line {
        start += text[start..].find('\n')? + 1;
    }
    let mut units = 0u64;
    for (at, c) in text[start..].char_indices() {
        if units >= character || c == '\n' {
            return (units == character).then_some(start + at);
        }
        units += c.len_utf16() as u64;
    }
    (units == character).then_some(text.len())
}

#[derive(Clone)]
struct Request {
    method: &'static str,
    params: Value,
}

struct Question {
    verb: Verb,
    target: String,
    unit: usize,
    offset: Option<usize>,
    requests: Vec<Request>,
}

fn questions(case: &Case, project: &Path) -> Vec<Question> {
    let mut out = Vec::new();
    for (verb, targets) in &case.verbs {
        for target in targets {
            if verb.per_file() {
                for (index, unit) in case.units.iter().enumerate() {
                    if !is_tt(&unit.name) || (target != "*" && *target != unit.name) {
                        continue;
                    }
                    let path = project.join(&unit.name).to_string_lossy().into_owned();
                    let method = match verb {
                        Verb::SemanticTokens => "documentSemanticTokens",
                        _ => "tsDiagnostics",
                    };
                    out.push(Question {
                        verb: *verb,
                        target: unit.name.clone(),
                        unit: index,
                        offset: None,
                        requests: vec![Request {
                            method,
                            params: json!({ "path": path }),
                        }],
                    });
                }
                continue;
            }
            let (index, offset) = case
                .units
                .iter()
                .enumerate()
                .find_map(|(index, unit)| {
                    unit.markers
                        .iter()
                        .find(|(marker, _)| marker == target)
                        .map(|(_, offset)| (index, *offset))
                })
                .expect("a checked marker");
            let unit = &case.units[index];
            let path = project.join(&unit.name).to_string_lossy().into_owned();
            let position = lsp_position(&unit.text, offset);
            let at = json!({ "line": position.line, "character": position.character });
            let located = json!({ "path": path, "position": at });
            let with_text = json!({ "path": path, "position": at, "text": unit.text });
            let requests = match verb {
                Verb::Hover => vec![
                    Request {
                        method: "hover",
                        params: located.clone(),
                    },
                    Request {
                        method: "ttSymbol",
                        params: with_text,
                    },
                ],
                Verb::Completions => vec![
                    Request {
                        method: "completion",
                        params: located.clone(),
                    },
                    Request {
                        method: "ttCompletions",
                        params: with_text,
                    },
                    Request {
                        method: "patternCompletions",
                        params: located.clone(),
                    },
                ],
                Verb::Definition => vec![Request {
                    method: "definition",
                    params: located,
                }],
                Verb::References => vec![Request {
                    method: "references",
                    params: located,
                }],
                Verb::Rename => vec![
                    Request {
                        method: "prepareRename",
                        params: located.clone(),
                    },
                    Request {
                        method: "rename",
                        params: located,
                    },
                ],
                Verb::SignatureHelp => vec![Request {
                    method: "signatureHelp",
                    params: located,
                }],
                Verb::SemanticTokens | Verb::Diagnostics => unreachable!(),
            };
            out.push(Question {
                verb: *verb,
                target: target.clone(),
                unit: index,
                offset: Some(offset),
                requests,
            });
        }
    }
    out
}

fn range_json(range: Range) -> Value {
    json!({
        "start": { "line": range.start.line, "character": range.start.character },
        "end": { "line": range.end.line, "character": range.end.character },
    })
}

fn location_json(location: Location) -> Value {
    json!({ "path": location.path, "range": range_json(location.range) })
}

fn tt_items_json(items: &[TtCompletion]) -> Vec<Value> {
    items
        .iter()
        .map(|item| {
            json!({
                "label": item.label,
                "kind": match item.kind {
                    TtCompletionKind::Case => "case",
                    TtCompletionKind::Field => "field",
                    TtCompletionKind::Literal => "literal",
                    TtCompletionKind::Wildcard => "wildcard",
                },
                "detail": item.detail,
                "covered": item.covered,
                "range": item.range.map(range_json),
            })
        })
        .collect()
}

fn position_param(params: &Value) -> Position {
    Position {
        line: params["position"]["line"].as_u64().unwrap_or(0) as u32,
        character: params["position"]["character"].as_u64().unwrap_or(0) as u32,
    }
}

fn engine_answer(workspace: &mut ttc::engine::Workspace, request: &Request) -> Value {
    let params = &request.params;
    let path = PathBuf::from(params["path"].as_str().expect("a path"));
    let path = path.as_path();
    let position = position_param(params);
    let text = params["text"].as_str().unwrap_or_default();
    let result: Result<Value, String> = match request.method {
        "hover" => workspace.project_for(path).and_then(|project| {
            Ok(match project.hover(path, position)? {
                None => Value::Null,
                Some(info) => json!({
                    "signature": info.signature,
                    "documentation": info.documentation,
                    "range": range_json(info.range),
                }),
            })
        }),
        "ttSymbol" => Ok(match workspace.tt_symbol_at(path, text, position) {
            None => Value::Null,
            Some(symbol) => json!({
                "kind": match symbol.kind {
                    TtSymbolKind::Variant => "variant",
                    TtSymbolKind::Case => "case",
                    TtSymbolKind::Field => "field",
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
            }),
        }),
        "definition" => workspace.project_for(path).and_then(|project| {
            let locations: Vec<_> = project
                .definition(path, position)?
                .into_iter()
                .map(location_json)
                .collect();
            Ok(json!({ "locations": locations }))
        }),
        "references" => workspace.references(path, position).map(|references| {
            let locations: Vec<_> = references
                .into_iter()
                .map(|reference| {
                    let mut value = location_json(reference.location);
                    value["isDefinition"] = json!(reference.is_definition);
                    value
                })
                .collect();
            json!({ "locations": locations })
        }),
        "completion" => workspace.project_for(path).and_then(|project| {
            let CompletionAnswer {
                items,
                member,
                probe,
            } = project.triggered_completion(path, position, false, None)?;
            Ok(json!({
                "items": items.iter().map(|item| json!({
                    "label": item.label,
                    "kind": item.kind.map(|kind| kind.lsp()),
                    "tags": item.tags.iter().map(|tag| tag.lsp()).collect::<Vec<_>>(),
                    "sortText": item.sort_text,
                    "insertText": item.insert_text,
                    "filterText": item.filter_text,
                    "snippet": item.snippet,
                    "range": item.range.map(range_json),
                    "source": item.source,
                    "detail": item.detail,
                    "labelDetails": (item.label_detail.is_some() || item.description.is_some())
                        .then(|| json!({
                            "detail": item.label_detail,
                            "description": item.description,
                        })),
                })).collect::<Vec<_>>(),
                "member": member,
                "probe": probe,
            }))
        }),
        "ttCompletions" => {
            let member = ttc::engine::member_access_at(path, text, position)
                .map(|access| json!({ "receiver": access.receiver }));
            let items = tt_items_json(&workspace.tt_completions_at(path, text, position));
            let pattern = workspace.is_pattern_position(path, text, position);
            let keywords: Vec<_> = ttc::engine::tt_keywords_at(path, text, position)
                .into_iter()
                .map(|keyword| json!({ "label": keyword.label(), "sortText": keyword.sort_text() }))
                .collect();
            Ok(
                json!({ "items": items, "member": member, "keywords": keywords, "pattern": pattern }),
            )
        }
        "patternCompletions" => workspace.project_for(path).and_then(|project| {
            Ok(match project.pattern_completions(path, position)? {
                None => Value::Null,
                Some(items) => json!({ "items": tt_items_json(&items) }),
            })
        }),
        "prepareRename" => workspace
            .prepare_rename(path, position)
            .map(|answer| match answer {
                PrepareRename::Range(range) => json!({ "range": range_json(range) }),
                PrepareRename::Refused(reason) => json!({ "range": null, "refusal": reason }),
            }),
        "rename" => workspace.rename(path, position).map(|edits| match edits {
            None => json!({ "edits": Value::Null }),
            Some(edits) => json!({
                "edits": edits.into_iter().map(|edit| {
                    let mut value = location_json(edit.location);
                    value["newText"] = match edit.new_text {
                        Some(text) => json!(text),
                        None => Value::Null,
                    };
                    value
                }).collect::<Vec<_>>(),
            }),
        }),
        "signatureHelp" => workspace.project_for(path).and_then(|project| {
            Ok(
                match project.triggered_signature_help(
                    path,
                    position,
                    &SignatureTrigger::Invoked,
                    false,
                )? {
                    None => Value::Null,
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
                },
            )
        }),
        "documentSemanticTokens" => workspace.project_for(path).and_then(|project| {
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
        "tsDiagnostics" => workspace.project_for(path).and_then(|project| {
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
            let restates: Vec<_> = project
                .service_restates(path)?
                .into_iter()
                .map(|code| code.as_str())
                .collect();
            Ok(json!({ "diagnostics": diagnostics, "restates": restates }))
        }),
        other => panic!("no engine mirror for {other}"),
    };
    match result {
        Ok(result) => json!({ "result": result }),
        Err(error) => json!({ "error": error }),
    }
}

struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    next: u64,
}

impl Server {
    fn start(project: &Path) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
            .arg("--server")
            .current_dir(project)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("ttc --server starts");
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
        Server {
            child,
            stdin,
            stdout,
            next: 1,
        }
    }

    fn ask(&mut self, method: &str, params: &Value) -> Value {
        let id = self.next;
        self.next += 1;
        let line = json!({ "id": id, "method": method, "params": params }).to_string();
        writeln!(self.stdin, "{line}").expect("the server reads");
        self.stdin.flush().expect("the server reads");
        let mut answer = String::new();
        self.stdout
            .read_line(&mut answer)
            .expect("the server answers");
        let mut answer: Value = serde_json::from_str(&answer).unwrap_or_else(|e| {
            panic!("the server answered {method} with no JSON ({e}): {answer}")
        });
        assert_eq!(
            answer["id"],
            json!(id),
            "the server answered another request"
        );
        match answer.get_mut("error") {
            Some(error) => json!({ "error": error.take() }),
            None => json!({ "result": answer["result"].take() }),
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Default)]
struct Published {
    sequence: u64,
    by_uri: BTreeMap<String, Value>,
}

struct Lsp {
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    responses: Receiver<(i64, Value)>,
    published: Arc<Mutex<Published>>,
    next: i64,
    name: &'static str,
    legend: (Vec<String>, Vec<String>),
}

fn frame(stdin: &Mutex<ChildStdin>, message: &Value) {
    let body = message.to_string();
    let mut stdin = stdin.lock().unwrap_or_else(|p| p.into_inner());
    let _ = write!(stdin, "Content-Length: {}\r\n\r\n{body}", body.len());
    let _ = stdin.flush();
}

fn uri(path: &Path) -> String {
    format!("file://{}", path.to_string_lossy())
}

fn tsgo_binary() -> Option<PathBuf> {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    };
    let exe = if cfg!(windows) { "tsc.exe" } else { "tsc" };
    let path = root()
        .join("node_modules/@typescript")
        .join(format!("typescript-{os}-{arch}"))
        .join("lib")
        .join(exe);
    path.exists().then_some(path)
}

fn extension_server() -> Option<PathBuf> {
    let server = root().join("editors/vscode/server/out/server.js");
    let dependency = root().join("editors/vscode/server/node_modules/vscode-languageserver");
    (server.is_file() && dependency.is_dir()).then_some(server)
}

fn extension_required() -> bool {
    std::env::var_os("TT_REQUIRE_EXTENSION").is_some_and(|v| !v.is_empty() && v != "0")
}

impl Lsp {
    fn start(binary: &Path, dir: &Path) -> Lsp {
        let mut command = Command::new(binary);
        command.args(["--lsp", "-stdio"]);
        Lsp::spawn(command, "tsgo --lsp", dir, json!({}))
    }

    fn editor(server: &Path, dir: &Path) -> Lsp {
        let mut command = Command::new("node");
        command.arg(server).arg("--stdio");
        Lsp::spawn(
            command,
            "the VS Code adapter",
            dir,
            json!({ "compilerPath": env!("CARGO_BIN_EXE_ttc"), "sidecar": "off" }),
        )
    }

    fn spawn(mut command: Command, name: &'static str, dir: &Path, settings: Value) -> Lsp {
        let mut child = command
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("{name} starts: {e}"));
        let stdin = Arc::new(Mutex::new(child.stdin.take().expect("piped stdin")));
        let stdout = child.stdout.take().expect("piped stdout");
        let (tx, rx) = channel();
        let replies = Arc::clone(&stdin);
        let published = Arc::new(Mutex::new(Published::default()));
        let publishes = Arc::clone(&published);
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut length = None;
                loop {
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(0) | Err(_) => return,
                        Ok(_) => {}
                    }
                    let line = line.trim_end();
                    if line.is_empty() {
                        break;
                    }
                    if let Some(value) = line.strip_prefix("Content-Length:") {
                        length = value.trim().parse::<usize>().ok();
                    }
                }
                let Some(length) = length else { return };
                let mut body = vec![0u8; length];
                if reader.read_exact(&mut body).is_err() {
                    return;
                }
                let Ok(message) = serde_json::from_slice::<Value>(&body) else {
                    continue;
                };
                match (
                    message.get("id"),
                    message.get("method").and_then(Value::as_str),
                ) {
                    (Some(id), None) => {
                        let value = match message.get("error") {
                            Some(error) => json!({ "error": error }),
                            None => json!({ "result": message["result"] }),
                        };
                        if tx.send((id.as_i64().unwrap_or(-1), value)).is_err() {
                            return;
                        }
                    }
                    (Some(id), Some(method)) => {
                        let result = if method == "workspace/configuration" {
                            let count = message["params"]["items"].as_array().map_or(1, Vec::len);
                            Value::Array(vec![settings.clone(); count])
                        } else {
                            Value::Null
                        };
                        frame(
                            &replies,
                            &json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                        );
                    }
                    (None, Some("textDocument/publishDiagnostics")) => {
                        let params = &message["params"];
                        if let Some(target) = params["uri"].as_str() {
                            let mut published = publishes.lock().unwrap_or_else(|p| p.into_inner());
                            published.sequence += 1;
                            published
                                .by_uri
                                .insert(target.to_string(), params["diagnostics"].clone());
                        }
                    }
                    _ => {}
                }
            }
        });
        let mut lsp = Lsp {
            child,
            stdin,
            responses: rx,
            published,
            next: 1,
            name,
            legend: (Vec::new(), Vec::new()),
        };
        let root_uri = uri(dir);
        let initialized = lsp.request(
            "initialize",
            json!({
                "processId": std::process::id(),
                "rootUri": root_uri,
                "workspaceFolders": [{ "uri": root_uri, "name": "twin" }],
                "capabilities": {
                    "textDocument": {
                        "synchronization": { "dynamicRegistration": true },
                        "hover": { "contentFormat": ["markdown", "plaintext"] },
                        "definition": {},
                        "references": {},
                        "completion": { "completionItem": {
                            "labelDetailsSupport": true,
                            "tagSupport": { "valueSet": [1] },
                        } },
                        "signatureHelp": {},
                        "rename": { "prepareSupport": true },
                        "semanticTokens": {
                            "requests": { "full": true },
                            "tokenTypes": TOKEN_TYPES,
                            "tokenModifiers": TOKEN_MODIFIERS,
                            "formats": ["relative"],
                            "multilineTokenSupport": false,
                            "overlappingTokenSupport": false,
                        },
                        "diagnostic": {
                            "relatedInformation": true,
                            "tagSupport": { "valueSet": [1, 2] },
                        },
                    },
                    "workspace": { "configuration": true, "workspaceFolders": true },
                },
            }),
        );
        let legend = &initialized["result"]["capabilities"]["semanticTokensProvider"]["legend"];
        let names = |list: &Value| -> Vec<String> {
            list.as_array()
                .into_iter()
                .flatten()
                .filter_map(|name| name.as_str().map(String::from))
                .collect()
        };
        lsp.legend = (
            names(&legend["tokenTypes"]),
            names(&legend["tokenModifiers"]),
        );
        frame(
            &lsp.stdin,
            &json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
        );
        lsp
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        frame(
            &self.stdin,
            &json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }),
        );
        loop {
            match self.responses.recv_timeout(Duration::from_secs(120)) {
                Ok((answered, value)) if answered == id => return value,
                Ok(_) => continue,
                Err(_) => panic!("{} did not answer {method}", self.name),
            }
        }
    }

    fn open(&mut self, path: &Path, text: &str) {
        let language = match path.extension().and_then(|e| e.to_str()) {
            Some("tsx") => "typescriptreact",
            Some("tt") => "tt",
            Some("ttx") => "ttx",
            _ => "typescript",
        };
        frame(
            &self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": { "textDocument": {
                    "uri": uri(path), "languageId": language, "version": 1, "text": text,
                } },
            }),
        );
    }
}

impl Lsp {
    fn settled_publishes(&self, paths: &[PathBuf]) -> BTreeMap<String, Value> {
        let wanted: Vec<String> = paths.iter().map(|path| uri(path)).collect();
        let deadline = std::time::Instant::now() + Duration::from_secs(120);
        let mut seen = None;
        let mut quiet_since = std::time::Instant::now();
        loop {
            let (sequence, complete, snapshot) = {
                let published = self.published.lock().unwrap_or_else(|p| p.into_inner());
                (
                    published.sequence,
                    wanted.iter().all(|u| published.by_uri.contains_key(u)),
                    published.by_uri.clone(),
                )
            };
            let now = std::time::Instant::now();
            if seen != Some(sequence) {
                seen = Some(sequence);
                quiet_since = now;
            }
            if complete && now.duration_since(quiet_since) >= Duration::from_millis(1500) {
                return snapshot;
            }
            assert!(
                now < deadline,
                "{} published no diagnostics for {wanted:?}",
                self.name
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for Lsp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct Files<'a> {
    dir: &'a Path,
    units: &'a [Unit],
    current: std::cell::RefCell<String>,
}

impl Files<'_> {
    fn name(&self, path: &str) -> String {
        let path = path.strip_prefix("file://").unwrap_or(path);
        match Path::new(path).strip_prefix(self.dir) {
            Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
            Err(_) => format!(
                "<external>/{}",
                Path::new(path).file_name().map_or_else(
                    || path.to_string(),
                    |name| name.to_string_lossy().into_owned()
                )
            ),
        }
    }

    fn text(&self, name: &str) -> Option<&str> {
        self.units
            .iter()
            .find(|unit| unit.name == name)
            .map(|unit| unit.text.as_str())
    }

    fn span(&self, name: &str, range: &Value) -> String {
        let (sl, sc, el, ec) = (
            range["start"]["line"].as_u64().unwrap_or(0),
            range["start"]["character"].as_u64().unwrap_or(0),
            range["end"]["line"].as_u64().unwrap_or(0),
            range["end"]["character"].as_u64().unwrap_or(0),
        );
        let coordinates = format!("{}:{}-{}:{}", sl + 1, sc + 1, el + 1, ec + 1);
        let covered = self.text(name).and_then(|text| {
            let start = offset_of(text, sl, sc)?;
            let end = offset_of(text, el, ec)?;
            (start <= end).then(|| text[start..end].to_string())
        });
        match covered {
            Some(text) => format!("{coordinates} {text:?}"),
            None => coordinates,
        }
    }

    fn located(&self, location: &Value) -> String {
        let path = location["path"]
            .as_str()
            .or_else(|| location["uri"].as_str())
            .unwrap_or_default();
        let name = self.name(path);
        format!("{name} {}", self.span(&name, &location["range"]))
    }

    fn covered(&self, location: &Value) -> String {
        let path = location["path"]
            .as_str()
            .or_else(|| location["uri"].as_str())
            .unwrap_or_default();
        let name = self.name(path);
        let range = &location["range"];
        let text = self.text(&name).and_then(|text| {
            let start = offset_of(
                text,
                range["start"]["line"].as_u64()?,
                range["start"]["character"].as_u64()?,
            )?;
            let end = offset_of(
                text,
                range["end"]["line"].as_u64()?,
                range["end"]["character"].as_u64()?,
            )?;
            (start <= end).then(|| text[start..end].to_string())
        });
        match text {
            Some(text) => format!("{} {text:?}", stem(&name)),
            None => format!(
                "{} {}",
                name,
                self.span(&name, range)
                    .split(' ')
                    .next()
                    .unwrap_or_default()
            ),
        }
    }
}

fn indent(text: &str) -> String {
    text.lines()
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render(files: &Files<'_>, method: &str, answer: &Value, out: &mut String) {
    if let Some(error) = answer.get("error") {
        out.push_str(&format!(
            "{method}: error {}\n",
            error.as_str().unwrap_or("?")
        ));
        return;
    }
    let result = &answer["result"];
    if result.is_null() {
        out.push_str(&format!("{method}: null\n"));
        return;
    }
    match method {
        "hover" => {
            out.push_str(&format!(
                "hover: {}\n{}\n",
                files.span_of_answer(result),
                indent(result["signature"].as_str().unwrap_or_default())
            ));
            let docs = result["documentation"].as_str().unwrap_or_default();
            if !docs.is_empty() {
                out.push_str(&format!("  documentation:\n{}\n", indent(docs)));
            }
        }
        "ttSymbol" => {
            out.push_str(&format!(
                "ttSymbol: {} {} of {} at {}{}\n",
                result["kind"].as_str().unwrap_or_default(),
                result["name"].as_str().unwrap_or_default(),
                result["variantName"].as_str().unwrap_or_default(),
                files.span_of_answer(result),
                if result["binds"].as_bool() == Some(true) {
                    ", binds"
                } else {
                    ""
                }
            ));
            out.push_str(&format!(
                "{}\n",
                indent(result["signature"].as_str().unwrap_or_default())
            ));
            let detail = result["detail"].as_str().unwrap_or_default();
            if !detail.is_empty() {
                out.push_str(&format!("  detail:\n{}\n", indent(detail)));
            }
            if !result["definition"].is_null() {
                out.push_str(&format!(
                    "  definition: {}\n",
                    files.located(&result["definition"])
                ));
            }
        }
        "completion" => {
            let mut items = result["items"].as_array().cloned().unwrap_or_default();
            items.sort_by_key(|item| {
                (
                    item["sortText"].as_str().unwrap_or_default().to_string(),
                    item["label"].as_str().unwrap_or_default().to_string(),
                    item.to_string(),
                )
            });
            let (local, rest): (Vec<_>, Vec<_>) = items.iter().partition(|item| {
                item["sortText"]
                    .as_str()
                    .unwrap_or("")
                    .trim_start_matches('z')
                    != GLOBALS_OR_KEYWORDS
                    || !item["source"].is_null()
            });
            out.push_str(&format!(
                "completion: {} item(s), member {}, probe {}\n",
                items.len(),
                result["member"],
                if result["probe"].is_null() {
                    "no"
                } else {
                    "yes"
                }
            ));
            for item in local {
                out.push_str(&format!("  {}\n", completion_line(item)));
            }
            if !rest.is_empty() {
                out.push_str(&format!(
                    "  ... {} global or keyword item(s) (sortText {GLOBALS_OR_KEYWORDS}, no source)\n",
                    rest.len()
                ));
            }
        }
        "ttCompletions" => {
            out.push_str(&format!(
                "ttCompletions: pattern {}, member {}\n",
                result["pattern"], result["member"]
            ));
            for item in result["items"].as_array().into_iter().flatten() {
                out.push_str(&format!("  {}\n", tt_item_line(item)));
            }
            let keywords: Vec<String> = result["keywords"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|k| {
                    format!(
                        "{} ({})",
                        k["label"].as_str().unwrap_or_default(),
                        k["sortText"].as_str().unwrap_or_default()
                    )
                })
                .collect();
            if !keywords.is_empty() {
                out.push_str(&format!("  keywords: {}\n", keywords.join(", ")));
            }
        }
        "patternCompletions" => {
            out.push_str("patternCompletions:\n");
            for item in result["items"].as_array().into_iter().flatten() {
                out.push_str(&format!("  {}\n", tt_item_line(item)));
            }
        }
        "definition" | "references" => {
            let mut lines: Vec<String> = result["locations"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|location| {
                    format!(
                        "  {}{}",
                        files.located(location),
                        if location["isDefinition"].as_bool() == Some(true) {
                            " (definition)"
                        } else {
                            ""
                        }
                    )
                })
                .collect();
            lines.sort();
            out.push_str(&format!("{method}: {} location(s)\n", lines.len()));
            for line in lines {
                out.push_str(&format!("{line}\n"));
            }
        }
        "prepareRename" => {
            if result["range"].is_null() {
                out.push_str(&format!("prepareRename: refused {}\n", result["refusal"]));
            } else {
                out.push_str(&format!(
                    "prepareRename: {}\n",
                    files.span_of_answer(result)
                ));
            }
        }
        "rename" => {
            let mut lines: Vec<String> = result["edits"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|edit| {
                    format!(
                        "  {}{}",
                        files.located(edit),
                        edit["newText"]
                            .as_str()
                            .map(|text| format!(" -> {text:?}"))
                            .unwrap_or_default()
                    )
                })
                .collect();
            lines.sort();
            if result["edits"].is_null() {
                out.push_str("rename: no edits\n");
            } else {
                out.push_str(&format!("rename: {} edit(s)\n", lines.len()));
            }
            for line in lines {
                out.push_str(&format!("{line}\n"));
            }
        }
        "signatureHelp" => {
            out.push_str(&format!(
                "signatureHelp: active signature {}, active parameter {}\n",
                result["activeSignature"], result["activeParameter"]
            ));
            for signature in result["signatures"].as_array().into_iter().flatten() {
                let label = signature["label"].as_str().unwrap_or_default();
                out.push_str(&format!("  {label}\n"));
                let parameters: Vec<String> = signature["parameters"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|parameter| utf16_slice(label, &parameter["label"]))
                    .collect();
                if !parameters.is_empty() {
                    out.push_str(&format!("    parameters: {}\n", parameters.join(" | ")));
                }
                let docs = signature["documentation"].as_str().unwrap_or_default();
                if !docs.is_empty() {
                    out.push_str(&format!("    documentation:\n{}\n", indent(&indent(docs))));
                }
            }
        }
        "documentSemanticTokens" => {
            let tokens = result["tokens"].as_array().cloned().unwrap_or_default();
            out.push_str(&format!("semanticTokens: {} token(s)\n", tokens.len()));
            for token in tokens {
                out.push_str(&format!("  {}\n", token_line(files, &token)));
            }
        }
        "tsDiagnostics" => {
            let diagnostics = result["diagnostics"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            out.push_str(&format!(
                "diagnostics: {} diagnostic(s)\n",
                diagnostics.len()
            ));
            for d in diagnostics {
                out.push_str(&format!("  {}\n", diagnostic_line(files, &d)));
            }
            let restates: Vec<&str> = result["restates"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            if !restates.is_empty() {
                out.push_str(&format!("  restates: {}\n", restates.join(", ")));
            }
        }
        other => out.push_str(&format!("{other}: {result}\n")),
    }
}

impl Files<'_> {
    fn span_of_answer(&self, answer: &Value) -> String {
        let current = self.current.borrow().clone();
        self.span(&current, &answer["range"])
    }
}

fn completion_line(item: &Value) -> String {
    let mut line = format!(
        "{} ({}, {}){}",
        item["label"].as_str().unwrap_or_default(),
        lsp_completion_kind(&item["kind"]),
        item["sortText"].as_str().unwrap_or_default(),
        lsp_completion_tags(item)
    );
    if let Some(insert) = item["insertText"].as_str()
        && Some(insert) != item["label"].as_str()
    {
        line.push_str(&format!(" insert {insert:?}"));
    }
    if item["snippet"].as_bool() == Some(true) {
        line.push_str(" snippet");
    }
    if let Some(detail) = item["labelDetails"]["detail"].as_str() {
        line.push_str(&format!(" detail {detail:?}"));
    }
    if let Some(description) = item["labelDetails"]["description"].as_str() {
        line.push_str(&format!(" from {description:?}"));
    }
    if let Some(source) = item["source"].as_str() {
        line.push_str(&format!(" source {source:?}"));
    }
    line
}

fn tt_item_line(item: &Value) -> String {
    let range = &item["range"];
    format!(
        "{} ({}){}{}{}",
        item["label"].as_str().unwrap_or_default(),
        item["kind"].as_str().unwrap_or_default(),
        item["detail"]
            .as_str()
            .filter(|d| !d.is_empty())
            .map(|d| format!(" {d:?}"))
            .unwrap_or_default(),
        if item["covered"].as_bool() == Some(true) {
            " covered"
        } else {
            ""
        },
        if range.is_object() {
            format!(
                " replaces {}:{}-{}:{}",
                range["start"]["line"].as_u64().unwrap_or(0) + 1,
                range["start"]["character"].as_u64().unwrap_or(0) + 1,
                range["end"]["line"].as_u64().unwrap_or(0) + 1,
                range["end"]["character"].as_u64().unwrap_or(0) + 1
            )
        } else {
            String::new()
        }
    )
}

fn utf16_slice(label: &str, span: &Value) -> String {
    let from = span[0].as_u64().unwrap_or(0) as usize;
    let to = span[1].as_u64().unwrap_or(0) as usize;
    let units: Vec<u16> = label.encode_utf16().collect();
    String::from_utf16_lossy(&units[from.min(units.len())..to.min(units.len())])
}

fn token_line(files: &Files<'_>, token: &Value) -> String {
    let modifiers: Vec<&str> = token["modifiers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    format!(
        "{} {}{}",
        files.span_of_answer(token),
        token["type"].as_str().unwrap_or_default(),
        if modifiers.is_empty() {
            String::new()
        } else {
            format!(" [{}]", modifiers.join(", "))
        }
    )
}

fn diagnostic_line(files: &Files<'_>, d: &Value) -> String {
    let tags: Vec<&str> = d["tags"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let mut line = format!(
        "{} {} TS{}: {}{}",
        files.span_of_answer(d),
        d["severity"].as_str().unwrap_or_default(),
        d["code"],
        d["message"]
            .as_str()
            .unwrap_or_default()
            .replace('\n', "\n      "),
        if tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", tags.join(", "))
        }
    );
    for related in d["related"].as_array().into_iter().flatten() {
        line.push_str(&format!(
            "\n    related {}: {}",
            files.span_of_answer(related),
            related["message"].as_str().unwrap_or_default()
        ));
    }
    line
}

fn lsp_completion_kind(kind: &Value) -> String {
    const KINDS: [&str; 25] = [
        "Text",
        "Method",
        "Function",
        "Constructor",
        "Field",
        "Variable",
        "Class",
        "Interface",
        "Module",
        "Property",
        "Unit",
        "Value",
        "Enum",
        "Keyword",
        "Snippet",
        "Color",
        "File",
        "Reference",
        "Folder",
        "EnumMember",
        "Constant",
        "Struct",
        "Event",
        "Operator",
        "TypeParameter",
    ];
    match kind.as_u64() {
        Some(n) if (1..=25).contains(&n) => KINDS[n as usize - 1].to_string(),
        Some(n) => format!("kind {n}"),
        None => "no kind".to_string(),
    }
}

fn lsp_completion_tags(item: &Value) -> String {
    let tags: Vec<String> = item["tags"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|tag| match tag.as_u64() {
            Some(1) => "deprecated".to_string(),
            _ => format!("tag {tag}"),
        })
        .collect();
    if tags.is_empty() {
        String::new()
    } else {
        format!(" [{}]", tags.join(", "))
    }
}

fn render_editor_completion(answer: &Value, out: &mut String) {
    if let Some(error) = answer.get("error") {
        out.push_str(&format!("editor completion: error {error}\n"));
        return;
    }
    let result = &answer["result"];
    let mut items = result
        .as_array()
        .or_else(|| result["items"].as_array())
        .cloned()
        .unwrap_or_default();
    items.sort_by_key(|item| {
        (
            item["sortText"].as_str().unwrap_or_default().to_string(),
            item["label"].as_str().unwrap_or_default().to_string(),
            item.to_string(),
        )
    });
    let (local, rest): (Vec<_>, Vec<_>) = items.iter().partition(|item| {
        item["sortText"]
            .as_str()
            .and_then(|sort| sort.strip_prefix('2'))
            .map(|sort| sort.trim_start_matches('z'))
            != Some(GLOBALS_OR_KEYWORDS)
            || !item["data"]["source"].is_null()
    });
    out.push_str(&format!("editor completion: {} item(s)\n", items.len()));
    for item in local {
        out.push_str(&format!(
            "  {} ({}, {}){}\n",
            item["label"].as_str().unwrap_or_default(),
            lsp_completion_kind(&item["kind"]),
            item["sortText"].as_str().unwrap_or_default(),
            lsp_completion_tags(item)
        ));
    }
    if !rest.is_empty() {
        out.push_str(&format!(
            "  ... {} global or keyword item(s) (sortText 2{GLOBALS_OR_KEYWORDS}, no source)\n",
            rest.len()
        ));
    }
}

fn published_code(code: &Value) -> String {
    match code {
        Value::Number(n) => format!("TS{n}"),
        Value::String(text) => text.clone(),
        _ => "-".to_string(),
    }
}

fn render_published(files: &Files<'_>, list: &Value, out: &mut String) {
    let Some(diagnostics) = list.as_array() else {
        out.push_str("published: nothing\n");
        return;
    };
    out.push_str(&format!("published: {} diagnostic(s)\n", diagnostics.len()));
    for d in diagnostics {
        let severity = match d["severity"].as_u64() {
            Some(2) => "warning",
            Some(3) => "information",
            Some(4) => "hint",
            _ => "error",
        };
        let tags: Vec<&str> = d["tags"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|tag| match tag.as_u64() {
                Some(1) => "unnecessary",
                Some(2) => "deprecated",
                _ => "?",
            })
            .collect();
        out.push_str(&format!(
            "  {} {severity} {} ({}): {}{}\n",
            files.span_of_answer(d),
            published_code(&d["code"]),
            d["source"].as_str().unwrap_or("-"),
            d["message"]
                .as_str()
                .unwrap_or_default()
                .replace('\n', "\n      "),
            if tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", tags.join(", "))
            }
        ));
        for related in d["relatedInformation"].as_array().into_iter().flatten() {
            out.push_str(&format!(
                "    related {}: {}\n",
                files.located(&related["location"]),
                related["message"].as_str().unwrap_or_default()
            ));
        }
    }
}

fn caret(unit: &Unit, offset: usize, marker: &str) -> String {
    let start = unit.text[..offset].rfind('\n').map_or(0, |at| at + 1);
    let end = unit.text[offset..]
        .find('\n')
        .map_or(unit.text.len(), |at| offset + at);
    let column = unit.text[start..offset].chars().count();
    format!(
        "  | {}\n  | {}^ /*{marker}*/\n",
        &unit.text[start..end],
        " ".repeat(column)
    )
}

fn write_units(dir: &Path, units: &[Unit]) {
    for unit in units {
        let path = dir.join(&unit.name);
        fs::create_dir_all(path.parent().unwrap()).expect("a writable unit directory");
        fs::write(&path, &unit.text).expect("a writable unit");
    }
    if !dir.join("tsconfig.json").exists() {
        fs::write(dir.join("tsconfig.json"), TSCONFIG).expect("a writable tsconfig");
    }
}

fn relocate(value: &Value, from: &str, to: &str) -> Value {
    match value {
        Value::String(text) => Value::String(text.replace(from, to)),
        Value::Array(items) => Value::Array(items.iter().map(|v| relocate(v, from, to)).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), relocate(v, from, to)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn normalized(answer: &Value, dir: &str) -> Value {
    let mut answer = relocate(answer, dir, "$DIR");
    if let Some(probe) = answer.pointer_mut("/result/probe")
        && !probe.is_null()
    {
        *probe = json!("<probe>");
    }
    if let Some(Value::Array(items)) = answer.pointer_mut("/result/items") {
        items.sort_by_key(Value::to_string);
    }
    answer
}

fn difference(engine: &Value, server: &Value) -> String {
    let items = |answer: &Value| -> Option<Vec<String>> {
        answer["result"]["items"]
            .as_array()
            .map(|items| items.iter().map(Value::to_string).collect())
    };
    if let (Some(left), Some(right)) = (items(engine), items(server)) {
        let mut sorted_left = left.clone();
        let mut sorted_right = right.clone();
        sorted_left.sort();
        sorted_right.sort();
        if sorted_left == sorted_right {
            return format!("  the same {} item(s) in another order", left.len());
        }
        let only = |a: &[String], b: &[String]| -> Vec<String> {
            let b: BTreeSet<&String> = b.iter().collect();
            a.iter()
                .filter(|item| !b.contains(item))
                .take(10)
                .cloned()
                .collect()
        };
        return format!(
            "  engine only: {:?}\n  server only: {:?}",
            only(&sorted_left, &sorted_right),
            only(&sorted_right, &sorted_left)
        );
    }
    format!("  engine: {engine}\n  server: {server}")
}

struct Outcome {
    baseline: String,
    parity: Vec<String>,
    transport: Vec<String>,
}

fn range_check(
    case: &Case,
    question: &Question,
    answer: &Value,
    files: &Files<'_>,
) -> Option<String> {
    let offset = question.offset?;
    let inside = case.units[question.unit]
        .ranges
        .iter()
        .any(|(start, end)| *start <= offset && offset <= *end);
    if !inside {
        return None;
    }
    let expected: BTreeSet<String> = case
        .units
        .iter()
        .flat_map(|unit| {
            unit.ranges.iter().map(move |(start, end)| {
                let from = lsp_position(&unit.text, *start);
                let to = lsp_position(&unit.text, *end);
                format!(
                    "{} {}:{}-{}:{}",
                    unit.name,
                    from.line + 1,
                    from.character + 1,
                    to.line + 1,
                    to.character + 1
                )
            })
        })
        .collect();
    if expected.is_empty() {
        return None;
    }
    let key = if question.verb == Verb::References {
        "locations"
    } else {
        "edits"
    };
    let actual: BTreeSet<String> = answer["result"][key]
        .as_array()
        .into_iter()
        .flatten()
        .map(|location| {
            let path = location["path"].as_str().unwrap_or_default();
            let name = files.name(path);
            let span = files.span(&name, &location["range"]);
            format!("{name} {}", span.split(' ').next().unwrap_or_default())
        })
        .collect();
    (expected != actual).then(|| {
        format!(
            "{}: the {} at /*{}*/ are not the case's [|ranges|]\n  ranges: {expected:?}\n  answer: {actual:?}",
            case.path.display(),
            question.verb.name(),
            question.target
        )
    })
}

fn run(case: &Case) -> Outcome {
    let workspace = Workspace::in_repo("editor-cases");
    let dir = workspace.path().canonicalize().expect("a workspace");
    let project = dir.join("project");
    fs::create_dir_all(&project).expect("a writable project");
    write_units(&project, &case.units);
    let files = Files {
        dir: &project,
        units: &case.units,
        current: std::cell::RefCell::new(String::new()),
    };
    let questions = questions(case, &project);

    let mut engine = ttc::engine::Workspace::new(Engine::new(None));
    let mut server = Server::start(&project);
    for unit in case.units.iter().filter(|unit| is_tt(&unit.name)) {
        let path = project.join(&unit.name);
        engine
            .open_document(&path, unit.text.clone())
            .unwrap_or_else(|e| {
                panic!(
                    "{}: the engine did not open {}: {e}",
                    case.path.display(),
                    unit.name
                )
            });
        let opened = server.ask(
            "openDocument",
            &json!({ "path": path.to_string_lossy(), "text": unit.text }),
        );
        assert!(
            opened.get("error").is_none(),
            "{}: the server did not open {}: {opened}",
            case.path.display(),
            unit.name
        );
    }

    let asks_editor = case
        .verbs
        .iter()
        .any(|(verb, _)| matches!(verb, Verb::Diagnostics | Verb::Completions));
    let mut editor = asks_editor.then(|| {
        let server = extension_server().expect("a built extension server, checked before the run");
        let mut editor = Lsp::editor(&server, &project);
        for unit in case.units.iter().filter(|unit| is_tt(&unit.name)) {
            editor.open(&project.join(&unit.name), &unit.text);
        }
        editor
    });
    let mut published: Option<BTreeMap<String, Value>> = None;

    let dir_text = project.to_string_lossy().into_owned();
    let mut baseline = String::new();
    let mut transport = Vec::new();
    let mut answers: Vec<Vec<(String, Value)>> = Vec::new();
    for question in &questions {
        let unit = &case.units[question.unit];
        files.current.replace(unit.name.clone());
        match question.offset {
            Some(offset) => {
                let at = lsp_position(&unit.text, offset);
                baseline.push_str(&format!(
                    "=== {} /*{}*/ {}:{}:{} ===\n{}",
                    question.verb.name(),
                    question.target,
                    unit.name,
                    at.line + 1,
                    at.character + 1,
                    caret(unit, offset, &question.target)
                ));
            }
            None => baseline.push_str(&format!("=== {} {} ===\n", question.verb.name(), unit.name)),
        }
        let mut answered = Vec::new();
        for request in &question.requests {
            let from_engine = engine_answer(&mut engine, request);
            let from_server = server.ask(request.method, &request.params);
            let (engine_view, server_view) = (
                normalized(&from_engine, &dir_text),
                normalized(&from_server, &dir_text),
            );
            if engine_view != server_view {
                transport.push(format!(
                    "{}: {} for /*{}*/ differs between the transports\n{}",
                    case.path.display(),
                    request.method,
                    question.target,
                    difference(&engine_view, &server_view)
                ));
            }
            render(&files, request.method, &from_server, &mut baseline);
            answered.push((request.method.to_string(), from_server));
        }
        if let Some(editor) = editor.as_mut() {
            let path = project.join(&unit.name);
            match question.verb {
                Verb::Diagnostics => {
                    let all = published.get_or_insert_with(|| {
                        let paths: Vec<PathBuf> = case
                            .units
                            .iter()
                            .filter(|unit| is_tt(&unit.name))
                            .map(|unit| project.join(&unit.name))
                            .collect();
                        editor.settled_publishes(&paths)
                    });
                    let list = all.get(&uri(&path)).cloned().unwrap_or(Value::Null);
                    render_published(&files, &list, &mut baseline);
                    answered.push(("published".to_string(), list));
                }
                Verb::Completions => {
                    let offset = question.offset.expect("a marker");
                    let at = lsp_position(&unit.text, offset);
                    let answer = editor.request(
                        "textDocument/completion",
                        json!({
                            "textDocument": { "uri": uri(&path) },
                            "position": { "line": at.line, "character": at.character },
                            "context": { "triggerKind": 1 },
                        }),
                    );
                    render_editor_completion(&answer, &mut baseline);
                }
                _ => {}
            }
        }
        if matches!(question.verb, Verb::References | Verb::Rename) {
            let key = if question.verb == Verb::References {
                "references"
            } else {
                "rename"
            };
            if let Some((_, answer)) = answered.iter().find(|(method, _)| method == key)
                && let Some(problem) = range_check(case, question, answer, &files)
            {
                transport.push(problem);
            }
        }
        baseline.push('\n');
        answers.push(answered);
    }
    drop(server);
    drop(editor);

    let mut parity = Vec::new();
    if let Some(twin) = &case.twin {
        let twin_dir = dir.join("twin");
        fs::create_dir_all(&twin_dir).expect("a writable twin project");
        write_units(&twin_dir, twin);
        let twin_files = Files {
            dir: &twin_dir,
            units: twin,
            current: std::cell::RefCell::new(String::new()),
        };
        baseline.push_str("=== parity with the TypeScript twin ===\n");
        let binary = tsgo_binary().expect("the pinned TypeScript's language server");
        let mut lsp = Lsp::start(&binary, &twin_dir);
        for unit in twin
            .iter()
            .filter(|unit| unit.name.ends_with(".ts") || unit.name.ends_with(".tsx"))
        {
            lsp.open(&twin_dir.join(&unit.name), &unit.text);
        }
        for (question, answered) in questions.iter().zip(&answers) {
            let unit = &case.units[question.unit];
            let twin_unit_name = twin_name(&unit.name);
            let Some(twin_unit) = twin.iter().find(|u| u.name == twin_unit_name) else {
                continue;
            };
            let twin_offset = match question.offset {
                Some(_) => match twin_unit
                    .markers
                    .iter()
                    .find(|(marker, _)| *marker == question.target)
                {
                    Some((_, offset)) => Some(*offset),
                    None => continue,
                },
                None if twin_unit.text == unit.text => None,
                None => continue,
            };
            files.current.replace(unit.name.clone());
            let tt_view = parity_view(question.verb, &files, answered, None);
            let ts_answer = twin_answer(
                &mut lsp,
                question.verb,
                &twin_dir.join(&twin_unit.name),
                twin_unit,
                twin_offset,
            );
            twin_files.current.replace(twin_unit.name.clone());
            let ts_view = parity_view(question.verb, &twin_files, &ts_answer, Some(&lsp.legend));
            let label = format!("{} {} {}", case.name, question.verb.name(), question.target);
            if tt_view == ts_view {
                baseline.push_str(&format!(
                    "{} {}: same\n",
                    question.verb.name(),
                    question.target
                ));
            } else {
                baseline.push_str(&format!(
                    "{} {}: differs\n{}",
                    question.verb.name(),
                    question.target,
                    parity_difference(&tt_view, &ts_view)
                ));
                parity.push(label);
            }
        }
        baseline.push('\n');
    }
    Outcome {
        baseline: baseline.replace(&dir_text, "$DIR"),
        parity,
        transport,
    }
}

fn twin_answer(
    lsp: &mut Lsp,
    verb: Verb,
    path: &Path,
    unit: &Unit,
    offset: Option<usize>,
) -> Vec<(String, Value)> {
    let document = json!({ "uri": uri(path) });
    let position = offset.map(|offset| {
        let at = lsp_position(&unit.text, offset);
        json!({ "line": at.line, "character": at.character })
    });
    let at = || json!({ "textDocument": document, "position": position });
    let ask = |lsp: &mut Lsp, method: &str, params: Value| -> Value { lsp.request(method, params) };
    match verb {
        Verb::Hover => vec![("hover".into(), ask(lsp, "textDocument/hover", at()))],
        Verb::Completions => vec![(
            "completion".into(),
            ask(
                lsp,
                "textDocument/completion",
                json!({ "textDocument": document, "position": position, "context": { "triggerKind": 1 } }),
            ),
        )],
        Verb::Definition => vec![(
            "definition".into(),
            ask(lsp, "textDocument/definition", at()),
        )],
        Verb::References => vec![(
            "references".into(),
            ask(
                lsp,
                "textDocument/references",
                json!({ "textDocument": document, "position": position, "context": { "includeDeclaration": true } }),
            ),
        )],
        Verb::Rename => vec![(
            "rename".into(),
            ask(
                lsp,
                "textDocument/rename",
                json!({ "textDocument": document, "position": position, "newName": ttc::engine::RENAME_PLACEHOLDER }),
            ),
        )],
        Verb::SignatureHelp => vec![(
            "signatureHelp".into(),
            ask(
                lsp,
                "textDocument/signatureHelp",
                json!({ "textDocument": document, "position": position, "context": { "triggerKind": 1, "isRetrigger": false } }),
            ),
        )],
        Verb::SemanticTokens => vec![(
            "semanticTokens".into(),
            ask(
                lsp,
                "textDocument/semanticTokens/full",
                json!({ "textDocument": document }),
            ),
        )],
        Verb::Diagnostics => vec![(
            "diagnostics".into(),
            ask(
                lsp,
                "textDocument/diagnostic",
                json!({ "textDocument": document }),
            ),
        )],
    }
}

fn parity_difference(tt: &str, ts: &str) -> String {
    let count = |text: &str| {
        let mut lines: BTreeMap<String, usize> = BTreeMap::new();
        for line in text.lines() {
            *lines.entry(line.to_string()).or_default() += 1;
        }
        lines
    };
    let (tt_lines, ts_lines) = (count(tt), count(ts));
    let mut out = String::new();
    for (line, n) in &tt_lines {
        for _ in ts_lines.get(line).copied().unwrap_or(0)..*n {
            out.push_str(&format!("  tt only: {line}\n"));
        }
    }
    for (line, n) in &ts_lines {
        for _ in tt_lines.get(line).copied().unwrap_or(0)..*n {
            out.push_str(&format!("  ts only: {line}\n"));
        }
    }
    if out.is_empty() {
        out.push_str("  the same lines in another order\n");
    }
    out
}

fn split_markdown_hover(markdown: &str) -> (String, String) {
    let trimmed = markdown.trim();
    if let Some(rest) = trimmed.strip_prefix("```")
        && let Some(newline) = rest.find('\n')
    {
        let body = &rest[newline + 1..];
        let (code, prose) = match body.find("\n```") {
            Some(close) => {
                let after = &body[close + 4..];
                (
                    &body[..close],
                    after.find('\n').map_or("", |line| &after[line + 1..]),
                )
            }
            None => (body.strip_suffix("```").unwrap_or(body), ""),
        };
        return (code.trim().to_string(), prose.trim().to_string());
    }
    (String::new(), trimmed.to_string())
}

fn docs_text(documentation: &Value) -> String {
    match documentation {
        Value::String(s) => s.trim().to_string(),
        value => value["value"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

fn answer_of<'a>(answers: &'a [(String, Value)], method: &str) -> &'a Value {
    answers
        .iter()
        .find(|(name, _)| name == method)
        .map(|(_, value)| value)
        .unwrap_or(&Value::Null)
}

/// What an answer is compared as, the same way for the engine's answer on
/// the `.tt` source and `tsgo --lsp`'s on the TypeScript twin:
///
/// - a location is its file's stem and the text it covers (`main "x"`), so
///   the two files' different lengths and extensions do not count; a place
///   outside the case is its file name and position;
/// - hover is the signature and documentation, split out of TypeScript's
///   markdown the way the engine splits it;
/// - completion is the sorted set of labels with their LSP kinds, since the
///   ranking layer is the adapter's;
/// - signature help is each label with its parameters, and the active
///   signature and parameter;
/// - semantic tokens are compared only when the twin's text is the source's
///   (a `.tt` file with no tt syntax), decoded through TypeScript's legend;
/// - diagnostics likewise, the adapter's published list against
///   TypeScript's pull answer, as range, code, severity, and message;
/// - rename is the set of edited spans with their text; references are the
///   set of referenced spans.
fn parity_view(
    verb: Verb,
    files: &Files<'_>,
    answers: &[(String, Value)],
    legend: Option<&(Vec<String>, Vec<String>)>,
) -> String {
    let lsp = legend.is_some();
    let result = |method: &str| -> Value {
        let answer = answer_of(answers, method);
        match answer.get("error") {
            Some(error) => json!({ "error": error }),
            None => answer["result"].clone(),
        }
    };
    match verb {
        Verb::Hover => {
            let hover = result("hover");
            if hover.is_null() {
                return "null".into();
            }
            let (signature, documentation) = if lsp {
                match &hover["contents"] {
                    Value::String(markdown) => split_markdown_hover(markdown),
                    contents if contents["kind"] == "markdown" => {
                        split_markdown_hover(contents["value"].as_str().unwrap_or_default())
                    }
                    contents => (docs_text(contents), String::new()),
                }
            } else {
                (
                    hover["signature"].as_str().unwrap_or_default().to_string(),
                    hover["documentation"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                )
            };
            format!("{signature}\n---\n{documentation}")
        }
        Verb::Completions => {
            let completion = result("completion");
            let items = if lsp {
                completion["items"]
                    .as_array()
                    .or_else(|| completion.as_array())
                    .cloned()
                    .unwrap_or_default()
            } else {
                completion["items"].as_array().cloned().unwrap_or_default()
            };
            let labels: BTreeSet<String> = items
                .iter()
                .filter_map(|item| {
                    item["label"].as_str().map(|label| {
                        format!(
                            "{label} ({}){}",
                            lsp_completion_kind(&item["kind"]),
                            lsp_completion_tags(item)
                        )
                    })
                })
                .collect();
            labels.into_iter().collect::<Vec<_>>().join("\n")
        }
        Verb::Definition => {
            let definition = result("definition");
            let locations: Vec<Value> = if lsp {
                match &definition {
                    Value::Array(items) => items.clone(),
                    Value::Null => Vec::new(),
                    single => vec![single.clone()],
                }
                .into_iter()
                .map(|item| {
                    json!({
                        "uri": item["targetUri"].as_str().or(item["uri"].as_str()),
                        "range": if item["targetSelectionRange"].is_null() { item["range"].clone() } else { item["targetSelectionRange"].clone() },
                    })
                })
                .collect()
            } else {
                definition["locations"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
            };
            let set: BTreeSet<String> = locations.iter().map(|l| files.covered(l)).collect();
            set.into_iter().collect::<Vec<_>>().join("\n")
        }
        Verb::References => {
            let references = result("references");
            let locations = if lsp {
                references.as_array().cloned().unwrap_or_default()
            } else {
                references["locations"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
            };
            let set: BTreeSet<String> = locations.iter().map(|l| files.covered(l)).collect();
            set.into_iter().collect::<Vec<_>>().join("\n")
        }
        Verb::Rename => {
            let rename = result("rename");
            let mut set = BTreeSet::new();
            if lsp {
                for (target, edits) in rename["changes"].as_object().into_iter().flatten() {
                    for edit in edits.as_array().into_iter().flatten() {
                        let location = json!({ "uri": target, "range": edit["range"] });
                        set.insert(format!(
                            "{} -> {}",
                            files.covered(&location),
                            edit["newText"].as_str().unwrap_or_default()
                        ));
                    }
                }
                for change in rename["documentChanges"].as_array().into_iter().flatten() {
                    for edit in change["edits"].as_array().into_iter().flatten() {
                        let location =
                            json!({ "uri": change["textDocument"]["uri"], "range": edit["range"] });
                        set.insert(format!(
                            "{} -> {}",
                            files.covered(&location),
                            edit["newText"].as_str().unwrap_or_default()
                        ));
                    }
                }
            } else {
                for edit in rename["edits"].as_array().into_iter().flatten() {
                    set.insert(format!(
                        "{} -> {}",
                        files.covered(edit),
                        edit["newText"]
                            .as_str()
                            .unwrap_or(ttc::engine::RENAME_PLACEHOLDER)
                    ));
                }
            }
            set.into_iter().collect::<Vec<_>>().join("\n")
        }
        Verb::SignatureHelp => {
            let help = result("signatureHelp");
            if help.is_null() {
                return "null".into();
            }
            let mut out = format!(
                "active {} {}\n",
                help["activeSignature"].as_u64().unwrap_or(0),
                help["activeParameter"].as_u64().unwrap_or(0)
            );
            for signature in help["signatures"].as_array().into_iter().flatten() {
                let label = signature["label"].as_str().unwrap_or_default();
                let parameters: Vec<String> = signature["parameters"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|p| match &p["label"] {
                        Value::String(text) => text.clone(),
                        span => utf16_slice(label, span),
                    })
                    .collect();
                out.push_str(&format!("{label} [{}]\n", parameters.join(" | ")));
            }
            out
        }
        Verb::SemanticTokens => {
            let tokens = result("documentSemanticTokens");
            let mut lines = Vec::new();
            if let Some((types, modifiers)) = legend {
                let data = result("semanticTokens")["data"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                let (mut line, mut character) = (0u64, 0u64);
                for chunk in data.chunks(5) {
                    let delta_line = chunk[0].as_u64().unwrap_or(0);
                    let delta_start = chunk[1].as_u64().unwrap_or(0);
                    line += delta_line;
                    character = if delta_line == 0 {
                        character + delta_start
                    } else {
                        delta_start
                    };
                    let length = chunk[2].as_u64().unwrap_or(0);
                    let kind = types
                        .get(chunk[3].as_u64().unwrap_or(0) as usize)
                        .cloned()
                        .unwrap_or_default();
                    let bits = chunk[4].as_u64().unwrap_or(0);
                    let mods: Vec<&str> = modifiers
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| bits & (1 << i) != 0)
                        .map(|(_, m)| m.as_str())
                        .collect();
                    lines.push(format!(
                        "{}:{}+{} {kind} [{}]",
                        line + 1,
                        character + 1,
                        length,
                        mods.join(", ")
                    ));
                }
            } else {
                for token in tokens["tokens"].as_array().into_iter().flatten() {
                    let range = &token["range"];
                    let mut mods: Vec<&str> = token["modifiers"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .collect();
                    mods.sort_unstable();
                    lines.push(format!(
                        "{}:{}+{} {} [{}]",
                        range["start"]["line"].as_u64().unwrap_or(0) + 1,
                        range["start"]["character"].as_u64().unwrap_or(0) + 1,
                        range["end"]["character"].as_u64().unwrap_or(0)
                            - range["start"]["character"].as_u64().unwrap_or(0),
                        token["type"].as_str().unwrap_or_default(),
                        mods.join(", ")
                    ));
                }
            }
            if lsp {
                lines = lines
                    .into_iter()
                    .map(|line| {
                        let (head, mods) = line.split_once(" [").unwrap_or((&line, "]"));
                        let mut mods: Vec<&str> = mods
                            .trim_end_matches(']')
                            .split(", ")
                            .filter(|m| !m.is_empty())
                            .collect();
                        mods.sort_unstable();
                        format!("{head} [{}]", mods.join(", "))
                    })
                    .collect();
            }
            lines.join("\n")
        }
        Verb::Diagnostics => {
            let mut lines = Vec::new();
            if lsp {
                let report = result("diagnostics");
                for d in report["items"].as_array().into_iter().flatten() {
                    let severity = match d["severity"].as_u64() {
                        Some(2) => "warning",
                        Some(3) => "information",
                        Some(4) => "hint",
                        _ => "error",
                    };
                    lines.push(format!(
                        "{} {severity} TS{}: {}",
                        files.span_of_answer(d),
                        d["code"],
                        d["message"].as_str().unwrap_or_default()
                    ));
                }
            } else {
                let list = answer_of(answers, "published");
                for d in list.as_array().into_iter().flatten() {
                    let severity = match d["severity"].as_u64() {
                        Some(2) => "warning",
                        Some(3) => "information",
                        Some(4) => "hint",
                        _ => "error",
                    };
                    let code = match &d["code"] {
                        Value::String(text) => text
                            .strip_prefix("ts")
                            .filter(|digits| digits.bytes().all(|b| b.is_ascii_digit()))
                            .map_or_else(
                                || d["code"].clone(),
                                |digits| json!(digits.parse::<u64>().unwrap_or(0)),
                            ),
                        code => code.clone(),
                    };
                    lines.push(format!(
                        "{} {severity} TS{code}: {}",
                        files.span_of_answer(d),
                        d["message"].as_str().unwrap_or_default()
                    ));
                }
            }
            lines.sort();
            lines.join("\n")
        }
    }
}

#[test]
fn every_editor_case_matches_its_baseline() {
    assert!(
        !(updating() && !toolchain_installed()),
        "UPDATE_EXPECT would rewrite the editor baselines without a TypeScript to answer \
         them — run `npm ci` at the repository root first"
    );
    if !toolchain() {
        eprintln!("SKIP the editor cases: no TypeScript installed — run `npm ci`");
        return;
    }
    if extension_server().is_none() {
        assert!(
            !updating() && !extension_required(),
            "the editor cases ask the VS Code adapter what it publishes, and its language \
             server is not built — run `npm ci --prefix editors/vscode && npm --prefix \
             editors/vscode run compile`"
        );
        eprintln!(
            "SKIP the editor cases: the VS Code adapter is not built — run `npm ci --prefix \
             editors/vscode && npm --prefix editors/vscode run compile`"
        );
        return;
    }
    let cases = cases();
    let filtered = std::env::var("TT_CASES").is_ok_and(|f| !f.is_empty());
    let next = AtomicUsize::new(0);
    let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let parity: Mutex<BTreeMap<String, Vec<String>>> = Mutex::new(BTreeMap::new());
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
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let outcome = run(case);
                        expect(
                            &reference().join(format!("{}.baseline", case.name)),
                            &outcome.baseline,
                        );
                        outcome
                    }));
                    match outcome {
                        Ok(outcome) => {
                            parity
                                .lock()
                                .unwrap()
                                .insert(case.name.clone(), outcome.parity);
                            if !outcome.transport.is_empty() {
                                failures
                                    .lock()
                                    .unwrap()
                                    .push(outcome.transport.join("\n\n"));
                            }
                        }
                        Err(payload) => {
                            let message = payload
                                .downcast_ref::<String>()
                                .cloned()
                                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                                .unwrap_or_else(|| "a non-string panic".to_string());
                            failures
                                .lock()
                                .unwrap()
                                .push(format!("{}:\n{message}", case.path.display()));
                        }
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().unwrap();
    assert!(
        failures.is_empty(),
        "{} editor case(s) failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
    if !filtered {
        let listed: Vec<String> = parity
            .into_inner()
            .unwrap()
            .into_values()
            .flatten()
            .collect();
        let path = reference().join("failingParity.txt");
        if listed.is_empty() {
            expect_absent(&path);
        } else {
            expect(&path, &format!("{}\n", listed.join("\n")));
        }
    }
}
