//! Service projections, coordinate mapping, and TypeScript response conversion.

mod presentation;
mod targets;

pub(super) use presentation::{docs_text, parameter_span, split_hover};
pub(super) use targets::{TargetUse, map_shared_target, map_target, source_byte_span, source_edit};

use super::*;
use crate::lines::LineMap;

pub(super) fn projection_accepts_diagnostics(code: &str, source_kind: crate::SourceKind) -> bool {
    crate::verify::verify_output(code, source_kind).is_ok()
}

pub(super) fn service_doc(path: &Path, text: String) -> ServiceDoc {
    if crate::engine::project::is_host_source(path) {
        return ServiceDoc {
            source_utf16: std::sync::OnceLock::new(),
            code_utf16: std::sync::OnceLock::new(),
            coordinates: CoordinateSpace::Projected,
            identity_mapping: EmitMapping {
                src: 0,
                out: 0,
                len: text.len(),
            },
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
            relocated_operands: Vec::new(),
            completion_scopes: Vec::new(),
            recovered: Vec::new(),
            syntax_repairs: Vec::new(),
            tt_diagnostics: Vec::new(),
            generated_names: HashSet::new(),
            restatements: Vec::new(),
            inserted: Vec::new(),
            faithful: true,
            source_lines: Default::default(),
            code_lines: Default::default(),
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
        source_utf16: std::sync::OnceLock::new(),
        code_utf16: std::sync::OnceLock::new(),
        coordinates: CoordinateSpace::Projected,
        identity_mapping: EmitMapping {
            src: 0,
            out: 0,
            len: text.len(),
        },
        faithful,
        source_lines: Default::default(),
        code_lines: Default::default(),
        source: text,
        code: emit.code,
        mappings: emit.mappings,
        anchors: emit.anchors,
        declared_names: emit.declared_names,
        shared_bindings: emit.shared_bindings,
        destructured_lists: emit.destructured_lists,
        relocated_operands: emit.relocated_operands,
        completion_scopes: emit.completion_scopes,
        recovered,
        syntax_repairs: report.syntax_repairs,
        tt_diagnostics: report.diagnostics,
        generated_names: emit.generated_names,
        restatements: emit.restatements,
        inserted: emit.inserted,
    }
}

/// Serves one `.tt` file's projection, creating or refreshing it from the
/// overlay or the disk. `None` when the file cannot be read.
pub(super) fn serve_one(
    session: &mut ServiceSession,
    overlays: &HashMap<PathBuf, String>,
    path: &Path,
) -> Result<Option<Arc<ServiceDoc>>, String> {
    let text = match overlays.get(path) {
        Some(text) => text.clone(),
        None => match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(_) => return Ok(None),
        },
    };
    let doc = match session.docs.get(path) {
        Some(doc) if doc.source == text => doc.clone(),
        _ => {
            let mut doc = service_doc(path, text);
            if session.client.serves_authored_sources() {
                doc.coordinates = CoordinateSpace::Authored;
            }
            let doc = Arc::new(doc);
            session.docs.insert(path.to_path_buf(), doc.clone());
            doc
        }
    };
    Ok(Some(doc))
}

/// Publish a normal document with its source identity and full projection.
pub(super) fn open_document(
    session: &mut ServiceSession,
    path: &Path,
    doc: &ServiceDoc,
) -> Result<(), String> {
    if session.client.serves_authored_sources() && !crate::engine::project::is_host_source(path) {
        let uri = file_uri(path);
        if let Some(previous) = session.uris.insert(path.to_path_buf(), uri.clone())
            && previous != uri
        {
            session.client.close(&previous);
        }
        if session
            .client
            .open_source_projection(path, &doc.source, document_response(path, doc))?
        {
            session.last_completion.clear();
            session.last_probe = None;
        }
        session.served.insert(path.to_path_buf(), doc.code.clone());
    } else if session.served.get(path) != Some(&doc.code)
        || session.uris.get(path) != Some(&file_uri(&module_path_of(path)))
    {
        open_served(session, path, &doc.code);
    }
    Ok(())
}

pub(super) fn document_response(path: &Path, doc: &ServiceDoc) -> serde_json::Value {
    let kind = crate::SourceKind::from_path(path).unwrap_or_default();
    serde_json::json!({
        "text": doc.code,
        "extension": format!(".{}", kind.output_extension()),
        "mappings": document_span_mappings(doc),
        "diagnostics": doc.tt_diagnostics.iter().filter(|d| d.severity == crate::Severity::Error)
            .map(crate::content_projection::mapper_diagnostic).collect::<Vec<_>>(),
    })
}

/// The emitter's named declarations and shared bindings are semantic mapping
/// edges too. Include exact copied names without exposing surrounding glue.
fn document_span_mappings(doc: &ServiceDoc) -> Vec<serde_json::Value> {
    let mut mappings = doc.mappings.clone();
    let mut add_name = |src: usize, src_end: usize, out: usize, out_end: usize| {
        if doc.source.get(src..src_end) != doc.code.get(out..out_end)
            || out_end == out
            || mappings
                .iter()
                .any(|m| m.out < out_end && out < m.out + m.len)
        {
            return;
        }
        mappings.push(EmitMapping {
            src,
            out,
            len: out_end - out,
        });
    };
    for name in &doc.declared_names {
        add_name(name.src, name.src_end, name.out, name.out_end);
    }
    for binding in &doc.shared_bindings {
        if let Some(first) = binding.occurrences.first() {
            add_name(first.src, first.src_end, binding.out, binding.out_end);
        }
    }
    crate::content_projection::span_mappings(&mappings, &doc.anchors, &doc.recovered)
}

/// Restore the authored revision even when the temporary question failed.
pub(super) fn restore_document<T>(
    session: &mut ServiceSession,
    path: &Path,
    doc: &ServiceDoc,
    answer: Result<T, String>,
) -> Result<T, String> {
    let restored = open_document(session, path, doc);
    match (answer, restored) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(error), Err(restore)) => Err(format!("{error}; restoring document failed: {restore}")),
    }
}

/// Temporary projections have a virtual identity even in an authored session.
/// Their request/response coordinates remain projected until restoration.
pub(super) fn open_served(session: &mut ServiceSession, path: &Path, code: &str) {
    let uri = if crate::engine::project::is_host_source(path) {
        file_uri(path)
    } else {
        file_uri(&module_path_of(path))
    };
    if let Some(previous) = session.uris.insert(path.to_path_buf(), uri.clone())
        && previous != uri
    {
        session.client.close(&previous);
    }
    session.client.open(&uri, code);
    session.served.insert(path.to_path_buf(), code.to_string());
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
        None if session.client.serves_authored_sources() => file_uri(path),
        None => file_uri(&module_path_of(path)),
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
            || (label.starts_with("$tt_")
                && session
                    .docs
                    .values()
                    .any(|doc| doc.generated_names.contains(&label)))
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
        let replaced_range = ["replace", "range"]
            .iter()
            .map(|key| &item["textEdit"][*key])
            .find(|range| range.is_object());
        let insert_text = item["insertText"]
            .as_str()
            .or_else(|| item["textEdit"]["newText"].as_str());
        if let Some(range) = replaced_range
            && let Some((start, _)) =
                source_byte_span(code, text.mappings, text.inserted, text.splice, range)
            && pipe_step_dot(text.source, kind, start)
            && !insert_text.is_some_and(|text| text.starts_with('.') || text.starts_with("?."))
        {
            continue;
        }
        let replaced = replaced_range.and_then(|range| {
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
            kind: item["kind"]
                .as_u64()
                .and_then(crate::engine::CompletionItemKind::from_lsp),
            tags: item["tags"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|tag| {
                    tag.as_u64()
                        .and_then(crate::engine::CompletionItemTag::from_lsp)
                })
                .collect(),
            sort_text: item["sortText"].as_str().unwrap_or(&label).to_string(),
            insert_text: insert_text.map(str::to_owned),
            filter_text: item["filterText"].as_str().map(str::to_owned),
            snippet: item["insertTextFormat"].as_u64() == Some(2),
            range: replaced.map(|edit| edit.range),
            label_detail: item["labelDetails"]["detail"].as_str().map(str::to_owned),
            description: item["labelDetails"]["description"]
                .as_str()
                .map(|description| {
                    tt_module_specifier(session, path, description)
                        .unwrap_or_else(|| description.to_owned())
                }),
            detail: item["detail"].as_str().map(str::to_owned),
            source,
            label,
        });
    }
    Ok(CompletionAnswer {
        items: entries,
        // The served grammar must have a receiver, not merely a dot in
        // unfinished tt syntax where TypeScript answers with global names.
        member: is_member_context(code, at, kind),
        probe: None,
    })
}

const SWITCH_CASES_SOURCE: &str = "SwitchCases/";

fn pipe_step_dot(source: &str, kind: crate::SourceKind, at: usize) -> bool {
    use crate::lexer::{TokenKind, TplPart};
    fn search(tokens: &[crate::lexer::Token], at: usize) -> bool {
        tokens.iter().enumerate().any(|(index, token)| {
            (token.span.start == at
                && matches!(token.kind, TokenKind::Punct(b'.'))
                && index
                    .checked_sub(1)
                    .is_some_and(|previous| matches!(tokens[previous].kind, TokenKind::PipeOp)))
                || match &token.kind {
                    TokenKind::Template(parts) => parts.iter().any(|part| match part {
                        TplPart::Interp { tokens, .. } => search(tokens, at),
                        TplPart::Raw(_) => false,
                    }),
                    _ => false,
                }
        })
    }
    source.as_bytes().get(at) == Some(&b'.')
        && search(
            &crate::lexer::lex_with_kind(source, 0, source.len(), kind),
            at,
        )
}

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
                kind: Some(crate::engine::CompletionItemKind::File),
                tags: Vec::new(),
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
    let externs: Vec<crate::resolve::ExternDecl> =
        externs_of(path, source, &|target| texts.read(target))
            .iter()
            .map(Into::into)
            .collect();
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
) -> Vec<crate::resolve::ImportedVariant> {
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
) -> Vec<crate::resolve::ImportedVariant> {
    imported_variants(path, imports, exports_of)
        .into_iter()
        .map(|(_, imported)| imported)
        .collect()
}

pub(in super::super) fn imported_variants(
    path: &Path,
    imports: &[crate::TtImport],
    exports_of: &dyn Fn(&Path) -> Option<Vec<crate::VariantSymbol>>,
) -> Vec<(PathBuf, crate::resolve::ImportedVariant)> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut externs: Vec<(PathBuf, crate::resolve::ImportedVariant)> = Vec::new();
    let imported =
        |symbol: crate::VariantSymbol, specifier: &str| crate::resolve::ImportedVariant {
            specifier: specifier.to_string(),
            symbol,
        };
    for import in imports {
        if matches!(import.names, crate::TtImportNames::None) {
            continue; // a re-export brings nothing into scope
        }
        let target = match crate::engine::normalize_document_path(&dir.join(import.path())) {
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
                    (target.clone(), imported(d, &import.specifier))
                }));
            }
            crate::TtImportNames::Named(entries) => {
                for (name, alias) in entries {
                    if let Some(d) = decls.iter().find(|d| &d.name == name) {
                        let mut d = d.clone();
                        d.name = alias.clone().unwrap_or_else(|| name.clone());
                        externs.push((target.clone(), imported(d, &import.specifier)));
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
    let options = crate::Options {
        filename: path.to_str(),
        source_kind: crate::SourceKind::from_path(path).unwrap_or_default(),
        defer_to_checker: true,
        rewrite_imports: crate::ImportRewrite::Off,
        ..crate::Options::default()
    };
    let probe_end = at + PROBE_NAME.len();
    let report = crate::compile_projection_report(&spliced, &options);
    let report = match report
        .recovered
        .iter()
        .find(|&&(start, end)| start <= at && probe_end <= end)
        .and_then(|&(start, _)| closed_at(&spliced, start, probe_end, options.source_kind))
    {
        Some(closed) => {
            let mended = crate::compile_projection_report(&closed, &options);
            if mended
                .recovered
                .iter()
                .any(|&(start, end)| start <= at && probe_end <= end)
            {
                report
            } else {
                mended
            }
        }
        None => report,
    };
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

/// `text` with the brackets its construct starting at `start` leaves open
/// before `at` closed right there — what TypeScript's parser assumes of a
/// missing closer (`parseExpected` reports it and parses on), for a probe
/// written where the whole construct was recovered because it never
/// closes. `None` when nothing is open.
pub(super) fn closed_at(
    text: &str,
    start: usize,
    at: usize,
    source_kind: crate::SourceKind,
) -> Option<String> {
    use crate::lexer::TokenKind;
    let tokens = crate::lexer::lex_with_kind(text, start, at, source_kind);
    let mut open = Vec::new();
    for token in &tokens {
        if token.opens_bracket() {
            open.push(match token.kind {
                TokenKind::Punct(b'(') => ')',
                TokenKind::Punct(b'[') => ']',
                TokenKind::Punct(b'{') => '}',
                _ => '>',
            });
        } else if token.closes_bracket() {
            open.pop();
        }
    }
    if open.is_empty() {
        return None;
    }
    let closers: String = open.into_iter().rev().collect();
    Some(format!("{}{closers}{}", &text[..at], &text[at..]))
}

pub(super) fn signature_position(
    code: &str,
    mappings: &[EmitMapping],
    relocated: &[crate::RelocatedOperand],
    source_kind: crate::SourceKind,
    at: usize,
) -> usize {
    let tokens = crate::lexer::lex_with_kind(code, 0, code.len(), source_kind);
    let mut at = at;
    for _ in 0..=relocated.len() {
        let mut invocation = innermost_invocation(&tokens, at);
        while let Some(opener) = invocation {
            if mapper::to_source(mappings, opener).is_some() {
                break;
            }
            at = opener;
            invocation = innermost_invocation(&tokens, at);
        }
        let Some(source_at) = mapper::to_source_inclusive(mappings, at) else {
            break;
        };
        let Some(operand) = relocated
            .iter()
            .filter(|operand| {
                operand.src <= source_at
                    && source_at <= operand.src_end
                    && !(operand.out <= at && at <= operand.out_end)
            })
            .min_by_key(|operand| operand.src_end - operand.src)
        else {
            break;
        };
        let inside = invocation
            .and_then(|opener| mapper::to_source(mappings, opener))
            .is_some_and(|opener| operand.src <= opener && opener < operand.src_end);
        let read = if source_at == operand.src {
            operand.out
        } else {
            operand.out_end
        };
        if inside || innermost_invocation(&tokens, read).is_none() {
            break;
        }
        at = read;
    }
    at
}

/// Where to ask about an unmapped cursor inside an operand moved out of its
/// place: the end of the operand's read, when that read is an argument of a
/// call. `None` when no operand holds the cursor or its read is not in a
/// call (a call that moved with the operand is read whole).
pub(super) fn relocated_read_end(
    code: &str,
    relocated: &[crate::RelocatedOperand],
    source_kind: crate::SourceKind,
    at: usize,
) -> Option<usize> {
    let operand = relocated
        .iter()
        .filter(|operand| operand.src < at && at < operand.src_end)
        .min_by_key(|operand| operand.src_end - operand.src)?;
    let tokens = crate::lexer::lex_with_kind(code, 0, code.len(), source_kind);
    innermost_invocation(&tokens, operand.out_end).map(|_| operand.out_end)
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
    crate::stack::grow(|| tokens_holding_grown(tokens, at))
}

fn tokens_holding_grown(tokens: &[crate::lexer::Token], at: usize) -> &[crate::lexer::Token] {
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
    crate::stack::grow(|| open_brackets_before_grown(tokens, at, open));
}

fn open_brackets_before_grown(
    tokens: &[crate::lexer::Token],
    at: usize,
    open: &mut Vec<(usize, bool)>,
) {
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
    if doc.coordinates == CoordinateSpace::Authored {
        return Some(u16_offset(&doc.source, position));
    }
    let byte = doc
        .source_utf16()
        .to_byte(u16_offset(&doc.source, position));
    let affinity = match doc.source.as_bytes().get(byte) {
        Some(&b) if crate::scanner::is_ident_start(b) || b == b'#' || !b.is_ascii() => {
            mapper::Affinity::Following
        }
        _ => mapper::Affinity::Preceding,
    };
    let out = mapper::cursor_to_output(&doc.mappings, byte, affinity)?;
    Some(doc.code_utf16().to_utf16(out))
}

/// A tt position translated into the served text for a question about what
/// is being typed before the cursor (completion, signature help).
pub(super) fn to_service_typed(doc: &ServiceDoc, position: Position) -> Option<usize> {
    let byte = doc
        .source_utf16()
        .to_byte(u16_offset(&doc.source, position));
    let out = mapper::typed_cursor_to_output(&doc.mappings, &doc.anchors, &doc.source, byte)?;
    if doc.coordinates == CoordinateSpace::Authored {
        return Some(u16_offset(&doc.source, position));
    }
    Some(doc.code_utf16().to_utf16(out))
}

pub(super) fn to_service_name(doc: &ServiceDoc, position: Position) -> Option<usize> {
    let at = u16_offset(&doc.source, position);
    if recovery_intersects(doc, at, at + 1) {
        return None;
    }
    if doc.coordinates == CoordinateSpace::Authored {
        let byte = source_byte(&doc.source, position);
        for binding in &doc.shared_bindings {
            if let Some(occurrence) = binding
                .occurrences
                .iter()
                .find(|o| o.src <= byte && byte <= o.src_end)
            {
                let first = binding.occurrences.first()?;
                return Some(mapper::to_utf16(
                    &doc.source,
                    first.src + (byte - occurrence.src).min(first.src_end - first.src),
                ));
            }
        }
    }

    if let Some(at) = to_service(doc, position) {
        return Some(at);
    }
    let byte = doc
        .source_utf16()
        .to_byte(u16_offset(&doc.source, position));
    doc.shared_bindings.iter().find_map(|binding| {
        let occurrence = binding
            .occurrences
            .iter()
            .find(|occurrence| occurrence.src <= byte && byte <= occurrence.src_end)?;
        let within = (byte - occurrence.src).min(binding.out_end - binding.out);
        Some(doc.code_utf16().to_utf16(binding.out + within))
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
    let at = u16_offset(&doc.source, position);
    if recovery_intersects(doc, at, at + 1) {
        return Vec::new();
    }
    let byte = doc
        .source_utf16()
        .to_byte(u16_offset(&doc.source, position));
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
    let byte = doc
        .source_utf16()
        .to_byte(u16_offset(&doc.source, position));
    doc.declared_names
        .iter()
        .find(|name| name.src <= byte && byte <= name.src_end)
        .map(|name| {
            (
                doc.source_utf16().to_utf16(name.src),
                doc.source_utf16().to_utf16(name.src_end),
            )
        })
}

/// The service's outline of served text, on the source: an entry whose name
/// ttc wrote (a generated binding, the type and constructor a `variant`
/// becomes) is not the user's, and its mapped children take its place.
pub(super) fn source_symbols(doc: &ServiceDoc, items: &[serde_json::Value]) -> Vec<DocumentSymbol> {
    let code_lines = doc.service_lines();
    let source_lines = doc.source_lines();
    crate::stack::grow(|| source_symbols_grown(doc, items, &code_lines, &source_lines))
}

fn source_symbols_grown(
    doc: &ServiceDoc,
    items: &[serde_json::Value],
    code_lines: &LineMap<'_>,
    source_lines: &LineMap<'_>,
) -> Vec<DocumentSymbol> {
    let mut out = Vec::new();
    for item in items {
        let children = crate::stack::grow(|| {
            source_symbols_grown(
                doc,
                item["children"]
                    .as_array()
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
                code_lines,
                source_lines,
            )
        });
        let byte = |value: &serde_json::Value| byte_at(code_lines, position_of(value));
        let selection = &item["selectionRange"];
        let name = match doc.coordinates {
            CoordinateSpace::Authored => Some((byte(&selection["start"]), byte(&selection["end"]))),
            CoordinateSpace::Projected => {
                let (start, end) = (byte(&selection["start"]), byte(&selection["end"]));
                mapper::to_source_span(&doc.mappings, start, end).or_else(|| {
                    doc.shared_bindings
                        .iter()
                        .find(|binding| binding.out <= start && end <= binding.out_end)
                        .and_then(|binding| binding.occurrences.first())
                        .map(|occurrence| (occurrence.src, occurrence.src_end))
                })
            }
        };
        let Some((name_start, name_end)) = name else {
            out.extend(children);
            continue;
        };
        // The declaration's ends are the user's even when glue sits inside
        // it (a `match` in a function body); an end that is not keeps the
        // range to the name.
        let point = |value: &serde_json::Value| match doc.coordinates {
            CoordinateSpace::Authored => Some(byte(value)),
            CoordinateSpace::Projected => mapper::to_source_inclusive(&doc.mappings, byte(value)),
        };
        let start = point(&item["range"]["start"]).map_or(name_start, |at| at.min(name_start));
        let end = point(&item["range"]["end"]).map_or(name_end, |at| at.max(name_end));
        out.push(DocumentSymbol {
            name: item["name"].as_str().unwrap_or_default().to_string(),
            detail: item["detail"].as_str().unwrap_or_default().to_string(),
            kind: item["kind"].as_u64().unwrap_or(13) as u32,
            range: Range {
                start: byte_position(source_lines, start),
                end: byte_position(source_lines, end),
            },
            selection_range: Range {
                start: byte_position(source_lines, name_start),
                end: byte_position(source_lines, name_end),
            },
            children,
        });
    }
    // Lowering can move a declaration ahead of the text around it (a
    // pattern binding hoisted above its `match`, a local a `result` block
    // declares); the outline follows the source. A declaration written
    // inside another's source is that one's child, as TypeScript's
    // navigation tree places what an initializer holds under its variable.
    out.sort_by_key(|symbol| (symbol.range.start.line, symbol.range.start.character));
    let mut nested: Vec<DocumentSymbol> = Vec::with_capacity(out.len());
    for symbol in out {
        match nested.last_mut() {
            Some(parent) if encloses(&parent.range, &symbol.range) => adopt(parent, symbol),
            _ => nested.push(symbol),
        }
    }
    in_source_order(&mut nested);
    nested
}

/// Whether `outer` holds all of `inner` and more.
fn encloses(outer: &Range, inner: &Range) -> bool {
    let at = |position: &Position| (position.line, position.character);
    outer != inner && at(&outer.start) <= at(&inner.start) && at(&inner.end) <= at(&outer.end)
}

/// Places `symbol` under the innermost of `parent`'s descendants holding it.
fn adopt(parent: &mut DocumentSymbol, symbol: DocumentSymbol) {
    match parent.children.last_mut() {
        Some(child) if encloses(&child.range, &symbol.range) => adopt(child, symbol),
        _ => parent.children.push(symbol),
    }
}

/// Puts every level of `symbols` in source order.
fn in_source_order(symbols: &mut [DocumentSymbol]) {
    symbols.sort_by_key(|symbol| (symbol.range.start.line, symbol.range.start.character));
    for symbol in symbols {
        in_source_order(&mut symbol.children);
    }
}

pub(super) fn source_tokens(
    doc: &ServiceDoc,
    legend: &crate::typescript::service::SemanticLegend,
    data: &[u64],
) -> Vec<ClassifiedToken> {
    let code_lines = LineMap::lsp(doc.service_code());
    let source_lines = LineMap::lsp(&doc.source);
    let mut out: Vec<ClassifiedToken> = Vec::new();
    let mut seen = std::collections::HashSet::new();
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
        let modifiers: Vec<String> = legend
            .modifiers
            .iter()
            .enumerate()
            .filter(|(index, _)| *index < 64 && bits & (1 << index) != 0)
            .map(|(_, name)| name.clone())
            .collect();
        for (from, to) in token_sources(doc, start, end) {
            let classified = ClassifiedToken {
                range: Range {
                    start: byte_position(&source_lines, from),
                    end: byte_position(&source_lines, to),
                },
                token_type: token_type.clone(),
                modifiers: modifiers.clone(),
            };
            let key = (
                (
                    classified.range.start.line,
                    classified.range.start.character,
                ),
                (classified.range.end.line, classified.range.end.character),
                classified.token_type.clone(),
                classified.modifiers.clone(),
            );
            if seen.insert(key) {
                out.push(classified);
            }
        }
    }
    out
}

/// The source spans a service token over `start..end` of the served text
/// classifies. A binding an or-pattern's alternatives share is declared
/// once in the served text and written in every alternative, so its token
/// is each alternative's; any other token is the source it was copied from.
fn token_sources(doc: &ServiceDoc, start: usize, end: usize) -> Vec<(usize, usize)> {
    if doc.coordinates == CoordinateSpace::Authored {
        return vec![(start, end)];
    }
    if let Some(binding) = doc
        .shared_bindings
        .iter()
        .find(|binding| binding.out == start && binding.out_end == end)
    {
        return binding
            .occurrences
            .iter()
            .map(|occurrence| (occurrence.src, occurrence.src_end))
            .collect();
    }
    mapper::to_source_span(&doc.mappings, start, end)
        .filter(|(from, to)| from < to)
        .into_iter()
        .collect()
}

type RangeKey = ((u32, u32), (u32, u32));

pub(super) fn merge_tokens(
    own: Vec<crate::engine::tokens::SemanticToken>,
    service: Vec<ClassifiedToken>,
) -> Vec<ClassifiedToken> {
    let key = |range: &Range| {
        (
            (range.start.line, range.start.character),
            (range.end.line, range.end.character),
        )
    };
    // The service's modifiers for each range and type, first answer first.
    let mut service_modifiers: HashMap<(RangeKey, &str), &[String]> = HashMap::new();
    for other in &service {
        service_modifiers
            .entry((key(&other.range), other.token_type.as_str()))
            .or_insert(&other.modifiers);
    }
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
            if let Some(others) = service_modifiers.get(&(key(&token.range), token_type.as_str())) {
                for modifier in *others {
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
    // tt's own tokens claim their columns: a service token on a line a tt
    // token shares is kept only where no tt token overlaps it.
    // Per line, tt's tokens by start column with the furthest end reached so
    // far: a range overlaps one of them exactly when some token starting
    // before the range's end ends after its start.
    let mut owned_on: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
    for own in &out {
        owned_on
            .entry(own.range.start.line)
            .or_default()
            .push((own.range.start.character, own.range.end.character));
    }
    for columns in owned_on.values_mut() {
        columns.sort_unstable();
        let mut furthest = 0;
        for column in columns.iter_mut() {
            furthest = furthest.max(column.1);
            column.1 = furthest;
        }
    }
    let overlaps_own = |range: &Range| {
        owned_on.get(&range.start.line).is_some_and(|columns| {
            crate::work::tick("token merge comparisons");
            let before = columns.partition_point(|&(start, _)| start < range.end.character);
            before
                .checked_sub(1)
                .is_some_and(|last| range.start.character < columns[last].1)
        })
    };
    for token in service {
        if !overlaps_own(&token.range) {
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
    if doc.coordinates == CoordinateSpace::Authored {
        return Some((start, end));
    }
    from_projected_span(doc, start, end)
}

fn from_projected_span(doc: &ServiceDoc, start: usize, end: usize) -> Option<(usize, usize)> {
    let sb = doc.code_utf16().to_byte(start);
    let eb = doc.code_utf16().to_byte(end);
    let (ss, se) = mapper::to_source_span(&doc.mappings, sb, eb)?;
    Some((
        doc.source_utf16().to_utf16(ss),
        doc.source_utf16().to_utf16(se),
    ))
}

pub(super) fn declared_name_span(
    doc: &ServiceDoc,
    start: usize,
    end: usize,
) -> Option<(usize, usize)> {
    doc.declared_names
        .iter()
        .find(|name| name.out == start && name.out_end == end)
        .map(|name| (name.src, name.src_end))
}

/// A TypeScript diagnostic span translated back to source UTF-16 offsets.
///
/// Unlike navigation and rename, diagnostics on generated glue are still
/// useful: the CLI reports them at the construct that produced the glue, so
/// the language service follows the same policy.
/// The construct whose glue a served-text UTF-16 offset falls in.
pub(super) fn glue_anchor(doc: &ServiceDoc, utf16_start: usize) -> Option<crate::EmitAnchor> {
    let out = doc.code_utf16().to_byte(utf16_start);
    doc.anchors
        .iter()
        .find(|a| a.out <= out && out < a.end)
        .copied()
}

/// The unconfigured/foreign-mapper service keeps its existing virtual-file
/// project arrangement. Normalize its LSP reply to the same generated-span
/// contract as native diagnostics without changing module resolution.
pub(super) fn projected_service_diagnostics(
    session: &mut ServiceSession,
    path: &Path,
    doc: &ServiceDoc,
) -> Result<Vec<crate::typescript::backend::EditorDiagnostic>, String> {
    use crate::typescript::backend::{EditorDiagnostic, RelatedInformation};
    let served = served_uri(session, path);
    let answer = session.client.request(
        "textDocument/diagnostic",
        serde_json::json!({ "textDocument": { "uri": served } }),
    )?;
    let module = if crate::engine::project::is_host_source(path) {
        path.to_path_buf()
    } else {
        module_path_of(path)
    };
    Ok(answer["items"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|item| {
            let tags = item["tags"].as_array();
            EditorDiagnostic {
                file: module.clone(),
                start: u16_offset(&doc.code, position_of(&item["range"]["start"])),
                end: u16_offset(&doc.code, position_of(&item["range"]["end"])),
                code: item["code"].as_u64().unwrap_or(0) as u32,
                message: item["message"].as_str().unwrap_or_default().to_string(),
                category: match item["severity"].as_u64() {
                    Some(2) => 0,
                    Some(3) => 3,
                    Some(4) => 2,
                    _ => 1,
                },
                unnecessary: tags
                    .is_some_and(|tags| tags.iter().any(|tag| tag.as_u64() == Some(1))),
                deprecated: tags.is_some_and(|tags| tags.iter().any(|tag| tag.as_u64() == Some(2))),
                related: item["relatedInformation"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|entry| entry["location"]["uri"].as_str() == Some(served.as_str()))
                    .map(|entry| RelatedInformation {
                        file: module.clone(),
                        start: u16_offset(
                            &doc.code,
                            position_of(&entry["location"]["range"]["start"]),
                        ),
                        end: u16_offset(&doc.code, position_of(&entry["location"]["range"]["end"])),
                        message: entry["message"].as_str().unwrap_or_default().to_string(),
                    })
                    .collect(),
            }
        })
        .collect())
}

pub(super) fn diagnostic_source_span(
    doc: &ServiceDoc,
    start: usize,
    end: usize,
) -> Option<(usize, usize, mapper::DiagnosticOrigin)> {
    let sb = doc.code_utf16().to_byte(start);
    let eb = doc.code_utf16().to_byte(end);
    if crate::engine::projection::restated(&doc.restatements, sb, eb) {
        return None;
    }
    let origin = match doc
        .destructured_lists
        .iter()
        .find(|list| list.out == sb && list.out_end == eb)
    {
        Some(list) => mapper::DiagnosticOrigin::Exact {
            start: list.src,
            end: list.src_end,
        },
        None => match mapper::shared_binding_origin(&doc.shared_bindings, sb, eb) {
            Some(origin) => origin,
            None => mapper::diagnostic_origin(
                &doc.mappings,
                &doc.anchors,
                sb,
                eb,
                &doc.code,
                &doc.source,
            )?,
        },
    };
    let (start, end) = match origin {
        mapper::DiagnosticOrigin::Exact { start, end } => (start, end),
        mapper::DiagnosticOrigin::Anchor(anchor) => (anchor.src, anchor.src_end),
        mapper::DiagnosticOrigin::Nearest { start } => (start, start.saturating_add(1)),
    };
    Some((
        doc.source_utf16().to_utf16(start),
        doc.source_utf16().to_utf16(end),
        origin,
    ))
}

pub(super) fn recovery_intersects(doc: &ServiceDoc, start: usize, end: usize) -> bool {
    let end = end.max(start + 1);
    doc.recovered.iter().any(|&(recovery_start, recovery_end)| {
        let recovery_start = doc.source_utf16().to_utf16(recovery_start);
        let recovery_end = doc.source_utf16().to_utf16(recovery_end);
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

/// Whether the served TypeScript has a member receiver at this cursor.
pub(super) fn is_member_context(text: &str, offset: usize, kind: crate::SourceKind) -> bool {
    crate::engine::completions::typescript_member_access_at(
        text,
        kind,
        mapper::from_utf16(text, offset),
    )
}

/// `markdown` with the target of each link TypeScript writes for a
/// `{@link}` tag moved from a served tt document to its `.tt` source.
///
/// TypeScript renders a link to a declaration as a markdown link whose
/// target is the declaring file's URI with the declaration's range as a
/// fragment, one-based (`[name](file:///m.tt.ts#32,17-32,22)`). A served
/// document is the emission, so the file and the range are generated ones:
/// the range is mapped back through the emit mapping as a navigation target
/// is ([`map_target`]), and the link names the `.tt` file at the source
/// range. A target that is not a served tt document, or has no source
/// counterpart, is left as TypeScript wrote it.
pub(super) fn source_links(
    session: &mut ServiceSession,
    overlays: &HashMap<PathBuf, String>,
    markdown: &str,
) -> String {
    const OPEN: &str = "](file://";
    let mut out = String::with_capacity(markdown.len());
    let mut rest = markdown;
    while let Some(at) = rest.find(OPEN) {
        let target_start = at + 2;
        out.push_str(&rest[..target_start]);
        rest = &rest[target_start..];
        let Some(close) = rest.find(')') else {
            break;
        };
        let target = &rest[..close];
        out.push_str(&source_link(session, overlays, target).unwrap_or_else(|| target.to_string()));
        rest = &rest[close..];
    }
    out.push_str(rest);
    out
}

fn source_link(
    session: &mut ServiceSession,
    overlays: &HashMap<PathBuf, String>,
    target: &str,
) -> Option<String> {
    let (uri, fragment) = target.split_once('#')?;
    tt_document(&uri_path(uri)?)?;
    let (from, to) = fragment.split_once('-')?;
    let position = |text: &str| -> Option<serde_json::Value> {
        let (line, character) = text.split_once(',')?;
        let line = line.trim().parse::<u64>().ok()?.checked_sub(1)?;
        let character = character.trim().parse::<u64>().ok()?.checked_sub(1)?;
        Some(serde_json::json!({ "line": line, "character": character }))
    };
    let range = serde_json::json!({ "start": position(from)?, "end": position(to)? });
    let location = map_target(session, overlays, uri, &range, TargetUse::Navigation)?;
    Some(format!(
        "{}#{},{}-{},{}",
        file_uri(&location.path),
        location.range.start.line + 1,
        location.range.start.character + 1,
        location.range.end.line + 1,
        location.range.end.character + 1
    ))
}

/// The specifier tt writes for the module TypeScript names `specifier`
/// from `importer`, when that module is a tt source the session serves
/// under its lowered name ([`crate::engine::projection::module_path_of`]):
/// TypeScript writes the lowered `shapes.tt.ts` as `./shapes.tt`,
/// `./shapes.tt.ts` (with `allowImportingTsExtensions`), or `./shapes.tt.js`
/// (a `.js` ending, as `nodenext` requires), and tt imports the source as
/// `./shapes.tt` in every case, as import-path completion offers it
/// (TASK-609). `None` for any other specifier, already tt's included.
pub(super) fn tt_module_specifier(
    session: &ServiceSession,
    importer: &Path,
    specifier: &str,
) -> Option<String> {
    if !(specifier.starts_with("./") || specifier.starts_with("../")) {
        return None;
    }
    let directory = importer.parent()?;
    [".ts", ".tsx", ".js", ".jsx"]
        .into_iter()
        .find_map(|ending| {
            let written = specifier.strip_suffix(ending)?;
            let source = crate::engine::normalize_document_path(&directory.join(written)).ok()?;
            let kind = crate::SourceKind::from_tt_path(&source)?;
            let lowered = format!(".{}", kind.output_extension());
            let javascript: &[&str] = if lowered == ".tsx" {
                &[".js", ".jsx"]
            } else {
                &[".js"]
            };
            let served = session.served.contains_key(&source) || source.is_file();
            (served && (ending == lowered || javascript.contains(&ending)))
                .then(|| written.to_string())
        })
}

/// `text` (TypeScript an edit inserts) with each string literal naming a
/// served tt module in TypeScript's form written in tt's
/// ([`tt_module_specifier`]).
pub(super) fn tt_specifiers_in(session: &ServiceSession, importer: &Path, text: &str) -> String {
    let tokens = crate::lexer::lex(text, 0, text.len());
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    for token in &tokens {
        if !matches!(token.kind, crate::lexer::TokenKind::Str)
            || token.span.end - token.span.start < 2
        {
            continue;
        }
        let (start, end) = (token.span.start + 1, token.span.end - 1);
        if let Some(specifier) = tt_module_specifier(session, importer, &text[start..end]) {
            out.push_str(&text[copied..start]);
            out.push_str(&specifier);
            copied = end;
        }
    }
    out.push_str(&text[copied..]);
    out
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

impl Discriminant {
    pub(super) fn label(&self) -> &str {
        &self.label
    }
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
    wildcard: bool,
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
    if wildcard {
        out.push(crate::engine::completions::wildcard());
    }
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
