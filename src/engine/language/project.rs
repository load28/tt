//! Project-facing language service operations.

use super::*;

impl Project {
    /// Hover at a position: the checker's answer first, tt's own match
    /// analysis second.
    ///
    /// The service answers everything it can see — which is everything the
    /// emit-map ties to the source. What it cannot see is a pattern binding
    /// inside an or-pattern (`A(x) | B(x)`): the emitted destructuring
    /// speaks for every alternative at once, so those spans map to nothing
    /// (mapping them to one alternative would let a rename rewrite that one
    /// alone). For those, [`crate::pattern_analyses`] knows the span and the
    /// alternative it belongs to, and the answer is still the checker's
    /// wherever possible: the alternative is *isolated* — the same
    /// serve-a-stand-in move as the completion probe — so the service sees
    /// a single-alternative pattern narrowed to that constructor, payload
    /// types instantiated and all. Only when the checker cannot be asked at
    /// all does the analysis' declared type answer (`Ok(None)` when it too
    /// knows nothing).
    pub fn hover(&mut self, path: &Path, position: Position) -> Result<Option<HoverInfo>, String> {
        let (doc, path) = match self.serve(path) {
            Ok(served) => served,
            // No toolchain to ask: tt's own declaration table still answers
            // pattern bindings, instead of failing hover outright.
            Err(error) => {
                return match self.declared_hover_unserved(path, position) {
                    Some(info) => Ok(Some(info)),
                    None => Err(error),
                };
            }
        };
        if let Some(info) = self.service_hover(&doc, &path, position)? {
            return Ok(Some(info));
        }
        self.match_binding_hover(&doc, &path, position)
    }

    /// The plain service hover: the signature TypeScript shows, mapped onto
    /// the `.tt` source. `Ok(None)` when there is nothing to show.
    fn service_hover(
        &mut self,
        doc: &Arc<ServiceDoc>,
        path: &Path,
        position: Position,
    ) -> Result<Option<HoverInfo>, String> {
        let doc = doc.clone();
        let path = path.to_path_buf();
        let session = self.session();
        let Some(at) = to_service(&doc, position) else {
            return Ok(None);
        };
        let uri = served_uri(&path);
        let hover = session.client.request(
            "textDocument/hover",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": lsp_position(u16_position(&doc.code, at)),
            }),
        )?;
        let contents = match &hover["contents"] {
            serde_json::Value::String(s) => s.clone(),
            value => value["value"].as_str().unwrap_or_default().to_string(),
        };
        if contents.is_empty() {
            return Ok(None);
        }
        let (signature, documentation) = split_hover(&contents);
        if signature.is_empty() {
            return Ok(None);
        }
        let (start, end) = match hover.get("range").filter(|r| !r.is_null()) {
            Some(range) => (
                u16_offset(&doc.code, position_of(&range["start"])),
                u16_offset(&doc.code, position_of(&range["end"])),
            ),
            None => (at, at),
        };
        let Some((s, e)) = from_service_span(&doc, start, end) else {
            return Ok(None);
        };
        Ok(Some(HoverInfo {
            signature,
            documentation,
            range: source_range(&doc.source, s, e),
        }))
    }

    /// Hover answered from the match analysis, for positions the service
    /// could not see: a pattern binding span (isolate the alternative and
    /// ask the checker; fall back to the declared type), or a body
    /// reference of one (the merged declared type).
    fn match_binding_hover(
        &mut self,
        doc: &Arc<ServiceDoc>,
        path: &Path,
        position: Position,
    ) -> Result<Option<HoverInfo>, String> {
        let byte = source_byte(&doc.source, position);
        let semantics = self.semantic_analyses(path, &doc.source);
        let analyses = &semantics.analyses;
        if let Some(binding) = analyses.binding_at(byte) {
            let range = source_range(
                &doc.source,
                mapper::to_utf16(&doc.source, binding.start),
                mapper::to_utf16(&doc.source, binding.end),
            );
            if binding.alternatives > 1
                && let Some(info) = self.isolated_alternative_hover(doc, path, binding, byte, range)
            {
                return Ok(Some(info));
            }
            return Ok(declared_binding_hover(binding, range));
        }
        // A body reference the service had no answer for (the or-pattern
        // destructuring is glue): the merged declared type.
        if let Some((binding, (start, end))) = analyses.body_binding_at(&doc.source, byte)
            && let Some(ty) = &binding.ty
        {
            return Ok(Some(HoverInfo {
                signature: format!("const {}: {}", binding.name, ty),
                documentation: String::new(),
                range: source_range(
                    &doc.source,
                    mapper::to_utf16(&doc.source, start),
                    mapper::to_utf16(&doc.source, end),
                ),
            }));
        }
        Ok(None)
    }

    /// Asks the service about one or-pattern alternative in isolation: the
    /// buffer with the alternative list replaced by this alternative alone
    /// is emitted and served in the document's stead — the completion
    /// probe's move — so the emitted destructuring is single-alternative,
    /// mapped, and narrowed to this constructor. The checker's answer is
    /// then the alternative's own payload type. `None` (never an error —
    /// the declared type still stands behind it) when the probe cannot be
    /// built or the service has nothing to say.
    fn isolated_alternative_hover(
        &mut self,
        doc: &Arc<ServiceDoc>,
        path: &Path,
        binding: &crate::PatternBinding,
        byte: usize,
        range: Range,
    ) -> Option<HoverInfo> {
        let (code, offset) = isolate_alternative(&doc.source, binding, byte)?;

        let uri = served_uri(path);
        let session = self.session();
        session.client.open(&uri, &code);
        let answer = session.client.request(
            "textDocument/hover",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": lsp_position(u16_position(&code, offset)),
            }),
        );
        // The stand-in answered one question; the real projection is served
        // back before the answer is even read.
        session.client.open(&uri, &doc.code);
        session.served.insert(path.to_path_buf(), doc.code.clone());

        let hover = answer.ok()?;
        let contents = match &hover["contents"] {
            serde_json::Value::String(s) => s.clone(),
            value => value["value"].as_str().unwrap_or_default().to_string(),
        };
        let (signature, documentation) = split_hover(&contents);
        if signature.is_empty() {
            return None;
        }
        Some(HoverInfo {
            signature,
            documentation,
            // The span is the binding the user is looking at, not the
            // probe's — the probe was never their text.
            range,
        })
    }

    /// The declaration-table hover for a session whose toolchain could not
    /// be reached: the file is read as the editor sees it (overlay first)
    /// and only tt's own analysis answers.
    fn declared_hover_unserved(&mut self, path: &Path, position: Position) -> Option<HoverInfo> {
        let canonical = path.canonicalize().ok()?;
        let source = match self.overlays.get(&canonical) {
            Some(text) => text.clone(),
            None => std::fs::read_to_string(&canonical).ok()?,
        };
        let byte = source_byte(&source, position);
        let semantics = self.semantic_analyses(&canonical, &source);
        let binding = semantics.analyses.binding_at(byte)?;
        let range = source_range(
            &source,
            mapper::to_utf16(&source, binding.start),
            mapper::to_utf16(&source, binding.end),
        );
        declared_binding_hover(binding, range)
    }

    /// Go to definition, every target already in its own file's coordinates.
    pub fn definition(&mut self, path: &Path, position: Position) -> Result<Vec<Location>, String> {
        let found = self.locations(
            path,
            position,
            "textDocument/definition",
            serde_json::json!({}),
        )?;
        if !found.is_empty() {
            return Ok(found);
        }
        // The service found nothing — for a name an or-pattern binds, the
        // target it resolved to is compiler glue, which navigation drops.
        // The match analysis knows the spans the user actually wrote: a
        // body reference goes to every alternative's binding; a binding is
        // its own declaration.
        self.match_binding_definitions(path, position)
    }

    /// Definition targets from the match analysis — the fallback for names
    /// the emitted glue owns. Empty when the position is not on one.
    fn match_binding_definitions(
        &mut self,
        path: &Path,
        position: Position,
    ) -> Result<Vec<Location>, String> {
        let (doc, path) = self.serve(path)?;
        let byte = source_byte(&doc.source, position);
        let semantics = self.semantic_analyses(&path, &doc.source);
        let analyses = &semantics.analyses;
        let spans = match analyses.binding_at(byte) {
            Some(binding) => vec![(binding.start, binding.end)],
            None => analyses.body_definitions(&doc.source, byte),
        };
        Ok(spans
            .into_iter()
            .map(|(start, end)| Location {
                path: path.clone(),
                range: source_range(
                    &doc.source,
                    mapper::to_utf16(&doc.source, start),
                    mapper::to_utf16(&doc.source, end),
                ),
            })
            .collect())
    }

    /// Find references. `is_definition` marks the first result, as the
    /// editor has always presented it.
    pub fn references(
        &mut self,
        path: &Path,
        position: Position,
    ) -> Result<Vec<Reference>, String> {
        let locations = self.locations(
            path,
            position,
            "textDocument/references",
            serde_json::json!({ "context": { "includeDeclaration": true } }),
        )?;
        Ok(locations
            .into_iter()
            .enumerate()
            .map(|(index, location)| Reference {
                location,
                is_definition: index == 0,
            })
            .collect())
    }

    fn locations(
        &mut self,
        path: &Path,
        position: Position,
        method: &str,
        extra: serde_json::Value,
    ) -> Result<Vec<Location>, String> {
        let (doc, path) = self.serve(path)?;
        let Project {
            service, overlays, ..
        } = self;
        let session = service.as_mut().expect("serve started it");
        let Some(at) = to_service(&doc, position) else {
            return Ok(Vec::new());
        };
        let mut params = serde_json::json!({
            "textDocument": { "uri": served_uri(&path) },
            "position": lsp_position(u16_position(&doc.code, at)),
        });
        if let (Some(into), Some(from)) = (params.as_object_mut(), extra.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        let answer = session.client.request(method, params)?;
        let raw: Vec<serde_json::Value> = match answer {
            serde_json::Value::Array(items) => items,
            serde_json::Value::Null => Vec::new(),
            one => vec![one],
        };
        let mut out = Vec::new();
        for location in raw {
            let Some(uri) = location["uri"].as_str() else {
                continue;
            };
            // Anything unmappable is dropped — a reference into glue is not
            // a place the user can go.
            if let Some(mapped) = map_target(session, overlays, uri, &location["range"]) {
                out.push(mapped);
            }
        }
        Ok(out)
    }

    /// Completions at a position. `member` says whether the *source* cursor
    /// sits at a member access (the adapter knows, from the tt syntax layer)
    /// — at a member access only a member answer means anything, and when
    /// the plain answer is not one, a probe mends the unfinished construct
    /// and asks again.
    pub fn completion(
        &mut self,
        path: &Path,
        position: Position,
        member: bool,
    ) -> Result<CompletionAnswer, String> {
        let (doc, path) = self.serve(path)?;
        let session = self.session();
        let plain = match to_service(&doc, position) {
            Some(at) => ts_completions(session, &path, at, &doc.code)?,
            None => CompletionAnswer::default(),
        };
        if !member {
            return Ok(plain);
        }
        if plain.member && !plain.items.is_empty() {
            return Ok(plain);
        }

        // The construct is unfinished (`x |> .`): splice the placeholder in,
        // emit, and ask at its mapped position. The probe stands in for the
        // buffer only for this question — the next serve restores the real
        // text — and no diagnostic is ever computed from it.
        let source_at = {
            let u16 = u16_offset(&doc.source, position);
            mapper::from_utf16(&doc.source, u16)
        };
        let Some(probe) = build_probe(&path, &doc.source, source_at, session.probe_count + 1)
        else {
            return Ok(if plain.member {
                plain
            } else {
                CompletionAnswer::default()
            });
        };
        session.probe_count += 1;
        session.client.open(&served_uri(&path), &probe.code);
        session.served.insert(path.clone(), probe.code.clone());
        let mut probed = ts_completions(session, &path, probe.offset, &probe.code)?;
        probed.probe = Some(probe.version);
        session.last_probe = Some(probe);
        Ok(if probed.member {
            probed
        } else {
            CompletionAnswer::default()
        })
    }

    /// The signature and documentation behind one completion entry, fetched
    /// when the consumer asks about the one entry the user is looking at.
    /// `probe` re-installs the probed text the entry was listed from;
    /// `Ok(None)` when that probe is gone (the buffer has moved on) or the
    /// entry cannot be resolved.
    pub fn completion_resolve(
        &mut self,
        path: &Path,
        position: Position,
        label: &str,
        probe: Option<u64>,
    ) -> Result<Option<CompletionDetail>, String> {
        let (doc, path) = self.serve(path)?;
        let session = self.session();
        let at = match probe {
            Some(version) => {
                let Some(installed) = session
                    .last_probe
                    .clone()
                    .filter(|p| p.version == version && p.path == path)
                else {
                    return Ok(None);
                };
                session.client.open(&served_uri(&path), &installed.code);
                session.served.insert(path.clone(), installed.code.clone());
                installed.offset
            }
            None => match to_service(&doc, position) {
                Some(at) => at,
                None => return Ok(None),
            },
        };
        let code = session
            .served
            .get(&path)
            .cloned()
            .unwrap_or_else(|| doc.code.clone());

        let key = (path.clone(), at, label.to_string());
        if !session.last_completion.contains_key(&key) {
            // The server resolves the item *it* produced, not a name, so the
            // list has to have been asked for first.
            let _ = ts_completions(session, &path, at, &code)?;
        }
        let Some(item) = session.last_completion.get(&key).cloned() else {
            return Ok(None);
        };
        let resolved = session.client.request("completionItem/resolve", item)?;
        if resolved.is_null() {
            return Ok(None);
        }
        Ok(Some(CompletionDetail {
            signature: resolved["detail"].as_str().unwrap_or_default().to_string(),
            documentation: docs_text(&resolved["documentation"]),
        }))
    }

    /// Rename: every edit, each mapped to the file the user can open — or
    /// `Ok(None)` when the rename cannot be done *whole*. An edit that lands
    /// in compiler-written glue, a target that is not a file, or an edit
    /// shape that cannot be accounted for refuses the entire rename: a
    /// program renamed by halves is corrupted, not renamed.
    pub fn rename(
        &mut self,
        path: &Path,
        position: Position,
    ) -> Result<Option<Vec<RenameEdit>>, String> {
        let (doc, path) = self.serve(path)?;
        let Project {
            service, overlays, ..
        } = self;
        let session = service.as_mut().expect("serve started it");
        let Some(at) = to_service(&doc, position) else {
            return Ok(None);
        };
        let uri = served_uri(&path);
        let lsp_at = lsp_position(u16_position(&doc.code, at));

        // The server's own "can this be renamed?" — a keyword or a literal
        // answers null, and forcing it would rename nothing while looking
        // like it worked.
        let prepared = session.client.request(
            "textDocument/prepareRename",
            serde_json::json!({ "textDocument": { "uri": uri }, "position": lsp_at }),
        )?;
        if prepared.is_null() {
            return Ok(None);
        }

        let edit = session.client.request(
            "textDocument/rename",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": lsp_at,
                "newName": RENAME_PLACEHOLDER,
            }),
        )?;
        let Some(changes) = edit["changes"].as_object() else {
            return Ok(None);
        };

        let changes = changes.clone();
        let mut out = Vec::new();
        for (edited_uri, edits) in &changes {
            let Some(edits) = edits.as_array() else {
                return Ok(None);
            };
            for one in edits {
                let Some(location) = map_target(session, overlays, edited_uri, &one["range"])
                else {
                    return Ok(None);
                };
                let new_text = one["newText"].as_str().map(String::from);
                if let Some(text) = &new_text
                    && text != RENAME_PLACEHOLDER
                    && !text.contains(RENAME_PLACEHOLDER)
                {
                    // A shape we cannot account for — refusing beats
                    // silently rebinding a different field.
                    return Ok(None);
                }
                out.push(RenameEdit { location, new_text });
            }
        }
        Ok(if out.is_empty() { None } else { Some(out) })
    }

    /// Signature help at a call site.
    pub fn signature_help(
        &mut self,
        path: &Path,
        position: Position,
    ) -> Result<Option<SignatureHelp>, String> {
        let (doc, path) = self.serve(path)?;
        let session = self.session();
        let Some(at) = to_service(&doc, position) else {
            return Ok(None);
        };
        let help = session.client.request(
            "textDocument/signatureHelp",
            serde_json::json!({
                "textDocument": { "uri": served_uri(&path) },
                "position": lsp_position(u16_position(&doc.code, at)),
            }),
        )?;
        let Some(signatures) = help["signatures"].as_array().filter(|s| !s.is_empty()) else {
            return Ok(None);
        };
        Ok(Some(SignatureHelp {
            signatures: signatures
                .iter()
                .map(|signature| {
                    let label = signature["label"].as_str().unwrap_or_default().to_string();
                    Signature {
                        parameters: signature["parameters"]
                            .as_array()
                            .map(|parameters| {
                                parameters
                                    .iter()
                                    .map(|parameter| SignatureParameter {
                                        label: parameter_span(&label, &parameter["label"]),
                                        documentation: docs_text(&parameter["documentation"]),
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                        documentation: docs_text(&signature["documentation"]),
                        label,
                    }
                })
                .collect(),
            active_signature: help["activeSignature"].as_u64().unwrap_or(0) as u32,
            active_parameter: help["activeParameter"].as_u64().unwrap_or(0) as u32,
        }))
    }

    /// TypeScript's type errors for one file, mapped onto its `.tt` source.
    /// Exact source spans are reported as-is. Diagnostics that land in
    /// compiler-written glue use their lowering anchor's primary source
    /// span, matching the batch typed-check path.
    pub fn service_diagnostics(&mut self, path: &Path) -> Result<Vec<ServiceDiagnostic>, String> {
        let (doc, path) = self.serve(path)?;
        // An unhandled projection failure still gets the old raw emit-map
        // fallback. Do not trust TypeScript's recovery from that malformed
        // document; parser-owned recoveries have already made ordinary edit
        // states valid before this point.
        if !projection_accepts_diagnostics(
            &doc.code,
            crate::SourceKind::from_path(&path).unwrap_or_default(),
        ) {
            return Ok(Vec::new());
        }
        let session = self.session();
        let answer = session.client.request(
            "textDocument/diagnostic",
            serde_json::json!({ "textDocument": { "uri": served_uri(&path) } }),
        )?;
        let items = answer["items"].as_array().cloned().unwrap_or_default();
        let mut out = Vec::new();
        // The declaration table a translated message names its types from,
        // built on the first translation of this pass: most passes
        // translate nothing, and building it parses the file and its
        // imports.
        let mut declarations: Option<Vec<crate::analysis::DeclaredVariant>> = None;
        let mut translated_seen: HashSet<(usize, crate::AnchorKind, &'static str)> = HashSet::new();
        for item in items {
            let severity = item["severity"].as_u64().unwrap_or(1);
            if severity > 2 {
                continue;
            }
            let start = u16_offset(&doc.code, position_of(&item["range"]["start"]));
            let end = u16_offset(&doc.code, position_of(&item["range"]["end"]));
            let Some((s, e, origin)) = diagnostic_source_span(&doc, start, end) else {
                continue;
            };
            if recovery_intersects(&doc, s, e) {
                continue;
            }
            if projection::origin_intersects_tt_error(origin, &doc.tt_diagnostics) {
                continue;
            }
            let (exact, projected_anchor) = match origin {
                mapper::DiagnosticOrigin::Exact { .. } => (true, None),
                mapper::DiagnosticOrigin::Anchor(anchor) => (false, Some(anchor)),
                mapper::DiagnosticOrigin::Nearest { .. } => (false, None),
            };
            // An empty span (an error at a position, not over one) would
            // render as an invisible squiggle; give it the character it
            // points at.
            let e = if e > s { e } else { s + 1 };
            let raw = item["message"].as_str().unwrap_or_default().to_string();
            let code = item["code"].as_u64().unwrap_or(0) as u32;
            let glue = projected_anchor.or_else(|| glue_anchor(&doc, start));
            // The diagnostic's secondary places: the pipeline anchor's
            // producing step, then the checker's own related information —
            // the same two producers the CLI report attaches
            // (`semantics::checker_labels`). Same-file spans only: a place
            // in another module has no projection at hand here.
            let mut related: Vec<ServiceRelated> = Vec::new();
            if let Some(anchor) = glue
                && anchor.kind == crate::AnchorKind::Pipe
                && let Some((context_start, context_end)) = anchor.context
            {
                let from = mapper::to_utf16(&doc.source, context_start);
                let to = mapper::to_utf16(&doc.source, context_end).max(from + 1);
                related.push(ServiceRelated {
                    path: None,
                    range: source_range(&doc.source, from, to),
                    message: "the piped value is produced here".to_string(),
                });
            }
            // The tsgo preview omits `relatedInformation` from pull
            // diagnostics today; when it starts sending it, these entries
            // become labels with no further work here.
            let served = served_uri(&path);
            for entry in item["relatedInformation"].as_array().into_iter().flatten() {
                if entry["location"]["uri"].as_str() != Some(served.as_str()) {
                    continue;
                }
                let from = u16_offset(&doc.code, position_of(&entry["location"]["range"]["start"]));
                let to = u16_offset(&doc.code, position_of(&entry["location"]["range"]["end"]));
                let Some((from, to, _)) = diagnostic_source_span(&doc, from, to) else {
                    continue;
                };
                let to = if to > from { to } else { from + 1 };
                related.push(ServiceRelated {
                    path: None,
                    range: source_range(&doc.source, from, to),
                    message: entry["message"].as_str().unwrap_or_default().to_string(),
                });
                if related.len() >= 3 {
                    break;
                }
            }
            // The whole-pipeline anchor shares its kind with the step
            // anchors but not their meaning: only a step anchor (one
            // carrying a producer context) may speak in step vocabulary —
            // the same gate the CLI report applies.
            let translates = |anchor: &crate::EmitAnchor| {
                anchor.kind != crate::AnchorKind::Pipe || anchor.context.is_some()
            };
            if let Some((anchor, class)) = glue.filter(translates).and_then(|anchor| {
                crate::engine::semantics::translation_class(anchor.kind, code)
                    .map(|class| (anchor, class))
            }) && !translated_seen.insert((anchor.src, anchor.kind, class))
            {
                continue;
            }
            // On glue the construct is the diagnostic's extent: its own
            // text is underlined, and where ttc can say what the construct
            // meant it says that instead — the same table the CLI reports
            // through, so the two surfaces cannot drift.
            if !exact && let Some(anchor) = glue {
                let from = mapper::to_utf16(&doc.source, anchor.src);
                let to = mapper::to_utf16(&doc.source, anchor.src_end).max(from + 1);
                let range = source_range(&doc.source, from, to);
                let declared = declarations.get_or_insert_with(|| {
                    self.semantic_analyses(&path, &doc.source)
                        .analyses
                        .declarations
                        .clone()
                });
                let translated = translates(&anchor)
                    .then(|| crate::engine::semantics::translate(anchor.kind, code, &raw, declared))
                    .flatten();
                let entry = match translated {
                    Some(said) => ServiceDiagnostic {
                        range,
                        message: said,
                        code,
                        warning: severity == 2,
                        related: related.clone(),
                    },
                    None => ServiceDiagnostic {
                        range,
                        message: format!("{raw} (in code ttc generated for this construct)"),
                        code,
                        warning: severity == 2,
                        related: related.clone(),
                    },
                };
                // One construct's glue can draw several TypeScript errors
                // that all mean the same tt thing.
                if !out.contains(&entry) {
                    out.push(entry);
                }
                continue;
            }
            let declared = declarations.get_or_insert_with(|| {
                self.semantic_analyses(&path, &doc.source)
                    .analyses
                    .declarations
                    .clone()
            });
            let mut message = match crate::engine::semantics::name_types(&raw, declared) {
                Some(named) => format!("{raw} (in tt's names: {named})"),
                None => raw,
            };
            if !exact {
                message.push_str(" (in code ttc generated for this construct)");
            }
            out.push(ServiceDiagnostic {
                range: source_range(&doc.source, s, e),
                message,
                code,
                warning: severity == 2,
                related,
            });
        }
        Ok(out)
    }

    /// Starts (or reuses) the service session and serves `path` and its
    /// transitive `.tt` imports as the TypeScript they lower to. Returns the
    /// file's projection and its canonical path; the session is then live.
    fn serve(&mut self, path: &Path) -> Result<(Arc<ServiceDoc>, PathBuf), String> {
        let canonical = path
            .canonicalize()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if !self.service.as_ref().is_some_and(|s| s.client.alive()) {
            // (Re)start: the previous conversation, if any, is gone — served
            // state with it. The next questions rebuild both.
            //
            // Both of tt's own packages go in now, before the service can
            // resolve anything. Which of them a *file* needs is not a
            // property this layer may wait on: a probe mends a buffer the
            // user is in the middle of typing, and the mended text can use
            // a pipeline the unparseable original did not (TASK-217).
            ensure_std_module(&self.root);
            ensure_runtime_module(&self.root);
            let binary = service_binary(&self.root)?;
            let client = Service::start(&binary, &self.root)?;
            self.service = Some(ServiceSession {
                client,
                served: HashMap::new(),
                host_served: HashMap::new(),
                docs: HashMap::new(),
                last_completion: HashMap::new(),
                last_probe: None,
                probe_count: 0,
            });
        }
        let Project {
            service, overlays, ..
        } = self;
        let session = service.as_mut().expect("just ensured");

        let closed: Vec<_> = session
            .host_served
            .keys()
            .filter(|path| !overlays.contains_key(*path))
            .cloned()
            .collect();
        for path in closed {
            session.client.close(&file_uri(&path));
            session.host_served.remove(&path);
        }
        for (path, text) in overlays
            .iter()
            .filter(|(path, _)| super::super::project::is_host_source(path))
        {
            if session.host_served.get(path) != Some(text) {
                session.client.open(&file_uri(path), text);
                session.host_served.insert(path.clone(), text.clone());
            }
        }

        let doc = serve_one(session, overlays, &canonical)
            .ok_or_else(|| format!("cannot read {}", canonical.display()))?;

        // The `.tt` modules it imports are served too, transitively. That is
        // not an optimization: the server resolves `"./x.tt"` to `x.tt.ts`,
        // and that name only exists as a document *this session serves*.
        let mut seen: HashSet<PathBuf> = HashSet::from([canonical.clone()]);
        let mut stack = vec![(canonical.clone(), doc.clone())];
        while let Some((file, doc)) = stack.pop() {
            for import in crate::tt_imports(&doc.source) {
                let target = match file
                    .parent()
                    .unwrap_or(Path::new("."))
                    .join(&import.specifier)
                    .canonicalize()
                {
                    Ok(target) => target,
                    Err(_) => continue, // unresolvable — tsc's TS2307, not ours
                };
                if !seen.insert(target.clone()) {
                    continue;
                }
                if let Some(imported) = serve_one(session, overlays, &target) {
                    stack.push((target, imported));
                }
            }
        }
        Ok((doc, canonical))
    }

    /// The live session, after [`Project::serve`] has run.
    fn session(&mut self) -> &mut ServiceSession {
        self.service.as_mut().expect("serve started it")
    }
}
