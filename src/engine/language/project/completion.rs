//! Completion requests and resolution under the existing Project owner.

use super::*;

const STATEMENT_TAG_PLACEHOLDER: &str = "Tag";

impl Project {
    pub(super) fn complete_at(
        &mut self,
        path: &Path,
        position: Position,
        member: bool,
        trigger: Option<&str>,
    ) -> Result<CompletionAnswer, String> {
        let mut answer = self.service_completion(path, position, member, trigger)?;
        let (doc, path) = self.serve(path)?;
        let documents = self.overlays.clone();
        for entry in tt_module_entries(&path, &doc.source, position, &documents.read()) {
            if !answer.items.iter().any(|item| item.label == entry.label) {
                answer.items.push(entry);
            }
        }
        if let Some(crate::engine::completions::PatternQuestion {
            typed: Some(crate::engine::completions::TypedSite::Field { written, .. }),
            ..
        }) = crate::engine::completions::pattern_question(
            &path,
            &doc.source,
            position,
            Texts::Open(&self.overlays),
        ) {
            answer
                .items
                .retain(|item| is_payload_field(&item.label, &written));
        }
        Ok(answer)
    }

    fn service_completion(
        &mut self,
        path: &Path,
        position: Position,
        member: bool,
        trigger: Option<&str>,
    ) -> Result<CompletionAnswer, String> {
        let (doc, path) = self.serve(path)?;
        let session = self.session();
        let kind = crate::SourceKind::from_path(&path).unwrap_or_default();
        let source_at = {
            let u16 = doc.source_offset(position);
            doc.source_utf16().to_byte(u16)
        };
        let recovered = doc
            .recovered
            .iter()
            .any(|&(start, end)| start <= source_at && source_at < end);
        let plain = match to_service_typed(&doc, position).filter(|_| !recovered) {
            Some(at) => {
                let mut plain = ts_completions(
                    session,
                    &path,
                    at,
                    doc.served(),
                    &doc.generated_names,
                    trigger,
                )?;
                // Scope walks inspect the emitted TypeScript even when
                // the service request and reply use authored coordinates.
                let projected_at = mapper::typed_cursor_to_output(
                    &doc.mappings,
                    &doc.anchors,
                    &doc.source,
                    source_at,
                )
                .expect("to_service_typed requires a projected cursor");
                super::scope::restate_completions(
                    &mut plain,
                    &doc,
                    doc.projected(),
                    kind,
                    doc.code_utf16().to_utf16(projected_at),
                    source_at,
                );
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
        let in_string = crate::lexer::lex_with_kind(&doc.source, 0, doc.source.len(), kind)
            .iter()
            .any(|token| {
                matches!(token.kind, crate::lexer::TokenKind::Str)
                    && token.span.start < source_at
                    && source_at < token.span.end
            });
        let probe = (!in_string)
            .then(|| build_probe(&path, &doc.source, source_at, session.probe_count + 1))
            .flatten();
        let Some(probe) = probe else {
            return Ok(if plain.member {
                plain
            } else {
                CompletionAnswer::default()
            });
        };
        session.probe_count += 1;
        open_served(session, &path, &probe.code);
        let answer = ts_completions(
            session,
            &path,
            probe.offset,
            probe.served(),
            &probe.generated_names,
            trigger,
        );
        let mut probed = restore_document(session, &path, &doc, answer)?;
        super::scope::restate_completions(
            &mut probed,
            &doc,
            probe.served(),
            kind,
            probe.offset,
            source_at,
        );
        probed.probe = Some(probe.version);
        session.last_probe = Some(probe);
        Ok(if !member || probed.member {
            probed
        } else {
            CompletionAnswer::default()
        })
    }

    pub(super) fn complete_pattern_at(
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
        let finish = question.finisher();
        let items = match question.typed {
            Some(TypedSite::Arm {
                prefix,
                family,
                covered,
                literals,
                position: slot,
                statement,
            }) => {
                let at =
                    prefix.map_or_else(|| source_byte(&doc.source, position), |(start, _)| start);
                let source = match (prefix, statement) {
                    (prefix, true) => {
                        let after = prefix.map_or(at, |(_, end)| end);
                        let called = crate::lexer::lex_with_kind(
                            &doc.source,
                            after,
                            doc.source.len(),
                            crate::SourceKind::from_path(&path).unwrap_or_default(),
                        )
                        .first()
                        .is_some_and(|token| {
                            matches!(token.kind, crate::lexer::TokenKind::Punct(b'('))
                        });
                        let tag = if called {
                            STATEMENT_TAG_PLACEHOLDER.to_string()
                        } else {
                            format!("{STATEMENT_TAG_PLACEHOLDER}()")
                        };
                        format!("{}{tag}{}", &doc.source[..at], &doc.source[after..])
                    }
                    (Some((start, end)), false) => {
                        format!("{}{}", &doc.source[..start], &doc.source[end..])
                    }
                    (None, false) => doc.source.clone(),
                };
                match self.discriminant_candidates(&doc, &path, &source, at, slot, family)? {
                    Some((family, candidates)) => arm_candidates(
                        question.items,
                        family,
                        candidates,
                        &covered,
                        &literals,
                        !statement,
                    ),
                    None => question.items,
                }
            }
            Some(TypedSite::Field {
                written,
                claimed: true,
            }) => {
                let fields = self.field_candidates(&doc, &path, position)?;
                field_candidates(question.items, fields, &written)
            }
            Some(TypedSite::Nested { at, prefix }) => {
                match self.nested_candidates(&doc, &path, at, prefix)? {
                    Some(candidates) => arm_candidates(
                        {
                            let tags: Vec<String> = candidates
                                .iter()
                                .map(|candidate| candidate.label().to_string())
                                .collect();
                            let mut items = question.items;
                            items.extend(crate::engine::completions::owner_cases(
                                &path,
                                &doc.source,
                                Texts::Open(&self.overlays),
                                &tags,
                            ));
                            items
                        },
                        crate::engine::completions::PatternFamily::Tags,
                        candidates,
                        &[],
                        &[],
                        false,
                    ),
                    None => question.items,
                }
            }
            Some(TypedSite::Field { claimed: false, .. }) | None => question.items,
        };
        Ok(Some(finish.finish(items)))
    }

    fn discriminant_candidates(
        &mut self,
        doc: &Arc<ServiceDoc>,
        path: &Path,
        source: &str,
        at: usize,
        position: Option<usize>,
        family: Option<crate::engine::completions::PatternFamily>,
    ) -> Result<Option<(crate::engine::completions::PatternFamily, Vec<Discriminant>)>, String>
    {
        use crate::engine::completions::PatternFamily;
        let kind = crate::SourceKind::from_path(path).unwrap_or_default();
        let options = crate::Options {
            filename: path.to_str(),
            source_kind: kind,
            defer_to_checker: true,
            rewrite_imports: crate::ImportRewrite::Off,
            ..crate::Options::default()
        };
        let lowered = |text: &str| {
            let (start, end) = crate::engine::declarations::scrutinee_at(text, kind, at, position)?;
            let report = crate::compile_projection_report(text, &options);
            let emit = report.emit.or(report.withheld)?;
            let out_start = mapper::to_output(&emit.mappings, start)?;
            let out_end = out_start + (end - start);
            (mapper::to_source_span(&emit.mappings, out_start, out_end) == Some((start, end)))
                .then_some((emit, out_start, out_end))
        };
        let repair = if position.is_some() {
            "_"
        } else {
            WILDCARD_ARM
        };
        let repaired = format!("{}{repair}{}", &source[..at], &source[at..]);
        let closed = || {
            let end = at + repair.len();
            crate::compile_projection_report(&repaired, &options)
                .recovered
                .iter()
                .find(|&&(start, stop)| start <= at && end <= stop)
                .and_then(|&(start, _)| closed_at(&repaired, start, end, kind))
                .filter(|(_, closers)| closers.iter().all(|&(closer, _)| closer >= end))
                .map(|(closed, _)| closed)
        };
        let Some((emit, out_start, out_end)) = lowered(source)
            .or_else(|| lowered(&repaired))
            .or_else(|| lowered(&closed()?))
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
                None,
            );
            let candidates: Vec<Discriminant> = restore_document(session, path, doc, answer)?
                .items
                .iter()
                .filter(|item| item.kind != Some(crate::engine::CompletionItemKind::Keyword))
                .filter_map(|item| discriminant(&item.label, family))
                .collect();
            if !candidates.is_empty() {
                return Ok(Some((family, candidates)));
            }
        }
        Ok(None)
    }

    /// The case a nested pattern's tag names when only the checker can
    /// identify its variant (a payload typed by a type parameter): `None`
    /// anywhere else, where [`crate::engine::tt_symbol_at`] answers or no
    /// tt name is written.
    pub(super) fn pattern_symbol_at(
        &mut self,
        path: &Path,
        position: Position,
    ) -> Result<Option<crate::engine::TtSymbol>, String> {
        use crate::engine::completions::{TypedSite, pattern_question};
        let (doc, path) = self.serve(path)?;
        let Some(question) =
            pattern_question(&path, &doc.source, position, Texts::Open(&self.overlays))
        else {
            return Ok(None);
        };
        let Some(TypedSite::Nested {
            at,
            prefix: Some(tag),
        }) = question.typed
        else {
            return Ok(None);
        };
        let Some(candidates) = self.nested_candidates(&doc, &path, at, Some(tag))? else {
            return Ok(None);
        };
        let tags: Vec<String> = candidates
            .iter()
            .map(|candidate| candidate.label().to_string())
            .collect();
        Ok(crate::engine::names::owned_case_symbol(
            &path,
            &doc.source,
            Texts::Open(&self.overlays),
            &tags,
            tag,
        ))
    }

    /// The tags TypeScript admits at a nested pattern's position: the
    /// payload's discriminant, asked where the lowered arm compares it
    /// ([`crate::PayloadTemp::tag`]), so a generic payload is answered by
    /// the type the scrutinee gives it.
    fn nested_candidates(
        &mut self,
        doc: &Arc<ServiceDoc>,
        path: &Path,
        at: usize,
        prefix: Option<(usize, usize)>,
    ) -> Result<Option<Vec<Discriminant>>, String> {
        let end = prefix.map_or(at, |(_, end)| end);
        let called = crate::lexer::lex_with_kind(
            &doc.source,
            end,
            doc.source.len(),
            crate::SourceKind::from_path(path).unwrap_or_default(),
        )
        .first()
        .is_some_and(|token| matches!(token.kind, crate::lexer::TokenKind::Punct(b'(')));
        let tag = if called {
            STATEMENT_TAG_PLACEHOLDER.to_string()
        } else {
            format!("{STATEMENT_TAG_PLACEHOLDER}()")
        };
        let source = format!("{}{tag}{}", &doc.source[..at], &doc.source[end..]);
        let report = crate::compile_projection_report(
            &source,
            &crate::Options {
                filename: path.to_str(),
                source_kind: crate::SourceKind::from_path(path).unwrap_or_default(),
                defer_to_checker: true,
                rewrite_imports: crate::ImportRewrite::Off,
                ..crate::Options::default()
            },
        );
        let Some(emit) = report.emit.or(report.withheld) else {
            return Ok(None);
        };
        let Some(&crate::PayloadTemp {
            tag: (start, end), ..
        }) = emit.payload_temps.iter().find(|temp| temp.src == at)
        else {
            return Ok(None);
        };
        let code = format!("{}{PROBE_NAME}{}", &emit.code[..start], &emit.code[end..]);
        let session = self.session();
        open_served(session, path, &code);
        let answer = ts_completions(
            session,
            path,
            mapper::to_utf16(&code, start),
            ServedText {
                code: &code,
                mappings: &[],
                inserted: &[],
                source: "",
                splice: None,
            },
            &emit.generated_names,
            None,
        );
        let candidates: Vec<Discriminant> = restore_document(session, path, doc, answer)?
            .items
            .iter()
            .filter(|item| item.kind != Some(crate::engine::CompletionItemKind::Keyword))
            .filter_map(|item| {
                discriminant(&item.label, crate::engine::completions::PatternFamily::Tags)
            })
            .collect();
        Ok((!candidates.is_empty()).then_some(candidates))
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
            None,
        );
        Ok(restore_document(session, path, doc, answer)?
            .items
            .into_iter()
            .map(|item| item.label)
            .collect())
    }

    pub(super) fn resolve_completion_at(
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
        let answer = (|| -> Result<Option<CompletionDetail>, String> {
            let text = installed
                .as_ref()
                .map_or_else(|| doc.served(), |probe| probe.served());
            let key = (
                path.clone(),
                at,
                label.to_string(),
                source.map(str::to_owned),
            );
            if !session.last_completion.contains_key(&key) {
                let _ = ts_completions(session, &path, at, text, &generated_names, None)?;
            }
            let Some(item) = session.last_completion.get(&key).cloned() else {
                return Ok(None);
            };
            let resolved = session.client.request("completionItem/resolve", item)?;
            if resolved.is_null() {
                return Ok(None);
            }
            let additional_edits = resolved["additionalTextEdits"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|edit| {
                    source_edit(
                        text.code,
                        text.mappings,
                        text.inserted,
                        &doc.source,
                        text.splice,
                        edit,
                    )
                    .map(|edit| TextEdit {
                        new_text: tt_specifiers_in(session, &path, &edit.new_text),
                        ..edit
                    })
                })
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default();
            Ok(Some(CompletionDetail {
                signature: resolved["detail"].as_str().unwrap_or_default().to_string(),
                documentation: docs_text(&resolved["documentation"]),
                additional_edits,
            }))
        })();
        let answer = if installed.is_some() {
            restore_document(session, &path, &doc, answer)
        } else {
            answer
        }?;
        Ok(answer.map(|detail| CompletionDetail {
            documentation: self.source_links(&detail.documentation),
            ..detail
        }))
    }
}
