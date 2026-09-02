//! Semantic report assembly and diagnostic ownership.

use super::*;

/// Builds the pass's diagnostics from the checker's answers, in the exact
/// order and wording the report has always had.
pub(crate) fn report(
    snapshot: &Snapshot,
    answers: &Answers,
    probes: &Probes,
    tt_only: bool,
    semantics: &HashMap<PathBuf, Arc<FileSemantics>>,
    requested: &HashSet<PathBuf>,
) -> Vec<Diagnostic> {
    let all_files = snapshot.files();
    let member_sources = typed_member_sources(snapshot, answers, requested);
    let files_storage;
    let files = match &member_sources {
        Some(members) => {
            files_storage = all_files
                .iter()
                .filter(|file| members.contains(&file.source_path))
                .cloned()
                .collect::<Vec<_>>();
            files_storage.as_slice()
        }
        None => all_files,
    };
    let mut out = Vec::new();

    for file in snapshot.blocked().iter().filter(|file| {
        member_sources
            .as_ref()
            .is_none_or(|members| members.contains(&file.source_path))
    }) {
        for diagnostic in &file.diagnostics {
            out.push(Diagnostic {
                path: file.source_path.clone(),
                position: diagnostic.start.map(|at| crate::line_col(&file.source, at)),
                end: diagnostic.end.map(|at| crate::line_col(&file.source, at)),
                message: diagnostic.message.clone(),
                code: Some(diagnostic.code.as_str().to_string()),
                suggestions: diagnostic.suggestions.clone(),
                labels: Vec::new(),
            });
        }
    }

    // The tt layer first: the diagnostics each file's projection found on
    // its own (duplicate arms, unknown cases, misplaced constructs). They
    // are tt's answers about tt's constructs, so they are reported on the
    // tt-only path too — and they no longer gate the rest of this report
    // (TASK-117 symptom 3): the typed answers below follow either way.
    for file in files {
        for d in &file.tt_diagnostics {
            out.push(Diagnostic {
                path: file.source_path.clone(),
                position: d.start.map(|at| crate::line_col(&file.source, at)),
                end: d.end.map(|at| crate::line_col(&file.source, at)),
                message: d.message.clone(),
                code: Some(d.code.as_str().to_string()),
                suggestions: d.suggestions.clone(),
                labels: Vec::new(),
            });
        }
    }

    if !tt_only {
        for shape in &answers.result_shapes {
            let Some(anchor) = probes.result_returns.get(shape.index) else {
                continue;
            };
            let Some(file) = files
                .iter()
                .find(|file| file.source_path == anchor.source_path)
            else {
                continue;
            };
            out.push(Diagnostic {
                path: anchor.source_path.clone(),
                position: Some(crate::line_col(&file.source, anchor.offset)),
                end: Some(crate::line_col(&file.source, anchor.end)),
                message: "`return` here would wrap an already-Result value".to_string(),
                code: Some(
                    crate::DiagnosticCode::ResultReturnNested
                        .as_str()
                        .to_string(),
                ),
                suggestions: vec![crate::Suggestion {
                    message: "propagate this Result instead".to_string(),
                    edit: Some(crate::Edit {
                        start: anchor.offset,
                        end: anchor.offset,
                        replacement: "try ".to_string(),
                    }),
                }],
                labels: Vec::new(),
            });
        }
    }

    // A projection is deliberately file-local, so it cannot resolve names
    // against declarations imported from another source. The cached semantic
    // file can. Render those answers through sema's one diagnostic author and
    // merge them with the projection results; local names may appear in both,
    // while imported names appear only here.
    let mut resolution_spans: HashMap<PathBuf, Vec<(usize, usize)>> = HashMap::new();
    for file in files {
        let Some(semantics) = semantics.get(&file.source_path) else {
            continue;
        };
        for error in crate::sema::resolution_errors(&semantics.analyses) {
            if let (Some(start), Some(end)) = (error.offset, error.end) {
                resolution_spans
                    .entry(file.source_path.clone())
                    .or_default()
                    .push((start, end));
            }
            let diagnostic = Diagnostic {
                path: file.source_path.clone(),
                position: error.offset.map(|at| crate::line_col(&file.source, at)),
                end: error.end.map(|at| crate::line_col(&file.source, at)),
                message: error.message,
                code: Some(error.code.as_str().to_string()),
                suggestions: error.suggestions,
                labels: Vec::new(),
            };
            if !out.contains(&diagnostic) {
                out.push(diagnostic);
            }
        }
    }

    // TypeScript's own diagnostics, at the position in the `.tt` file the
    // offending code was written at.
    let type_diagnostics: &[TsDiagnostic] = if tt_only { &[] } else { &answers.diagnostics };
    let structured_glue: HashSet<(PathBuf, usize, AnchorKind)> = type_diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.mismatch.is_some())
        .filter_map(|diagnostic| {
            let file = files
                .iter()
                .find(|file| file.module_path == diagnostic.file)?;
            let (start, end) = diagnostic_span(diagnostic);
            let DiagnosticOrigin::Anchor(anchor) = projection::diagnostic_origin(file, start, end)?
            else {
                return None;
            };
            Some((file.source_path.clone(), anchor.src, anchor.kind))
        })
        .collect();
    let mut translated_seen: HashSet<(PathBuf, usize, AnchorKind, &'static str)> = HashSet::new();
    for diagnostic in type_diagnostics {
        let (diagnostic_start, diagnostic_end) = diagnostic_span(diagnostic);
        let Some(file) = files.iter().find(|f| f.module_path == diagnostic.file) else {
            // A hand-written file: TypeScript's own coordinates already name
            // a file the user can open, so they are used as they are.
            out.push(Diagnostic {
                path: diagnostic.file.clone(),
                position: None,
                end: None,
                message: diagnostic_message(diagnostic, &[]),
                code: Some(format!("ts{}", diagnostic.code)),
                suggestions: Vec::new(),
                labels: Vec::new(),
            });
            continue;
        };
        if projection::diagnostic_intersects_recovery(file, diagnostic) {
            continue;
        }
        if projection::diagnostic_intersects_tt_error(file, diagnostic) {
            continue;
        }
        let Some(origin) = projection::diagnostic_origin(file, diagnostic_start, diagnostic_end)
        else {
            out.push(Diagnostic {
                path: file.source_path.clone(),
                position: None,
                end: None,
                message: diagnostic_message(diagnostic, &[]),
                code: Some(format!("ts{}", diagnostic.code)),
                suggestions: Vec::new(),
                labels: Vec::new(),
            });
            continue;
        };
        if matches!(origin, DiagnosticOrigin::Exact { .. })
            && diagnostic
                .mismatch
                .as_ref()
                .and_then(|mismatch| mismatch.declaration.as_ref())
                .and_then(|declaration| {
                    files
                        .iter()
                        .find(|candidate| candidate.module_path == declaration.file)
                        .map(|declaration_file| (declaration, declaration_file))
                })
                .is_some_and(|(declaration, declaration_file)| {
                    let out = crate::typescript::mapper::from_utf16(
                        &declaration_file.emit.code,
                        declaration.start,
                    );
                    declaration_file.emit.anchors.iter().any(|anchor| {
                        anchor.out <= out
                            && out < anchor.end
                            && structured_glue.contains(&(
                                declaration_file.source_path.clone(),
                                anchor.src,
                                anchor.kind,
                            ))
                    })
                })
        {
            continue;
        }
        if let DiagnosticOrigin::Exact { start, end } = origin
            && resolution_spans
                .get(&file.source_path)
                .is_some_and(|spans| {
                    spans
                        .iter()
                        .any(|&(owner_start, owner_end)| start < owner_end && owner_start < end)
                })
        {
            continue;
        }
        // Glue is not the user's code. When ttc can say what the construct
        // meant, it says that — over the construct's own text. The
        // declaration table its wording names types from is built only for
        // a file that has a diagnostic on glue at all, and once for it.
        if let DiagnosticOrigin::Anchor(anchor) = origin {
            if anchor.kind == AnchorKind::Match
                && semantics.get(&file.source_path).is_some_and(|semantics| {
                    semantics.analyses.match_has_resolution_error(anchor.src)
                })
            {
                continue;
            }
            let structured_key = (file.source_path.clone(), anchor.src, anchor.kind);
            if structured_glue.contains(&structured_key) {
                // Several checker diagnostics can describe one failed
                // lowering. The contextual expected/found relation is the
                // cause; property and comparison errors on the same glue are
                // consequences and must not become separate user errors.
                if diagnostic.mismatch.is_none()
                    || !translated_seen.insert((
                        file.source_path.clone(),
                        anchor.src,
                        anchor.kind,
                        "structured-type-mismatch",
                    ))
                {
                    continue;
                }
                let declared: &[DeclaredVariant] = semantics
                    .get(&file.source_path)
                    .map(|s| s.analyses.declarations.as_slice())
                    .unwrap_or_default();
                out.push(Diagnostic {
                    path: file.source_path.clone(),
                    position: Some(crate::line_col(&file.source, anchor.src)),
                    end: Some(crate::line_col(&file.source, anchor.src_end)),
                    message: anchored_diagnostic_message(&anchor, diagnostic, declared),
                    code: Some(format!("ts{}", diagnostic.code)),
                    suggestions: Vec::new(),
                    labels: checker_labels(files, file, Some(&anchor), diagnostic),
                });
                continue;
            }
            // The whole-pipeline anchor shares its kind with the step
            // anchors but not their meaning: only a step anchor may speak
            // in step vocabulary.
            let translates = anchor.kind != AnchorKind::Pipe || pipe_step_anchor(&anchor);
            if translates
                && let Some(class) = translation_class(anchor.kind, diagnostic.code)
                && !translated_seen.insert((
                    file.source_path.clone(),
                    anchor.src,
                    anchor.kind,
                    class,
                ))
            {
                continue;
            }
            let declared: &[DeclaredVariant] = semantics
                .get(&file.source_path)
                .map(|s| s.analyses.declarations.as_slice())
                .unwrap_or_default();
            if translates
                && let Some(said) =
                    translate(anchor.kind, diagnostic.code, &diagnostic.message, declared)
            {
                let entry = Diagnostic {
                    path: file.source_path.clone(),
                    position: Some(crate::line_col(&file.source, anchor.src)),
                    end: Some(crate::line_col(&file.source, anchor.src_end)),
                    message: said,
                    code: Some(format!("ts{}", diagnostic.code)),
                    suggestions: Vec::new(),
                    labels: checker_labels(files, file, Some(&anchor), diagnostic),
                };
                // One construct's glue can draw several TypeScript errors
                // that all mean the same tt thing (`$tt_t.kind` and
                // `$tt_t.value`).
                if !out.contains(&entry) {
                    out.push(entry);
                }
                continue;
            }
        }
        let declared: &[DeclaredVariant] = semantics
            .get(&file.source_path)
            .map(|s| s.analyses.declarations.as_slice())
            .unwrap_or_default();
        match origin {
            DiagnosticOrigin::Exact { start, end } => {
                out.push(Diagnostic {
                    path: file.source_path.clone(),
                    position: Some(crate::line_col(&file.source, start)),
                    end: (end > start).then(|| crate::line_col(&file.source, end)),
                    message: diagnostic_message(diagnostic, declared),
                    code: Some(format!("ts{}", diagnostic.code)),
                    suggestions: Vec::new(),
                    labels: checker_labels(files, file, None, diagnostic),
                });
            }
            DiagnosticOrigin::Anchor(anchor) => out.push(Diagnostic {
                path: file.source_path.clone(),
                position: Some(crate::line_col(&file.source, anchor.src)),
                end: Some(crate::line_col(&file.source, anchor.src_end)),
                message: format!(
                    "{} (in code ttc generated for this construct)",
                    diagnostic_message(diagnostic, declared)
                ),
                code: Some(format!("ts{}", diagnostic.code)),
                suggestions: Vec::new(),
                labels: checker_labels(files, file, Some(&anchor), diagnostic),
            }),
            DiagnosticOrigin::Nearest { start } => out.push(Diagnostic {
                path: file.source_path.clone(),
                position: Some(crate::line_col(&file.source, start)),
                end: None,
                message: format!(
                    "{} (in code ttc generated near this position)",
                    diagnostic_message(diagnostic, declared)
                ),
                code: Some(format!("ts{}", diagnostic.code)),
                suggestions: Vec::new(),
                labels: checker_labels(files, file, None, diagnostic),
            }),
        }
    }

    // Literal-match exhaustiveness, decided by the type TypeScript computes
    // at the scrutinee — narrowing included.
    for missing in &answers.literal_missing {
        let Some(anchor) = probes.literals.get(missing.index) else {
            continue;
        };
        let Some(file) = files
            .iter()
            .find(|f| f.source_path == anchor.anchor.source_path)
        else {
            continue;
        };
        // A literal arm is written as the value itself, so the witness the
        // checker names *is* the arm pattern — the same text the message
        // quotes, which is why both come from `display_literal`.
        let uncovered: Vec<String> = missing.missing.iter().map(display_literal).collect();
        out.push(Diagnostic {
            path: file.source_path.clone(),
            position: Some(crate::line_col(&file.source, anchor.anchor.offset)),
            end: Some(crate::line_col(&file.source, anchor.anchor.end)),
            message: crate::diagnostics::non_exhaustive_message(
                Some("literal union"),
                &uncovered,
                false,
            ),
            code: Some(
                crate::DiagnosticCode::MatchNotExhaustive
                    .as_str()
                    .to_string(),
            ),
            suggestions: crate::diagnostics::non_exhaustive_suggestions(
                &file.source,
                site_of(anchor),
                &uncovered,
            ),
            labels: Vec::new(),
        });
    }

    // Tag exhaustiveness. The checker names the constituents the
    // scrutinee's type still has — narrowing included — and tt runs its
    // own algorithm over that alphabet, which is what sees a hole *inside*
    // a payload as well as a missing case (TASK-108).
    //
    // A witness tt is not certain of is dropped here: the default path
    // reports those because it has nothing better, but on this path the
    // honest answer for an unidentifiable column is to ask the checker,
    // and that question is not asked yet.
    // Per file, per match: the alphabet of each scrutinee position, in
    // position order (a single match has one).
    let mut by_file: HashMap<PathBuf, Vec<MatchAlphabets>> = HashMap::new();
    // The payload answers ride in the same list, after the match ones —
    // they name the alphabet of a `(constructor, field)` column, which is
    // the one thing tt cannot work out from declarations alone.
    let mut payloads: HashMap<PathBuf, Vec<PayloadAlphabet>> = HashMap::new();
    // `(file, match keyword) -> end of `match (scrutinee)``.
    let mut match_ends: HashMap<(PathBuf, usize), usize> = HashMap::new();
    // The same key -> where the match's body braces are, so a coverage
    // hole's fix can be written as an edit on this path too.
    let mut sites: HashMap<(PathBuf, usize), crate::diagnostics::MatchSite> = HashMap::new();
    for members in &answers.tag_members {
        if let Some(anchor) = probes.tags.get(members.index) {
            let per_match = by_file
                .entry(anchor.anchor.source_path.clone())
                .or_default();
            match per_match
                .iter_mut()
                .find(|(at, _)| *at == anchor.anchor.offset)
            {
                Some((_, positions)) => positions.push(members.tags.clone()),
                None => per_match.push((anchor.anchor.offset, vec![members.tags.clone()])),
            }
            // The keyword offset keys the alphabets; the range it opens is
            // what the diagnostic underlines, and the braces are where its
            // fix is written.
            sites.insert(
                (anchor.anchor.source_path.clone(), anchor.anchor.offset),
                site_of(anchor),
            );
            match_ends.insert(
                (anchor.anchor.source_path.clone(), anchor.anchor.offset),
                anchor.anchor.end,
            );
            continue;
        }
        let Some(anchor) = probes
            .payloads
            .get(members.index.wrapping_sub(probes.tags.len()))
        else {
            continue;
        };
        payloads
            .entry(anchor.source_path.clone())
            .or_default()
            .push((
                (anchor.tag.clone(), anchor.field.clone()),
                members.tags.clone(),
            ));
    }
    for file in files {
        let Some(asked) = by_file.get(&file.source_path) else {
            continue;
        };
        // The nested columns are resolved from declarations, so the
        // imported ones have to be collected — otherwise a payload whose
        // type is an imported variant reads as an unknown alphabet and its
        // holes go unreported. The cached semantics carry them.
        let externs: &[crate::VariantSymbol] = semantics
            .get(&file.source_path)
            .map(|s| s.externs.as_slice())
            .unwrap_or_default();
        let asked_payloads = payloads
            .get(&file.source_path)
            .map_or(&[][..], Vec::as_slice);
        for (offset, coverage) in
            crate::analysis::checked_coverage(&file.source, externs, asked, asked_payloads)
        {
            if semantics
                .get(&file.source_path)
                .is_some_and(|semantics| semantics.analyses.match_has_resolution_error(offset))
            {
                continue;
            }
            // A single match's witness is one pattern, quoted the way the
            // default path quotes one; a tuple match's is a combination of
            // positions, written as one `(a, b)` and left unquoted — the
            // quotes would read as part of the pattern.
            let uncovered: Vec<String> = coverage
                .missing
                .iter()
                .filter(|m| m.certain)
                .map(|m| {
                    if m.pattern.len() > 1 {
                        format!("({})", m.pattern.join(", "))
                    } else {
                        format!("{:?}", m.pattern.first().cloned().unwrap_or_default())
                    }
                })
                .collect();
            if uncovered.is_empty() {
                continue;
            }
            // The arms that close the hole, from the same witnesses in
            // their binding form — one authoring, both pipelines.
            let arms: Vec<String> = coverage
                .missing
                .iter()
                .filter(|m| m.certain)
                .map(|m| {
                    if m.arm.len() > 1 {
                        format!("({})", m.arm.join(", "))
                    } else {
                        m.arm.first().cloned().unwrap_or_else(|| "_".to_string())
                    }
                })
                .collect();
            // The typed pass knows the alphabet but not the declaration,
            // so the shared renderer gets no subject — one renderer, one
            // wording, on both pipelines (TASK-120).
            let tuple = coverage.positions.len() > 1;
            out.push(Diagnostic {
                path: file.source_path.clone(),
                position: Some(crate::line_col(&file.source, offset)),
                end: match_ends
                    .get(&(file.source_path.clone(), offset))
                    .map(|at| crate::line_col(&file.source, *at)),
                message: crate::diagnostics::non_exhaustive_message(None, &uncovered, tuple),
                code: Some(
                    crate::DiagnosticCode::MatchNotExhaustive
                        .as_str()
                        .to_string(),
                ),
                suggestions: match sites.get(&(file.source_path.clone(), offset)) {
                    Some(site) => {
                        crate::diagnostics::non_exhaustive_suggestions(&file.source, *site, &arms)
                    }
                    None => vec![non_exhaustive_help()],
                },
                labels: Vec::new(),
            });
        }
    }

    // `val`: two resolutions decide, and ttc guesses neither of them.
    //
    // 1. Which binding a path is rooted at — the root identifier and the
    //    binding's declaration are the same binding when they are the same
    //    symbol. Shadowing, redeclaration and destructuring come out right
    //    because this is TypeScript's own resolution, not a model of it.
    // 2. For a method call, whether the method is a built-in — declared in
    //    TypeScript's own lib files. A user-defined method that shares the
    //    name is not, and anything unresolved is left alone.
    let symbols: HashMap<usize, &Resolution> =
        answers.resolutions.iter().map(|r| (r.index, r)).collect();
    let mut val_symbols = HashMap::new();
    for binding in &probes.val_bindings {
        let Some(symbol) = symbols.get(&binding.root) else {
            continue;
        };
        val_symbols
            .entry(symbol.id)
            .and_modify(|entry| *entry = None)
            .or_insert(Some(binding));
    }

    for mutation in &probes.mutations {
        let Some(root) = symbols.get(&mutation.root) else {
            continue; // unresolved — never a verdict
        };
        let Some(binding) = val_symbols.get(&root.id) else {
            continue; // not this binding, whatever it is called
        };
        if let Some(method) = mutation.method {
            match symbols.get(&method) {
                // Two halves make the verdict: the checker's — the
                // method is one of TypeScript's own — and tt's policy —
                // that method is one of the mutating ones. A built-in
                // `get` fails the second; a user-defined `set`, or a
                // method the checker could not resolve, fails the first.
                Some(resolution)
                    if resolution.builtin && crate::is_builtin_mutator_name(&resolution.name) => {}
                _ => continue,
            }
        }
        let Some(file) = files
            .iter()
            .find(|f| f.source_path == mutation.anchor.source_path)
        else {
            continue;
        };
        let message = match &mutation.method_name {
            // The built-in itself is not named: the compiler answered
            // "this method is one of TypeScript's own", which is the
            // verdict — not which interface declares it.
            Some(method) => format!(
                "cannot call mutating method `{}` through val binding `{}` \
                 (the binding is declared with `val`, so every access path from it is \
                 read-only)",
                method, mutation.name,
            ),
            None => format!(
                "cannot mutate through val binding `{}` \
                 (the binding is declared with `val`, so every access path \
                 from it is read-only)",
                mutation.name,
            ),
        };
        let (suggestions, labels) = binding.map_or_else(
            || (Vec::new(), Vec::new()),
            |binding| {
                let declaration = files
                    .iter()
                    .find(|candidate| candidate.source_path == binding.anchor.source_path);
                let suggestions = vec![crate::Suggestion {
                    message: "remove `val` if this binding is intended to be mutable".to_string(),
                    edit: Some(crate::Edit {
                        start: binding.anchor.offset,
                        end: binding.modifier_end,
                        replacement: String::new(),
                    }),
                }];
                let labels = declaration.map_or_else(Vec::new, |declaration| {
                    vec![DiagnosticLabel {
                        path: (declaration.source_path != file.source_path)
                            .then(|| declaration.source_path.clone()),
                        position: crate::line_col(&declaration.source, binding.anchor.offset),
                        end: crate::line_col(&declaration.source, binding.anchor.end),
                        message: "the read-only binding is declared here".to_string(),
                    }]
                });
                (suggestions, labels)
            },
        );
        out.push(Diagnostic {
            path: file.source_path.clone(),
            position: Some(crate::line_col(&file.source, mutation.anchor.offset)),
            end: Some(crate::line_col(&file.source, mutation.anchor.end)),
            message,
            code: Some(crate::DiagnosticCode::ValMutation.as_str().to_string()),
            suggestions,
            labels,
        });
    }

    // The callee table: a declaration's symbol names its parameter
    // list. One symbol carrying declarations with *different* lists
    // (TypeScript overloads, `var` merging) makes that callee
    // ambiguous, and an ambiguous callee is not judged — the same
    // caution the name-keyed table of the untyped path takes, here at
    // symbol granularity, so two functions merely sharing a name stay
    // two callees.
    let mut callees: HashMap<i64, Option<&[crate::ValParam]>> = HashMap::new();
    for function in &probes.functions {
        let Some(resolution) = symbols.get(&function.root) else {
            continue;
        };
        match callees.get(&resolution.id) {
            Some(Some(prev)) if *prev == function.params.as_slice() => {}
            Some(_) => {
                callees.insert(resolution.id, None);
            }
            None => {
                callees.insert(resolution.id, Some(&function.params));
            }
        }
    }

    // The function boundary: a `val` binding may only be handed to a
    // parameter that is itself `val`. Which binding the argument names,
    // and which declaration the call names, are the same symbol
    // question the mutations above ask — an unresolved callee, or one
    // no collected declaration matches (an import, a method), is never
    // a verdict.
    for pass in &probes.passes {
        let Some(root) = symbols.get(&pass.root) else {
            continue;
        };
        if !val_symbols.contains_key(&root.id) {
            continue;
        }
        let Some(callee) = symbols.get(&pass.callee_symbol) else {
            continue;
        };
        let Some(Some(params)) = callees.get(&callee.id) else {
            continue;
        };
        let Some(param) = params.get(pass.arg_index) else {
            continue;
        };
        if param.is_val {
            continue;
        }
        let described = match &param.name {
            Some(name) => format!("`{name}`"),
            None => format!("#{}", pass.arg_index + 1),
        };
        let Some(file) = files
            .iter()
            .find(|f| f.source_path == pass.anchor.source_path)
        else {
            continue;
        };
        out.push(Diagnostic {
            path: file.source_path.clone(),
            position: Some(crate::line_col(&file.source, pass.anchor.offset)),
            end: Some(crate::line_col(&file.source, pass.anchor.end)),
            message: format!(
                "cannot pass val binding `{}` to mutable parameter {} of \
                 `{}` (the parameter is not declared with `val`, so the function may mutate \
                 through it)",
                pass.name, described, pass.callee,
            ),
            code: Some(crate::DiagnosticCode::ValPass.as_str().to_string()),
            suggestions: Vec::new(),
            labels: Vec::new(),
        });
    }

    finish_diagnostics(out)
}
