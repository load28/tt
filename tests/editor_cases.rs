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
use common::baseline::{expect, expect_absent, not_sampled, updating};
use common::matrix;
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

const STANDARD_LIBRARY: &str = "@tt/std";

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

#[derive(Clone)]
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
    ignores: BTreeSet<String>,
    twin: Option<Vec<Unit>>,
    matrix: Option<PathBuf>,
    server_only: bool,
    expected: Option<ExpectedDiagnostic>,
}

#[derive(Clone)]
struct ExpectedDiagnostic {
    code: String,
    typed_only: bool,
}

#[derive(Default)]
struct Directives {
    verbs: Vec<(Verb, Vec<String>)>,
    ignores: BTreeSet<String>,
    expected: Option<String>,
    typed_only: bool,
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
    directives: Option<&mut Directives>,
) -> Vec<Unit> {
    let mut directives = directives;
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
        let Some(directives) = directives.as_deref_mut() else {
            panic!("{}: a TypeScript twin takes no verbs", path.display());
        };
        let targets = value
            .split(',')
            .map(str::trim)
            .filter(|target| !target.is_empty())
            .map(String::from);
        if name == "parityignores" {
            directives.ignores.extend(targets);
            continue;
        }
        if name == "expectdiagnostic" {
            assert!(
                ttc::DiagnosticCode::parse(value).is_some(),
                "{}: @expectDiagnostic names `{value}`, which is no tt diagnostic code",
                path.display()
            );
            directives.expected = Some(value.to_string());
            continue;
        }
        if name == "typedonly" {
            assert_eq!(value, "true", "{}: @typedOnly takes `true`", path.display());
            directives.typed_only = true;
            continue;
        }
        let verb = Verb::parse(&name).unwrap_or_else(|| {
            panic!(
                "{}: unknown directive `@{name}`; an editor case takes @filename, @parityIgnores, \
                 @expectDiagnostic, @typedOnly, and the verbs hover, completions, definition, references, rename, \
                 signatureHelp, semanticTokens, diagnostics",
                path.display()
            )
        });
        directives.verbs.push((verb, targets.collect()));
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

const MATRIX: &str = "tests/cases/editor/matrix";

const MATRIX_SAMPLE: usize = 40;

const MATRIX_SEED: u64 = 0x7474_6564_6974_6f72;

fn case_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("a readable case entry").path();
        if path.is_dir() {
            case_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "tt" || e == "ttx") {
            out.push(path);
        }
    }
}

struct Selection {
    cases: Vec<Case>,
    unsampled: Vec<Case>,
    summary: Option<String>,
}

fn cases() -> Selection {
    let dir = root().join("tests/cases/editor");
    let mut files = Vec::new();
    case_files(&dir, &mut files);
    files.sort();
    assert!(
        !files.is_empty(),
        "no editor cases under tests/cases/editor"
    );
    let filter = std::env::var("TT_CASES").ok().filter(|f| !f.is_empty());
    let generated = root().join(MATRIX);
    let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut chosen = Vec::new();
    let mut matrix_cases = Vec::new();
    for path in files {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        if let Some(other) = seen.insert(name.clone(), path.clone()) {
            panic!(
                "editor case names must be distinct, because baselines are named by them: {} and {}",
                other.display(),
                path.display()
            );
        }
        if filter.as_deref().is_some_and(|f| !name.contains(f)) {
            continue;
        }
        if filter.is_none() && path.starts_with(&generated) {
            matrix_cases.push((name, path));
        } else {
            chosen.push((name, path));
        }
    }
    let matrix::Sample {
        sampled,
        unsampled,
        summary,
    } = matrix::sample(matrix_cases, MATRIX_SAMPLE, MATRIX_SEED);
    chosen.extend(sampled);
    let parse = |(name, path): (String, PathBuf)| parse_case(name, path, &generated);
    Selection {
        cases: chosen.into_iter().map(parse).collect(),
        unsampled: unsampled.into_iter().map(parse).collect(),
        summary,
    }
}

fn parse_case(name: String, path: PathBuf, generated: &Path) -> Case {
    let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
    let text = fs::read_to_string(&path).expect("a readable editor case");
    let mut directives = Directives::default();
    let units = parse_units(&text, &file_name, &path, Some(&mut directives));
    let Directives {
        verbs,
        ignores,
        expected,
        typed_only,
    } = directives;
    assert!(
        expected.is_some() || !typed_only,
        "{}: @typedOnly qualifies an @expectDiagnostic",
        path.display()
    );
    assert!(
        expected.is_none() || verbs.iter().any(|(verb, _)| *verb == Verb::Diagnostics),
        "{}: @expectDiagnostic is a claim about the published diagnostics, so the case asks @diagnostics",
        path.display()
    );
    let expected = expected.map(|code| ExpectedDiagnostic { code, typed_only });
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
            let mut own = parse_units(&text, &twin_file, &twin, None);
            let expected: BTreeSet<String> = units.iter().map(|u| twin_name(&u.name)).collect();
            let actual: BTreeSet<String> = own.iter().map(|u| u.name.clone()).collect();
            assert!(
                actual.is_subset(&expected)
                    && units
                        .iter()
                        .filter(|u| is_tt(&u.name))
                        .all(|u| actual.contains(&twin_name(&u.name))),
                "{}: the twin's units must be the case's, with .ts/.tsx for .tt/.ttx; a unit that is not .tt/.ttx may be left out to share the case's",
                path.display()
            );
            own.extend(
                units
                    .iter()
                    .filter(|u| !actual.contains(&u.name))
                    .cloned(),
            );
            own
        });
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
    let matrix = path
        .strip_prefix(generated)
        .ok()
        .map(|inside| inside.parent().unwrap_or(Path::new("")).to_path_buf());
    Case {
        name,
        path,
        units,
        verbs,
        ignores,
        twin,
        server_only: matrix.is_some(),
        matrix,
        expected,
    }
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
            let retains: Vec<_> = project.service_retained_syntax(path)?.into_iter()
                .map(|(code, start)| json!({ "code": code.as_str(), "start": { "line": start.line, "character": start.character } }))
                .collect();
            Ok(json!({ "diagnostics": diagnostics, "restates": restates, "retains": retains }))
        }),
        "completionResolve" => workspace.project_for(path).and_then(|project| {
            Ok(
                match project.completion_resolve(
                    path,
                    position,
                    params["label"].as_str().unwrap_or_default(),
                    params["source"].as_str(),
                    params["probe"].as_u64(),
                )? {
                    None => Value::Null,
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
                                    json!({ "range": range_json(edit.range), "newText": edit.new_text })
                                })
                                .collect();
                        }
                        answer
                    }
                },
            )
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
        let decoded = path.strip_prefix("file://").map(percent_decoded);
        let path = decoded.as_deref().unwrap_or(path);
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
        let unit = self.units.iter().find(|unit| unit.name == name);
        let text = unit.and_then(|unit| {
            let text = &unit.text;
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
            let anchor = unit
                .markers
                .iter()
                .find(|(_, offset)| *offset == start)
                .map_or_else(String::new, |(marker, _)| format!("/*{marker}*/ "));
            (start <= end).then(|| format!("{anchor}{:?}", &text[start..end]))
        });
        match text {
            Some(text) => format!("{} {text}", stem(&name)),
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

fn percent_decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let escaped = (bytes[at] == b'%')
            .then(|| text.get(at + 1..at + 3))
            .flatten()
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escaped {
            Some(byte) => {
                out.push(byte);
                at += 3;
            }
            None => {
                out.push(bytes[at]);
                at += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_string())
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

    fn diagnostic_place(&self, answer: &Value, as_location: bool) -> String {
        if !as_location {
            return self.span_of_answer(answer);
        }
        let current = self.current.borrow().clone();
        let path = self.dir.join(&current);
        self.covered(&json!({ "path": path.to_string_lossy(), "range": answer["range"] }))
    }
}

struct Token {
    line: u64,
    start: u64,
    length: u64,
    kind: String,
    modifiers: Vec<String>,
}

impl Token {
    fn shown(&self) -> String {
        format!("{} [{}]", self.kind, self.modifiers.join(", "))
    }

    fn covers(&self, at: Position) -> bool {
        self.line == u64::from(at.line)
            && self.start <= u64::from(at.character)
            && u64::from(at.character) < self.start + self.length
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

/// The entries of a completion answer that import their name from a
/// module: an auto-import, whose `source` is the module specifier.
fn imported_entries(answer: &Value) -> Vec<(String, String)> {
    let mut entries: Vec<(String, String)> = answer["result"]["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|item| item["labelDetails"]["description"].is_string())
        .filter_map(|item| {
            Some((
                item["label"].as_str()?.to_string(),
                item["source"].as_str()?.to_string(),
            ))
        })
        .collect();
    entries.sort();
    entries
}

fn render_resolve(files: &Files<'_>, label: &str, source: &str, answer: &Value, out: &mut String) {
    let result = &answer["result"];
    if let Some(error) = answer.get("error") {
        out.push_str(&format!(
            "  resolve {label} from {source:?}: error {error}\n"
        ));
        return;
    }
    let edits = result["additionalEdits"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    out.push_str(&format!(
        "  resolve {label} from {source:?}: {} edit(s)\n",
        edits.len()
    ));
    for edit in edits {
        out.push_str(&format!(
            "    {} -> {:?}\n",
            files.span_of_answer(&edit),
            edit["newText"].as_str().unwrap_or_default()
        ));
    }
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
    compared: Vec<Compared>,
    transport: Vec<String>,
    /// Where the surfaces that show an `@expectDiagnostic` case's
    /// diagnostic disagree: a one-line summary, then the detail.
    surfaces: Vec<String>,
}

struct Compared {
    question: String,
    heading: String,
    difference: Option<String>,
}

fn heading(question: &Question, unit: &Unit) -> String {
    match question.offset {
        Some(offset) => {
            let at = lsp_position(&unit.text, offset);
            format!(
                "=== {} /*{}*/ {}:{}:{} ===\n{}",
                question.verb.name(),
                question.target,
                unit.name,
                at.line + 1,
                at.character + 1,
                caret(unit, offset, &question.target)
            )
        }
        None => format!("=== {} {} ===\n", question.verb.name(), unit.name),
    }
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

    let asks_editor = case.verbs.iter().any(|(verb, _)| {
        *verb == Verb::Diagnostics || (*verb == Verb::Completions && !case.server_only)
    });
    let mut editor = asks_editor.then(|| {
        let server = extension_server().expect("a built extension server, checked before the run");
        let mut editor = Lsp::editor(&server, &project);
        for unit in case.units.iter().filter(|unit| is_tt(&unit.name)) {
            editor.open(&project.join(&unit.name), &unit.text);
        }
        editor
    });
    let mut engine = (!case.server_only).then(|| ttc::engine::Workspace::new(Engine::new(None)));
    let mut server = Server::start(&project);
    let opened_by_twin = |unit: &Unit| {
        (unit.name.ends_with(".ts") || unit.name.ends_with(".tsx"))
            && case
                .twin
                .as_ref()
                .is_some_and(|twin| twin.iter().any(|other| other.name == unit.name))
    };
    for unit in case
        .units
        .iter()
        .filter(|unit| is_tt(&unit.name) || opened_by_twin(unit))
    {
        let path = project.join(&unit.name);
        if let Some(engine) = engine.as_mut() {
            engine
                .open_document(&path, unit.text.clone())
                .unwrap_or_else(|e| {
                    panic!(
                        "{}: the engine did not open {}: {e}",
                        case.path.display(),
                        unit.name
                    )
                });
        }
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

    let mut published: Option<BTreeMap<String, Value>> = None;

    let dir_text = project.to_string_lossy().into_owned();
    let mut baseline = String::new();
    let mut transport = Vec::new();
    let mut answers: Vec<Vec<(String, Value)>> = Vec::new();
    for question in &questions {
        let unit = &case.units[question.unit];
        files.current.replace(unit.name.clone());
        baseline.push_str(&heading(question, unit));
        let mut answered = Vec::new();
        for request in &question.requests {
            let from_server = server.ask(request.method, &request.params);
            let Some(engine) = engine.as_mut() else {
                answered.push((request.method.to_string(), from_server));
                continue;
            };
            let from_engine = engine_answer(engine, request);
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
            if request.method == "completion" {
                for (label, source) in imported_entries(&from_server) {
                    let resolve = |answer: &Value| Request {
                        method: "completionResolve",
                        params: json!({
                            "path": request.params["path"],
                            "position": request.params["position"],
                            "label": label,
                            "source": source,
                            "probe": answer["result"]["probe"],
                        }),
                    };
                    let (engine_request, server_request) =
                        (resolve(&from_engine), resolve(&from_server));
                    let resolved_engine = engine_answer(engine, &engine_request);
                    let resolved_server = server.ask(server_request.method, &server_request.params);
                    let (engine_view, server_view) = (
                        normalized(&resolved_engine, &dir_text),
                        normalized(&resolved_server, &dir_text),
                    );
                    if engine_view != server_view {
                        transport.push(format!(
                            "{}: completionResolve {label} for /*{}*/ differs between the transports\n{}",
                            case.path.display(),
                            question.target,
                            difference(&engine_view, &server_view)
                        ));
                    }
                    render_resolve(&files, &label, &source, &resolved_server, &mut baseline);
                }
            }
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
                Verb::Completions if !case.server_only => {
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
    let surfaces = match (&case.expected, &published) {
        (Some(expected), Some(published)) => surfaces_agree(case, expected, published, &project),
        _ => Vec::new(),
    };

    let mut parity = Vec::new();
    let mut compared = Vec::new();
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
                None => None,
            };
            let shared: BTreeSet<String> = unit
                .markers
                .iter()
                .filter(|(marker, _)| twin_unit.markers.iter().any(|(other, _)| other == marker))
                .map(|(marker, _)| marker.clone())
                .collect();
            let context = Parity {
                ignores: &case.ignores,
                shared: (twin_unit.text != unit.text).then_some(&shared),
            };
            files.current.replace(unit.name.clone());
            let tt_view = parity_view(question.verb, &files, answered, None, &context);
            let ts_answer = twin_answer(
                &mut lsp,
                question.verb,
                &twin_dir.join(&twin_unit.name),
                twin_unit,
                twin_offset,
            );
            twin_files.current.replace(twin_unit.name.clone());
            let ts_view = parity_view(
                question.verb,
                &twin_files,
                &ts_answer,
                Some(&lsp.legend),
                &context,
            );
            let asked = format!("{} {}", question.verb.name(), question.target);
            let difference = (tt_view != ts_view).then(|| parity_difference(&tt_view, &ts_view));
            match &difference {
                None => baseline.push_str(&format!("{asked}: same\n")),
                Some(lines) => {
                    baseline.push_str(&format!("{asked}: differs\n{lines}"));
                    parity.push(format!("{} {asked}", case.name));
                }
            }
            compared.push(Compared {
                question: asked,
                heading: heading(question, unit),
                difference,
            });
        }
        baseline.push('\n');
    }
    Outcome {
        baseline: baseline.replace(&dir_text, "$DIR"),
        parity,
        compared,
        transport,
        surfaces: surfaces
            .into_iter()
            .map(|problem| problem.replace(&dir_text, "$DIR"))
            .collect(),
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

/// `markdown` with each link to a place in a file (`[name](file:///m.ts#l,c-l,c)`,
/// TypeScript's `{@link}` rendering, one-based) written as the place's unit
/// stem and covered text, the way parity compares locations.
fn linked_places(files: &Files<'_>, markdown: &str) -> String {
    const OPEN: &str = "](file://";
    let mut out = String::new();
    let mut rest = markdown;
    while let Some(at) = rest.find(OPEN) {
        out.push_str(&rest[..at + 2]);
        rest = &rest[at + 2..];
        let Some(close) = rest.find(')') else {
            break;
        };
        let target = &rest[..close];
        let place = target.split_once('#').and_then(|(uri, fragment)| {
            let (from, to) = fragment.split_once('-')?;
            let position = |text: &str| -> Option<Value> {
                let (line, character) = text.split_once(',')?;
                Some(json!({
                    "line": line.trim().parse::<u64>().ok()?.checked_sub(1)?,
                    "character": character.trim().parse::<u64>().ok()?.checked_sub(1)?,
                }))
            };
            let range = json!({ "start": position(from)?, "end": position(to)? });
            Some(files.covered(&json!({ "uri": uri, "range": range })))
        });
        out.push_str(&place.unwrap_or_else(|| target.to_string()));
        rest = &rest[close..];
    }
    out.push_str(rest);
    out
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
///   the two files' different lengths and extensions do not count, led by
///   the marker it starts at when there is one (`main /*use*/ "x"`), so the
///   case's markers tell apart two places with the same text; a place
///   outside the case is its file name and position;
/// - hover is the signature and documentation, split out of TypeScript's
///   markdown the way the engine splits it;
/// - completion is the sorted set of labels with their LSP kinds, since the
///   ranking layer is the adapter's, without the labels the case's
///   `@parityIgnores` names and without entries that import from
///   `@tt/std`, a package a twin's project does not have;
/// - signature help is each label with its parameters, and the active
///   signature and parameter;
/// - semantic tokens, decoded through TypeScript's legend, are every token
///   when the twin's text is the source's, and otherwise the token at each
///   marker the two files share;
/// - diagnostics are the adapter's published list against TypeScript's pull
///   answer, as range, code, severity, and message (the project's directory
///   written `$DIR`), the range written as a location when the twin's text
///   is not the source's;
/// - rename is the set of edited spans with their text; references are the
///   set of referenced spans.
struct Parity<'a> {
    ignores: &'a BTreeSet<String>,
    shared: Option<&'a BTreeSet<String>>,
}

fn parity_view(
    verb: Verb,
    files: &Files<'_>,
    answers: &[(String, Value)],
    legend: Option<&(Vec<String>, Vec<String>)>,
    parity: &Parity<'_>,
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
            format!("{signature}\n---\n{}", linked_places(files, &documentation))
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
                .filter(|item| {
                    item["label"]
                        .as_str()
                        .is_none_or(|label| !parity.ignores.contains(label))
                        && !item["source"]
                            .as_str()
                            .is_some_and(|source| source.starts_with(STANDARD_LIBRARY))
                })
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
            let mut tokens: Vec<Token> = Vec::new();
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
                    let kind = types
                        .get(chunk[3].as_u64().unwrap_or(0) as usize)
                        .cloned()
                        .unwrap_or_default();
                    let bits = chunk[4].as_u64().unwrap_or(0);
                    let modifiers = modifiers
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| bits & (1 << i) != 0)
                        .map(|(_, m)| m.clone())
                        .collect();
                    tokens.push(Token {
                        line,
                        start: character,
                        length: chunk[2].as_u64().unwrap_or(0),
                        kind,
                        modifiers,
                    });
                }
            } else {
                for token in result("documentSemanticTokens")["tokens"]
                    .as_array()
                    .into_iter()
                    .flatten()
                {
                    let range = &token["range"];
                    let start = range["start"]["character"].as_u64().unwrap_or(0);
                    tokens.push(Token {
                        line: range["start"]["line"].as_u64().unwrap_or(0),
                        start,
                        length: range["end"]["character"].as_u64().unwrap_or(0) - start,
                        kind: token["type"].as_str().unwrap_or_default().to_string(),
                        modifiers: token["modifiers"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|m| m.as_str().map(String::from))
                            .collect(),
                    });
                }
            }
            for token in &mut tokens {
                token.modifiers.sort_unstable();
            }
            match parity.shared {
                None => tokens
                    .iter()
                    .map(|token| {
                        format!(
                            "{}:{}+{} {}",
                            token.line + 1,
                            token.start + 1,
                            token.length,
                            token.shown()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
                Some(shared) => {
                    let current = files.current.borrow().clone();
                    let unit = files.units.iter().find(|unit| unit.name == current);
                    shared
                        .iter()
                        .map(|marker| {
                            let at = unit.and_then(|unit| {
                                unit.markers
                                    .iter()
                                    .find(|(name, _)| name == marker)
                                    .map(|(_, offset)| lsp_position(&unit.text, *offset))
                            });
                            let token =
                                at.and_then(|at| tokens.iter().find(|token| token.covers(at)));
                            format!(
                                "/*{marker}*/ {}",
                                token.map_or_else(|| "none".to_string(), Token::shown)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            }
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
                        files.diagnostic_place(d, parity.shared.is_some()),
                        d["code"],
                        d["message"]
                            .as_str()
                            .unwrap_or_default()
                            .replace(&files.dir.to_string_lossy().into_owned(), "$DIR")
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
                        files.diagnostic_place(d, parity.shared.is_some()),
                        d["message"]
                            .as_str()
                            .unwrap_or_default()
                            .replace(&files.dir.to_string_lossy().into_owned(), "$DIR")
                    ));
                }
            }
            lines.sort();
            lines.join("\n")
        }
    }
}

const DIFFERENCES: &str = "tests/editor-matrix-differences.txt";

struct Listed {
    pattern: String,
    question: String,
    line: usize,
}

fn listed_differences(file: &str, cite: &str) -> Vec<Listed> {
    let text = fs::read_to_string(root().join(file)).unwrap_or_default();
    let mut out: Vec<Listed> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [pattern, question, class, reason] = fields[..] else {
            panic!(
                "{file}:{}: a line is a case name (`*` matches any text), a tab, the verb and \
                 marker, a tab, `by-design` or `defect`, a tab, and the reason",
                index + 1
            );
        };
        match class {
            "by-design" => assert!(
                reason.contains(cite),
                "{file}:{}: a by-design difference cites where {cite} documents it",
                index + 1
            ),
            "defect" => assert!(
                reason.starts_with("TASK-"),
                "{file}:{}: a defect names the TASK-NNN that records it",
                index + 1
            ),
            other => panic!(
                "{file}:{}: `{other}` is neither `by-design` nor `defect`",
                index + 1
            ),
        }
        assert!(
            !out.iter()
                .any(|entry| entry.pattern == pattern && entry.question == question),
            "{file}:{}: `{pattern}` `{question}` is listed twice",
            index + 1
        );
        out.push(Listed {
            pattern: pattern.to_string(),
            question: question.to_string(),
            line: index + 1,
        });
    }
    out
}

fn glob(pattern: &str, name: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = name.strip_prefix(first) else {
        return false;
    };
    let parts: Vec<&str> = parts.collect();
    for (index, part) in parts.iter().enumerate() {
        if index + 1 == parts.len() {
            return rest.ends_with(part);
        }
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    rest.is_empty()
}

fn asked(case: &Case) -> Vec<String> {
    let mut out = Vec::new();
    for (verb, targets) in &case.verbs {
        for target in targets {
            if verb.per_file() {
                for unit in case.units.iter().filter(|unit| is_tt(&unit.name)) {
                    if target == "*" || *target == unit.name {
                        out.push(format!("{} {}", verb.name(), unit.name));
                    }
                }
            } else {
                out.push(format!("{} {target}", verb.name()));
            }
        }
    }
    out
}

fn matrix_baseline(case: &Case) -> Option<PathBuf> {
    case.matrix.as_ref().map(|dir| {
        reference()
            .join("matrix")
            .join(dir)
            .join(format!("{}.baseline", case.name))
    })
}

fn matrix_differences(case: &Case, compared: &[Compared]) -> String {
    let mut out = String::new();
    for entry in compared {
        if let Some(lines) = &entry.difference {
            out.push_str(&entry.heading);
            out.push_str(lines);
            out.push('\n');
        }
    }
    if out.is_empty() {
        return out;
    }
    format!(
        "differences from {} at the questions listed in {DIFFERENCES}\n\n{out}",
        case.name
    )
}

fn judge_matrix(
    selection: &Selection,
    results: &BTreeMap<String, Vec<(String, bool)>>,
    unfiltered: bool,
) -> Vec<String> {
    let listed = listed_differences(DIFFERENCES, "docs/ai/tt.md");
    let mut failures = Vec::new();
    for (name, questions) in results {
        for (question, differs) in questions {
            let covering: Vec<&Listed> = listed
                .iter()
                .filter(|entry| entry.question == *question && glob(&entry.pattern, name))
                .collect();
            match (differs, covering.first()) {
                (true, None) => failures.push(format!(
                    "{name}: {question} differs from the TypeScript twin, and {DIFFERENCES} does not \
                     list it; the difference is in the case's baseline. A twin error is fixed in \
                     tests/matrix; a tt difference is listed with its class and reason: \
                     `{name}<TAB>{question}<TAB>by-design|defect<TAB>...`"
                )),
                (false, Some(entry)) => failures.push(format!(
                    "{DIFFERENCES}:{}: {name} {question} agrees with the TypeScript twin now; narrow \
                     or remove the line",
                    entry.line
                )),
                _ => {}
            }
        }
    }
    if unfiltered {
        let every: Vec<(&String, Vec<String>)> = selection
            .cases
            .iter()
            .chain(&selection.unsampled)
            .filter(|case| case.matrix.is_some())
            .map(|case| (&case.name, asked(case)))
            .collect();
        for entry in &listed {
            let names_one = every.iter().any(|(name, questions)| {
                glob(&entry.pattern, name) && questions.contains(&entry.question)
            });
            if !names_one {
                failures.push(format!(
                    "{DIFFERENCES}:{}: `{}` `{}` names no question of a matrix case; remove the line",
                    entry.line, entry.pattern, entry.question
                ));
            }
        }
    }
    failures
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
    let selection = cases();
    if let Some(summary) = &selection.summary {
        eprintln!("editor case matrix: {summary}");
    }
    for case in &selection.unsampled {
        if let Some(path) = matrix_baseline(case)
            && path.exists()
        {
            not_sampled(&path);
        }
    }
    let cases = &selection.cases;
    let filtered = std::env::var("TT_CASES").is_ok_and(|f| !f.is_empty());
    let next = AtomicUsize::new(0);
    let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let parity: Mutex<BTreeMap<String, Vec<String>>> = Mutex::new(BTreeMap::new());
    let matrix_results: Mutex<BTreeMap<String, Vec<(String, bool)>>> = Mutex::new(BTreeMap::new());
    let surface_results: Mutex<BTreeMap<String, Vec<String>>> = Mutex::new(BTreeMap::new());
    let workers = std::thread::available_parallelism()
        .map_or(1, |n| n.get() * 2)
        .min(8);
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
                        match matrix_baseline(case) {
                            Some(path) => {
                                let differences = matrix_differences(case, &outcome.compared);
                                if differences.is_empty() {
                                    expect_absent(&path);
                                } else {
                                    expect(&path, &differences);
                                }
                            }
                            None => expect(
                                &reference().join(format!("{}.baseline", case.name)),
                                &outcome.baseline,
                            ),
                        }
                        outcome
                    }));
                    match outcome {
                        Ok(outcome) => {
                            if case.matrix.is_some() {
                                matrix_results.lock().unwrap().insert(
                                    case.name.clone(),
                                    outcome
                                        .compared
                                        .iter()
                                        .map(|c| (c.question.clone(), c.difference.is_some()))
                                        .collect(),
                                );
                            } else {
                                parity
                                    .lock()
                                    .unwrap()
                                    .insert(case.name.clone(), outcome.parity);
                            }
                            if !outcome.transport.is_empty() {
                                failures
                                    .lock()
                                    .unwrap()
                                    .push(outcome.transport.join("\n\n"));
                            }
                            if case.expected.is_some() {
                                surface_results
                                    .lock()
                                    .unwrap()
                                    .insert(case.name.clone(), outcome.surfaces);
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
    let mut failures = failures.into_inner().unwrap();
    failures.extend(judge_matrix(
        &selection,
        &matrix_results.into_inner().unwrap(),
        !filtered,
    ));
    failures.extend(judge_surfaces(
        &selection,
        &surface_results.into_inner().unwrap(),
        !filtered,
    ));
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

struct Reported {
    code: String,
    message: String,
    start: (usize, usize),
    width: Option<usize>,
    labels: Vec<Label>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Label {
    message: String,
    start: (usize, usize),
    width: Option<usize>,
}

fn command_line_reports(text: &str, unit: &str) -> Vec<Reported> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some((code, message)) = line
            .strip_prefix("error[")
            .and_then(|rest| rest.split_once("]: "))
        else {
            continue;
        };
        if ttc::DiagnosticCode::parse(code).is_none() {
            continue;
        }
        let Some(at) = lines
            .get(index + 1)
            .and_then(|next| next.trim_start().strip_prefix("--> "))
        else {
            continue;
        };
        let mut parts = at.rsplitn(3, ':');
        let col: usize = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
        let line_number: usize = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
        let path = parts.next().unwrap_or_default();
        if path.strip_prefix("./").unwrap_or(path) != unit {
            continue;
        }
        let mut width = None;
        let mut labels = Vec::new();
        let mut quoted = 0;
        for row in lines[index + 2..]
            .iter()
            .take_while(|line| !line.starts_with("error") && !line.starts_with("warning"))
        {
            let trimmed = row.trim_start();
            if let Some(note) = trimmed.strip_prefix("= note: ") {
                if let Some((message, place)) = note.rsplit_once(" --> ") {
                    let mut parts = place.rsplitn(3, ':');
                    let col = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
                    let line = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
                    labels.push(Label {
                        message: message.to_string(),
                        start: (line, col),
                        width: None,
                    });
                }
                continue;
            }
            let Some(bar) = row.find(" |") else {
                continue;
            };
            let gutter = row[..bar].trim();
            if let Ok(number) = gutter.parse::<usize>() {
                quoted = number;
                continue;
            }
            if !gutter.is_empty() {
                continue;
            }
            let picture = &row[(bar + 3).min(row.len())..];
            let Some(from) = picture.find(['^', '-']) else {
                continue;
            };
            let marker = picture.as_bytes()[from];
            let run = picture[from..]
                .bytes()
                .take_while(|byte| *byte == marker)
                .count();
            if marker == b'^' {
                width.get_or_insert(run);
            } else {
                labels.push(Label {
                    message: picture[from + run..].trim().to_string(),
                    start: (quoted, from + 1),
                    width: Some(run),
                });
            }
        }
        labels.sort();
        out.push(Reported {
            code: code.to_string(),
            message: message.to_string(),
            start: (line_number, col),
            width,
            labels,
        });
    }
    out
}

fn surfaces_agree(
    case: &Case,
    expected: &ExpectedDiagnostic,
    published: &BTreeMap<String, Value>,
    project: &Path,
) -> Vec<String> {
    let mut problems = Vec::new();
    let ttc = |mode: &str| {
        let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
            .args([mode, "."])
            .current_dir(project)
            .output()
            .expect("ttc runs");
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    };
    let reports = [
        ("ttc --check", ttc("--check")),
        ("ttc --check-types", ttc("--check-types")),
    ];
    let restates = ttc::DiagnosticCode::parse(&expected.code)
        .is_some_and(ttc::DiagnosticCode::restates_typescript_syntax);
    let position = |value: &Value| Position {
        line: value["line"].as_u64().unwrap_or(0) as u32,
        character: value["character"].as_u64().unwrap_or(0) as u32,
    };
    let shown = |ranges: &[(Position, Position)]| {
        ranges
            .iter()
            .map(|(start, end)| {
                format!(
                    "{}:{}-{}:{}",
                    start.line + 1,
                    start.character + 1,
                    end.line + 1,
                    end.character + 1
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    for unit in case.units.iter().filter(|unit| is_tt(&unit.name)) {
        let list = published
            .get(&uri(&project.join(&unit.name)))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let is_tt_code = |d: &&Value| {
            d["code"]
                .as_str()
                .is_some_and(|code| ttc::DiagnosticCode::parse(code).is_some())
        };
        let restated = |d: &&Value| {
            d["source"] == "ttc"
                && d["code"].as_str().is_some_and(|code| {
                    code.strip_prefix("ts").is_some_and(|number| {
                        !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())
                    })
                })
        };
        let entries: Vec<&Value> = if restates {
            list.iter().filter(restated).collect()
        } else {
            list.iter().filter(is_tt_code).collect()
        };
        let mut wanted: Vec<(Position, Position)> = unit
            .ranges
            .iter()
            .map(|(start, end)| {
                (
                    lsp_position(&unit.text, *start),
                    lsp_position(&unit.text, *end),
                )
            })
            .collect();
        wanted.sort_by_key(|(start, end)| (start.line, start.character, end.line, end.character));
        let mut got: Vec<(Position, Position)> = entries
            .iter()
            .map(|d| (position(&d["range"]["start"]), position(&d["range"]["end"])))
            .collect();
        got.sort_by_key(|(start, end)| (start.line, start.character, end.line, end.character));
        if got != wanted {
            problems.push(format!(
                "the editor publishes {} at [{}] in {}, and the case's [|ranges|] are [{}]",
                if restates {
                    "TypeScript's syntax diagnostics"
                } else {
                    "tt diagnostics"
                },
                shown(&got),
                unit.name,
                shown(&wanted)
            ));
        }
        if restates && list.iter().any(|d| is_tt_code(&d)) {
            problems.push(format!(
                "the editor publishes a tt diagnostic in {} beside TypeScript's own words for `{}`",
                unit.name, expected.code
            ));
        }
        for d in &entries {
            let code = if restates {
                expected.code.clone()
            } else {
                d["code"].as_str().unwrap_or_default().to_string()
            };
            if code != expected.code {
                problems.push(format!(
                    "the editor publishes `{code}` in {}, and the case expects `{}`",
                    unit.name, expected.code
                ));
            }
            if d["severity"].as_u64() != Some(1) {
                problems.push(format!(
                    "the editor publishes `{code}` with severity {}, and every tt rule is an error (1)",
                    d["severity"]
                ));
            }
            if d["tags"].as_array().is_some_and(|tags| !tags.is_empty()) {
                problems.push(format!(
                    "the editor publishes `{code}` with tags {}, which no tt diagnostic carries",
                    d["tags"]
                ));
            }
            let start = position(&d["range"]["start"]);
            let end = position(&d["range"]["end"]);
            let at = (start.line as usize + 1, start.character as usize + 1);
            let width = (start.line == end.line)
                .then(|| end.character.saturating_sub(start.character).max(1) as usize);
            let message = d["message"].as_str().unwrap_or_default();
            let mut related: Vec<Label> = d["relatedInformation"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|related| {
                    let start = position(&related["location"]["range"]["start"]);
                    let end = position(&related["location"]["range"]["end"]);
                    Label {
                        message: related["message"].as_str().unwrap_or_default().to_string(),
                        start: (start.line as usize + 1, start.character as usize + 1),
                        width: (start.line == end.line)
                            .then(|| end.character.saturating_sub(start.character).max(1) as usize),
                    }
                })
                .collect();
            related.sort();
            for (surface, text) in &reports {
                if *surface == "ttc --check" && expected.typed_only {
                    continue;
                }
                let found = command_line_reports(text, &unit.name);
                let same = found.iter().find(|reported| {
                    reported.code == code
                        && reported.start == at
                        && (restates || reported.message == message)
                        && (width.is_none() || reported.width == width)
                });
                match same {
                    None => problems.push(format!(
                        "the editor publishes `{code}` at {}:{}:{}, and `{surface}` does not report it there{}\npublished: {message}\n{text}",
                        unit.name,
                        at.0,
                        at.1,
                        if restates {
                            ""
                        } else {
                            " with those words and that width"
                        }
                    )),
                    Some(reported) if !restates && reported.labels.iter().zip(&related).any(|(label, related)| {
                        label.message != related.message
                            || label.start != related.start
                            || (label.width.is_some() && label.width != related.width)
                    }) || (!restates && reported.labels.len() != related.len()) => problems.push(format!(
                        "the editor publishes `{code}` with {} related place(s), and `{surface}` shows {} other one(s)\npublished: {related:?}\n{text}",
                        related.len(),
                        reported.labels.len()
                    )),
                    Some(_) => {}
                }
            }
        }
        for (surface, text) in &reports {
            let found = command_line_reports(text, &unit.name).len();
            let wanted = if *surface == "ttc --check" && expected.typed_only {
                0
            } else {
                entries.len()
            };
            if found != wanted {
                problems.push(format!(
                    "`{surface}` reports {found} tt diagnostic(s) in {}, and the editor publishes {wanted}\n{text}",
                    unit.name
                ));
            }
        }
    }
    problems
}

const SURFACE_DIFFERENCES: &str = "tests/editor-diagnostic-differences.txt";

fn judge_surfaces(
    selection: &Selection,
    results: &BTreeMap<String, Vec<String>>,
    unfiltered: bool,
) -> Vec<String> {
    let text = fs::read_to_string(root().join(SURFACE_DIFFERENCES)).unwrap_or_default();
    let mut listed: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut failures = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [name, difference, task] = fields[..] else {
            panic!(
                "{SURFACE_DIFFERENCES}:{}: a line is a case name, a tab, the difference as the failure's first line states it, a tab, and the TASK-NNN that records it",
                index + 1
            );
        };
        assert!(
            task.starts_with("TASK-"),
            "{SURFACE_DIFFERENCES}:{}: the last field names the TASK-NNN that records the defect",
            index + 1
        );
        listed.insert((name.to_string(), difference.to_string()), index + 1);
    }
    let known: BTreeSet<&str> = selection
        .cases
        .iter()
        .chain(&selection.unsampled)
        .map(|case| case.name.as_str())
        .collect();
    for ((name, difference), line) in &listed {
        let observed = results.get(name).map(|problems| {
            problems
                .iter()
                .any(|problem| problem.lines().next() == Some(difference))
        });
        match observed {
            Some(false) => failures.push(format!(
                "{SURFACE_DIFFERENCES}:{line}: `{name}` no longer shows `{difference}`; remove the line"
            )),
            None if unfiltered && !known.contains(name.as_str()) => failures.push(format!(
                "{SURFACE_DIFFERENCES}:{line}: `{name}` names no editor case; remove the line"
            )),
            _ => {}
        }
    }
    for (name, problems) in results {
        for problem in problems {
            let first = problem.lines().next().unwrap_or_default().to_string();
            if !listed.contains_key(&(name.clone(), first.clone())) {
                failures.push(format!(
                    "{name}: {problem}\nA difference between the surfaces is a defect, listed in {SURFACE_DIFFERENCES}: `{name}<TAB>{first}<TAB>TASK-NNN: ...`"
                ));
            }
        }
    }
    failures
}

const FOURSLASH_DIFFERENCES: &str = "tests/fourslash-differences.txt";

const FOURSLASH_SAMPLE: usize = 60;

const ANONYMOUS: &str = "anonymous";

const FOURSLASH_SEED: u64 = 0x7474_666f_7572_736c;

const FOURSLASH_READ_ONLY: [&str; 40] = [
    "GoToMarker",
    "MarkTestAsStradaServer",
    "Ranges",
    "Markers",
    "MarkerNames",
    "MarkerByName",
    "GetRangesByText",
    "GetOptions",
    "VerifyQuickInfoAt",
    "VerifyQuickInfoIs",
    "VerifyQuickInfoExists",
    "VerifyNotQuickInfoExists",
    "VerifyBaselineHover",
    "VerifyBaselineHoverWithVerbosity",
    "VerifyCompletions",
    "VerifyBaselineGoToDefinition",
    "VerifyBaselineGoToTypeDefinition",
    "VerifyBaselineGoToImplementation",
    "VerifyBaselineGoToSourceDefinition",
    "VerifyBaselineFindAllReferences",
    "VerifyBaselineRename",
    "VerifyBaselineRenameAtRangesWithText",
    "VerifyRenameSucceeded",
    "VerifyRenameFailed",
    "VerifySignatureHelp",
    "VerifyBaselineSignatureHelp",
    "VerifyNoSignatureHelpForMarkers",
    "VerifySemanticTokens",
    "VerifyBaselineDocumentHighlights",
    "VerifyBaselineDocumentSymbol",
    "VerifyBaselineInlayHints",
    "VerifyBaselineCallHierarchy",
    "VerifyBaselineSelectionRanges",
    "VerifyOutliningSpans",
    "VerifyNoErrors",
    "VerifyNumberOfErrorsInCurrentFile",
    "VerifyBaselineNonSuggestionDiagnostics",
    "VerifyNonSuggestionDiagnostics",
    "VerifySuggestionDiagnostics",
    "VerifyErrorExistsBetweenMarkers",
];

const FOURSLASH_UNSUPPORTED_OPTIONS: [&str; 2] = ["tsc", "currentdirectory"];

struct GoCall {
    method: String,
    args: Vec<String>,
    chained: bool,
}

struct GoTest {
    name: String,
    content: String,
    calls: Vec<GoCall>,
}

fn go_string(text: &str) -> Option<(String, usize)> {
    let mut chars = text.char_indices();
    let (_, quote) = chars.next()?;
    let mut out = String::new();
    match quote {
        '`' => {
            let end = text[1..].find('`')? + 1;
            Some((text[1..end].replace('\r', ""), end + 1))
        }
        '"' => {
            let mut escaped = false;
            let mut index = 1;
            let bytes = text.as_bytes();
            while index < text.len() {
                let ch = text[index..].chars().next()?;
                if escaped {
                    escaped = false;
                    match ch {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        '\\' => out.push('\\'),
                        '"' => out.push('"'),
                        '\'' => out.push('\''),
                        'a' => out.push('\u{7}'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'v' => out.push('\u{b}'),
                        'x' | 'u' | 'U' => {
                            let width = match ch {
                                'x' => 2,
                                'u' => 4,
                                _ => 8,
                            };
                            let digits = text.get(index + 1..index + 1 + width)?;
                            let value = u32::from_str_radix(digits, 16).ok()?;
                            out.push(char::from_u32(value)?);
                            index += width;
                        }
                        '0'..='7' => {
                            let digits = text.get(index..index + 3)?;
                            let value = u32::from_str_radix(digits, 8).ok()?;
                            out.push(char::from_u32(value)?);
                            index += 2;
                        }
                        _ => return None,
                    }
                    index += ch.len_utf8();
                    continue;
                }
                match bytes[index] {
                    b'\\' => escaped = true,
                    b'"' => return Some((out, index + 1)),
                    _ => out.push(ch),
                }
                index += ch.len_utf8();
            }
            None
        }
        _ => None,
    }
}

fn go_skip_space(text: &str, mut at: usize) -> usize {
    loop {
        let rest = &text[at..];
        let trimmed = rest.trim_start();
        at += rest.len() - trimmed.len();
        if trimmed.starts_with("//") {
            at += trimmed.find('\n').unwrap_or(trimmed.len());
        } else if trimmed.starts_with("/*") {
            at += trimmed.find("*/").map_or(trimmed.len(), |end| end + 2);
        } else {
            return at;
        }
    }
}

fn go_concatenation(text: &str) -> Option<(String, usize)> {
    let mut at = go_skip_space(text, 0);
    let mut out = String::new();
    loop {
        let (piece, length) = go_string(&text[at..])?;
        out.push_str(&piece);
        at = go_skip_space(text, at + length);
        if text[at..].starts_with('+') {
            at = go_skip_space(text, at + 1);
        } else {
            return Some((out, at));
        }
    }
}

fn go_balanced(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut at = open;
    while at < bytes.len() {
        match bytes[at] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            b'"' | b'`' => {
                let (_, length) = go_string(&text[at..])?;
                at += length;
                continue;
            }
            b'\'' => {
                at += 1;
                while at < bytes.len() && bytes[at] != b'\'' {
                    at += if bytes[at] == b'\\' { 2 } else { 1 };
                }
            }
            b'/' if bytes.get(at + 1) == Some(&b'/') => {
                at += text[at..].find('\n').unwrap_or(text.len() - at);
                continue;
            }
            b'/' if bytes.get(at + 1) == Some(&b'*') => {
                at += text[at..].find("*/").map_or(text.len() - at, |end| end + 2);
                continue;
            }
            _ => {}
        }
        at += 1;
    }
    None
}

fn go_arguments(inner: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut at = 0;
    let bytes = inner.as_bytes();
    while at < bytes.len() {
        match bytes[at] {
            b'(' | b'[' | b'{' => {
                at = go_balanced(inner, at).map_or(bytes.len(), |end| end + 1);
                continue;
            }
            b'"' | b'`' => {
                at += go_string(&inner[at..]).map_or(bytes.len() - at, |(_, length)| length);
                continue;
            }
            b',' => {
                out.push(inner[start..at].trim().to_string());
                start = at + 1;
            }
            _ => {}
        }
        at += 1;
    }
    let last = inner[start..].trim();
    if !last.is_empty() {
        out.push(last.to_string());
    }
    out
}

fn go_test(source: &str) -> Result<GoTest, String> {
    let header = source.find("func Test").ok_or("no Test function")?;
    let name_start = header + "func Test".len();
    let name_end = name_start + source[name_start..].find('(').ok_or("no Test function")?;
    let name = source[name_start..name_end].to_string();
    let open = name_end + source[name_end..].find('{').ok_or("no body")?;
    let close = go_balanced(source, open).ok_or("an unbalanced body")?;
    let body = &source[open + 1..close];
    let content_at = ["const content = ", "content := "]
        .iter()
        .find_map(|head| body.find(head).map(|at| at + head.len()))
        .ok_or("no content literal")?;
    let (content, _) =
        go_concatenation(&body[content_at..]).ok_or("content that is not a string literal")?;
    let created = body
        .find("fourslash.NewFourslash(t, ")
        .ok_or("no fourslash.NewFourslash")?;
    let capabilities = &body[created + "fourslash.NewFourslash(t, ".len()..];
    if !capabilities.starts_with("nil") {
        return Err("custom client capabilities".to_string());
    }
    let after = created + body[created..].find('\n').unwrap_or(0);
    let mut calls = Vec::new();
    let mut at = after;
    let bytes = body.as_bytes();
    while at < bytes.len() {
        at = go_skip_space(body, at);
        if at >= bytes.len() {
            break;
        }
        let rest = &body[at..];
        let word: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if matches!(
            word.as_str(),
            "for" | "if" | "switch" | "go" | "func" | "select"
        ) {
            return Err("control flow in the test body".to_string());
        }
        if let Some(method_start) = rest.strip_prefix("f.") {
            let method: String = method_start
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            let paren = at + 2 + method.len();
            if body.as_bytes().get(paren) == Some(&b'(') {
                let end = go_balanced(body, paren).ok_or("an unbalanced call")?;
                let args = go_arguments(&body[paren + 1..end]);
                let line_end = body[end..].find('\n').map_or(body.len(), |n| end + n);
                let chained = body[end + 1..line_end].trim_start().starts_with('.');
                calls.push(GoCall {
                    method,
                    args,
                    chained,
                });
                at = line_end;
                continue;
            }
        }
        let mut cursor = at;
        loop {
            match bytes.get(cursor) {
                None | Some(b'\n') => break,
                Some(b'(' | b'[' | b'{') => {
                    cursor = go_balanced(body, cursor).map_or(bytes.len(), |end| end + 1);
                }
                Some(b'"' | b'`') => {
                    cursor += go_string(&body[cursor..]).map_or(1, |(_, length)| length);
                }
                Some(_) => cursor += 1,
            }
        }
        at = cursor;
    }
    Ok(GoTest {
        name,
        content,
        calls,
    })
}

struct FourslashFile {
    name: String,
    unit: Unit,
}

fn chomp_leading_space(content: &str) -> String {
    let lines: Vec<&str> = content.split('\n').collect();
    if lines
        .iter()
        .any(|line| !line.is_empty() && !line.starts_with(' '))
    {
        return content.to_string();
    }
    lines
        .iter()
        .map(|line| line.get(1..).unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn fourslash_markers(raw: &str) -> Result<Unit, String> {
    let content = chomp_leading_space(raw);
    let chars: Vec<(usize, char)> = content.char_indices().collect();
    let mut output = String::new();
    let mut markers: Vec<(String, usize)> = Vec::new();
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut open_ranges: Vec<usize> = Vec::new();
    let mut difference = 0usize;
    let mut last_normal = 0usize;
    let mut open_marker: Option<(usize, usize)> = None;
    let mut state = 0u8;
    let flush = |output: &mut String, from: usize, to: Option<usize>| {
        output.push_str(&content[from..to.unwrap_or(content.len())]);
    };
    let Some(&(_, first)) = chars.first() else {
        return Ok(Unit {
            name: String::new(),
            text: String::new(),
            markers,
            ranges,
        });
    };
    let mut previous = first;
    for &(i, current) in chars.iter().skip(1) {
        match state {
            0 => {
                if previous == '[' && current == '|' {
                    open_ranges.push(i - 1 - difference);
                    flush(&mut output, last_normal, Some(i - 1));
                    last_normal = i + 1;
                    difference += 2;
                } else if previous == '|' && current == ']' {
                    let start = open_ranges.pop().ok_or("a range end with no start")?;
                    ranges.push((start, i - 1 - difference));
                    flush(&mut output, last_normal, Some(i - 1));
                    last_normal = i + 1;
                    difference += 2;
                } else if previous == '/'
                    && current == '*'
                    && content.as_bytes().get(i + 1) != Some(&b'/')
                {
                    state = 1;
                    open_marker = Some((i - 1 - difference, i - 1));
                } else if previous == '{' && current == '|' {
                    state = 2;
                    open_marker = Some((i - 1 - difference, i - 1));
                    flush(&mut output, last_normal, Some(i - 1));
                }
            }
            2 => {
                if previous == '|' && current == '}' {
                    let (position, source) = open_marker.take().expect("an open marker");
                    let data = content[source + 2..i - 1].trim();
                    let value: Value = serde_json::from_str(&format!("{{ {data} }}"))
                        .map_err(|_| format!("an object marker `{data}` that is not JSON"))?;
                    if let Some(name) = value["name"].as_str().filter(|name| !name.is_empty()) {
                        markers.push((name.to_string(), position));
                    }
                    last_normal = i + 1;
                    difference += i + 1 - source;
                    state = 0;
                }
            }
            _ => {
                if previous == '*' && current == '/' {
                    let (position, source) = open_marker.take().expect("an open marker");
                    let name = content[source + 2..i - 1].trim().to_string();
                    markers.push((name, position));
                    flush(&mut output, last_normal, Some(source));
                    last_normal = i + 1;
                    difference += i + 1 - source;
                    state = 0;
                } else if !(current.is_ascii_alphanumeric() || current == '$' || current == '_') {
                    let closing = current == '*' && content.as_bytes().get(i + 1) == Some(&b'/');
                    if !closing {
                        flush(&mut output, last_normal, Some(i));
                        last_normal = i;
                        open_marker = None;
                        state = 0;
                    }
                }
            }
        }
        if current == '\n' && previous == '\r' {
            continue;
        }
        previous = if i >= last_normal {
            current
        } else {
            '\u{fffd}'
        };
    }
    flush(&mut output, last_normal, None);
    if !open_ranges.is_empty() {
        return Err("an unterminated range".to_string());
    }
    if open_marker.is_some() {
        return Err("an unterminated marker".to_string());
    }
    ranges.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    Ok(Unit {
        name: String::new(),
        text: output,
        markers,
        ranges,
    })
}

fn fourslash_files(
    content: &str,
    default_name: &str,
) -> Result<(Vec<FourslashFile>, BTreeMap<String, String>), String> {
    let mut files = Vec::new();
    let mut globals: BTreeMap<String, String> = BTreeMap::new();
    let mut current_name = default_name.to_string();
    let mut current = String::new();
    let mut seen_content = false;
    let mut seen_file = false;
    let save = |files: &mut Vec<FourslashFile>, name: &str, text: &str| -> Result<(), String> {
        let unit = fourslash_markers(text)?;
        files.push(FourslashFile {
            name: name.to_string(),
            unit,
        });
        Ok(())
    };
    for line in content.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if let Some((name, value)) = directive(line) {
            if (name == "link" && value.contains("->")) || name == "symlink" {
                return Err("a harness symlink".to_string());
            }
            if name != "filename" {
                if name != "emitthisfile" && name != "noopen" {
                    globals.insert(name, value.to_string());
                }
                continue;
            }
            if !current.is_empty() || seen_file {
                save(&mut files, &current_name, &current)?;
                seen_file = true;
            }
            current.clear();
            seen_content = false;
            current_name = value.to_string();
            continue;
        }
        if seen_content {
            current.push('\n');
        }
        seen_content = true;
        current.push_str(line);
    }
    save(&mut files, &current_name, &current)?;
    Ok((files, globals))
}

fn lower_first(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn fourslash_relative(name: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for part in name.trim_start_matches('/').split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

fn fourslash_renameable(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    (lower.ends_with(".ts") || lower.ends_with(".tsx"))
        && !lower.contains(".d.")
        && !lower.split('/').any(|part| part == "node_modules")
}

fn fourslash_tt_name(name: &str) -> String {
    if let Some(stem) = name.strip_suffix(".tsx") {
        format!("{stem}.ttx")
    } else if let Some(stem) = name.strip_suffix(".ts") {
        format!("{stem}.tt")
    } else {
        name.to_string()
    }
}

fn marker_list(
    argument: &str,
    named: &[String],
    ranges: &[String],
    ranges_by_text: &BTreeMap<String, Vec<String>>,
) -> Option<Vec<String>> {
    let argument = argument.trim().trim_end_matches("...");
    if let Some((text, _)) = go_string(argument) {
        return Some(vec![text]);
    }
    if argument == "f.Markers()" || argument == "f.MarkerNames()" {
        return Some(named.to_vec());
    }
    if argument == "f.Ranges()" {
        return Some(ranges.to_vec());
    }
    if let Some(index) = argument
        .strip_prefix("f.Ranges()[")
        .and_then(|rest| rest.strip_suffix(']'))
        .and_then(|index| index.parse::<usize>().ok())
    {
        return ranges.get(index).map(|range| vec![range.clone()]);
    }
    if let Some(inner) = argument
        .strip_prefix("f.MarkerByName(t, ")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        return go_string(inner).map(|(name, _)| vec![name]);
    }
    if let Some(inner) = argument
        .strip_prefix("f.GetRangesByText().Get(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let (text, _) = go_string(inner)?;
        return ranges_by_text.get(&text).cloned();
    }
    if let Some(inner) = argument
        .strip_prefix("[]string{")
        .and_then(|rest| rest.strip_suffix('}'))
    {
        return go_arguments(inner)
            .iter()
            .map(|item| go_string(item).map(|(name, _)| name))
            .collect();
    }
    None
}

type Answered = Vec<(String, Option<String>)>;

struct Converted {
    case: Case,
    dir: Workspace,
}

fn fourslash_case(
    path: &Path,
    oracle: &mut common::typescript_cases::Oracle,
) -> Result<Converted, String> {
    let source = fs::read_to_string(path).map_err(|_| "not UTF-8".to_string())?;
    let test = go_test(&source)?;
    let default_name = format!("{}.ts", lower_first(&test.name));
    let (files, globals) = fourslash_files(&test.content, &default_name)?;
    for option in FOURSLASH_UNSUPPORTED_OPTIONS {
        if globals.contains_key(option) {
            return Err(format!("the harness option @{option}"));
        }
    }
    let mut units: Vec<Unit> = Vec::new();
    for file in files {
        let lower = file.name.to_ascii_lowercase();
        if lower.ends_with("tsconfig.json") || lower.ends_with("jsconfig.json") {
            return Err("the test brings its own tsconfig.json".to_string());
        }
        let name = fourslash_relative(&file.name).ok_or("a file outside the root")?;
        if units
            .iter()
            .any(|unit| unit.name.eq_ignore_ascii_case(&name))
        {
            return Err("two files at one path".to_string());
        }
        let mut unit = file.unit;
        unit.name = name;
        units.push(unit);
    }
    let mut named: Vec<String> = Vec::new();
    for unit in &units {
        for (marker, _) in &unit.markers {
            if named.contains(marker) {
                return Err(format!("the marker `{marker}` twice"));
            }
            named.push(marker.clone());
        }
    }
    let mut range_names = Vec::new();
    let mut ranges_by_text: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut counter = 0usize;
    for unit in units.iter_mut() {
        let mut added = Vec::new();
        for (start, end) in unit.ranges.clone() {
            let name = format!("range{counter}");
            counter += 1;
            if named.contains(&name) {
                return Err(format!("a marker named `{name}`"));
            }
            ranges_by_text
                .entry(unit.text[start..end].to_string())
                .or_default()
                .push(name.clone());
            range_names.push(name.clone());
            added.push((name, start));
        }
        unit.markers.extend(added);
    }
    let entries: Vec<Value> = globals
        .iter()
        .map(|(name, value)| json!([name, value]))
        .collect();
    let converted = oracle.ask(&json!({ "options": entries }));
    for key in ["unknown", "varies", "invalid"] {
        if converted[key]
            .as_array()
            .is_some_and(|list| !list.is_empty())
        {
            return Err(format!(
                "an option the oracle reports as {key}: {}",
                converted[key]
            ));
        }
    }
    let mut options = json!({
        "target": "esnext",
        "jsx": "preserve",
        "skipDefaultLibCheck": true,
        "noEmit": true,
    });
    let chosen = converted["configurations"][0].clone();
    for (key, value) in chosen.as_object().into_iter().flatten() {
        options[key] = value.clone();
    }
    let unsupported = [
        ("target", &["es5", "es3"][..]),
        ("module", &["amd", "umd", "system", "none"][..]),
        ("moduleResolution", &["node10", "node", "classic"][..]),
    ];
    for (key, values) in unsupported {
        if let Some(value) = options[key].as_str()
            && values.contains(&value.to_ascii_lowercase().as_str())
        {
            return Err(format!(
                "an option value the fourslash harness skips ({key}: {value})"
            ));
        }
    }
    for key in ["outFile", "baseUrl"] {
        if !options[key].is_null() {
            return Err(format!("an option the fourslash harness skips ({key})"));
        }
    }
    for key in [
        "esModuleInterop",
        "allowSyntheticDefaultImports",
        "alwaysStrict",
    ] {
        if options[key] == json!(false) {
            return Err(format!(
                "an option the fourslash harness skips ({key}: false)"
            ));
        }
    }
    let config = json!({ "compilerOptions": options }).to_string();
    let dir = Workspace::in_repo("fourslash-reach");
    let twin_dir = dir.path().to_path_buf();
    write_units(&twin_dir, &units);
    fs::write(twin_dir.join("tsconfig.json"), &config).expect("a writable tsconfig");
    let members: Vec<String> = units
        .iter()
        .map(|unit| twin_dir.join(&unit.name).to_string_lossy().into_owned())
        .collect();
    let answer = oracle.ask(&json!({
        "check": twin_dir.join("tsconfig.json").to_string_lossy(),
        "units": members,
    }));
    let reached: BTreeSet<String> = answer["reached"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|path| path.as_str())
        .filter_map(|path| Path::new(path).strip_prefix(&twin_dir).ok())
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .collect();
    let renamed: BTreeSet<String> = units
        .iter()
        .map(|unit| unit.name.clone())
        .filter(|name| fourslash_renameable(name) && !reached.contains(name))
        .collect();
    if renamed.is_empty() {
        return Err("no .ts/.tsx file to rename that no other file imports".to_string());
    }
    let in_renamed = |marker: &str| {
        units.iter().any(|unit| {
            renamed.contains(&unit.name) && unit.markers.iter().any(|(name, _)| name == marker)
        })
    };
    let mut verbs: Vec<(Verb, Vec<String>)> = Vec::new();
    let mut current: Option<String> = None;
    let mut push = |verb: Verb, targets: Vec<String>| {
        let targets: Vec<String> = targets
            .into_iter()
            .filter(|t| t == "*" || in_renamed(t))
            .collect();
        if targets.is_empty() {
            return;
        }
        match verbs.iter_mut().find(|(known, _)| *known == verb) {
            Some((_, known)) => {
                for target in targets {
                    if !known.contains(&target) {
                        known.push(target);
                    }
                }
            }
            None => verbs.push((verb, targets)),
        }
    };
    let named_only: Vec<String> = named.clone();
    for call in &test.calls {
        if !FOURSLASH_READ_ONLY.contains(&call.method.as_str()) {
            break;
        }
        let args = &call.args;
        let markers_from = |from: usize| -> Option<Vec<String>> {
            let mut out = Vec::new();
            for argument in args.iter().skip(from) {
                out.extend(marker_list(
                    argument,
                    &named_only,
                    &range_names,
                    &ranges_by_text,
                )?);
            }
            Some(out)
        };
        let here = || current.clone().into_iter().collect::<Vec<_>>();
        match call.method.as_str() {
            "GoToMarker" => {
                current = args
                    .get(1)
                    .and_then(|argument| go_string(argument))
                    .map(|(name, _)| name);
            }
            "VerifyQuickInfoAt" => {
                if let Some(marker) = args.get(1).and_then(|a| go_string(a)) {
                    push(Verb::Hover, vec![marker.0]);
                }
            }
            "VerifyBaselineHover" => push(Verb::Hover, named_only.clone()),
            "VerifyQuickInfoIs" | "VerifyQuickInfoExists" | "VerifyNotQuickInfoExists" => {
                push(Verb::Hover, here())
            }
            "VerifyCompletions" => {
                let targets = match args.get(1).map(String::as_str) {
                    Some("nil") => here(),
                    Some(argument) => {
                        marker_list(argument, &named_only, &range_names, &ranges_by_text)
                            .unwrap_or_default()
                    }
                    None => Vec::new(),
                };
                if let Some(last) = targets.last() {
                    current = Some(last.clone());
                }
                push(Verb::Completions, targets);
            }
            "VerifyBaselineGoToDefinition" => {
                let targets = if args.len() <= 2 {
                    Some(range_names.clone())
                } else {
                    markers_from(2)
                };
                push(Verb::Definition, targets.unwrap_or_default());
            }
            "VerifyBaselineFindAllReferences" => {
                let targets = if args.len() <= 1 {
                    Some(range_names.clone())
                } else {
                    markers_from(1)
                };
                push(Verb::References, targets.unwrap_or_default());
            }
            "VerifyBaselineRename" if args.get(1).map(String::as_str) == Some("nil") => {
                push(Verb::Rename, markers_from(2).unwrap_or_default());
            }
            "VerifyBaselineRenameAtRangesWithText"
                if args.get(1).map(String::as_str) == Some("nil") =>
            {
                let mut targets = Vec::new();
                for argument in args.iter().skip(2) {
                    if let Some((text, _)) = go_string(argument) {
                        targets.extend(ranges_by_text.get(&text).cloned().unwrap_or_default());
                    }
                }
                push(Verb::Rename, targets);
            }
            "VerifyRenameSucceeded" | "VerifyRenameFailed"
                if args.get(1).map(String::as_str) == Some("nil") =>
            {
                push(Verb::Rename, here())
            }
            "VerifySignatureHelp" => push(Verb::SignatureHelp, here()),
            "VerifyBaselineSignatureHelp" => push(Verb::SignatureHelp, named_only.clone()),
            "VerifyNoSignatureHelpForMarkers" => {
                push(Verb::SignatureHelp, markers_from(1).unwrap_or_default())
            }
            "VerifySemanticTokens" => push(Verb::SemanticTokens, vec!["*".to_string()]),
            _ => {}
        }
        if call.chained {
            break;
        }
    }
    if verbs.is_empty() {
        return Err("no question this runner asks at a marker in a renamed file".to_string());
    }
    if named.iter().any(|name| name == ANONYMOUS) {
        return Err(format!("a marker named `{ANONYMOUS}`"));
    }
    let shown = |name: &str| {
        if name.is_empty() {
            ANONYMOUS.to_string()
        } else {
            name.to_string()
        }
    };
    for (_, targets) in verbs.iter_mut() {
        for target in targets.iter_mut() {
            *target = shown(target);
        }
    }
    for unit in units.iter_mut() {
        for (marker, _) in unit.markers.iter_mut() {
            *marker = shown(marker);
        }
    }
    let twin: Vec<Unit> = units.clone();
    let mut tt_units = units;
    for unit in tt_units.iter_mut() {
        if renamed.contains(&unit.name) {
            unit.name = fourslash_tt_name(&unit.name);
        }
    }
    tt_units.push(Unit {
        name: "tsconfig.json".to_string(),
        text: config.clone(),
        markers: Vec::new(),
        ranges: Vec::new(),
    });
    let mut twin = twin;
    twin.push(Unit {
        name: "tsconfig.json".to_string(),
        text: config,
        markers: Vec::new(),
        ranges: Vec::new(),
    });
    Ok(Converted {
        case: Case {
            name: path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .trim_end_matches("_test.go")
                .to_string(),
            path: path.to_path_buf(),
            units: tt_units,
            verbs,
            ignores: BTreeSet::new(),
            twin: Some(twin),
            matrix: None,
            server_only: true,
            expected: None,
        },
        dir,
    })
}

fn fourslash_tests(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .expect("a readable fourslash directory")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with("_test.go"))
        })
        .collect();
    out.sort();
    out
}

#[test]
fn typescript_fourslash_tests_answer_as_their_twins() {
    let manifest = common::typescript_cases::manifest();
    let Some(checkout) = common::typescript_cases::cases_checkout(&manifest) else {
        assert!(
            !common::typescript_cases::cases_required(),
            "TTC_REQUIRE_TYPESCRIPT_CASES is set but TypeScript's tests are not fetched — run \
             scripts/fetch-typescript-cases"
        );
        eprintln!(
            "SKIP TypeScript's fourslash tests: not fetched (scripts/fetch-typescript-cases)"
        );
        return;
    };
    if !toolchain() || tsgo_binary().is_none() {
        eprintln!("SKIP TypeScript's fourslash tests: no TypeScript installed — run `npm ci`");
        return;
    }
    let tests_dir = checkout.join(common::typescript_cases::tree(
        &manifest,
        "/fourslash/tests",
    ));
    let all = fourslash_tests(&tests_dir);
    let total = all.len();
    let requested = std::env::var("TT_FOURSLASH").unwrap_or_default();
    let filter = std::env::var("TT_FOURSLASH_FILTER")
        .ok()
        .filter(|f| !f.is_empty());
    let full = requested == "all" && filter.is_none();
    let (chosen, description): (Vec<PathBuf>, String) = if let Some(filter) = &filter {
        let patterns: Vec<&str> = filter.split(',').collect();
        let picked: Vec<PathBuf> = all
            .into_iter()
            .filter(|path| {
                let name = path.file_name().unwrap().to_string_lossy();
                patterns.iter().any(|p| name.contains(p))
            })
            .collect();
        let description = format!(
            "{} of {total} tests matching TT_FOURSLASH_FILTER",
            picked.len()
        );
        (picked, description)
    } else if requested == "all" {
        (all, format!("all {total} tests"))
    } else {
        let count = requested.parse().unwrap_or(FOURSLASH_SAMPLE);
        let seed = std::env::var("TT_FOURSLASH_SEED")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(FOURSLASH_SEED);
        let picked = common::typescript_cases::seeded_choice(total, count, seed);
        (
            all.into_iter()
                .enumerate()
                .filter(|(index, _)| picked.contains(index))
                .map(|(_, path)| path)
                .collect(),
            format!(
                "{} of {total} tests, seed {seed:#x} (TT_FOURSLASH=all for every one)",
                count.min(total)
            ),
        )
    };
    let started = std::time::Instant::now();
    let next = AtomicUsize::new(0);
    let skipped: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());
    let results: Mutex<BTreeMap<String, Answered>> = Mutex::new(BTreeMap::new());
    let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let questions_asked = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism()
        .map_or(1, |n| n.get() * 2)
        .min(8);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                let mut oracle = common::typescript_cases::Oracle::start();
                loop {
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some(path) = chosen.get(index) else {
                        break;
                    };
                    let converted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        fourslash_case(path, &mut oracle)
                    }));
                    let converted = match converted {
                        Ok(Ok(converted)) => converted,
                        Ok(Err(reason)) => {
                            if std::env::var_os("TT_FOURSLASH_VERBOSE").is_some() {
                                eprintln!("skip {}: {reason}", path.display());
                            }
                            let class = reason.split([':', '`', '(']).next().unwrap_or(&reason);
                            *skipped
                                .lock()
                                .unwrap()
                                .entry(class.trim().to_string())
                                .or_default() += 1;
                            continue;
                        }
                        Err(_) => {
                            oracle = common::typescript_cases::Oracle::start();
                            *skipped
                                .lock()
                                .unwrap()
                                .entry("the conversion failed".to_string())
                                .or_default() += 1;
                            continue;
                        }
                    };
                    let case = &converted.case;
                    let outcome =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(case)));
                    drop(converted.dir);
                    match outcome {
                        Ok(outcome) => {
                            questions_asked.fetch_add(outcome.compared.len(), Ordering::SeqCst);
                            results.lock().unwrap().insert(
                                case.name.clone(),
                                outcome
                                    .compared
                                    .iter()
                                    .map(|c| {
                                        (
                                            c.question.clone(),
                                            c.difference
                                                .as_ref()
                                                .map(|lines| format!("{}{lines}", c.heading)),
                                        )
                                    })
                                    .collect(),
                            );
                        }
                        Err(payload) => {
                            let message = payload
                                .downcast_ref::<String>()
                                .cloned()
                                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                                .unwrap_or_else(|| "a non-string panic".to_string());
                            results.lock().unwrap().insert(
                                case.name.clone(),
                                vec![("run".to_string(), Some(format!("{message}\n")))],
                            );
                        }
                    }
                }
            });
        }
    });
    let results = results.into_inner().unwrap();
    let skipped = skipped.into_inner().unwrap();
    let mut failures = failures.into_inner().unwrap();
    let listed = listed_differences(FOURSLASH_DIFFERENCES, "docs/");
    let mut differing = 0usize;
    for (name, questions) in &results {
        for (question, difference) in questions {
            let covering: Vec<&Listed> = listed
                .iter()
                .filter(|entry| entry.question == *question && glob(&entry.pattern, name))
                .collect();
            if difference.is_some() {
                differing += 1;
            }
            match (difference, covering.first()) {
                (Some(lines), None) => failures.push(format!(
                    "{name}: {question} differs from the TypeScript twin, and {FOURSLASH_DIFFERENCES} \
                     does not list it:\n{lines}  list it as \
                     `{name}<TAB>{question}<TAB>by-design|defect<TAB>...`"
                )),
                (None, Some(entry)) => failures.push(format!(
                    "{FOURSLASH_DIFFERENCES}:{}: {name} {question} agrees with the TypeScript twin \
                     now; narrow or remove the line",
                    entry.line
                )),
                _ => {}
            }
        }
    }
    if full {
        for entry in &listed {
            let names_one = results.iter().any(|(name, questions)| {
                glob(&entry.pattern, name) && questions.iter().any(|(q, _)| *q == entry.question)
            });
            if !names_one {
                failures.push(format!(
                    "{FOURSLASH_DIFFERENCES}:{}: `{}` `{}` names no question of a compared test; \
                     remove the line",
                    entry.line, entry.pattern, entry.question
                ));
            }
        }
    }
    let skipped_total: usize = skipped.values().sum();
    println!(
        "fourslash parity: {description}; {} compared ({} questions, {differing} differ), \
         {skipped_total} skipped, {:.1}s",
        results.len(),
        questions_asked.load(Ordering::SeqCst),
        started.elapsed().as_secs_f64()
    );
    for (reason, count) in &skipped {
        println!("  skipped {count}: {reason}");
    }
    assert!(
        failures.is_empty(),
        "{} fourslash difference(s):\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
