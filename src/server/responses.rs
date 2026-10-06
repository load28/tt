//! Projects computed engine answers into server protocol JSON.

use std::path::Path;
use ttc::engine::{
    CompletionAnswer, CompletionDetail, Location, Range, ServiceDiagnostic, ServiceSeverity,
    ServiceTag, SignatureHelp,
};
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

/// Projects completion items, membership, and probe identity into protocol JSON.
pub(super) fn completion_json(answer: CompletionAnswer) -> serde_json::Value {
    use serde_json::json;
    let CompletionAnswer {
        items,
        member,
        probe,
    } = answer;
    json!({
        "items": items.iter().map(|item| json!({
            "label": item.label,
            "kind": item.kind.map(|kind| kind.lsp()),
            "tags": item.tags.iter().map(|tag| tag.lsp()).collect::<Vec<_>>(),
            "sortText": item.sort_text,
            "insertText": item.insert_text,
            "filterText": item.filter_text,
            "snippet": item.snippet,
            "range": item.range.map(range_json),
            "source": item.source,
            "detail": item.detail,
            "labelDetails": (item.label_detail.is_some() || item.description.is_some())
                .then(|| json!({
                    "detail": item.label_detail,
                    "description": item.description,
                })),
        })).collect::<Vec<_>>(),
        "member": member,
        "probe": probe,
    })
}

/// Projects a resolved completion and its optional additional edits into protocol JSON.
pub(super) fn completion_detail_json(detail: Option<CompletionDetail>) -> serde_json::Value {
    use serde_json::json;
    match detail {
        None => serde_json::Value::Null,
        Some(detail) => {
            let mut answer = json!({
                "signature": detail.signature,
                "documentation": detail.documentation,
            });
            if !detail.additional_edits.is_empty() {
                answer["additionalEdits"] = detail
                    .additional_edits
                    .into_iter()
                    .map(|edit| {
                        json!({
                            "range": range_json(edit.range),
                            "newText": edit.new_text,
                        })
                    })
                    .collect();
            }
            answer
        }
    }
}

/// Projects signature labels, parameter offsets, and active indices into protocol JSON.
pub(super) fn signature_help_json(help: Option<SignatureHelp>) -> serde_json::Value {
    use serde_json::json;
    match help {
        None => serde_json::Value::Null,
        Some(help) => json!({
            "signatures": help.signatures.iter().map(|signature| json!({
                "label": signature.label,
                "documentation": signature.documentation,
                "parameters": signature.parameters.iter().map(|parameter| json!({
                    "label": [parameter.label.0, parameter.label.1],
                    "documentation": parameter.documentation,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "activeSignature": help.active_signature,
            "activeParameter": help.active_parameter,
        }),
    }
}

/// Projects a service diagnostic and its optional tags and related spans into protocol JSON.
pub(super) fn service_diagnostic_json(d: ServiceDiagnostic) -> serde_json::Value {
    use serde_json::json;
    let mut entry = json!({
        "range": range_json(d.range),
        "message": d.message,
        "code": d.code,
        "severity": match d.severity {
            ServiceSeverity::Error => "error",
            ServiceSeverity::Warning => "warning",
            ServiceSeverity::Information => "information",
            ServiceSeverity::Hint => "hint",
        },
    });
    if !d.tags.is_empty() {
        entry["tags"] = d
            .tags
            .iter()
            .map(|tag| match tag {
                ServiceTag::Unnecessary => "unnecessary",
                ServiceTag::Deprecated => "deprecated",
            })
            .collect();
    }
    // Secondary labeled spans ride only when there are any,
    // so consumers of the existing shape see no new field
    // until a diagnostic actually carries one.
    if !d.related.is_empty() {
        entry["related"] = d
            .related
            .iter()
            .map(|r| {
                let mut related = json!({
                    "range": range_json(r.range),
                    "message": r.message,
                });
                if let Some(path) = &r.path {
                    related["path"] = json!(path);
                }
                related
            })
            .collect();
    }
    entry
}

/// A [`ttc::engine::TtSymbol`] as `ttSymbol` and `patternSymbol` answer it.
pub(super) fn tt_symbol_json(symbol: ttc::engine::TtSymbol) -> serde_json::Value {
    serde_json::json!({
        "kind": match symbol.kind {
            ttc::engine::TtSymbolKind::Variant => "variant",
            ttc::engine::TtSymbolKind::Case => "case",
            ttc::engine::TtSymbolKind::Field => "field",
        },
        "range": range_json(symbol.range),
        "name": symbol.name,
        "variantName": symbol.variant_name,
        "signature": symbol.signature,
        "detail": symbol.detail,
        "definition": symbol.definition.map(|location| serde_json::json!({
            "path": location.path.to_string_lossy(),
            "range": range_json(location.range),
        })),
        "binds": symbol.binds,
    })
}
