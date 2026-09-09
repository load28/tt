//! Pattern-site traversal, binding analysis, and declaration lookup.

use super::*;

/// How much of each match to analyze — bindings cost work no coverage
/// consumer would read.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Depth {
    Full,
    CoverageOnly,
}

pub(super) fn analyze(program: &Program, table: &Table, depth: Depth) -> PatternAnalyses {
    let mut analyses = PatternAnalyses::default();
    walk(program, table, depth, &mut analyses);
    analyses.declarations = table
        .entries
        .iter()
        .map(|e| DeclaredVariant {
            name: e.name.clone(),
            origin: e.origin.clone(),
            constructors: e.constructors.clone(),
        })
        .collect();
    analyses
}

/// One candidate variant of the analysis' declaration table.
pub(super) struct Entry {
    /// The variant's name in the analyzed file's scope.
    pub(super) name: String,
    /// Where it was declared — carried into [`Coverage`] so a consumer can
    /// name the origin without a table of its own.
    pub(super) origin: Origin,
    /// The constructors, in declaration order, including payload fields.
    pub(super) constructors: Vec<MatchConstructor>,
}

/// The candidate variants a match's subject can resolve to, in shadowing
/// order — the analysis' declaration table.
pub(super) struct Table {
    /// Local declarations first (in source order), then imported ones, then
    /// the built-ins; each name appears once, so the nearer origin wins.
    entries: Vec<Entry>,
}

impl Table {
    /// The table, derived from the resolver's world — **one** construction
    /// of the visibility rules (local later-wins, imports shadowed by
    /// locals, built-ins by both), owned by [`crate::resolve`]. What stays
    /// here is only the coverage/typed-model *view* of it: names, origins
    /// and constructors with declared field text.
    pub(super) fn from_resolution(resolution: &crate::resolve::Resolution) -> Table {
        use crate::resolve::{DeclOrigin, DefKind};
        let entries = resolution
            .defs
            .iter()
            .filter_map(|(id, def)| {
                let DefKind::Variant(data) = &def.kind else {
                    return None;
                };
                // Only the winner of each name is a candidate.
                if resolution.type_ns.get(&def.name) != Some(&id) {
                    return None;
                }
                Some(Entry {
                    name: def.name.clone(),
                    origin: match &data.origin {
                        DeclOrigin::Local(_) => Origin::Local,
                        DeclOrigin::Imported { from } => Origin::Imported { from: from.clone() },
                        DeclOrigin::Builtin => Origin::Builtin,
                    },
                    constructors: {
                        // A duplicate declaration is diagnosed by sema, but a
                        // variant's semantic alphabet still contains each
                        // constructor exactly once. Keeping that invariant here
                        // prevents every downstream consumer (coverage,
                        // completion, and suggested arms) from duplicating it.
                        let mut seen = std::collections::HashSet::<String>::new();
                        data.variants
                            .iter()
                            .filter(|variant| seen.insert(variant.name.clone()))
                            .map(|variant| MatchConstructor {
                                tag: variant.name.clone(),
                                fields: variant.fields.as_ref().map(|fields| {
                                    fields
                                        .iter()
                                        .map(|field| PayloadField {
                                            name: field.name.clone(),
                                            optional: field.optional,
                                            ty: field.ty_text.clone(),
                                        })
                                        .collect()
                                }),
                            })
                            .collect()
                    },
                })
            })
            .collect();
        Table { entries }
    }

    /// The first variant whose cases contain every tag — `None` for an empty
    /// tag set (nothing identifies an variant) or when no candidate fits.
    pub(super) fn resolve(&self, tags: &[&str]) -> Option<(&str, &[MatchConstructor])> {
        if tags.is_empty() {
            return None;
        }
        self.candidates(tags)
            .first()
            .map(|entry| (entry.name.as_str(), entry.constructors.as_slice()))
    }

    /// Every candidate for a tag set, in shadowing order.
    pub(super) fn candidates(&self, tags: &[&str]) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|entry| {
                tags.iter()
                    .all(|tag| entry.constructors.iter().any(|c| c.tag == *tag))
            })
            .collect()
    }

    /// A subject the *checker* named: its constituents' `kind` literals,
    /// in the type's own order.
    ///
    /// The payload of each tag is filled from this table when some
    /// declaration has that tag, so a nested pattern under it still
    /// analyzes; a tag no declaration knows becomes a constructor with no
    /// field list, which specializes to nothing and so covers only itself.
    /// The entry has no name — the checker answers with a *type*, not a
    /// declaration, which is why the typed path's message names no variant.
    pub(super) fn entry_of_members(&self, tags: &[String]) -> Entry {
        Entry {
            name: String::new(),
            origin: Origin::Local,
            constructors: tags
                .iter()
                .map(|tag| MatchConstructor {
                    tag: tag.clone(),
                    fields: self
                        .entries
                        .iter()
                        .find_map(|e| e.constructors.iter().find(|c| c.tag == *tag))
                        .and_then(|c| c.fields.clone()),
                })
                .collect(),
        }
    }

    /// The variant a declared type text names: a bare (possibly dotted)
    /// identifier, optionally with type arguments — `Shape`,
    /// `Option<number>`, `ns.Token` — and nothing else. Type arguments are
    /// not substituted (ttc has no type system); the constructor's declared
    /// field text answers as written.
    pub(super) fn resolve_type(&self, ty: &str) -> Option<(&str, &[MatchConstructor])> {
        self.entry_of_type(ty)
            .map(|e| (e.name.as_str(), e.constructors.as_slice()))
    }

    /// [`Table::resolve_type`]'s answer as the table entry itself — what
    /// resolution needs, since it reports the variant's origin too.
    pub(super) fn entry_of_type(&self, ty: &str) -> Option<&Entry> {
        let trimmed = ty.trim();
        let base_len = trimmed
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'$' | b'.'))
            .count();
        if base_len == 0 {
            return None;
        }
        let rest = trimmed[base_len..].trim_start();
        let type_args = rest.starts_with('<') && rest.ends_with('>');
        if !rest.is_empty() && !type_args {
            return None; // a union, intersection, array, ... — not one variant
        }
        let base = &trimmed[..base_len];
        self.entries.iter().find(|e| e.name == base)
    }
}

impl Entry {
    pub(super) fn covered_variant(&self) -> CoveredVariant {
        CoveredVariant {
            name: self.name.clone(),
            origin: self.origin.clone(),
        }
    }
}

pub(super) fn walk(program: &Program, table: &Table, depth: Depth, out: &mut PatternAnalyses) {
    for segment in &program.segments {
        match segment {
            Segment::Verbatim(_)
            | Segment::TtImport(_)
            | Segment::Variant(_)
            | Segment::ValModifier(_) => {}
            Segment::Match(expr) => {
                let analysis = analyze_match(expr, table, depth);
                out.matches.push(analysis);
                walk(&expr.scrutinee, table, depth, out);
                for arm in &expr.arms {
                    if let Some(guard) = &arm.guard {
                        walk(&guard.expr, table, depth, out);
                    }
                    walk(&arm.body, table, depth, out);
                }
            }
            Segment::TupleMatch(expr) => {
                let analysis = analyze_tuple_match(expr, table, depth);
                out.matches.push(analysis);
                for (_, scrutinee) in &expr.scrutinees {
                    walk(scrutinee, table, depth, out);
                }
                for arm in &expr.arms {
                    if let Some(guard) = &arm.guard {
                        walk(&guard.expr, table, depth, out);
                    }
                    walk(&arm.body, table, depth, out);
                }
            }
            Segment::Try(stmt) => walk(&stmt.expr, table, depth, out),
            Segment::TryExpr(expr) => walk(&expr.expr, table, depth, out),
            Segment::LetElse(stmt) => {
                let site = analyze_let_else(stmt, table, depth);
                out.sites.push(site);
                walk(&stmt.expr, table, depth, out);
                walk(&stmt.else_body, table, depth, out);
            }
            Segment::IfLet(stmt) => walk_if_let(stmt, table, depth, out),
            Segment::Pipe(pipe) => {
                if let Some(head) = &pipe.head {
                    walk(head, table, depth, out);
                }
                for step in &pipe.steps {
                    walk(&step.body, table, depth, out);
                }
            }
            Segment::ResultBlock(block) => {
                for item in &block.items {
                    let ResultItem::Stmts(stmts) = item;
                    walk(stmts, table, depth, out);
                }
                if let Some(value) = &block.value {
                    walk(value, table, depth, out);
                }
            }
            Segment::Template(template) => {
                for chunk in &template.chunks {
                    if let TemplateChunk::Interp(interp) = chunk {
                        walk(interp, table, depth, out);
                    }
                }
            }
        }
    }
}

pub(super) fn walk_if_let(
    stmt: &IfLetStmt,
    table: &Table,
    depth: Depth,
    out: &mut PatternAnalyses,
) {
    let site = analyze_if_let(stmt, table, depth);
    out.sites.push(site);
    walk(&stmt.expr, table, depth, out);
    walk(&stmt.body, table, depth, out);
    match &stmt.else_part {
        Some(IfLetElse::Block(block)) => walk(block, table, depth, out),
        Some(IfLetElse::IfLet(inner)) => walk_if_let(inner, table, depth, out),
        None => {}
    }
}

pub(super) fn analyze_match(expr: &MatchExpr, table: &Table, depth: Depth) -> MatchAnalysis {
    // The subject is read from *every* arm's tags, guarded or not — the
    // type-reading counterpart of the resolver's identification (name
    // resolution itself is [`crate::resolve`]'s and attached afterwards).
    let tags: Vec<&str> = expr
        .arms
        .iter()
        .flat_map(|a| match &a.pattern {
            Pattern::Tags(alts) => alts.iter().map(|t| t.tag.as_str()).collect::<Vec<_>>(),
            Pattern::Wildcard | Pattern::Literals(_) | Pattern::Instances(_) => Vec::new(),
        })
        .collect();
    let subject = table.resolve(&tags);

    let arms = expr
        .arms
        .iter()
        .map(|arm| {
            let mut analyzed = AnalyzedArm {
                pattern_start: arm.pattern_span.start,
                body_start: arm.body_span.start,
                body_end: arm.body_span.end,
                pattern_bindings: Vec::new(),
                body_bindings: Vec::new(),
            };
            if let (Depth::Full, Pattern::Tags(alts)) = (depth, &arm.pattern) {
                analyze_group(alts, subject, table, &mut analyzed);
            }
            analyzed
        })
        .collect();

    let coverage = coverage_of(expr, table);
    MatchAnalysis {
        keyword_off: expr.keyword_off,
        head_end: expr.scrutinee_span.end + 1,
        body_open: expr.body_open,
        body_close: expr.body_close,
        subjects: vec![subject.map(to_subject)],
        arms,
        coverage,
    }
}

pub(super) fn analyze_tuple_match(
    expr: &TupleMatchExpr,
    table: &Table,
    depth: Depth,
) -> MatchAnalysis {
    let arity = expr.scrutinees.len();
    // Each position reads its subject independently, from the tags every
    // arm uses there.
    let subjects: Vec<Option<(&str, &[MatchConstructor])>> = (0..arity)
        .map(|p| {
            let tags: Vec<&str> = expr
                .arms
                .iter()
                .flat_map(|a| match &a.pattern {
                    TuplePattern::Elems(elems) => match elems.get(p) {
                        Some(Pattern::Tags(alts)) => {
                            alts.iter().map(|t| t.tag.as_str()).collect::<Vec<_>>()
                        }
                        _ => Vec::new(),
                    },
                    TuplePattern::Wildcard => Vec::new(),
                })
                .collect();
            table.resolve(&tags)
        })
        .collect();

    let arms = expr
        .arms
        .iter()
        .map(|arm| {
            let mut analyzed = AnalyzedArm {
                pattern_start: arm.pattern_span.start,
                body_start: arm.body_span.start,
                body_end: arm.body_span.end,
                pattern_bindings: Vec::new(),
                body_bindings: Vec::new(),
            };
            if let (Depth::Full, TuplePattern::Elems(elems)) = (depth, &arm.pattern) {
                for (p, elem) in elems.iter().enumerate() {
                    if let Pattern::Tags(alts) = elem {
                        analyze_group(
                            alts,
                            subjects.get(p).copied().flatten(),
                            table,
                            &mut analyzed,
                        );
                    }
                }
            }
            analyzed
        })
        .collect();

    MatchAnalysis {
        keyword_off: expr.keyword_off,
        head_end: expr
            .scrutinees
            .last()
            .map_or(expr.keyword_off, |(span, _)| span.end + 1),
        body_open: expr.body_open,
        body_close: expr.body_close,
        subjects: subjects.into_iter().map(|s| s.map(to_subject)).collect(),
        arms,
        coverage: tuple_coverage_of(expr, table),
    }
}

pub(super) fn to_subject((name, constructors): (&str, &[MatchConstructor])) -> MatchSubject {
    MatchSubject {
        variant_name: name.to_string(),
        constructors: constructors.to_vec(),
    }
}

/// Analyzes a let-else's pattern as a [`PatternSite`] — one or more
/// alias-only alternatives, no nested patterns.
pub(super) fn analyze_let_else(stmt: &LetElseStmt, table: &Table, depth: Depth) -> PatternSite {
    analyze_alt_site(
        SiteKind::LetElse,
        stmt.keyword_off,
        &stmt.alternatives,
        table,
        depth,
    )
}

/// Analyzes one `if let` link as a [`PatternSite`]. Chained `else if let`s
/// are separate sites, recorded by the walk.
pub(super) fn analyze_if_let(stmt: &IfLetStmt, table: &Table, depth: Depth) -> PatternSite {
    analyze_alt_site(
        SiteKind::IfLet,
        stmt.keyword_off,
        &stmt.alternatives,
        table,
        depth,
    )
}

/// The body both statement pattern sites share: identify the subject from
/// every alternative's tag — the same evidence rule a match arm list uses
/// — and record each alternative's bindings with their declared types,
/// occurrence spans kept apart exactly as [`analyze_group`] keeps a match
/// or-arm's. (Resolving the names — and the near-miss report when they do
/// not resolve — is [`crate::resolve`]'s, attached afterwards.)
pub(super) fn analyze_alt_site(
    kind: SiteKind,
    keyword_off: usize,
    alts: &[TagPattern],
    table: &Table,
    depth: Depth,
) -> PatternSite {
    let tags: Vec<&str> = alts.iter().map(|alt| alt.tag.as_str()).collect();
    let subject = table.resolve(&tags);
    let group = (alts[0].tag_off, alts.last().expect("non-empty").end);

    let mut pattern_bindings = Vec::new();
    if depth == Depth::Full {
        for alt in alts {
            let constructor = subject
                .and_then(|(_, cases)| cases.iter().find(|c| c.tag == alt.tag))
                .map(|c| (subject.expect("just matched").0, c));
            let mut leaves = Vec::new();
            collect_bindings(
                alt.bindings.as_deref().unwrap_or_default(),
                constructor,
                &alt.tag,
                table,
                &mut leaves,
            );
            for leaf in leaves {
                pattern_bindings.push(PatternBinding {
                    group_start: group.0,
                    group_end: group.1,
                    alt_start: alt.tag_off,
                    alt_end: alt.end,
                    alternatives: alts.len(),
                    ..leaf
                });
            }
        }
    }

    PatternSite {
        kind,
        keyword_off,
        subject: subject.map(to_subject),
        pattern_bindings,
    }
}

/// Analyzes one alternative list (`A(x) | B(x)`): every alternative
/// independently against the subject, occurrences recorded apart, body
/// bindings merged at the end — never the other way around.
pub(super) fn analyze_group(
    alts: &[TagPattern],
    subject: Option<(&str, &[MatchConstructor])>,
    table: &Table,
    arm: &mut AnalyzedArm,
) {
    let group = (alts[0].tag_off, alts.last().expect("non-empty").end);
    // Bound name → the type each alternative gives it, in source order.
    let mut merged: Vec<(String, Vec<Option<String>>)> = Vec::new();
    for alt in alts {
        let constructor = subject
            .and_then(|(_, cases)| cases.iter().find(|c| c.tag == alt.tag))
            .map(|c| (subject.expect("just matched").0, c));
        let mut leaves = Vec::new();
        collect_bindings(
            alt.bindings.as_deref().unwrap_or_default(),
            constructor,
            &alt.tag,
            table,
            &mut leaves,
        );
        for leaf in leaves {
            let binding = PatternBinding {
                group_start: group.0,
                group_end: group.1,
                alt_start: alt.tag_off,
                alt_end: alt.end,
                alternatives: alts.len(),
                ..leaf
            };
            match merged.iter_mut().find(|(name, _)| *name == binding.name) {
                Some((_, types)) => types.push(binding.ty.clone()),
                None => merged.push((binding.name.clone(), vec![binding.ty.clone()])),
            }
            arm.pattern_bindings.push(binding);
        }
    }
    for (name, types) in merged {
        arm.body_bindings.push(BodyBinding {
            ty: merge_types(&types),
            name,
        });
    }
}

/// Walks one alternative's bindings, nested patterns included, recording a
/// [`PatternBinding`] per leaf. `constructor` is `(variant name, constructor)`
/// when the expected type is known; group fields are filled by the caller.
pub(super) fn collect_bindings(
    bindings: &[Binding],
    constructor: Option<(&str, &MatchConstructor)>,
    tag: &str,
    table: &Table,
    out: &mut Vec<PatternBinding>,
) {
    for b in bindings {
        let field = constructor.and_then(|(_, c)| {
            c.fields
                .as_deref()
                .unwrap_or_default()
                .iter()
                .find(|f| f.name == b.name)
        });
        match &b.nested {
            Some(inner) => {
                // The field's declared type is the nested pattern's
                // expected type; resolve it to an variant and recurse.
                let nested_constructor =
                    field
                        .and_then(|f| table.resolve_type(&f.ty))
                        .and_then(|(name, cases)| {
                            cases.iter().find(|c| c.tag == inner.tag).map(|c| (name, c))
                        });
                collect_bindings(
                    inner.bindings.as_deref().unwrap_or_default(),
                    nested_constructor,
                    &inner.tag,
                    table,
                    out,
                );
            }
            None => {
                let (name, span) = match (&b.alias, b.alias_span) {
                    (Some(alias), Some(span)) => (alias.clone(), span),
                    _ => (b.name.clone(), b.name_span),
                };
                out.push(PatternBinding {
                    name,
                    start: span.start,
                    end: span.end,
                    tag: tag.to_string(),
                    ty: field.map(field_type),
                    variant_name: field
                        .and(constructor)
                        .map(|(variant_name, _)| variant_name.to_string()),
                    // Filled by the caller with the top-level group.
                    group_start: 0,
                    group_end: 0,
                    alt_start: 0,
                    alt_end: 0,
                    alternatives: 0,
                });
            }
        }
    }
}

/// The type a destructured binding sees: the declared text, `| undefined`
/// for an optional field (exactly what the emitted destructuring yields).
pub(super) fn field_type(field: &PayloadField) -> String {
    if field.optional {
        format!("{} | undefined", field.ty)
    } else {
        field.ty.clone()
    }
}

/// Merges one bound name's per-alternative types into what the body sees:
/// duplicates collapse, distinct types union in source order, and any
/// unknown makes the whole answer unknown — a partial union would claim
/// more than is known.
pub(super) fn merge_types(types: &[Option<String>]) -> Option<String> {
    let mut distinct: Vec<&str> = Vec::new();
    for ty in types {
        let ty = ty.as_deref()?;
        if !distinct.contains(&ty) {
            distinct.push(ty);
        }
    }
    match distinct.len() {
        0 => None,
        1 => Some(distinct[0].to_string()),
        _ => Some(distinct.join(" | ")),
    }
}

/// True when the alternative carries a nested pattern — like a guard, such
/// an arm may mismatch at runtime, so it identifies the variant but covers
/// nothing (sema's rule, and now the only copy of it).
pub(crate) fn has_nested(alt: &TagPattern) -> bool {
    alt.bindings
        .as_deref()
        .unwrap_or_default()
        .iter()
        .any(|b| b.nested.is_some())
}

/// Whether an arm covers what it matches: guarded arms and arms with a
/// nested pattern identify the subject but cover nothing.
pub(super) fn covers(guard: &Option<GuardExpr>, alts: &[TagPattern]) -> bool {
    guard.is_none() && !alts.iter().any(has_nested)
}
