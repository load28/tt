//! Coverage diagnostics produced from shared match analysis.

use super::*;

/// Turns analysis coverage into positioned tt errors.
pub(crate) fn report_coverage(
    source: &str,
    analyses: &crate::analysis::PatternAnalyses,
    errors: &mut Vec<TtError>,
) {
    let uncovered = analyses
        .matches
        .iter()
        .filter(|m| !analyses.match_has_resolution_error(m.keyword_off))
        .filter_map(|m| m.coverage.as_ref().map(|c| (m, c)))
        .filter(|(_, c)| !c.missing.is_empty());

    for (analysis, coverage) in uncovered {
        let (offset, head_end) = (analysis.keyword_off, analysis.head_end);
        let Some(hole) = non_exhaustive(coverage, Witnesses::All) else {
            continue;
        };
        let (message, arms) = (hole.message, hole.arms);
        let mut error =
            TtError::span(offset, head_end, message).code(DiagnosticCode::MatchNotExhaustive);
        error.suggestions = non_exhaustive_suggestions(
            source,
            MatchSite {
                keyword_off: offset,
                body_open: analysis.body_open,
                body_close: analysis.body_close,
                tail: analysis.tail,
            },
            &arms,
        );
        errors.push(error);
    }
}

/// Which of a coverage's witnesses a report states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Witnesses {
    /// Every witness: the default path, which has no better oracle for a
    /// column whose alphabet it could not identify.
    All,
    /// The witnesses decided from known constructor sets: the typed path,
    /// where the honest answer for an unidentified column is the checker's.
    Certain,
}

/// A match's coverage hole as both pipelines report it: the message, and
/// the arms that close it when the witnesses are all of them.
pub(crate) struct Hole {
    pub(crate) message: String,
    pub(crate) arms: Vec<String>,
}

/// The one rendering of a match's coverage hole. A single match's witness
/// is quoted (`"Point"`); a tuple match's is a combination written `(a, b)`
/// and left unquoted, since quotes would read as part of the pattern. The
/// subject is the variant each position is read against, `_` for a
/// position that constrains nothing. `None` when the coverage leaves no
/// witness of the kind asked for.
pub(crate) fn non_exhaustive(
    coverage: &crate::analysis::Coverage,
    witnesses: Witnesses,
) -> Option<Hole> {
    let shown: Vec<&crate::analysis::Uncovered> = coverage
        .missing
        .iter()
        .filter(|row| witnesses == Witnesses::All || row.certain)
        .collect();
    if shown.is_empty() {
        return None;
    }
    let total = match witnesses {
        Witnesses::All => coverage.total,
        Witnesses::Certain => coverage.certain_total,
    };
    let tuple = coverage.positions.len() > 1;
    let subject = if tuple {
        let names = coverage
            .positions
            .iter()
            .map(|p| p.as_ref().map_or("_", |e| e.name.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        Some(format!("({names})"))
    } else {
        coverage
            .positions
            .first()
            .and_then(Option::as_ref)
            .map(describe)
    };
    let missing: Vec<String> = shown
        .iter()
        .map(|row| {
            if tuple {
                format!("({})", row.pattern.join(", "))
            } else {
                format!("\"{}\"", row.pattern.first().map_or("", String::as_str))
            }
        })
        .collect();
    let whole = coverage.exact && shown.len() == total;
    let arms = shown
        .iter()
        .filter(|_| whole)
        .map(|row| {
            if row.arm.len() > 1 {
                format!("({})", row.arm.join(", "))
            } else {
                row.arm.first().cloned().unwrap_or_else(|| "_".to_string())
            }
        })
        .collect();
    Some(Hole {
        message: non_exhaustive_message(subject.as_deref(), &missing, total, coverage.exact, tuple),
        arms,
    })
}

/// How an error names the variant a match is over — the declaration's origin,
/// so "which `Token`?" is answerable from the message alone.
pub(super) fn describe(subject: &CoveredVariant) -> String {
    match &subject.origin {
        Origin::Local => format!("variant {}", subject.name),
        Origin::Builtin => format!("built-in variant {}", subject.name),
        Origin::Imported { from: Some(from) } => {
            format!("variant {} (imported from \"{from}\")", subject.name)
        }
        Origin::Imported { from: None } => format!("imported variant {}", subject.name),
    }
}
