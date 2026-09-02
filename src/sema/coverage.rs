//! Coverage diagnostics produced from shared match analysis.

use super::*;

/// Turns analysis coverage into positioned tt errors.
pub(super) fn report_coverage(
    source: &str,
    analyses: &crate::analysis::PatternAnalyses,
    suppressed: &[usize],
    errors: &mut Vec<TtError>,
) {
    let uncovered = analyses
        .matches
        .iter()
        .filter(|m| {
            !analyses.match_has_resolution_error(m.keyword_off)
                && !suppressed.contains(&m.keyword_off)
        })
        .filter_map(|m| m.coverage.as_ref().map(|c| (m, c)))
        .filter(|(_, c)| !c.missing.is_empty());

    for (analysis, coverage) in uncovered {
        let (offset, head_end) = (analysis.keyword_off, analysis.head_end);
        let message = if coverage.positions.len() == 1 {
            // A single match's one position always resolved — that is what
            // makes it a coverage answer at all.
            let Some(subject) = coverage.positions[0].as_ref() else {
                continue;
            };
            let missing: Vec<String> = coverage
                .missing_tags()
                .iter()
                .map(|m| format!("\"{m}\""))
                .collect();
            non_exhaustive_message(Some(&describe(subject)), &missing, false)
        } else {
            let names = coverage
                .positions
                .iter()
                .map(|p| p.as_ref().map_or("_", |e| e.name.as_str()))
                .collect::<Vec<_>>()
                .join(", ");
            let combinations: Vec<String> = coverage
                .missing
                .iter()
                .map(|row| format!("({})", row.pattern.join(", ")))
                .collect();
            non_exhaustive_message(Some(&format!("({names})")), &combinations, true)
        };
        // The arms that close the hole, written out: one per witness, each
        // position's binding form joined the way a tuple pattern is
        // written. Everything in the text comes from the analysis, so the
        // edit and the message answer from one model.
        let arms: Vec<String> = coverage
            .missing
            .iter()
            .map(|row| {
                if row.arm.len() > 1 {
                    format!("({})", row.arm.join(", "))
                } else {
                    row.arm.first().cloned().unwrap_or_else(|| "_".to_string())
                }
            })
            .collect();
        let mut error =
            TtError::span(offset, head_end, message).code(DiagnosticCode::MatchNotExhaustive);
        error.suggestions = non_exhaustive_suggestions(
            source,
            MatchSite {
                keyword_off: offset,
                body_open: analysis.body_open,
                body_close: analysis.body_close,
            },
            &arms,
        );
        errors.push(error);
    }
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
