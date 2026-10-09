//! Mapping service edits and navigation targets into authored source coordinates.

use super::*;

/// An edit the service computed over served text, as an edit of `source`:
/// `mappings` maps `source` onto `code`, with a completion probe's
/// placeholder spliced in at `splice` when there is one. An insertion
/// before or after a declaration of glue written at a source point
/// (`inserted`) is an insertion at that point. `None` when either end of
/// the range was not copied from the source, when the edit changes glue,
/// or when it falls inside the placeholder.
pub(in super::super) fn source_edit(
    code: &str,
    mappings: &[EmitMapping],
    inserted: &[crate::InsertedGlue],
    source: &str,
    splice: Option<usize>,
    edit: &serde_json::Value,
) -> Option<TextEdit> {
    let (start, end) = source_byte_span(code, mappings, inserted, splice, &edit["range"])?;
    Some(TextEdit {
        range: span_range(source, start, end),
        new_text: edit["newText"].as_str()?.to_string(),
    })
}

pub(in super::super) fn source_byte_span(
    code: &str,
    mappings: &[EmitMapping],
    inserted: &[crate::InsertedGlue],
    splice: Option<usize>,
    range: &serde_json::Value,
) -> Option<(usize, usize)> {
    let start = mapper::from_utf16(code, u16_offset(code, position_of(&range["start"])));
    let end = mapper::from_utf16(code, u16_offset(code, position_of(&range["end"])));
    let (start, end) = mapper::to_source_span(mappings, start, end).or_else(|| {
        if start == 0 && end == 0 {
            return Some((0, 0));
        }
        let glue = inserted
            .iter()
            .find(|glue| start == end && (start == glue.out || start == glue.out_end))?;
        Some((glue.src, glue.src))
    })?;
    let unsplice = |byte: usize| match splice {
        Some(at) if byte > at => byte.checked_sub(PROBE_NAME.len()).filter(|&b| b >= at),
        _ => Some(byte),
    };
    Some((unsplice(start)?, unsplice(end)?))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum TargetUse {
    Navigation,
    Edit,
}

enum TargetCoordinates {
    Authored(PathBuf),
    Projected(PathBuf),
}

/// A service document carries the coordinates of the text we opened. A
/// virtual `.tt.ts`/`.ttx.tsx` document also names that projection. Other
/// source URIs carry authored coordinates, including targets TypeScript
/// has already followed through a declaration map. Merely having a cached
/// projection of a source does not make that source URI a projection.
fn target_coordinates(_session: &ServiceSession, uri: &str) -> Option<TargetCoordinates> {
    let path = uri_path(uri)?;
    let Some(source) = tt_document(&path) else {
        return Some(TargetCoordinates::Authored(path));
    };
    if source != path {
        Some(TargetCoordinates::Projected(source))
    } else {
        Some(TargetCoordinates::Authored(path))
    }
}

/// Maps one service answer target back to a user-visible file. `None` when
/// the target is not a file, cannot be read, or the span has no source
/// counterpart for `purpose` — the caller decides whether that skips one
/// result (navigation) or refuses the whole operation (rename).
/// Whether `range` of the projected document `uri` names lies in text the
/// lowering restates from the source (a capture's `typeof` query): the
/// lowering writes it again from the source, so an edit of the source
/// covers it.
pub(in super::super) fn restated_target(
    session: &mut ServiceSession,
    overlays: &HashMap<PathBuf, String>,
    uri: &str,
    range: &serde_json::Value,
) -> bool {
    let Some(TargetCoordinates::Projected(tt_path)) = target_coordinates(session, uri) else {
        return false;
    };
    let Some(doc) = serve_doc_only(session, overlays, &tt_path) else {
        return false;
    };
    let code_lines = doc.code_lines();
    let start = byte_at(&code_lines, position_of(&range["start"]));
    let end = byte_at(&code_lines, position_of(&range["end"]));
    crate::engine::projection::restated(&doc.restatements, start, end)
}

pub(in super::super) fn map_target(
    session: &mut ServiceSession,
    overlays: &HashMap<PathBuf, String>,
    uri: &str,
    range: &serde_json::Value,
    purpose: TargetUse,
) -> Option<Location> {
    let target = target_coordinates(session, uri)?;
    let lsp_range = Range {
        start: position_of(&range["start"]),
        end: position_of(&range["end"]),
    };
    let path = match target {
        TargetCoordinates::Authored(path) => {
            if crate::SourceKind::from_tt_path(&path).is_some()
                && let Some(doc) = serve_doc_only(session, overlays, &path)
            {
                let lines = doc.source_lines();
                let start = byte_at(&lines, lsp_range.start);
                let end = byte_at(&lines, lsp_range.end);
                if authored_shared_binding(&doc, start, end).is_some() {
                    return None;
                }
            }
            path
        }
        TargetCoordinates::Projected(tt_path) => {
            let doc = serve_doc_only(session, overlays, &tt_path)?;
            let code_lines = doc.code_lines();
            let start = byte_at(&code_lines, lsp_range.start);
            let end = byte_at(&code_lines, lsp_range.end);
            let (s, e) = match mapper::to_source_span(&doc.mappings, start, end) {
                Some(span) => span,
                None if start == 0 && end == 0 => (0, 0),
                None if start == 0 && end == doc.code.len() => (0, doc.source.len()),
                None if purpose == TargetUse::Navigation => declared_name_span(&doc, start, end)?,
                None => return None,
            };
            let source_lines = doc.source_lines();
            return Some(Location {
                path: tt_path,
                range: Range {
                    start: byte_position(&source_lines, s),
                    end: byte_position(&source_lines, e),
                },
            });
        }
    };
    // An authored target already uses the file's own coordinates.
    Some(Location {
        path,
        range: lsp_range,
    })
}

pub(in super::super) struct SharedTarget {
    pub location: Location,
    pub name: String,
    pub shorthand: bool,
}

fn authored_shared_binding(
    doc: &ServiceDoc,
    start: usize,
    end: usize,
) -> Option<&crate::SharedBinding> {
    doc.shared_bindings.iter().find(|binding| {
        binding
            .occurrences
            .iter()
            .any(|occurrence| occurrence.src == start && occurrence.src_end == end)
    })
}

pub(in super::super) fn map_shared_target(
    session: &mut ServiceSession,
    overlays: &HashMap<PathBuf, String>,
    uri: &str,
    range: &serde_json::Value,
) -> Option<(String, Vec<SharedTarget>)> {
    let (tt_path, authored) = match target_coordinates(session, uri)? {
        TargetCoordinates::Authored(path) => (path, true),
        TargetCoordinates::Projected(path) => (path, false),
    };
    let doc = serve_doc_only(session, overlays, &tt_path)?;
    let code = if authored { &doc.source } else { &doc.code };
    let start = mapper::from_utf16(code, u16_offset(code, position_of(&range["start"])));
    let end = mapper::from_utf16(code, u16_offset(code, position_of(&range["end"])));
    let binding = if authored {
        authored_shared_binding(&doc, start, end)
    } else {
        doc.shared_bindings
            .iter()
            .find(|binding| binding.out == start && binding.out_end == end)
    }?;
    let targets = binding
        .occurrences
        .iter()
        .map(|occurrence| SharedTarget {
            location: Location {
                path: tt_path.clone(),
                range: span_range(&doc.source, occurrence.src, occurrence.src_end),
            },
            name: doc.source[occurrence.src..occurrence.src_end].to_string(),
            shorthand: occurrence.shorthand,
        })
        .collect();
    Some((doc.code[binding.out..binding.out_end].to_string(), targets))
}
