//! Project-facing language service operations.

use super::*;

impl Project {
    /// Hover at a position: the checker's answer first, tt's own match
    /// analysis second.
    ///
    /// The service answers everything it can see — which is everything the
    /// emit-map ties to the source. What it cannot see is a pattern binding
    /// inside an or-pattern (`A(x) | B(x)`): the emitted destructuring
    /// speaks for every alternative at once, so no byte mapping ties it to
    /// one of them; navigation and rename reach it through its shared
    /// binding, which names every alternative together, but a hover wants
    /// the one alternative's type. For those, [`crate::pattern_analyses`] knows the span and the
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
        let uri = served_uri(session, &path);
        // A name ttc declares in glue (a variant, a case, a field) is asked
        // at each place the emission declares it, and the answer covers the
        // name as written.
        let declared = to_service(&doc, position).is_none();
        for at in to_service_names(&doc, position) {
            let hover = session.client.request(
                "textDocument/hover",
                serde_json::json!({
                    "textDocument": { "uri": uri },
                    "position": lsp_position(u16_position(&doc.code, at)),
                }),
            )?;
            let (signature, documentation) = split_hover(&hover["contents"]);
            if signature.is_empty() {
                continue;
            }
            let span = if declared {
                declared_name_at(&doc, position)
            } else {
                let (start, end) = match hover.get("range").filter(|r| !r.is_null()) {
                    Some(range) => (
                        u16_offset(&doc.code, position_of(&range["start"])),
                        u16_offset(&doc.code, position_of(&range["end"])),
                    ),
                    None => (at, at),
                };
                from_service_span(&doc, start, end)
            };
            let Some((s, e)) = span else {
                continue;
            };
            return Ok(Some(HoverInfo {
                signature,
                documentation,
                range: source_range(&doc.source, s, e),
            }));
        }
        Ok(None)
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
        let (code, offset) = isolate_alternative(path, &doc.source, binding, byte)?;

        let session = self.session();
        open_served(session, path, &code);
        let uri = served_uri(session, path);
        let answer = session.client.request(
            "textDocument/hover",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": lsp_position(u16_position(&code, offset)),
            }),
        );
        // The stand-in answered one question; the real projection is served
        // back before the answer is even read.
        open_served(session, path, &doc.code);

        let hover = answer.ok()?;
        let (signature, documentation) = split_hover(&hover["contents"]);
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
        let canonical = crate::engine::normalize_document_path(path).ok()?;
        let source = match self.overlays.read().get(&canonical) {
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
        let found = self.builtin_case_definition(path, position)?;
        if !found.is_empty() {
            return Ok(found);
        }
        // The service found nothing mappable. The match analysis knows the
        // spans the user actually wrote: a body reference goes to every
        // alternative's binding; a binding is its own declaration.
        self.match_binding_definitions(path, position)
    }

    fn builtin_case_definition(
        &mut self,
        path: &Path,
        position: Position,
    ) -> Result<Vec<Location>, String> {
        let (doc, path) = self.serve(path)?;
        let byte = source_byte(&doc.source, position);
        let semantics = self.semantic_analyses(&path, &doc.source);
        let Some(resolved) = semantics.analyses.resolved.iter().find(|resolved| {
            resolved.kind == crate::analysis::NameKind::Case
                && resolved.origin == crate::analysis::Origin::Builtin
                && resolved.start <= byte
                && byte <= resolved.end
        }) else {
            return Ok(Vec::new());
        };
        let Some(module) = crate::StdModule::constructing(&resolved.variant_name) else {
            return Ok(Vec::new());
        };
        let head = format!(
            "{}\ntype {PROBE_NAME} = typeof import(\"{}\").",
            doc.code,
            module.specifier()
        );
        let question = format!("{head}{};\n", resolved.name);
        let documents = self.overlays.clone();
        let overlays = &*documents.read();
        let session = self.session();
        open_served(session, &path, &question);
        let answer = session.client.request(
            "textDocument/definition",
            serde_json::json!({
                "textDocument": { "uri": served_uri(session, &path) },
                "position": lsp_position(u16_position(&question, mapper::to_utf16(&question, head.len()))),
            }),
        );
        open_served(session, &path, &doc.code);
        let targets = match answer? {
            serde_json::Value::Array(items) => items,
            serde_json::Value::Null => Vec::new(),
            one => vec![one],
        };
        Ok(targets
            .iter()
            .filter_map(|target| {
                let uri = target["uri"]
                    .as_str()
                    .or_else(|| target["targetUri"].as_str())?;
                let range = if target["targetSelectionRange"].is_object() {
                    &target["targetSelectionRange"]
                } else {
                    &target["range"]
                };
                map_target(session, overlays, uri, range, TargetUse::Navigation)
            })
            .filter(|location| location.path != path)
            .collect())
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

    /// Find references. `is_definition` marks each reference that is a
    /// definition the checker names for the same position.
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
        let definitions = self.locations(
            path,
            position,
            "textDocument/definition",
            serde_json::json!({}),
        )?;
        let mut references: Vec<Reference> = locations
            .into_iter()
            .map(|location| Reference {
                is_definition: definitions.contains(&location),
                location,
            })
            .collect();
        let Some(declaration) = self.tt_declaration(path, position, &definitions)? else {
            return Ok(references);
        };
        // A tt name's uses are of two kinds: the TypeScript ones the
        // emission declares it for (`Shape.Circle(1)`, `s: Shape`), asked at
        // the declaration, and the pattern ones only tt resolves.
        let mut found = vec![Reference {
            location: declaration.clone(),
            is_definition: true,
        }];
        for location in self.locations(
            &declaration.path,
            declaration.range.start,
            "textDocument/references",
            serde_json::json!({ "context": { "includeDeclaration": true } }),
        )? {
            found.push(Reference {
                is_definition: false,
                location,
            });
        }
        for file in self.tt_files()? {
            let Some(text) = self.text_of(&file) else {
                continue;
            };
            for range in crate::engine::names::tt_pattern_references(
                &file,
                &text,
                &declaration,
                Texts::Open(&self.overlays),
            ) {
                found.push(Reference {
                    is_definition: false,
                    location: Location {
                        path: file.clone(),
                        range,
                    },
                });
            }
        }
        for reference in found {
            match references
                .iter_mut()
                .find(|r| crate::engine::names::same_location(&r.location, &reference.location))
            {
                Some(known) => known.is_definition |= reference.is_definition,
                None => references.push(reference),
            }
        }
        Ok(references)
    }

    /// The tt declaration a name at `position` refers to: the name itself
    /// when tt resolves it (a declaration, a pattern tag or field), or the
    /// place TypeScript's `definitions` of it land when that place is a tt
    /// declaration (`Shape.Circle` in a `.ts` file lands on the case).
    fn tt_declaration(
        &mut self,
        path: &Path,
        position: Position,
        definitions: &[Location],
    ) -> Result<Option<Location>, String> {
        let is_tt = |path: &Path| crate::SourceKind::from_tt_path(path).is_some();
        if is_tt(path)
            && let Some(text) = self.text_of(path)
            && let Some(symbol) =
                crate::engine::names::symbol_at(path, &text, position, Texts::Open(&self.overlays))
        {
            return Ok(symbol.definition);
        }
        for definition in definitions {
            if !is_tt(&definition.path) {
                continue;
            }
            let Some(text) = self.text_of(&definition.path) else {
                continue;
            };
            if crate::engine::names::symbol_at(
                &definition.path,
                &text,
                definition.range.start,
                Texts::Open(&self.overlays),
            )
            .and_then(|symbol| symbol.definition)
            .is_some_and(|found| crate::engine::names::same_location(&found, definition))
            {
                return Ok(Some(definition.clone()));
            }
        }
        Ok(None)
    }

    /// The project's `.tt`/`.ttx` files, the documents opened through it
    /// included.
    fn tt_files(&self) -> Result<Vec<PathBuf>, String> {
        let mut files = self.scan().map_err(|error| error.to_string())?;
        files.extend(
            self.opened
                .iter()
                .filter(|path| crate::SourceKind::from_tt_path(path).is_some())
                .cloned(),
        );
        files.sort();
        files.dedup();
        Ok(files)
    }

    /// A file's text as the project sees it: the open buffer, else the disk.
    fn text_of(&self, path: &Path) -> Option<String> {
        self.overlays.text(path)
    }

    fn locations(
        &mut self,
        path: &Path,
        position: Position,
        method: &str,
        extra: serde_json::Value,
    ) -> Result<Vec<Location>, String> {
        let (doc, path) = self.serve(path)?;
        let documents = self.overlays.clone();
        let overlays = &*documents.read();
        let session = self.session();
        let mut raw: Vec<serde_json::Value> = Vec::new();
        for at in to_service_names(&doc, position) {
            let mut params = serde_json::json!({
                "textDocument": { "uri": served_uri(session, &path) },
                "position": lsp_position(u16_position(&doc.code, at)),
            });
            if let (Some(into), Some(from)) = (params.as_object_mut(), extra.as_object()) {
                for (key, value) in from {
                    into.insert(key.clone(), value.clone());
                }
            }
            match session.client.request(method, params)? {
                serde_json::Value::Array(items) => raw.extend(items),
                serde_json::Value::Null => {}
                one => raw.push(one),
            }
        }
        let mut out = Vec::new();
        for location in raw {
            let Some(uri) = location["uri"].as_str() else {
                continue;
            };
            // A shared binding stands for every alternative that writes it;
            // anything else unmappable is dropped — a reference into glue is
            // not a place the user can go.
            let mapped = match map_target(
                session,
                overlays,
                uri,
                &location["range"],
                TargetUse::Navigation,
            ) {
                Some(mapped) => vec![mapped],
                None => map_shared_target(session, overlays, uri, &location["range"])
                    .map(|(_, targets)| targets.into_iter().map(|t| t.location).collect())
                    .unwrap_or_default(),
            };
            for mapped in mapped {
                if !out.contains(&mapped) {
                    out.push(mapped);
                }
            }
        }
        Ok(out)
    }

    /// Completions at a position. `member` says whether the *source* cursor
    /// sits at a member access (the adapter knows, from the tt syntax layer)
    /// — at a member access only a member answer means anything, and when
    /// the plain answer is not one, a probe mends the unfinished construct
    /// and asks again. A cursor the served text has no place for (text of
    /// an unfinished construct the emission did not copy) is asked through
    /// a probe too.
    pub fn completion(
        &mut self,
        path: &Path,
        position: Position,
        member: bool,
    ) -> Result<CompletionAnswer, String> {
        let mut answer = self.service_completion(path, position, member)?;
        let (doc, path) = self.serve(path)?;
        let documents = self.overlays.clone();
        for entry in tt_module_entries(&path, &doc.source, position, &documents.read()) {
            if !answer.items.iter().any(|item| item.label == entry.label) {
                answer.items.push(entry);
            }
        }
        Ok(answer)
    }

    fn service_completion(
        &mut self,
        path: &Path,
        position: Position,
        member: bool,
    ) -> Result<CompletionAnswer, String> {
        let (doc, path) = self.serve(path)?;
        let session = self.session();
        let plain = match to_service_typed(&doc, position) {
            Some(at) => {
                let plain = ts_completions(session, &path, at, doc.served(), &doc.generated_names)?;
                if !member || (plain.member && !plain.items.is_empty()) {
                    return Ok(plain);
                }
                plain
            }
            None => CompletionAnswer::default(),
        };

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
        open_served(session, &path, &probe.code);
        let mut probed = ts_completions(
            session,
            &path,
            probe.offset,
            probe.served(),
            &probe.generated_names,
        )?;
        probed.probe = Some(probe.version);
        session.last_probe = Some(probe);
        Ok(if !member || probed.member {
            probed
        } else {
            CompletionAnswer::default()
        })
    }

    /// What can be written at a pattern position, typed by TypeScript where
    /// it can answer: `None` when `position` is not a pattern position.
    pub fn pattern_completions(
        &mut self,
        path: &Path,
        position: Position,
    ) -> Result<Option<Vec<crate::engine::TtCompletion>>, String> {
        use crate::engine::completions::{TypedSite, pattern_question};
        let (doc, path) = self.serve(path)?;
        let Some(question) =
            pattern_question(&path, &doc.source, position, Texts::Open(&self.overlays))
        else {
            return Ok(None);
        };
        let items = match question.typed {
            Some(TypedSite::Arm {
                prefix,
                family,
                covered,
                literals,
            }) => {
                let at =
                    prefix.map_or_else(|| source_byte(&doc.source, position), |(start, _)| start);
                let source = match prefix {
                    Some((start, end)) => format!("{}{}", &doc.source[..start], &doc.source[end..]),
                    None => doc.source.clone(),
                };
                match self.discriminant_candidates(&doc, &path, &source, at, family)? {
                    Some((family, candidates)) => {
                        arm_candidates(question.items, family, candidates, &covered, &literals)
                    }
                    None => question.items,
                }
            }
            Some(TypedSite::Field { written }) => {
                let fields = self.field_candidates(&doc, &path, position)?;
                field_candidates(question.items, fields, &written)
            }
            None => question.items,
        };
        Ok(Some(items))
    }

    fn discriminant_candidates(
        &mut self,
        doc: &Arc<ServiceDoc>,
        path: &Path,
        source: &str,
        at: usize,
        family: Option<crate::engine::completions::PatternFamily>,
    ) -> Result<Option<(crate::engine::completions::PatternFamily, Vec<Discriminant>)>, String>
    {
        use crate::engine::completions::PatternFamily;
        let kind = crate::SourceKind::from_path(path).unwrap_or_default();
        let lowered = |text: &str| {
            let (start, end) = crate::engine::declarations::scrutinee_at(text, kind, at)?;
            let report = crate::compile_projection_report(
                text,
                &crate::Options {
                    filename: path.to_str(),
                    source_kind: kind,
                    defer_to_checker: true,
                    rewrite_imports: crate::ImportRewrite::Off,
                    ..crate::Options::default()
                },
            );
            let emit = report.emit.or(report.withheld)?;
            let out_start = mapper::to_output(&emit.mappings, start)?;
            let out_end = out_start + (end - start);
            (mapper::to_source_span(&emit.mappings, out_start, out_end) == Some((start, end)))
                .then_some((emit, out_start, out_end))
        };
        let Some((emit, out_start, out_end)) = lowered(source)
            .or_else(|| lowered(&format!("{}{WILDCARD_ARM}{}", &source[..at], &source[at..])))
        else {
            return Ok(None);
        };
        let families: &[PatternFamily] = match family {
            Some(PatternFamily::Tags) => &[PatternFamily::Tags],
            Some(PatternFamily::Literals) => &[PatternFamily::Literals],
            Some(PatternFamily::Instances) => &[],
            None => &[PatternFamily::Tags, PatternFamily::Literals],
        };
        for &family in families {
            let access = match family {
                PatternFamily::Tags => format!(".{}", crate::core_ir::VARIANT_TAG_FIELD),
                _ => String::new(),
            };
            let head = format!(
                "{}({}){access} === ",
                &emit.code[..out_start],
                &emit.code[out_start..out_end]
            );
            let code = format!("{head}{PROBE_NAME}{}", &emit.code[out_end..]);
            let session = self.session();
            open_served(session, path, &code);
            let answer = ts_completions(
                session,
                path,
                mapper::to_utf16(&code, head.len()),
                ServedText {
                    code: &code,
                    mappings: &[],
                    inserted: &[],
                    source: "",
                    splice: None,
                },
                &emit.generated_names,
            );
            open_served(session, path, &doc.code);
            let candidates: Vec<Discriminant> = answer?
                .items
                .iter()
                .filter(|item| item.kind != "keyword")
                .filter_map(|item| discriminant(&item.label, family))
                .collect();
            if !candidates.is_empty() {
                return Ok(Some((family, candidates)));
            }
        }
        Ok(None)
    }

    fn field_candidates(
        &mut self,
        doc: &Arc<ServiceDoc>,
        path: &Path,
        position: Position,
    ) -> Result<Vec<String>, String> {
        let at = source_byte(&doc.source, position);
        let session = self.session();
        let Some(probe) = build_probe(path, &doc.source, at, session.probe_count + 1) else {
            return Ok(Vec::new());
        };
        session.probe_count += 1;
        open_served(session, path, &probe.code);
        let answer = ts_completions(
            session,
            path,
            probe.offset,
            probe.served(),
            &probe.generated_names,
        );
        open_served(session, path, &doc.code);
        Ok(answer?
            .items
            .into_iter()
            .filter(|item| {
                matches!(
                    crate::parser::pattern_of(&item.label),
                    Some(crate::ast::Pattern::Tags(tags))
                        if tags.len() == 1 && tags[0].bindings.is_none() && tags[0].tag == item.label
                )
            })
            .map(|item| item.label)
            .collect())
    }

    /// The signature and documentation behind one completion entry, fetched
    /// when the consumer asks about the one entry the user is looking at.
    /// The entry is the one listed with `label` and `source`
    /// ([`CompletionItem::source`]). `probe` re-installs the probed text the entry was listed from;
    /// `Ok(None)` when that probe is gone (the buffer has moved on) or the
    /// entry cannot be resolved.
    pub fn completion_resolve(
        &mut self,
        path: &Path,
        position: Position,
        label: &str,
        source: Option<&str>,
        probe: Option<u64>,
    ) -> Result<Option<CompletionDetail>, String> {
        let (doc, path) = self.serve(path)?;
        let session = self.session();
        let installed =
            match probe {
                Some(version) => {
                    let Some(installed) = session.last_probe.clone().filter(|p| {
                        p.version == version && p.path == path && p.source == doc.source
                    }) else {
                        return Ok(None);
                    };
                    open_served(session, &path, &installed.code);
                    Some(installed)
                }
                None => None,
            };
        let (at, generated_names) = match &installed {
            Some(installed) => (installed.offset, installed.generated_names.clone()),
            None => match to_service_typed(&doc, position) {
                Some(at) => (at, doc.generated_names.clone()),
                None => return Ok(None),
            },
        };
        let code = session
            .served
            .get(&path)
            .cloned()
            .unwrap_or_else(|| doc.code.clone());

        let key = (
            path.clone(),
            at,
            label.to_string(),
            source.map(str::to_owned),
        );
        if !session.last_completion.contains_key(&key) {
            // The server resolves the item *it* produced, not a name, so the
            // list has to have been asked for first.
            let text = match &installed {
                Some(installed) => installed.served(),
                None => doc.served(),
            };
            let _ = ts_completions(session, &path, at, text, &generated_names)?;
        }
        let Some(item) = session.last_completion.get(&key).cloned() else {
            return Ok(None);
        };
        let resolved = session.client.request("completionItem/resolve", item)?;
        if resolved.is_null() {
            return Ok(None);
        }
        let (mappings, inserted, splice) = match &installed {
            Some(installed) => (
                &installed.mappings,
                &installed.inserted,
                Some(installed.splice),
            ),
            None => (&doc.mappings, &doc.inserted, None),
        };
        let additional_edits = resolved["additionalTextEdits"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|edit| source_edit(&code, mappings, inserted, &doc.source, splice, edit))
            .collect::<Option<Vec<_>>>()
            .unwrap_or_default();
        Ok(Some(CompletionDetail {
            signature: resolved["detail"].as_str().unwrap_or_default().to_string(),
            documentation: docs_text(&resolved["documentation"]),
            additional_edits,
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
        Ok(self.rename_answer(path, position)?.ok())
    }

    /// [`Project::rename`], with TypeScript's reason when it refuses the
    /// rename at `position` itself.
    pub fn rename_answer(
        &mut self,
        path: &Path,
        position: Position,
    ) -> Result<Result<Vec<RenameEdit>, Option<String>>, String> {
        let (doc, path) = self.serve(path)?;
        let documents = self.overlays.clone();
        let overlays = &*documents.read();
        let session = self.session();
        let Some(at) = to_service_name(&doc, position) else {
            return Ok(Err(None));
        };
        let uri = served_uri(session, &path);
        let lsp_at = lsp_position(u16_position(&doc.code, at));

        // The server's own "can this be renamed?" — a keyword or a literal
        // answers null, and forcing it would rename nothing while looking
        // like it worked.
        let prepared = match session.client.answer(
            "textDocument/prepareRename",
            serde_json::json!({ "textDocument": { "uri": uri }, "position": lsp_at }),
        )? {
            Ok(prepared) => prepared,
            Err(reason) => return Ok(Err(Some(reason))),
        };
        if prepared.is_null() {
            return Ok(Err(None));
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
            return Ok(Err(None));
        };

        let changes = changes.clone();
        let mut out = Vec::new();
        for (edited_uri, edits) in &changes {
            let Some(edits) = edits.as_array() else {
                return Ok(Err(None));
            };
            for one in edits {
                let Some(location) = map_target(
                    session,
                    overlays,
                    edited_uri,
                    &one["range"],
                    TargetUse::Edit,
                ) else {
                    let Some((generated, targets)) =
                        map_shared_target(session, overlays, edited_uri, &one["range"])
                    else {
                        return Ok(Err(None));
                    };
                    let text = one["newText"].as_str().unwrap_or(RENAME_PLACEHOLDER);
                    if text != RENAME_PLACEHOLDER
                        && text != format!("{generated}: {RENAME_PLACEHOLDER}")
                    {
                        return Ok(Err(None));
                    }
                    for target in targets {
                        out.push(RenameEdit {
                            location: target.location,
                            new_text: Some(if target.shorthand {
                                format!("{}: {RENAME_PLACEHOLDER}", target.name)
                            } else {
                                RENAME_PLACEHOLDER.to_string()
                            }),
                        });
                    }
                    continue;
                };
                let new_text = one["newText"].as_str().map(String::from);
                if let Some(text) = &new_text
                    && text != RENAME_PLACEHOLDER
                    && !text.contains(RENAME_PLACEHOLDER)
                {
                    // A shape we cannot account for — refusing beats
                    // silently rebinding a different field.
                    return Ok(Err(None));
                }
                // Text a lowering writes more than once (a variant field's
                // type, in its union and its constructor) is one place in
                // the source, renamed once.
                let edit = RenameEdit { location, new_text };
                if !out.contains(&edit) {
                    out.push(edit);
                }
            }
        }
        Ok(if out.is_empty() { Err(None) } else { Ok(out) })
    }

    /// The file's outline as TypeScript sees its declarations, on the source.
    /// A `variant` is tt's to describe and is not among them.
    pub fn document_symbols(&mut self, path: &Path) -> Result<Vec<DocumentSymbol>, String> {
        let (doc, path) = self.serve(path)?;
        let session = self.session();
        let answer = session.client.request(
            "textDocument/documentSymbol",
            serde_json::json!({ "textDocument": { "uri": served_uri(session, &path) } }),
        )?;
        Ok(source_symbols(
            &doc,
            answer.as_array().map(Vec::as_slice).unwrap_or_default(),
        ))
    }

    /// The file's semantic tokens: TypeScript's classification of the text
    /// the emission copied from the source, on the source, with tt's own
    /// classification of its constructs over it. A token TypeScript gives
    /// compiler-written text is not the user's and is not reported.
    pub fn semantic_tokens(&mut self, path: &Path) -> Result<Vec<ClassifiedToken>, String> {
        let (doc, path) = self.serve(path)?;
        let session = self.session();
        let answer = session.client.request(
            "textDocument/semanticTokens/full",
            serde_json::json!({ "textDocument": { "uri": served_uri(session, &path) } }),
        )?;
        let data: Vec<u64> = answer["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_u64)
            .collect();
        let service = source_tokens(&doc, session.client.semantic_legend(), &data);
        let own = crate::engine::tokens::semantic_tokens_with_kind(
            &doc.source,
            crate::SourceKind::from_path(&path).unwrap_or_default(),
        );
        Ok(merge_tokens(own, service))
    }

    /// Signature help at a call site.
    pub fn signature_help(
        &mut self,
        path: &Path,
        position: Position,
    ) -> Result<Option<SignatureHelp>, String> {
        let (doc, path) = self.serve(path)?;
        let session = self.session();
        // A cursor the served text has no place for is asked through a
        // probe, as completion asks there.
        let (code, mappings, at) = match to_service_typed(&doc, position) {
            Some(at) => (doc.code.clone(), doc.mappings.clone(), at),
            None => {
                let source_at = mapper::from_utf16(&doc.source, u16_offset(&doc.source, position));
                let Some(probe) =
                    build_probe(&path, &doc.source, source_at, session.probe_count + 1)
                else {
                    return Ok(None);
                };
                session.probe_count += 1;
                open_served(session, &path, &probe.code);
                (probe.code, probe.mappings, probe.offset)
            }
        };
        let kind = crate::SourceKind::from_path(&path).unwrap_or_default();
        let at = signature_position(&code, &mappings, kind, mapper::from_utf16(&code, at));
        let question = signature_question(&code, &mappings, &doc.source, kind, at);
        let (asked, at) = match &question {
            Some((question, at)) => {
                open_served(session, &path, question);
                (question, *at)
            }
            None => (&code, at),
        };
        let help = session.client.request(
            "textDocument/signatureHelp",
            serde_json::json!({
                "textDocument": { "uri": served_uri(session, &path) },
                "position": lsp_position(u16_position(asked, mapper::to_utf16(asked, at))),
            }),
        );
        if question.is_some() {
            open_served(session, &path, &code);
        }
        let help = help?;
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
        // A faithful projection is read as a `.ts` file is: syntax errors
        // and all, they are the user's. The raw emit-map fallback leaves tt
        // text as written, and TypeScript's recovery from that says nothing
        // about the user's code unless the text parses.
        if !doc.faithful
            && !projection_accepts_diagnostics(
                &doc.code,
                crate::SourceKind::from_path(&path).unwrap_or_default(),
            )
        {
            return Ok(Vec::new());
        }
        let session = self.session();
        let answer = session.client.request(
            "textDocument/diagnostic",
            serde_json::json!({ "textDocument": { "uri": served_uri(session, &path) } }),
        )?;
        let items = answer["items"].as_array().cloned().unwrap_or_default();
        let served = served_uri(session, &path);
        let mut out = Vec::new();
        // The declaration table a translated message names its types from,
        // built on the first translation of this pass: most passes
        // translate nothing, and building it parses the file and its
        // imports.
        let mut declarations: Option<Vec<crate::analysis::DeclaredVariant>> = None;
        let mut translated_seen: HashSet<(usize, crate::AnchorKind, &'static str)> = HashSet::new();
        for item in items {
            let severity = ServiceSeverity::from_lsp(item["severity"].as_u64());
            let tags: Vec<ServiceTag> = item["tags"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|tag| tag.as_u64().and_then(ServiceTag::from_lsp))
                .collect();
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
            // A suggestion (an unused name, a deprecated call) is about the
            // text it covers. Only text the user wrote can be faded or
            // struck through; one landing in glue is about ttc's emission.
            if !exact
                && matches!(
                    severity,
                    ServiceSeverity::Information | ServiceSeverity::Hint
                )
            {
                continue;
            }
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
            // The checker's `relatedInformation`, which the service sends
            // because the session declares the capability
            // (`typescript::service`).
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
            }) && !translated_seen.insert((anchor.display().0, anchor.kind, class))
            {
                continue;
            }
            // On glue the construct is the diagnostic's extent: its own
            // text is underlined, and where ttc can say what the construct
            // meant it says that instead — the same table the CLI reports
            // through, so the two surfaces cannot drift.
            if !exact && let Some(anchor) = glue {
                let (display_start, display_end) = anchor.display();
                let from = mapper::to_utf16(&doc.source, display_start);
                let to = mapper::to_utf16(&doc.source, display_end).max(from + 1);
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
                        severity,
                        tags: tags.clone(),
                        related: related.clone(),
                    },
                    None => ServiceDiagnostic {
                        range,
                        message: format!("{raw} (in code ttc generated for this construct)"),
                        code,
                        severity,
                        tags: tags.clone(),
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
            let entry = ServiceDiagnostic {
                range: source_range(&doc.source, s, e),
                message,
                code,
                severity,
                tags,
                related,
            };
            // Text a lowering writes more than once (a variant field's
            // type) draws the checker's error at each copy; the user wrote
            // it once.
            if !out.contains(&entry) {
                out.push(entry);
            }
        }
        Ok(out)
    }

    /// The tt-level diagnostics of `path` that [`Project::service_diagnostics`]
    /// states in TypeScript's own words: TypeScript's syntax verdict
    /// ([`crate::DiagnosticCode::restates_typescript_syntax`]) about text the
    /// faithful projection carries as the user wrote it. A verdict inside a
    /// recovered span is about text TypeScript never reads, and a projection
    /// that is not faithful restates nothing.
    pub fn service_restates(&mut self, path: &Path) -> Result<Vec<crate::DiagnosticCode>, String> {
        let (doc, _) = self.serve(path)?;
        if !doc.faithful {
            return Ok(Vec::new());
        }
        let mut codes: Vec<crate::DiagnosticCode> = Vec::new();
        for diagnostic in &doc.tt_diagnostics {
            let read = diagnostic.start.is_none_or(|start| {
                !doc.recovered
                    .iter()
                    .any(|&(from, to)| from <= start && start < to)
            });
            if diagnostic.code.restates_typescript_syntax()
                && read
                && !codes.contains(&diagnostic.code)
            {
                codes.push(diagnostic.code);
            }
        }
        Ok(codes)
    }

    /// Starts (or reuses) the service session and serves `path` and its
    /// transitive `.tt` imports as the TypeScript they lower to. Returns the
    /// file's projection and its canonical path; the session is then live.
    fn serve(&mut self, path: &Path) -> Result<(Arc<ServiceDoc>, PathBuf), String> {
        let canonical = crate::engine::normalize_document_path(path)?;
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
            let arrangement = self.service_arrangement();
            let client = Service::start(&binary, &self.root, &arrangement)?;
            self.service = Some(ServiceSession {
                client,
                served: HashMap::new(),
                uris: HashMap::new(),
                host_served: HashMap::new(),
                docs: HashMap::new(),
                last_completion: HashMap::new(),
                last_probe: None,
                probe_count: 0,
            });
        }
        let documents = self.overlays.clone();
        // The store is read in this scope only: taking the snapshot below
        // reads it again.
        let (doc, files) = {
            let overlays = &*documents.read();
            let session = self.session();

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
                    let target = match crate::engine::paths::canonical(
                        &file
                            .parent()
                            .unwrap_or(Path::new("."))
                            .join(&import.specifier),
                    ) {
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
            // Serve the same contextualized graph used by typed compilation. A
            // host overlay can change an imported expected type without changing
            // this document's source, so source equality alone cannot cache it.
            let mut files = self.initial_files();
            files.extend(seen);
            files.sort();
            files.dedup();
            files.retain(|path| path.is_file() || overlays.contains_key(path));
            (doc, files)
        };
        let snapshot = self
            .update(&files)
            .map_err(|blocked| blocked.error.to_string())?;
        let session = self.session();
        for projected in snapshot.files {
            let path = &projected.source_path;
            if session.served.get(path) != Some(&projected.emit.code) {
                open_served(session, path, &projected.emit.code);
            }
            session.docs.insert(
                path.clone(),
                Arc::new(ServiceDoc {
                    source: projected.source.clone(),
                    code: projected.emit.code.clone(),
                    mappings: projected.emit.mappings.clone(),
                    anchors: projected.emit.anchors.clone(),
                    declared_names: projected.emit.declared_names.clone(),
                    shared_bindings: projected.emit.shared_bindings.clone(),
                    recovered: projected.recovered.clone(),
                    tt_diagnostics: projected.tt_diagnostics.clone(),
                    generated_names: projected.emit.generated_names.clone(),
                    inserted: projected.emit.inserted.clone(),
                    faithful: true,
                }),
            );
        }
        let doc = session.docs.get(&canonical).cloned().unwrap_or(doc);
        Ok((doc, canonical))
    }

    /// The live session, after [`Project::serve`] has run.
    fn session(&mut self) -> &mut ServiceSession {
        self.service.as_mut().expect("serve started it")
    }
}
