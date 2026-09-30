//! Service projections, coordinate mapping, and TypeScript response conversion.

use super::*;
use crate::lines::LineMap;

pub(super) fn projection_accepts_diagnostics(code: &str, source_kind: crate::SourceKind) -> bool {
    crate::verify::verify_output(code, source_kind).is_ok()
}

pub(super) fn service_doc(path: &Path, text: String) -> ServiceDoc {
    if crate::engine::project::is_host_source(path) {
        return ServiceDoc {
            mappings: vec![EmitMapping {
                src: 0,
                out: 0,
                len: text.len(),
            }],
            code: text.clone(),
            source: text,
            anchors: Vec::new(),
            declared_names: Vec::new(),
            shared_bindings: Vec::new(),
            destructured_lists: Vec::new(),
            recovered: Vec::new(),
            tt_diagnostics: Vec::new(),
            generated_names: HashSet::new(),
            inserted: Vec::new(),
            faithful: true,
        };
    }
    let options = crate::Options {
        filename: Some(path.to_str().unwrap_or("<input>")),
        source_kind: crate::SourceKind::from_path(path).unwrap_or_default(),
        defer_to_checker: true,
        rewrite_imports: crate::ImportRewrite::Off,
        ..crate::Options::default()
    };
    let report = crate::compile_projection_report(&text, &options);
    let (emit, recovered, faithful) = match (report.emit, report.withheld) {
        (Some(emit), _) | (None, Some(emit)) => (emit, report.recovered, true),
        (None, None) => (
            crate::emit_mapped_with_kind(
                &text,
                crate::SourceKind::from_path(path).unwrap_or_default(),
            ),
            Vec::new(),
            false,
        ),
    };
    ServiceDoc {
        faithful,
        source: text,
        code: emit.code,
        mappings: emit.mappings,
        anchors: emit.anchors,
        declared_names: emit.declared_names,
        shared_bindings: emit.shared_bindings,
        destructured_lists: emit.destructured_lists,
        recovered,
        tt_diagnostics: report.diagnostics,
        generated_names: emit.generated_names,
        inserted: emit.inserted,
    }
}

/// Serves one `.tt` file's projection, creating or refreshing it from the
/// overlay or the disk. `None` when the file cannot be read.
pub(super) fn serve_one(
    session: &mut ServiceSession,
    overlays: &HashMap<PathBuf, String>,
    path: &Path,
) -> Option<Arc<ServiceDoc>> {
    let text = match overlays.get(path) {
        Some(text) => text.clone(),
        None => std::fs::read_to_string(path).ok()?,
    };
    let doc = match session.docs.get(path) {
        Some(doc) if doc.source == text => doc.clone(),
        _ => {
            let doc = Arc::new(service_doc(path, text));
            session.docs.insert(path.to_path_buf(), doc.clone());
            doc
        }
    };
    if !crate::engine::project::is_host_source(path) && session.served.get(path) != Some(&doc.code)
    {
        open_served(session, path, &doc.code);
    }
    Some(doc)
}

pub(super) fn open_served(session: &mut ServiceSession, path: &Path, code: &str) {
    let uri = if crate::engine::project::is_host_source(path) {
        file_uri(path)
    } else {
        session
            .client
            .document_uri(path, &module_path_of(path), || {
                lowering_reproduces(path, code)
            })
    };
    if let Some(previous) = session.uris.insert(path.to_path_buf(), uri.clone())
        && previous != uri
    {
        session.client.close(&previous);
    }
    session.client.open(&uri, code);
    session.served.insert(path.to_path_buf(), code.to_string());
}

fn lowering_reproduces(path: &Path, code: &str) -> bool {
    let report = crate::compile_projection_report(
        code,
        &crate::Options {
            filename: path.to_str(),
            source_kind: crate::SourceKind::from_path(path).unwrap_or_default(),
            rewrite_imports: crate::ImportRewrite::Off,
            ..crate::Options::default()
        },
    );
    report.emit.is_some_and(|emit| emit.code == code)
        && report
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != crate::Severity::Error)
}

/// The URI a file is served under. An `.tt` file is served under the name
/// [`open_served`] last opened it as; a hand-written TypeScript file is its
/// own module, served as the buffer the session opened (or read from disk)
/// under its own name.
pub(super) fn served_uri(session: &ServiceSession, path: &Path) -> String {
    if crate::engine::project::is_host_source(path) {
        return file_uri(path);
    }
    match session.uris.get(path) {
        Some(uri) => uri.clone(),
        None => session
            .client
            .document_uri(path, &module_path_of(path), || true),
    }
}

fn tt_document(path: &Path) -> Option<PathBuf> {
    let name = path.to_string_lossy();
    let source = name
        .strip_suffix(".tsx")
        .filter(|n| n.ends_with(".ttx"))
        .or_else(|| name.strip_suffix(".ts").filter(|n| n.ends_with(".tt")))
        .or_else(|| (name.ends_with(".tt") || name.ends_with(".ttx")).then_some(&*name))?;
    Some(PathBuf::from(source))
}

/// Completions at a service offset, with the raw items cached for resolve.
pub(super) fn ts_completions(
    session: &mut ServiceSession,
    path: &Path,
    at: usize,
    text: ServedText<'_>,
    generated_names: &HashSet<String>,
    trigger: Option<&str>,
) -> Result<CompletionAnswer, String> {
    let code = text.code;
    let context = match trigger {
        Some(character) => serde_json::json!({ "triggerKind": 2, "triggerCharacter": character }),
        None => serde_json::json!({ "triggerKind": 1 }),
    };
    let answer = session.client.request(
        "textDocument/completion",
        serde_json::json!({
            "textDocument": { "uri": served_uri(session, path) },
            "position": lsp_position(u16_position(code, at)),
            "context": context,
        }),
    )?;
    let items: Vec<serde_json::Value> = match answer {
        serde_json::Value::Array(items) => items,
        value => value["items"].as_array().cloned().unwrap_or_default(),
    };
    session.last_completion.clear();
    let mut entries = Vec::with_capacity(items.len());
    let kind = crate::SourceKind::from_path(path).unwrap_or_default();
    let generated_switch = || {
        enclosing_switch(code, kind, mapper::from_utf16(code, at))
            .is_some_and(|keyword| mapper::to_source(text.mappings, keyword).is_none())
    };
    for item in items {
        let label = item["label"].as_str().unwrap_or_default().to_string();
        if generated_names.contains(&label)
            || imports_from_runtime(&item)
            || (item["data"]["source"].as_str() == Some(SWITCH_CASES_SOURCE) && generated_switch())
        {
            continue;
        }
        let source = item["data"]["source"].as_str().map(str::to_owned);
        session.last_completion.insert(
            (path.to_path_buf(), at, label.clone(), source.clone()),
            item.clone(),
        );
        let replaced = ["replace", "range"]
            .iter()
            .map(|key| &item["textEdit"][*key])
            .find(|range| range.is_object())
            .and_then(|range| {
                source_edit(
                    code,
                    text.mappings,
                    text.inserted,
                    text.source,
                    text.splice,
                    &serde_json::json!({ "range": range, "newText": "" }),
                )
            });
        entries.push(CompletionItem {
            kind: completion_kind(item["kind"].as_u64()),
            sort_text: item["sortText"].as_str().unwrap_or(&label).to_string(),
            insert_text: item["insertText"]
                .as_str()
                .or_else(|| item["textEdit"]["newText"].as_str())
                .map(str::to_owned),
            filter_text: item["filterText"].as_str().map(str::to_owned),
            snippet: item["insertTextFormat"].as_u64() == Some(2),
            range: replaced.map(|edit| edit.range),
            label_detail: item["labelDetails"]["detail"].as_str().map(str::to_owned),
            description: item["labelDetails"]["description"]
                .as_str()
                .map(str::to_owned),
            detail: item["detail"].as_str().map(str::to_owned),
            source,
            label,
        });
    }
    Ok(CompletionAnswer {
        items: entries,
        // A member completion is one the server answered for a `.` — what
        // tells a real member list from the global scope.
        member: is_member_context(code, at),
        probe: None,
    })
}

const SWITCH_CASES_SOURCE: &str = "SwitchCases/";

fn module_specifier_at(source: &str, kind: crate::SourceKind, at: usize) -> Option<(usize, usize)> {
    use crate::lexer::TokenKind;
    let tokens = crate::lexer::lex_with_kind(source, 0, source.len(), kind);
    let index = tokens.iter().position(|token| {
        matches!(token.kind, TokenKind::Str) && token.span.start < at && at <= token.span.end
    })?;
    let token = &tokens[index];
    let quote = source.as_bytes()[token.span.start];
    let closed =
        token.span.end - token.span.start >= 2 && source.as_bytes()[token.span.end - 1] == quote;
    let end = if closed {
        token.span.end - 1
    } else {
        token.span.end
    };
    if at > end {
        return None;
    }
    let word = |index: usize, text: &str| {
        tokens.get(index).is_some_and(|token| {
            matches!(token.kind, TokenKind::Ident)
                && &source[token.span.start..token.span.end] == text
        })
    };
    let previous = index.checked_sub(1)?;
    let specifier = word(previous, "from")
        || (word(previous, "import")
            && !previous
                .checked_sub(1)
                .is_some_and(|dot| matches!(tokens[dot].kind, TokenKind::Punct(b'.'))))
        || (matches!(tokens[previous].kind, TokenKind::Punct(b'('))
            && previous
                .checked_sub(1)
                .is_some_and(|callee| word(callee, "import") || word(callee, "require")));
    specifier.then_some((token.span.start + 1, end))
}

pub(super) fn tt_module_entries(
    path: &Path,
    source: &str,
    position: Position,
    overlays: &HashMap<PathBuf, String>,
) -> Vec<CompletionItem> {
    let at = source_byte(source, position);
    let kind = crate::SourceKind::from_path(path).unwrap_or_default();
    let Some((start, end)) = module_specifier_at(source, kind, at) else {
        return Vec::new();
    };
    let typed = &source[start..at];
    if !typed.starts_with("./") && !typed.starts_with("../") {
        return Vec::new();
    }
    let slash = typed.rfind('/').map_or(0, |slash| slash + 1);
    let Some(directory) = path
        .parent()
        .and_then(|parent| crate::engine::paths::canonical(&parent.join(&typed[..slash])).ok())
    else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = std::fs::read_dir(&directory)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.path())
        .chain(
            overlays
                .keys()
                .filter(|open| open.parent() == Some(directory.as_path()))
                .cloned(),
        )
        .filter(|file| crate::SourceKind::from_tt_path(file).is_some())
        .filter(|file| file.file_name() != path.file_name() || file.parent() != path.parent())
        .collect();
    files.sort();
    files.dedup();
    let range = span_range(source, start + slash, end);
    files
        .into_iter()
        .filter_map(|file| {
            let name = file.file_name()?.to_str()?.to_string();
            Some(CompletionItem {
                detail: Some(name.clone()),
                label: name,
                kind: "script".to_string(),
                sort_text: "11".to_string(),
                insert_text: None,
                filter_text: None,
                snippet: false,
                range: Some(range),
                label_detail: None,
                description: None,
                source: None,
            })
        })
        .collect()
}

fn enclosing_switch(code: &str, kind: crate::SourceKind, at: usize) -> Option<usize> {
    use crate::lexer::TokenKind;
    let tokens = crate::lexer::lex_with_kind(code, 0, code.len(), kind);
    let mut open: Vec<(usize, Option<usize>)> = Vec::new();
    let mut openers: HashMap<usize, usize> = HashMap::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.span.start >= at {
            break;
        }
        if token.opens_bracket() {
            let keyword = (matches!(token.kind, TokenKind::Punct(b'{')) && index > 0)
                .then(|| openers.get(&(index - 1)))
                .flatten()
                .and_then(|&paren| paren.checked_sub(1))
                .map(|keyword| &tokens[keyword])
                .filter(|keyword| {
                    matches!(keyword.kind, TokenKind::Ident)
                        && &code[keyword.span.start..keyword.span.end] == "switch"
                })
                .map(|keyword| keyword.span.start);
            open.push((index, keyword));
        } else if token.closes_bracket()
            && let Some((opener, _)) = open.pop()
        {
            openers.insert(index, opener);
        }
    }
    open.into_iter().rev().find_map(|(_, keyword)| keyword)
}

/// Whether a completion entry imports an export of the pipeline runtime.
/// The runtime is compiler-owned: every export of it is a helper the
/// emitter calls under a generated name, never a name the user writes.
fn imports_from_runtime(item: &serde_json::Value) -> bool {
    item["data"]["autoImport"]["moduleSpecifier"].as_str()
        == Some(crate::StdPackage::Runtime.name())
}

/// The source byte a position names — the analysis speaks bytes, the
/// protocol UTF-16.
pub(in super::super) fn source_byte(source: &str, position: Position) -> usize {
    byte_at(&LineMap::lsp(source), position)
}

/// The match analysis of one file as a parse-only question: imported
/// declarations are the CLI's 1-hop collection, read through `texts` — the
/// session's open documents, or the disk when there is no session. This is
/// what the parse-only surfaces ([`super::names`], [`super::hints`],
/// [`super::completions`]) ask; a surface with a [`Project`] asks
/// [`Project::semantic_analyses`] instead and shares the typed pass's
/// cross-snapshot cache.
pub(in super::super) fn analyses_for(
    path: &Path,
    source: &str,
    texts: Texts<'_>,
) -> crate::PatternAnalyses {
    let externs = externs_of(path, source, &|target| texts.read(target));
    crate::analysis::pattern_analyses_with_kind(
        source,
        &externs,
        crate::SourceKind::from_path(path).unwrap_or_default(),
    )
}

/// A byte span of `text` as a [`Range`] — the byte↔UTF-16 conversion every
/// answer crosses on its way out.
pub(in super::super) fn span_range(text: &str, start: usize, end: usize) -> Range {
    let lines = LineMap::lsp(text);
    Range {
        start: byte_position(&lines, start),
        end: byte_position(&lines, end),
    }
}

/// The variant declarations a file's direct relative `.tt` imports bring into
/// scope, under the names the imports give them — the same 1-hop
/// collection the CLI does for sema.
///
/// `read` decides what "the imported file's text" means: an editor prefers
/// the open buffer, a batch pass the file on disk. The rule the *names*
/// follow is the same either way, which is why it lives here once.
pub(in super::super) fn externs_of(
    path: &Path,
    source: &str,
    read: &dyn Fn(&Path) -> Option<String>,
) -> Vec<crate::VariantSymbol> {
    externs_from(
        path,
        &crate::tt_imports_with_kind(
            source,
            crate::SourceKind::from_path(path).unwrap_or_default(),
        ),
        &|target| {
            let text = read(target)?;
            Some(crate::exported_variant_symbols_with_kind(
                &text,
                crate::SourceKind::from_path(target).unwrap_or_default(),
            ))
        },
    )
}

/// [`externs_of`] over already-parsed pieces: the file's imports and a
/// provider of each target's **exported** declarations. This is the layer
/// the semantic cache uses — a target whose projection is cached hands its
/// symbols over without a re-parse.
pub(in super::super) fn externs_from(
    path: &Path,
    imports: &[crate::TtImport],
    exports_of: &dyn Fn(&Path) -> Option<Vec<crate::VariantSymbol>>,
) -> Vec<crate::VariantSymbol> {
    imported_variants(path, imports, exports_of)
        .into_iter()
        .map(|(_, symbol)| symbol)
        .collect()
}

pub(in super::super) fn imported_variants(
    path: &Path,
    imports: &[crate::TtImport],
    exports_of: &dyn Fn(&Path) -> Option<Vec<crate::VariantSymbol>>,
) -> Vec<(PathBuf, crate::VariantSymbol)> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut externs: Vec<(PathBuf, crate::VariantSymbol)> = Vec::new();
    for import in imports {
        if matches!(import.names, crate::TtImportNames::None) {
            continue; // a re-export brings nothing into scope
        }
        let target = match crate::engine::paths::canonical(&dir.join(&import.specifier)) {
            Ok(target) => target,
            Err(_) => continue, // unresolvable — tsc's TS2307, not ours
        };
        let Some(decls) = exports_of(&target) else {
            continue;
        };
        match &import.names {
            crate::TtImportNames::Namespace(ns) => {
                externs.extend(decls.into_iter().map(|mut d| {
                    d.name = format!("{ns}.{}", d.name);
                    (target.clone(), d)
                }));
            }
            crate::TtImportNames::Named(entries) => {
                for (name, alias) in entries {
                    if let Some(d) = decls.iter().find(|d| &d.name == name) {
                        let mut d = d.clone();
                        d.name = alias.clone().unwrap_or_else(|| name.clone());
                        externs.push((target.clone(), d));
                    }
                }
            }
            crate::TtImportNames::None => unreachable!("skipped above"),
        }
    }
    externs
}

/// The isolated-alternative stand-in: the source with `binding`'s whole
/// alternative list replaced by this occurrence's own alternative, emitted
/// — and the hovered byte's UTF-16 offset in that output. Isolation makes
/// codegen take its single-alternative path, whose destructuring is mapped
/// and narrowed to the one constructor, so the checker's answer at the
/// offset is that alternative's own payload type. `None` when the spans do
/// not line up (a stale analysis) or the byte lands in glue anyway.
pub(super) fn isolate_alternative(
    path: &Path,
    source: &str,
    binding: &crate::PatternBinding,
    byte: usize,
) -> Option<(String, usize)> {
    let ordered = binding.group_start <= binding.alt_start
        && binding.alt_start <= binding.start
        && binding.start <= binding.end
        && binding.end <= binding.alt_end
        && binding.alt_end <= binding.group_end
        && binding.group_end <= source.len();
    if !ordered {
        return None;
    }
    let mut synthetic = String::with_capacity(source.len());
    synthetic.push_str(&source[..binding.group_start]);
    synthetic.push_str(&source[binding.alt_start..binding.alt_end]);
    synthetic.push_str(&source[binding.group_end..]);
    let emit = crate::emit_mapped_with_kind(
        &synthetic,
        crate::SourceKind::from_path(path).unwrap_or_default(),
    );
    // The hovered byte, relocated into the isolated pattern.
    let at = binding.group_start + (byte.clamp(binding.start, binding.end) - binding.alt_start);
    let out = mapper::to_output_inclusive(&emit.mappings, at)?;
    let offset = mapper::to_utf16(&emit.code, out);
    Some((emit.code, offset))
}

/// The declared-type hover of one pattern binding — the analysis' own
/// answer, shown when the checker cannot be asked. `None` when the subject
/// is unknown (an unknown type would be a claim, not an answer).
pub(super) fn declared_binding_hover(
    binding: &crate::PatternBinding,
    range: Range,
) -> Option<HoverInfo> {
    let ty = binding.ty.as_deref()?;
    let case = match &binding.variant_name {
        Some(variant_name) => format!("{variant_name}.{}", binding.tag),
        None => binding.tag.clone(),
    };
    Some(HoverInfo {
        signature: format!("const {}: {}", binding.name, ty),
        documentation: format!("Pattern binding of `{case}` (declared type)."),
        range,
    })
}

/// Builds a completion probe: the source with the placeholder spliced in at
/// `at` (a byte offset), emitted, and the placeholder's mapped position. The
/// emission is the one the service would serve for that text, the faithful
/// projection of TypeScript that does not parse included. `None` when the
/// buffer is broken somewhere a placeholder does not reach.
pub(super) fn build_probe(path: &Path, source: &str, at: usize, version: u64) -> Option<ProbeDoc> {
    if !source.is_char_boundary(at) {
        return None;
    }
    let spliced = format!("{}{}{}", &source[..at], PROBE_NAME, &source[at..]);
    let report = crate::compile_projection_report(
        &spliced,
        &crate::Options {
            filename: path.to_str(),
            source_kind: crate::SourceKind::from_path(path).unwrap_or_default(),
            defer_to_checker: true,
            rewrite_imports: crate::ImportRewrite::Off,
            ..crate::Options::default()
        },
    );
    let emit = report.emit.or(report.withheld)?;
    let out = mapper::to_output_inclusive(&emit.mappings, at)?;
    Some(ProbeDoc {
        path: path.to_path_buf(),
        offset: mapper::to_utf16(&emit.code, out),
        mappings: emit.mappings,
        source: source.to_string(),
        splice: at,
        code: emit.code,
        version,
        generated_names: emit.generated_names,
        inserted: emit.inserted,
    })
}

pub(super) fn signature_position(
    code: &str,
    mappings: &[EmitMapping],
    source_kind: crate::SourceKind,
    at: usize,
) -> usize {
    let tokens = crate::lexer::lex_with_kind(code, 0, code.len(), source_kind);
    let mut at = at;
    while let Some(opener) = innermost_invocation(&tokens, at) {
        if mapper::to_source(mappings, opener).is_some() {
            break;
        }
        at = opener;
    }
    at
}

pub(super) fn signature_question(
    code: &str,
    mappings: &[EmitMapping],
    source: &str,
    source_kind: crate::SourceKind,
    at: usize,
) -> Option<(String, usize)> {
    let tokens = crate::lexer::lex_with_kind(code, 0, code.len(), source_kind);
    let opener = innermost_invocation(&tokens, at)?;
    let written = callee_name(tokens_holding(&tokens, opener), opener)?;
    if mapper::to_source(mappings, written.start).is_some() {
        return None;
    }
    let source_opener = mapper::to_source(mappings, opener)?;
    let source_tokens = crate::lexer::lex_with_kind(source, 0, source.len(), source_kind);
    let name = callee_name(tokens_holding(&source_tokens, source_opener), source_opener)?;
    let name = &source[name.start..name.end];
    let question = format!("{}{name}{}", &code[..written.start], &code[written.end..]);
    let at = if at >= written.end {
        at + name.len() - (written.end - written.start)
    } else {
        at
    };
    Some((question, at))
}

fn tokens_holding(tokens: &[crate::lexer::Token], at: usize) -> &[crate::lexer::Token] {
    use crate::lexer::{TokenKind, TplPart};
    for token in tokens {
        if let TokenKind::Template(parts) = &token.kind
            && token.span.start < at
            && at < token.span.end
        {
            for part in parts.iter() {
                if let TplPart::Interp { span, tokens } = part
                    && span.start <= at
                    && at <= span.end
                {
                    return tokens_holding(tokens, at);
                }
            }
        }
    }
    tokens
}

fn callee_name(tokens: &[crate::lexer::Token], opener: usize) -> Option<crate::ast::Span> {
    use crate::lexer::TokenKind;
    let mut index = tokens
        .iter()
        .position(|token| token.span.start >= opener)
        .unwrap_or(tokens.len())
        .checked_sub(1)?;
    if matches!(tokens[index].kind, TokenKind::Punct(b'>')) && tokens[index].closes_bracket() {
        let mut depth = 0usize;
        loop {
            let token = &tokens[index];
            if token.closes_bracket() {
                depth += 1;
            } else if token.opens_bracket() {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            index = index.checked_sub(1)?;
        }
        index = index.checked_sub(1)?;
    }
    let token = &tokens[index];
    matches!(token.kind, TokenKind::Ident).then_some(token.span)
}

fn innermost_invocation(tokens: &[crate::lexer::Token], at: usize) -> Option<usize> {
    let mut open = Vec::new();
    open_brackets_before(tokens, at, &mut open);
    open.into_iter()
        .rev()
        .find_map(|(start, invocation)| invocation.then_some(start))
}

fn open_brackets_before(tokens: &[crate::lexer::Token], at: usize, open: &mut Vec<(usize, bool)>) {
    use crate::lexer::{TokenKind, TplPart};
    for (index, token) in tokens.iter().enumerate() {
        if token.span.start >= at {
            return;
        }
        if let TokenKind::Template(parts) = &token.kind {
            let interpolation = parts.iter().find_map(|part| match part {
                TplPart::Interp { span, tokens } if span.start <= at && at <= span.end => {
                    Some(tokens)
                }
                _ => None,
            });
            if let Some(tokens) = interpolation {
                open_brackets_before(tokens, at, open);
                return;
            }
            continue;
        }
        if token.opens_bracket() {
            let invocation = matches!(token.kind, TokenKind::Punct(b'(' | b'<'))
                && index.checked_sub(1).is_some_and(|previous| {
                    let previous = &tokens[previous];
                    previous.facts.ends_expression() || matches!(previous.kind, TokenKind::OptChain)
                });
            open.push((token.span.start, invocation));
        } else if token.closes_bracket() {
            open.pop();
        }
    }
}

/// An edit the service computed over served text, as an edit of `source`:
/// `mappings` maps `source` onto `code`, with a completion probe's
/// placeholder spliced in at `splice` when there is one. An insertion
/// before or after a declaration of glue written at a source point
/// (`inserted`) is an insertion at that point. `None` when either end of
/// the range was not copied from the source, when the edit changes glue,
/// or when it falls inside the placeholder.
pub(super) fn source_edit(
    code: &str,
    mappings: &[EmitMapping],
    inserted: &[crate::InsertedGlue],
    source: &str,
    splice: Option<usize>,
    edit: &serde_json::Value,
) -> Option<TextEdit> {
    let start = mapper::from_utf16(code, u16_offset(code, position_of(&edit["range"]["start"])));
    let end = mapper::from_utf16(code, u16_offset(code, position_of(&edit["range"]["end"])));
    let (start, end) = mapper::to_source_span(mappings, start, end).or_else(|| {
        let glue = inserted
            .iter()
            .find(|glue| start == end && (start == glue.out || start == glue.out_end))?;
        Some((glue.src, glue.src))
    })?;
    let unsplice = |byte: usize| match splice {
        Some(at) if byte > at => byte.checked_sub(PROBE_NAME.len()).filter(|&b| b >= at),
        _ => Some(byte),
    };
    Some(TextEdit {
        range: span_range(source, unsplice(start)?, unsplice(end)?),
        new_text: edit["newText"].as_str()?.to_string(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TargetUse {
    Navigation,
    Edit,
}

/// Maps one service answer target back to a user-visible file. `None` when
/// the target is not a file, cannot be read, or the span has no source
/// counterpart for `purpose` — the caller decides whether that skips one
/// result (navigation) or refuses the whole operation (rename).
pub(super) fn map_target(
    session: &mut ServiceSession,
    overlays: &HashMap<PathBuf, String>,
    uri: &str,
    range: &serde_json::Value,
    purpose: TargetUse,
) -> Option<Location> {
    let path = uri_path(uri)?;
    let lsp_range = Range {
        start: position_of(&range["start"]),
        end: position_of(&range["end"]),
    };
    if let Some(tt_path) = tt_document(&path) {
        let doc = serve_doc_only(session, overlays, &tt_path)?;
        let start = u16_offset(&doc.code, lsp_range.start);
        let end = u16_offset(&doc.code, lsp_range.end);
        let (s, e) = match from_service_span(&doc, start, end) {
            Some(span) => span,
            None if purpose == TargetUse::Navigation => declared_name_span(&doc, start, end)?,
            None => return None,
        };
        return Some(Location {
            path: tt_path,
            range: source_range(&doc.source, s, e),
        });
    }
    // A hand-written TypeScript file: the answer's coordinates are already
    // the file's own.
    Some(Location {
        path,
        range: lsp_range,
    })
}

/// A projection for mapping an answer's coordinates — built (and cached)
/// without serving, for targets the question never travelled through.
pub(super) fn serve_doc_only(
    session: &mut ServiceSession,
    overlays: &HashMap<PathBuf, String>,
    path: &Path,
) -> Option<Arc<ServiceDoc>> {
    let text = match overlays.get(path) {
        Some(text) => text.clone(),
        None => std::fs::read_to_string(path).ok()?,
    };
    match session.docs.get(path) {
        Some(doc) if doc.source == text => Some(doc.clone()),
        _ => {
            let doc = Arc::new(service_doc(path, text));
            session.docs.insert(path.to_path_buf(), doc.clone());
            Some(doc)
        }
    }
}

/// A tt position translated into the served text for a question about the
/// name the cursor touches (hover, navigation), or `None` when it sits in
/// compiler-written glue. As TypeScript resolves a touching name, the name
/// starting at the cursor wins over the one ending there.
pub(super) fn to_service(doc: &ServiceDoc, position: Position) -> Option<usize> {
    let byte = mapper::from_utf16(&doc.source, u16_offset(&doc.source, position));
    let affinity = match doc.source.as_bytes().get(byte) {
        Some(&b) if crate::scanner::is_ident_start(b) || b == b'#' || !b.is_ascii() => {
            mapper::Affinity::Following
        }
        _ => mapper::Affinity::Preceding,
    };
    let out = mapper::cursor_to_output(&doc.mappings, byte, affinity)?;
    Some(mapper::to_utf16(&doc.code, out))
}

/// A tt position translated into the served text for a question about what
/// is being typed before the cursor (completion, signature help).
pub(super) fn to_service_typed(doc: &ServiceDoc, position: Position) -> Option<usize> {
    let byte = mapper::from_utf16(&doc.source, u16_offset(&doc.source, position));
    let out = mapper::cursor_to_output(&doc.mappings, byte, mapper::Affinity::Preceding)?;
    Some(mapper::to_utf16(&doc.code, out))
}

pub(super) fn to_service_name(doc: &ServiceDoc, position: Position) -> Option<usize> {
    if let Some(at) = to_service(doc, position) {
        return Some(at);
    }
    let byte = mapper::from_utf16(&doc.source, u16_offset(&doc.source, position));
    doc.shared_bindings.iter().find_map(|binding| {
        let occurrence = binding
            .occurrences
            .iter()
            .find(|occurrence| occurrence.src <= byte && byte <= occurrence.src_end)?;
        let within = (byte - occurrence.src).min(binding.out_end - binding.out);
        Some(mapper::to_utf16(&doc.code, binding.out + within))
    })
}

/// Where to ask the service about the name at `position`: the served text
/// the name was copied to, or — for a name ttc declares in glue, a variant's
/// type and constructor, a case's constructor, a payload field's properties
/// — every place the emission declares it. Empty when neither holds.
pub(super) fn to_service_names(doc: &ServiceDoc, position: Position) -> Vec<usize> {
    if let Some(at) = to_service_name(doc, position) {
        return vec![at];
    }
    let byte = mapper::from_utf16(&doc.source, u16_offset(&doc.source, position));
    doc.declared_names
        .iter()
        .filter(|name| name.src <= byte && byte <= name.src_end)
        .map(|name| {
            mapper::to_utf16(
                &doc.code,
                name.out + (byte - name.src).min(name.out_end - name.out),
            )
        })
        .collect()
}

/// The source span (UTF-16) of the glue-declared name covering `position`.
pub(super) fn declared_name_at(doc: &ServiceDoc, position: Position) -> Option<(usize, usize)> {
    let byte = mapper::from_utf16(&doc.source, u16_offset(&doc.source, position));
    doc.declared_names
        .iter()
        .find(|name| name.src <= byte && byte <= name.src_end)
        .map(|name| {
            (
                mapper::to_utf16(&doc.source, name.src),
                mapper::to_utf16(&doc.source, name.src_end),
            )
        })
}

pub(super) struct SharedTarget {
    pub location: Location,
    pub name: String,
    pub shorthand: bool,
}

pub(super) fn map_shared_target(
    session: &mut ServiceSession,
    overlays: &HashMap<PathBuf, String>,
    uri: &str,
    range: &serde_json::Value,
) -> Option<(String, Vec<SharedTarget>)> {
    let tt_path = tt_document(&uri_path(uri)?)?;
    let doc = serve_doc_only(session, overlays, &tt_path)?;
    let start = mapper::from_utf16(
        &doc.code,
        u16_offset(&doc.code, position_of(&range["start"])),
    );
    let end = mapper::from_utf16(&doc.code, u16_offset(&doc.code, position_of(&range["end"])));
    let binding = doc
        .shared_bindings
        .iter()
        .find(|binding| binding.out == start && binding.out_end == end)?;
    let targets = binding
        .occurrences
        .iter()
        .map(|occurrence| SharedTarget {
            location: Location {
                path: tt_path.clone(),
                range: span_range(&doc.source, occurrence.src, occurrence.src_end),
            },
            name: doc.source[occurrence.src..occurrence.src_end].to_string(),
            shorthand: occurrence.shorthand,
        })
        .collect();
    Some((doc.code[start..end].to_string(), targets))
}

/// The service's outline of served text, on the source: an entry whose name
/// ttc wrote (a generated binding, the type and constructor a `variant`
/// becomes) is not the user's, and its mapped children take its place.
pub(super) fn source_symbols(doc: &ServiceDoc, items: &[serde_json::Value]) -> Vec<DocumentSymbol> {
    let mut out = Vec::new();
    for item in items {
        let children = source_symbols(
            doc,
            item["children"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default(),
        );
        let offset = |value: &serde_json::Value| u16_offset(&doc.code, position_of(value));
        let selection = &item["selectionRange"];
        let Some((name_start, name_end)) =
            from_service_span(doc, offset(&selection["start"]), offset(&selection["end"]))
        else {
            out.extend(children);
            continue;
        };
        // The declaration's ends are the user's even when glue sits inside
        // it (a `match` in a function body); an end that is not keeps the
        // range to the name.
        let point = |value: &serde_json::Value| {
            let byte = mapper::from_utf16(&doc.code, offset(value));
            mapper::to_source_inclusive(&doc.mappings, byte)
                .map(|source| mapper::to_utf16(&doc.source, source))
        };
        let start = point(&item["range"]["start"]).map_or(name_start, |at| at.min(name_start));
        let end = point(&item["range"]["end"]).map_or(name_end, |at| at.max(name_end));
        out.push(DocumentSymbol {
            name: item["name"].as_str().unwrap_or_default().to_string(),
            detail: item["detail"].as_str().unwrap_or_default().to_string(),
            kind: item["kind"].as_u64().unwrap_or(13) as u32,
            range: source_range(&doc.source, start, end),
            selection_range: source_range(&doc.source, name_start, name_end),
            children,
        });
    }
    // Lowering can move a declaration ahead of the text around it (a
    // pattern binding hoisted above its `match`); the outline follows the
    // source.
    out.sort_by_key(|symbol| (symbol.range.start.line, symbol.range.start.character));
    out
}

pub(super) fn source_tokens(
    doc: &ServiceDoc,
    legend: &crate::typescript::service::SemanticLegend,
    data: &[u64],
) -> Vec<ClassifiedToken> {
    let code_lines = LineMap::lsp(&doc.code);
    let source_lines = LineMap::lsp(&doc.source);
    let mut out: Vec<ClassifiedToken> = Vec::new();
    let (mut line, mut character) = (0u64, 0u64);
    for &[delta_line, delta_start, length, kind, bits] in data.as_chunks::<5>().0 {
        if delta_line > 0 {
            line += delta_line;
            character = delta_start;
        } else {
            character += delta_start;
        }
        let Some(token_type) = legend.types.get(kind as usize) else {
            continue;
        };
        let start = byte_at(
            &code_lines,
            Position {
                line: line as u32,
                character: character as u32,
            },
        );
        let end = byte_at(
            &code_lines,
            Position {
                line: line as u32,
                character: (character + length) as u32,
            },
        );
        let Some((from, to)) = mapper::to_source_span(&doc.mappings, start, end) else {
            continue;
        };
        if to <= from {
            continue;
        }
        let modifiers = legend
            .modifiers
            .iter()
            .enumerate()
            .filter(|(index, _)| *index < 64 && bits & (1 << index) != 0)
            .map(|(_, name)| name.clone())
            .collect();
        let classified = ClassifiedToken {
            range: Range {
                start: byte_position(&source_lines, from),
                end: byte_position(&source_lines, to),
            },
            token_type: token_type.clone(),
            modifiers,
        };
        if !out.contains(&classified) {
            out.push(classified);
        }
    }
    out
}

pub(super) fn merge_tokens(
    own: Vec<crate::engine::tokens::SemanticToken>,
    service: Vec<ClassifiedToken>,
) -> Vec<ClassifiedToken> {
    let overlaps = |a: &Range, b: &Range| {
        a.start.line == b.start.line
            && a.start.character < b.end.character
            && b.start.character < a.end.character
    };
    let mut out: Vec<ClassifiedToken> = own
        .into_iter()
        .map(|token| {
            let token_type = token.kind.as_str().to_string();
            let mut modifiers: Vec<String> = token
                .kind
                .modifiers()
                .iter()
                .map(|m| m.to_string())
                .collect();
            if let Some(other) = service
                .iter()
                .find(|other| other.range == token.range && other.token_type == token_type)
            {
                for modifier in &other.modifiers {
                    if !modifiers.contains(modifier) {
                        modifiers.push(modifier.clone());
                    }
                }
            }
            ClassifiedToken {
                range: token.range,
                token_type,
                modifiers,
            }
        })
        .collect();
    let owned = out.len();
    for token in service {
        if !out[..owned]
            .iter()
            .any(|own| overlaps(&own.range, &token.range))
        {
            out.push(token);
        }
    }
    out.sort_by_key(|token| (token.range.start.line, token.range.start.character));
    out
}

/// A service span translated back to source UTF-16 offsets, or `None` when
/// any byte of it was not copied verbatim from the source.
pub(super) fn from_service_span(
    doc: &ServiceDoc,
    start: usize,
    end: usize,
) -> Option<(usize, usize)> {
    let sb = mapper::from_utf16(&doc.code, start);
    let eb = mapper::from_utf16(&doc.code, end);
    let (ss, se) = mapper::to_source_span(&doc.mappings, sb, eb)?;
    Some((
        mapper::to_utf16(&doc.source, ss),
        mapper::to_utf16(&doc.source, se),
    ))
}

pub(super) fn declared_name_span(
    doc: &ServiceDoc,
    start: usize,
    end: usize,
) -> Option<(usize, usize)> {
    let sb = mapper::from_utf16(&doc.code, start);
    let eb = mapper::from_utf16(&doc.code, end);
    let name = doc
        .declared_names
        .iter()
        .find(|name| name.out == sb && name.out_end == eb)?;
    Some((
        mapper::to_utf16(&doc.source, name.src),
        mapper::to_utf16(&doc.source, name.src_end),
    ))
}

/// A TypeScript diagnostic span translated back to source UTF-16 offsets.
///
/// Unlike navigation and rename, diagnostics on generated glue are still
/// useful: the CLI reports them at the construct that produced the glue, so
/// the language service follows the same policy.
/// The construct whose glue a served-text UTF-16 offset falls in.
pub(super) fn glue_anchor(doc: &ServiceDoc, utf16_start: usize) -> Option<crate::EmitAnchor> {
    let out = mapper::from_utf16(&doc.code, utf16_start);
    doc.anchors
        .iter()
        .find(|a| a.out <= out && out < a.end)
        .copied()
}

pub(super) fn diagnostic_source_span(
    doc: &ServiceDoc,
    start: usize,
    end: usize,
) -> Option<(usize, usize, mapper::DiagnosticOrigin)> {
    let sb = mapper::from_utf16(&doc.code, start);
    let eb = mapper::from_utf16(&doc.code, end);
    let origin = match doc
        .destructured_lists
        .iter()
        .find(|list| list.out == sb && list.out_end == eb)
    {
        Some(list) => mapper::DiagnosticOrigin::Exact {
            start: list.src,
            end: list.src_end,
        },
        None => mapper::diagnostic_origin(&doc.mappings, &doc.anchors, sb, eb)?,
    };
    let (start, end) = match origin {
        mapper::DiagnosticOrigin::Exact { start, end } => (start, end),
        mapper::DiagnosticOrigin::Anchor(anchor) => (anchor.src, anchor.src_end),
        mapper::DiagnosticOrigin::Nearest { start } => (start, start.saturating_add(1)),
    };
    Some((
        mapper::to_utf16(&doc.source, start),
        mapper::to_utf16(&doc.source, end),
        origin,
    ))
}

pub(super) fn recovery_intersects(doc: &ServiceDoc, start: usize, end: usize) -> bool {
    let end = end.max(start + 1);
    doc.recovered.iter().any(|&(recovery_start, recovery_end)| {
        let recovery_start = mapper::to_utf16(&doc.source, recovery_start);
        let recovery_end = mapper::to_utf16(&doc.source, recovery_end);
        start < recovery_end && recovery_start < end
    })
}

/// A `[start, end)` pair of UTF-16 offsets as a [`Range`] over `text`.
pub(super) fn source_range(text: &str, start: usize, end: usize) -> Range {
    Range {
        start: u16_position(text, start),
        end: u16_position(text, end),
    }
}

/// The UTF-16 offset a zero-based line/character names in `text` — the LSP
/// convention (3.17): its line breaks, a character past the line's end
/// defaulting back to the line's length, and a line past the text's end
/// clamping to the text's end.
pub(crate) fn u16_offset(text: &str, position: Position) -> usize {
    mapper::to_utf16(text, byte_at(&LineMap::lsp(text), position))
}

/// The zero-based line/character a UTF-16 offset names in `text`.
pub(crate) fn u16_position(text: &str, offset: usize) -> Position {
    byte_position(&LineMap::lsp(text), mapper::from_utf16(text, offset))
}

/// The byte a protocol position names over measured lines.
pub(in super::super) fn byte_at(lines: &LineMap<'_>, position: Position) -> usize {
    lines.utf16_offset(position.line as usize, position.character as usize)
}

/// A byte as a protocol position over measured lines.
pub(in super::super) fn byte_position(lines: &LineMap<'_>, byte: usize) -> Position {
    let (line, character) = lines.utf16_position(byte);
    Position {
        line: line as u32,
        character: character as u32,
    }
}

/// Whether the offset follows a `.` (walking back over identifier
/// characters), which is what makes an answer a member list.
pub(super) fn is_member_context(text: &str, offset: usize) -> bool {
    let mut i = mapper::from_utf16(text, offset);
    let bytes = text.as_bytes();
    while i > 0 {
        let b = bytes[i - 1];
        if b.is_ascii_alphanumeric() || b == b'_' || b == b'$' {
            i -= 1;
        } else {
            break;
        }
    }
    i > 0 && bytes[i - 1] == b'.'
}

pub(super) fn split_hover(contents: &serde_json::Value) -> (String, String) {
    let value = |contents: &serde_json::Value| {
        contents["value"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    match contents {
        serde_json::Value::String(markdown) => split_markdown_hover(markdown),
        serde_json::Value::Object(_) if contents["kind"] == "markdown" => {
            split_markdown_hover(contents["value"].as_str().unwrap_or_default())
        }
        serde_json::Value::Object(_) => (value(contents), String::new()),
        _ => (String::new(), String::new()),
    }
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

/// Documentation as plain text, whichever shape the server used.
pub(super) fn docs_text(documentation: &serde_json::Value) -> String {
    match documentation {
        serde_json::Value::String(s) => s.trim().to_string(),
        value => value["value"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

/// Where a parameter's label sits inside its signature — the span form the
/// presentation needs, computed from the substring form when the server
/// used that.
pub(super) fn parameter_span(signature: &str, label: &serde_json::Value) -> (u32, u32) {
    if let Some(span) = label.as_array()
        && span.len() == 2
    {
        return (
            span[0].as_u64().unwrap_or(0) as u32,
            span[1].as_u64().unwrap_or(0) as u32,
        );
    }
    let Some(text) = label.as_str() else {
        return (0, 0);
    };
    // The span is in UTF-16 units of the label string.
    match signature.find(text) {
        Some(byte) => {
            let start = signature[..byte].encode_utf16().count();
            (start as u32, (start + text.encode_utf16().count()) as u32)
        }
        None => (0, 0),
    }
}

/// The LSP completion kinds the server answers with, as the element-kind
/// strings the editor has always mapped. Anything else is a plain property.
pub(super) fn completion_kind(kind: Option<u64>) -> String {
    match kind {
        Some(3) => "function",
        Some(2) | Some(4) => "method",
        Some(5) => "property",
        Some(6) => "var",
        Some(7) | Some(22) => "class",
        Some(8) => "interface",
        Some(9) => "module",
        Some(13) => "enum",
        Some(14) => "keyword",
        Some(17) => "script",
        Some(19) => "directory",
        Some(21) => "const",
        Some(25) => "type",
        _ => "property",
    }
    .to_string()
}

/// A [`Position`] as the JSON the protocol speaks.
pub(super) fn lsp_position(position: Position) -> serde_json::Value {
    serde_json::json!({ "line": position.line, "character": position.character })
}

/// A JSON position as a [`Position`].
pub(super) fn position_of(value: &serde_json::Value) -> Position {
    Position {
        line: value["line"].as_u64().unwrap_or(0) as u32,
        character: value["character"].as_u64().unwrap_or(0) as u32,
    }
}

/// Makes every `@tt/std` entry resolvable in `root`, by putting the standard
/// library where the TypeScript server looks for a package of that name.
///
/// The compiler backend serves the library from memory; a language server
/// cannot — module resolution reads the file system. So for the service the
/// library has to *be* there. Only tt's own scoped package is ever written,
/// and never over one that already exists: a project that installs
/// the `@tt/std` package itself keeps its own copy.
pub(super) fn ensure_std_module(root: &Path) {
    let _ = crate::StdPackage::Std.materialize(root);
}

/// Makes the pipeline runtime resolvable in `root`, next to `@tt/std`.
///
/// Written at session start rather than when a served file is seen to use
/// a pipeline: "does this text use one" is answered by parsing it, and the
/// editor's hardest question — completion at a `.` the user has just typed
/// — is asked exactly when the text does *not* parse. The probe mends the
/// buffer, the mended form emits `$tt_ap`, and a module that was not there
/// when the service resolved makes the whole expression untyped, so the
/// answer comes back empty (TASK-217).
pub(super) fn ensure_runtime_module(root: &Path) {
    let _ = crate::StdPackage::Runtime.materialize(root);
}

pub(super) struct Discriminant {
    label: String,
    written: String,
    value: crate::ast::LiteralValue,
}

pub(super) fn discriminant(
    label: &str,
    family: crate::engine::completions::PatternFamily,
) -> Option<Discriminant> {
    use crate::ast::{LiteralValue, Pattern};
    use crate::engine::completions::PatternFamily;
    let Some(Pattern::Literals(mut literals)) = crate::parser::pattern_of(label) else {
        return None;
    };
    if literals.len() != 1 {
        return None;
    }
    let value = literals.remove(0).value;
    match family {
        PatternFamily::Literals => Some(Discriminant {
            label: label.to_string(),
            written: label.to_string(),
            value,
        }),
        PatternFamily::Tags => {
            let LiteralValue::Str(tag) = value.clone() else {
                return None;
            };
            let Some(Pattern::Tags(tags)) = crate::parser::pattern_of(&tag) else {
                return None;
            };
            (tags.len() == 1 && tags[0].bindings.is_none() && tags[0].tag == tag).then(|| {
                Discriminant {
                    label: tag,
                    written: label.to_string(),
                    value,
                }
            })
        }
        PatternFamily::Instances => None,
    }
}

pub(super) fn arm_candidates(
    parsed: Vec<crate::engine::TtCompletion>,
    family: crate::engine::completions::PatternFamily,
    typed: Vec<Discriminant>,
    covered: &[String],
    literals: &[crate::ast::LiteralValue],
) -> Vec<crate::engine::TtCompletion> {
    use crate::engine::TtCompletionKind;
    use crate::engine::completions::PatternFamily;
    let mut out: Vec<crate::engine::TtCompletion> = Vec::new();
    for candidate in typed {
        if out.iter().any(|item| item.label == candidate.label) {
            continue;
        }
        let item = match family {
            PatternFamily::Tags => parsed
                .iter()
                .find(|item| item.kind == TtCompletionKind::Case && item.label == candidate.label)
                .cloned()
                .unwrap_or_else(|| crate::engine::TtCompletion {
                    detail: format!(
                        "{}: {}",
                        crate::core_ir::VARIANT_TAG_FIELD,
                        candidate.written
                    ),
                    covered: covered.contains(&candidate.label),
                    label: candidate.label,
                    kind: TtCompletionKind::Case,
                    range: None,
                }),
            _ => crate::engine::TtCompletion {
                detail: format!("literal {}", candidate.written),
                covered: literals.contains(&candidate.value),
                label: candidate.label,
                kind: TtCompletionKind::Literal,
                range: None,
            },
        };
        out.push(item);
    }
    out.push(crate::engine::completions::wildcard());
    out
}

/// Whether a property TypeScript offers for the case a payload pattern
/// selects is a field name that payload can still bind: a name tt's pattern
/// grammar reads as a bare field, not the discriminant the tag already tests
/// (`VARIANT_TAG_FIELD`), and not one the payload already binds.
pub(super) fn is_payload_field(label: &str, written: &[String]) -> bool {
    label != crate::core_ir::VARIANT_TAG_FIELD
        && !written.iter().any(|name| name == label)
        && matches!(
            crate::parser::pattern_of(label),
            Some(crate::ast::Pattern::Tags(tags))
                if tags.len() == 1 && tags[0].bindings.is_none() && tags[0].tag == label
        )
}

pub(super) fn field_candidates(
    parsed: Vec<crate::engine::TtCompletion>,
    typed: Vec<String>,
    written: &[String],
) -> Vec<crate::engine::TtCompletion> {
    let mut out = parsed;
    for name in typed {
        if !is_payload_field(&name, written) || out.iter().any(|item| item.label == name) {
            continue;
        }
        out.push(crate::engine::TtCompletion {
            detail: name.clone(),
            label: name,
            kind: crate::engine::TtCompletionKind::Field,
            covered: false,
            range: None,
        });
    }
    out
}
