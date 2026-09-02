//! Service projections, coordinate mapping, and TypeScript response conversion.

use super::*;

pub(super) fn projection_accepts_diagnostics(code: &str, source_kind: crate::SourceKind) -> bool {
    crate::verify::verify_output(code, source_kind).is_ok()
}

pub(super) fn service_doc(path: &Path, text: String) -> ServiceDoc {
    let options = crate::Options {
        filename: Some(path.to_str().unwrap_or("<input>")),
        source_kind: crate::SourceKind::from_path(path).unwrap_or_default(),
        defer_to_checker: true,
        rewrite_imports: crate::ImportRewrite::Off,
        ..crate::Options::default()
    };
    let report = crate::compile_projection_report(&text, &options);
    let (emit, recovered) = match report.emit {
        Some(emit) => (emit, report.recovered),
        None => (
            crate::emit_mapped_with_kind(
                &text,
                crate::SourceKind::from_path(path).unwrap_or_default(),
            ),
            Vec::new(),
        ),
    };
    ServiceDoc {
        source: text,
        code: emit.code,
        mappings: emit.mappings,
        anchors: emit.anchors,
        recovered,
        tt_diagnostics: report.diagnostics,
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
    if session.served.get(path) != Some(&doc.code) {
        session.client.open(&served_uri(path), &doc.code);
        session.served.insert(path.to_path_buf(), doc.code.clone());
    }
    Some(doc)
}

/// The URI an `.tt` file is served under: the lowered module's name, which
/// is what an `import "./x.tt"` resolves to.
pub(super) fn served_uri(path: &Path) -> String {
    file_uri(&module_path_of(path))
}

/// Completions at a service offset, with the raw items cached for resolve.
pub(super) fn ts_completions(
    session: &mut ServiceSession,
    path: &Path,
    at: usize,
    code: &str,
) -> Result<CompletionAnswer, String> {
    let answer = session.client.request(
        "textDocument/completion",
        serde_json::json!({
            "textDocument": { "uri": served_uri(path) },
            "position": lsp_position(u16_position(code, at)),
        }),
    )?;
    let items: Vec<serde_json::Value> = match answer {
        serde_json::Value::Array(items) => items,
        value => value["items"].as_array().cloned().unwrap_or_default(),
    };
    session.last_completion.clear();
    let mut entries = Vec::with_capacity(items.len());
    for item in items {
        let label = item["label"].as_str().unwrap_or_default().to_string();
        session
            .last_completion
            .insert((path.to_path_buf(), at, label.clone()), item.clone());
        entries.push(CompletionItem {
            kind: completion_kind(item["kind"].as_u64()),
            sort_text: item["sortText"].as_str().unwrap_or(&label).to_string(),
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

/// The source byte a position names — the analysis speaks bytes, the
/// protocol UTF-16.
pub(in super::super) fn source_byte(source: &str, position: Position) -> usize {
    mapper::from_utf16(source, u16_offset(source, position))
}

/// The match analysis of one file as a stand-alone question: imported
/// declarations are the CLI's 1-hop collection, read from disk, since no
/// project session is involved. This is what the parse-only surfaces
/// ([`super::names`], [`super::hints`], [`super::completions`]) ask; a
/// surface with a [`Project`] asks [`Project::semantic_analyses`] instead
/// and shares the typed pass's cross-snapshot cache.
pub(in super::super) fn analyses_for(path: &Path, source: &str) -> crate::PatternAnalyses {
    let externs = externs_of(path, source, &|target| std::fs::read_to_string(target).ok());
    crate::pattern_analyses(source, &externs)
}

/// A byte span of `text` as a [`Range`] — the byte↔UTF-16 conversion every
/// answer crosses on its way out.
pub(in super::super) fn span_range(text: &str, start: usize, end: usize) -> Range {
    source_range(
        text,
        mapper::to_utf16(text, start),
        mapper::to_utf16(text, end),
    )
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
            Some(
                crate::variant_symbols_with_kind(
                    &text,
                    crate::SourceKind::from_path(target).unwrap_or_default(),
                )
                .into_iter()
                .filter(|d| d.exported)
                .collect(),
            )
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
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut externs: Vec<crate::VariantSymbol> = Vec::new();
    for import in imports {
        if matches!(import.names, crate::TtImportNames::None) {
            continue; // a re-export brings nothing into scope
        }
        let target = match dir.join(&import.specifier).canonicalize() {
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
                    d
                }));
            }
            crate::TtImportNames::Named(entries) => {
                for (name, alias) in entries {
                    if let Some(d) = decls.iter().find(|d| &d.name == name) {
                        let mut d = d.clone();
                        d.name = alias.clone().unwrap_or_else(|| name.clone());
                        externs.push(d);
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
    let emit = crate::emit_mapped(&synthetic);
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
/// `at` (a byte offset), emitted, and the placeholder's mapped position.
/// `None` when the buffer is broken somewhere a placeholder does not reach.
pub(super) fn build_probe(path: &Path, source: &str, at: usize, version: u64) -> Option<ProbeDoc> {
    if !source.is_char_boundary(at) {
        return None;
    }
    let spliced = format!("{}{}{}", &source[..at], PROBE_NAME, &source[at..]);
    let emit = crate::emit_mapped(&spliced);
    let out = mapper::to_output_inclusive(&emit.mappings, at)?;
    Some(ProbeDoc {
        path: path.to_path_buf(),
        offset: mapper::to_utf16(&emit.code, out),
        code: emit.code,
        version,
    })
}

/// Maps one service answer target back to a user-visible file. `None` when
/// the target is not a file, cannot be read, or the span has no source
/// counterpart — the caller decides whether that skips one result
/// (navigation) or refuses the whole operation (rename).
pub(super) fn map_target(
    session: &mut ServiceSession,
    overlays: &HashMap<PathBuf, String>,
    uri: &str,
    range: &serde_json::Value,
) -> Option<Location> {
    let path = uri_path(uri)?;
    let lsp_range = Range {
        start: position_of(&range["start"]),
        end: position_of(&range["end"]),
    };
    let name = path.to_string_lossy();
    let source_name = name
        .strip_suffix(".tsx")
        .filter(|n| n.ends_with(".ttx"))
        .or_else(|| name.strip_suffix(".ts").filter(|n| n.ends_with(".tt")));
    if let Some(tt) = source_name {
        let tt_path = PathBuf::from(tt);
        let doc = serve_doc_only(session, overlays, &tt_path)?;
        let start = u16_offset(&doc.code, lsp_range.start);
        let end = u16_offset(&doc.code, lsp_range.end);
        let (s, e) = from_service_span(&doc, start, end)?;
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

/// A tt position translated into the served text, or `None` when it sits
/// in compiler-written glue.
pub(super) fn to_service(doc: &ServiceDoc, position: Position) -> Option<usize> {
    let u16 = u16_offset(&doc.source, position);
    let byte = mapper::from_utf16(&doc.source, u16);
    let out = mapper::to_output_inclusive(&doc.mappings, byte)?;
    Some(mapper::to_utf16(&doc.code, out))
}

/// A service span translated back to source UTF-16 offsets, or `None` when
/// either end has no source counterpart.
pub(super) fn from_service_span(
    doc: &ServiceDoc,
    start: usize,
    end: usize,
) -> Option<(usize, usize)> {
    let sb = mapper::from_utf16(&doc.code, start);
    let eb = mapper::from_utf16(&doc.code, end);
    let ss = mapper::to_source_inclusive(&doc.mappings, sb)?;
    let se = mapper::to_source_inclusive(&doc.mappings, eb)?;
    if se < ss {
        return None;
    }
    Some((
        mapper::to_utf16(&doc.source, ss),
        mapper::to_utf16(&doc.source, se),
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
    let origin = mapper::diagnostic_origin(&doc.mappings, &doc.anchors, sb, eb)?;
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
/// convention: a character past the line's end spills forward, and both
/// clamp to the text's end.
pub(crate) fn u16_offset(text: &str, position: Position) -> usize {
    let mut line = 0u32;
    let mut u16 = 0usize;
    let mut line_start = 0usize;
    if position.line > 0 {
        for ch in text.chars() {
            u16 += ch.len_utf16();
            if ch == '\n' {
                line += 1;
                line_start = u16;
                if line == position.line {
                    break;
                }
            }
        }
        if line < position.line {
            return text.encode_utf16().count();
        }
    }
    let total = text.encode_utf16().count();
    (line_start + position.character as usize).min(total)
}

/// The zero-based line/character a UTF-16 offset names in `text`.
pub(crate) fn u16_position(text: &str, offset: usize) -> Position {
    let mut u16 = 0usize;
    let mut line = 0u32;
    let mut line_start = 0usize;
    for ch in text.chars() {
        if u16 >= offset {
            break;
        }
        u16 += ch.len_utf16();
        if ch == '\n' {
            line += 1;
            line_start = u16;
        }
    }
    Position {
        line,
        character: (u16.min(offset).max(line_start) - line_start) as u32,
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

/// Splits hover contents into the signature and the prose under it: the
/// first fenced block is the signature, else the first paragraph.
pub(super) fn split_hover(contents: &str) -> (String, String) {
    let trimmed = contents.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        // ```lang\n ... \n``` and whatever prose follows.
        if let Some(newline) = rest.find('\n') {
            let body = &rest[newline + 1..];
            if let Some(close) = body.find("\n```") {
                let signature = body[..close].trim().to_string();
                let documentation = body[close + 4..].trim().to_string();
                return (signature, documentation);
            }
        }
    }
    match trimmed.split_once("\n\n") {
        Some((first, rest)) => (first.trim().to_string(), rest.trim().to_string()),
        None => (trimmed.to_string(), String::new()),
    }
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
    let pkg = root.join("node_modules/@tt/std");
    let entry = pkg.join(crate::StdModule::Types.file_name());
    if !entry.exists() && !pkg.exists() && std::fs::create_dir_all(&pkg).is_ok() {
        for module in crate::StdModule::STANDARD {
            let source = format!(
                "// @generated by ttc --emit-std — do not edit directly.\n{}",
                module.source()
            );
            let _ = std::fs::write(pkg.join(module.file_name()), source);
        }
        let _ = std::fs::write(
            pkg.join("package.json"),
            "{\n  \"name\": \"@tt/std\",\n  \"version\": \"0.0.0\",\n  \"types\": \"index.ts\"\n}\n",
        );
    }
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
    let runtime_pkg = root.join("node_modules/@tt/runtime");
    if !runtime_pkg.exists() && std::fs::create_dir_all(&runtime_pkg).is_ok() {
        let source = format!(
            "// @generated by ttc --emit-std — do not edit directly.\n{}",
            crate::RUNTIME_SOURCE
        );
        let _ = std::fs::write(runtime_pkg.join("index.ts"), source);
        let _ = std::fs::write(
            runtime_pkg.join("package.json"),
            "{\n  \"name\": \"@tt/runtime\",\n  \"version\": \"0.0.0\",\n  \"types\": \"index.ts\"\n}\n",
        );
    }
}
