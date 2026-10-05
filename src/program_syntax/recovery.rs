//! A skipped host production cannot establish placement for tt values inside it.
//! Known tt owners retain their incomplete operands for editor queries.

use super::*;

pub(crate) fn lost_editor_values(
    semantic: &SemanticFile,
    core: &CoreFile,
    source: &str,
    source_kind: crate::SourceKind,
    tokens: &[crate::lexer::Token],
) -> Result<Vec<SourceSpan>, ProgramSyntaxError> {
    let projection = projection::ProjectionBuilder::new(semantic, core, source, tokens).build()?;
    let parsed = parse_module(
        &projection.code,
        &projection.source_segments,
        source_kind,
        SyntaxMode::Editor,
    )?;
    Ok(projection
        .pending
        .iter()
        .filter(|entry| {
            // Statement/declaration recovery must retain their known exports rather
            // than treating them as expression values.
            matches!(entry.category, SyntaxCategory::Expression)
                && parsed.recoveries.iter().any(|cause| {
                    cause.kind == swc_ecma_parser::RecoveryKind::SkippedInput
                        && parsed.start.byte(cause.span.lo) <= entry.projected.start.0
                        && entry.projected.end.0 <= parsed.start.byte(cause.span.hi)
                })
        })
        .map(|entry| entry.source)
        .collect())
}

/// Missing delimiters recorded by the host parser, in original coordinates.
/// The insertion point precedes the next token owned by the enclosing grammar.
/// Inserted trivia separates it from the following declaration and its prelude.
pub(crate) fn editor_delimiters(
    semantic: &SemanticFile,
    core: &CoreFile,
    source: &str,
    source_kind: crate::SourceKind,
    tokens: &[crate::lexer::Token],
) -> Result<Vec<(usize, String, SourceSpan)>, ProgramSyntaxError> {
    let projection = projection::ProjectionBuilder::new(semantic, core, source, tokens).build()?;
    let parsed = parse_module(
        &projection.code,
        &projection.source_segments,
        source_kind,
        SyntaxMode::Editor,
    )?;
    let source_at = |at: usize| {
        source_byte_for_projection(&projection.source_segments, ProjectedByte(at))
            .or_else(|| (at == projection.code.len()).then_some(source.len()))
    };
    Ok(parsed
        .recoveries
        .iter()
        .filter_map(|record| {
            if record.context != swc_ecma_parser::RecoveryContext::Delimiter {
                return None;
            }
            let at = source_at(parsed.start.byte(record.span.lo))?;
            let start = source_at(parsed.start.byte(record.owner.lo))?;
            Some((at, record.expected.clone(), SourceSpan { start, end: at }))
        })
        .collect())
}
