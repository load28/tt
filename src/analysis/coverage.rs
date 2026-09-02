//! Match coverage, witness, and unreachable-arm analysis.

use super::*;

/// [`Coverage`] of a single match, when the question means something: a tag
/// match with no wildcard arm whose tags identify a known variant.
///
/// The arms become a one-column matrix and the algorithm answers
/// ([`usefulness`]). Guarded arms stay out of it — a guard may be false —
/// but arms carrying nested patterns are now *in*: the recursion descends
/// into the payload, so such an arm covers exactly what it covers instead
/// of being written off.
pub(super) fn coverage_of(expr: &MatchExpr, table: &Table) -> Option<Coverage> {
    let rows = match_rows(expr)?;
    // Several variants can hold every tag. The one the arms *satisfy* is the
    // subject if there is one; otherwise the one they leave least of —
    // the rule sema has always reported, now measured in witnesses.
    let cx = Alphabets::of(table);
    let mut best: Option<(&Entry, Vec<Uncovered>)> = None;
    for entry in table.candidates(&rows.tags) {
        let types = [ColTy::Variant(entry)];
        let missing = render_witnesses(&usefulness::missing(&rows.rows, &types, &cx));
        if missing.is_empty() {
            best = Some((entry, missing));
            break;
        }
        if best.as_ref().is_none_or(|(_, m)| missing.len() < m.len()) {
            best = Some((entry, missing));
        }
    }
    let (entry, missing) = best?;
    Some(Coverage {
        positions: vec![Some(entry.covered_variant())],
        covered: rows.covered,
        missing,
        unreachable: unreachable_arms(&rows.arm_rows, &[ColTy::Variant(entry)], &cx),
    })
}

/// The same answer for a subject the caller names — the typed path, where
/// the checker says which constituents the scrutinee's type still has and
/// tt runs its own algorithm over that alphabet. One algorithm, a better
/// oracle for the one column the checker can speak about.
pub(crate) fn checked_coverage(
    source: &str,
    externs: &[VariantSymbol],
    members: &[(usize, Vec<Vec<String>>)],
    payloads: &[PayloadAlphabet],
) -> Vec<(usize, Coverage)> {
    let program = crate::parser::parse(source);
    let decls: Vec<crate::resolve::ExternDecl> = externs.iter().map(Into::into).collect();
    let mut hir = crate::hir::lower_program(crate::hir::FileId(0), source, &program);
    let resolution = crate::resolve::resolve_file(&mut hir, &decls);
    let table = Table::from_resolution(&resolution);
    let mut found = Vec::new();
    let mut matches = Vec::new();
    let mut tuples = Vec::new();
    collect_matches(&program, &mut matches, &mut tuples);
    for expr in matches {
        let Some((_, positions)) = members.iter().find(|(at, _)| *at == expr.keyword_off) else {
            continue;
        };
        let Some(tags) = positions.first() else {
            continue;
        };
        let entry = table.entry_of_members(tags);
        let Some(rows) = match_rows(expr) else {
            continue;
        };
        let cx = Alphabets {
            table: &table,
            payloads: payloads
                .iter()
                .map(|((tag, field), members)| {
                    (
                        (tag.clone(), field.clone()),
                        table.entry_of_members(members),
                    )
                })
                .collect(),
        };
        let types = [ColTy::Variant(&entry)];
        let missing = render_witnesses(&usefulness::missing(&rows.rows, &types, &cx));
        found.push((
            expr.keyword_off,
            Coverage {
                positions: vec![None],
                covered: rows.covered,
                missing,
                unreachable: unreachable_arms(&rows.arm_rows, &types, &cx),
            },
        ));
    }

    // A tuple match asks one question per position and enumerates the
    // product, exactly as the default path does — the only difference is
    // where each column's alphabet came from.
    for expr in tuples {
        let Some((_, positions)) = members.iter().find(|(at, _)| *at == expr.keyword_off) else {
            continue;
        };
        let arity = expr.scrutinees.len();
        if positions.len() != arity {
            continue;
        }
        let written = tuple_position_tags(expr, arity);
        let entries: Vec<Option<Entry>> = positions
            .iter()
            .enumerate()
            .map(|(index, tags)| {
                // A position no arm writes a tag at constrains nothing, and
                // saying `_` there is a shorter true answer than
                // enumerating a column nobody asked about.
                (!written[index].is_empty()).then(|| table.entry_of_members(tags))
            })
            .collect();
        let types: Vec<ColTy> = entries
            .iter()
            .map(|entry| match entry {
                Some(entry) => ColTy::Variant(entry),
                None => ColTy::Unconstrained,
            })
            .collect();
        let mut rows: Vec<Vec<Cell>> = Vec::new();
        let mut arm_rows: Vec<(usize, Vec<Vec<Cell>>)> = Vec::new();
        for (index, arm) in expr.arms.iter().enumerate() {
            let TuplePattern::Elems(elems) = &arm.pattern else {
                continue;
            };
            if elems.len() != arity || arm.guard.is_some() {
                continue;
            }
            let Some(this) = tuple_rows(elems) else {
                continue;
            };
            arm_rows.push((index, this.clone()));
            rows.extend(this);
        }
        let cx = Alphabets {
            table: &table,
            payloads: payloads
                .iter()
                .map(|((tag, field), members)| {
                    (
                        (tag.clone(), field.clone()),
                        table.entry_of_members(members),
                    )
                })
                .collect(),
        };
        found.push((
            expr.keyword_off,
            Coverage {
                positions: vec![None; arity],
                covered: Vec::new(),
                missing: render_witnesses(&usefulness::missing(&rows, &types, &cx)),
                unreachable: unreachable_arms(&arm_rows, &types, &cx),
            },
        ));
    }
    found
}

/// The tags any arm writes at each position — what says whether a position
/// constrains anything at all.
pub(super) fn tuple_position_tags(expr: &TupleMatchExpr, arity: usize) -> Vec<Vec<&str>> {
    let mut out: Vec<Vec<&str>> = vec![Vec::new(); arity];
    for arm in &expr.arms {
        let TuplePattern::Elems(elems) = &arm.pattern else {
            continue;
        };
        if elems.len() != arity {
            continue;
        }
        for (position, elem) in elems.iter().enumerate() {
            if let Pattern::Tags(alts) = elem {
                for alt in alts {
                    if !out[position].contains(&alt.tag.as_str()) {
                        out[position].push(&alt.tag);
                    }
                }
            }
        }
    }
    out
}

/// Every single `match` of a program, nested ones included, in source
/// order.
pub(super) fn collect_matches<'a>(
    program: &'a Program,
    out: &mut Vec<&'a MatchExpr>,
    tuples: &mut Vec<&'a TupleMatchExpr>,
) {
    for segment in &program.segments {
        match segment {
            Segment::Match(expr) => {
                out.push(expr);
                collect_matches(&expr.scrutinee, out, tuples);
                for arm in &expr.arms {
                    if let Some(guard) = &arm.guard {
                        collect_matches(&guard.expr, out, tuples);
                    }
                    collect_matches(&arm.body, out, tuples);
                }
            }
            Segment::TupleMatch(expr) => {
                tuples.push(expr);
                for (_, scrutinee) in &expr.scrutinees {
                    collect_matches(scrutinee, out, tuples);
                }
                for arm in &expr.arms {
                    if let Some(guard) = &arm.guard {
                        collect_matches(&guard.expr, out, tuples);
                    }
                    collect_matches(&arm.body, out, tuples);
                }
            }
            Segment::Try(stmt) => collect_matches(&stmt.expr, out, tuples),
            Segment::TryExpr(expr) => collect_matches(&expr.expr, out, tuples),
            Segment::LetElse(stmt) => {
                collect_matches(&stmt.expr, out, tuples);
                collect_matches(&stmt.else_body, out, tuples);
            }
            Segment::IfLet(stmt) => collect_if_let_matches(stmt, out, tuples),
            Segment::Pipe(pipe) => {
                if let Some(head) = &pipe.head {
                    collect_matches(head, out, tuples);
                }
                for step in &pipe.steps {
                    collect_matches(&step.body, out, tuples);
                }
            }
            Segment::ResultBlock(block) => {
                for item in &block.items {
                    let ResultItem::Stmts(stmts) = item;
                    collect_matches(stmts, out, tuples);
                }
                if let Some(value) = &block.value {
                    collect_matches(value, out, tuples);
                }
            }
            Segment::Template(template) => {
                for chunk in &template.chunks {
                    if let TemplateChunk::Interp(interp) = chunk {
                        collect_matches(interp, out, tuples);
                    }
                }
            }
            Segment::Verbatim(_)
            | Segment::TtImport(_)
            | Segment::Variant(_)
            | Segment::ValModifier(_) => {}
        }
    }
}

pub(super) fn collect_if_let_matches<'a>(
    stmt: &'a IfLetStmt,
    out: &mut Vec<&'a MatchExpr>,
    tuples: &mut Vec<&'a TupleMatchExpr>,
) {
    collect_matches(&stmt.expr, out, tuples);
    collect_matches(&stmt.body, out, tuples);
    match &stmt.else_part {
        Some(IfLetElse::Block(block)) => collect_matches(block, out, tuples),
        Some(IfLetElse::IfLet(inner)) => collect_if_let_matches(inner, out, tuples),
        None => {}
    }
}

/// One match's arms as the algorithm's input: the tags they name, the tags
/// they cover outright, the matrix, and the per-arm rows reachability
/// needs. `None` when the question does not arise — a wildcard arm covers
/// everything, or no arm carries a tag pattern.
pub(super) struct MatchRows<'a> {
    tags: Vec<&'a str>,
    covered: Vec<String>,
    rows: Vec<Vec<Cell<'a>>>,
    arm_rows: Vec<(usize, Vec<Vec<Cell<'a>>>)>,
}

pub(super) fn match_rows(expr: &MatchExpr) -> Option<MatchRows<'_>> {
    if expr
        .arms
        .iter()
        .any(|a| matches!(a.pattern, Pattern::Wildcard))
    {
        return None;
    }
    // Identification uses every arm's tags, guarded ones included.
    let mut tags: Vec<&str> = Vec::new();
    let mut covered: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let mut arm_rows: Vec<(usize, Vec<Vec<Cell>>)> = Vec::new();
    for (index, arm) in expr.arms.iter().enumerate() {
        let Pattern::Tags(alts) = &arm.pattern else {
            continue;
        };
        for alt in alts {
            if !tags.contains(&alt.tag.as_str()) {
                tags.push(&alt.tag);
            }
        }
        if arm.guard.is_some() {
            continue;
        }
        // An or-pattern is several rows: each alternative stands alone.
        let this: Vec<Vec<Cell>> = alts.iter().map(|alt| vec![Cell::Tag(alt)]).collect();
        arm_rows.push((index, this.clone()));
        rows.extend(this);
        if covers(&arm.guard, alts) {
            for alt in alts {
                if !covered.contains(&alt.tag) {
                    covered.push(alt.tag.clone());
                }
            }
        }
    }
    if tags.is_empty() {
        return None;
    }
    Some(MatchRows {
        tags,
        covered,
        rows,
        arm_rows,
    })
}

/// [`Coverage`] of a tuple match: the same algorithm over as many columns
/// as there are scrutinees. `None` when a bare `_` arm covers everything,
/// when a tagged position resolves to no known variant, or when no position
/// is tagged at all (nothing to enumerate).
pub(super) fn tuple_coverage_of(expr: &TupleMatchExpr, table: &Table) -> Option<Coverage> {
    let arity = expr.scrutinees.len();
    if expr
        .arms
        .iter()
        .any(|a| matches!(a.pattern, TuplePattern::Wildcard))
    {
        return None;
    }

    // Per position, the tags any arm writes there — identification, as in
    // a single match but one column at a time.
    let mut position_tags: Vec<Vec<&str>> = vec![Vec::new(); arity];
    for arm in &expr.arms {
        let TuplePattern::Elems(elems) = &arm.pattern else {
            continue;
        };
        if elems.len() != arity {
            continue; // sema reports the arity mismatch
        }
        for (position, elem) in elems.iter().enumerate() {
            if let Pattern::Tags(alts) = elem {
                for alt in alts {
                    if !position_tags[position].contains(&alt.tag.as_str()) {
                        position_tags[position].push(&alt.tag);
                    }
                }
            }
        }
    }

    let mut positions: Vec<Option<CoveredVariant>> = Vec::with_capacity(arity);
    let mut types: Vec<ColTy> = Vec::with_capacity(arity);
    for tags in &position_tags {
        if tags.is_empty() {
            // Universal position: only `_` was written here.
            positions.push(None);
            types.push(ColTy::Unconstrained);
            continue;
        }
        // A position whose tags name no variant makes the whole question
        // unanswerable — the same conservatism as before.
        let entry = *table.candidates(tags).first()?;
        positions.push(Some(entry.covered_variant()));
        types.push(ColTy::Variant(entry));
    }
    if positions.iter().all(Option::is_none) {
        return None;
    }

    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let mut arm_rows: Vec<(usize, Vec<Vec<Cell>>)> = Vec::new();
    for (index, arm) in expr.arms.iter().enumerate() {
        let TuplePattern::Elems(elems) = &arm.pattern else {
            continue;
        };
        if elems.len() != arity || arm.guard.is_some() {
            continue;
        }
        let Some(this) = tuple_rows(elems) else {
            continue;
        };
        arm_rows.push((index, this.clone()));
        rows.extend(this);
    }

    let cx = Alphabets::of(table);
    Some(Coverage {
        positions,
        covered: Vec::new(),
        missing: render_witnesses(&usefulness::missing(&rows, &types, &cx)),
        unreachable: unreachable_arms(&arm_rows, &types, &cx),
    })
}

/// One tuple arm as rows: the cartesian product of its elements'
/// alternatives, since `(A | B, C)` matches two combinations. `None` when
/// the arm can match no combination of tags at all (a literal element),
/// which is the same as contributing no row.
pub(super) fn tuple_rows<'a>(elems: &'a [Pattern]) -> Option<Vec<Vec<Cell<'a>>>> {
    let mut rows: Vec<Vec<Cell>> = vec![Vec::new()];
    for elem in elems {
        let cells: Vec<Cell> = match elem {
            Pattern::Wildcard => vec![Cell::Wild],
            Pattern::Literals(_) | Pattern::Instances(_) => return None,
            Pattern::Tags(alts) => alts.iter().map(Cell::Tag).collect(),
        };
        rows = rows
            .into_iter()
            .flat_map(|row| {
                cells.iter().map(move |cell| {
                    let mut next = row.clone();
                    next.push(*cell);
                    next
                })
            })
            .collect();
    }
    Some(rows)
}

/// The arms that match nothing an earlier arm has not: each arm's rows
/// against every row before it.
pub(super) fn unreachable_arms<'a>(
    arm_rows: &[(usize, Vec<Vec<Cell<'a>>>)],
    types: &[ColTy<'a>],
    cx: &'a Alphabets<'a>,
) -> Vec<usize> {
    let mut seen: Vec<Vec<Cell>> = Vec::new();
    let mut out = Vec::new();
    for (index, rows) in arm_rows {
        let useful = rows
            .iter()
            .any(|row| usefulness::is_useful(&seen, row, types, cx));
        if !useful {
            out.push(*index);
        }
        seen.extend(rows.iter().cloned());
    }
    out
}

pub(super) fn render_witnesses(found: &[Vec<usefulness::Witness>]) -> Vec<Uncovered> {
    found
        .iter()
        .map(|row| Uncovered {
            pattern: row.iter().map(usefulness::Witness::render).collect(),
            arm: row.iter().map(usefulness::Witness::arm).collect(),
            certain: row.iter().all(usefulness::Witness::certain),
        })
        .collect()
}

/// The identifier span containing `offset`, byte-based like the scanner:
/// ASCII identifier bytes plus opaque multi-byte UTF-8.
pub(super) fn identifier_at(source: &str, offset: usize) -> Option<(usize, usize)> {
    let bytes = source.as_bytes();
    if offset >= bytes.len() {
        return None;
    }
    let is_ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'$' || b >= 0x80;
    if !is_ident(bytes[offset]) {
        return None;
    }
    let mut start = offset;
    while start > 0 && is_ident(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = offset;
    while end < bytes.len() && is_ident(bytes[end]) {
        end += 1;
    }
    if bytes[start].is_ascii_digit() {
        return None; // a number, not a name
    }
    Some((start, end))
}
