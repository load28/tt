//! Projects computed engine answers into server protocol JSON.

use std::path::Path;
use ttc::engine::{Location, Range};
use ttc::lines::ProtocolPositions;

/// A diagnostic's suggestions as the JSON the protocol speaks.
///
/// The edit's byte offsets become the same 1-based line/column the
/// diagnostic itself is reported in, so one response never mixes two
/// coordinate spaces. `edit` is null for advice that names no replacement,
/// and for an edit whose source the server cannot resolve — a consumer
/// shows the message either way and only offers a fix when there is one.
pub(super) fn suggestions_json(
    suggestions: &[ttc::Suggestion],
    source: Option<&str>,
) -> serde_json::Value {
    use serde_json::json;
    let positions = source.map(ProtocolPositions::new);
    let positions = positions.as_ref();
    suggestions
        .iter()
        .map(|suggestion| {
            let edit = suggestion
                .edit
                .as_ref()
                .zip(positions)
                .map(|(edit, positions)| {
                    let (line, col) = positions.of_byte(edit.start);
                    let (end_line, end_col) = positions.of_byte(edit.end);
                    json!({
                        "line": line,
                        "col": col,
                        "endLine": end_line,
                        "endCol": end_col,
                        "replacement": edit.replacement,
                    })
                });
            json!({ "message": suggestion.message, "edit": edit })
        })
        .collect::<Vec<_>>()
        .into()
}

/// A [`Range`] as the JSON the protocol speaks.
pub(super) fn range_json(range: Range) -> serde_json::Value {
    serde_json::json!({
        "start": { "line": range.start.line, "character": range.start.character },
        "end": { "line": range.end.line, "character": range.end.character },
    })
}

/// A [`Location`] as the JSON the protocol speaks.
pub(super) fn symbol_json(symbol: &ttc::engine::DocumentSymbol) -> serde_json::Value {
    serde_json::json!({
        "name": symbol.name,
        "detail": symbol.detail,
        "kind": symbol.kind,
        "range": range_json(symbol.range),
        "selectionRange": range_json(symbol.selection_range),
        "children": symbol.children.iter().map(symbol_json).collect::<Vec<_>>(),
    })
}

pub(super) fn location_json(location: Location) -> serde_json::Value {
    serde_json::json!({
        "path": location.path,
        "range": range_json(location.range),
    })
}

pub(super) fn pattern_items_json(items: &[ttc::engine::TtCompletion]) -> Vec<serde_json::Value> {
    items
        .iter()
        .map(|item| {
            serde_json::json!({
                "label": item.label,
                "kind": match item.kind {
                    ttc::engine::TtCompletionKind::Case => "case",
                    ttc::engine::TtCompletionKind::Field => "field",
                    ttc::engine::TtCompletionKind::Literal => "literal",
                    ttc::engine::TtCompletionKind::Wildcard => "wildcard",
                },
                "detail": item.detail,
                "covered": item.covered,
                "range": item.range.map(range_json),
            })
        })
        .collect()
}

/// A diagnostic's secondary labeled spans as the JSON the protocol speaks:
/// 1-based line/column pairs like the diagnostic itself, plus the label's
/// words, and a `path` only when the span is in another file.
pub(super) fn labels_json<'a>(
    labels: &[ttc::engine::DiagnosticLabel],
    default_path: &Path,
    source_of: &dyn Fn(&Path) -> Option<&'a str>,
) -> serde_json::Value {
    use serde_json::json;
    labels
        .iter()
        .map(|label| {
            // A label carries a path only when it points into another
            // file, so its column is counted against that file's text and
            // otherwise against the diagnostic's own.
            let positions = source_of(label.path.as_deref().unwrap_or(default_path))
                .map(ProtocolPositions::new);
            let at = |position: (usize, usize)| match &positions {
                Some(positions) => positions.of_position(position),
                None => position,
            };
            let (line, col) = at(label.position);
            let (end_line, end_col) = at(label.end);
            let mut entry = json!({
                "line": line,
                "col": col,
                "endLine": end_line,
                "endCol": end_col,
                "message": label.message,
            });
            if let Some(path) = &label.path {
                entry["path"] = json!(path);
            }
            entry
        })
        .collect()
}
