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

pub(crate) struct EditorInsertion {
    pub at: usize,
    pub text: String,
    pub owner: SourceSpan,
    pub expected: String,
    /// Ends a skipped statement whose text stays as written: TypeScript
    /// reports its syntax there, so the insertion states no cause.
    pub terminates: bool,
}

/// Missing syntax recorded by the host parser, in original coordinates.
/// The insertion point precedes the next token owned by the enclosing grammar.
/// Inserted trivia separates it from the following declaration and its prelude.
pub(crate) fn editor_insertions(
    semantic: &SemanticFile,
    core: &CoreFile,
    source: &str,
    source_kind: crate::SourceKind,
    tokens: &[crate::lexer::Token],
) -> Result<Vec<EditorInsertion>, ProgramSyntaxError> {
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
    let mut insertions: Vec<EditorInsertion> = parsed
        .recoveries
        .iter()
        .filter_map(|record| {
            let text = record.replacement?;
            let at = source_at(parsed.start.byte(record.span.lo))?;
            let start = source_at(parsed.start.byte(record.owner.lo))?;
            // EOF cannot capture the prelude of a following statement, so host
            // text keeps TypeScript's own missing-token diagnostic there. A tt
            // value inside the unfinished production is placed by it, though:
            // lowering needs the production complete.
            if at == source.len()
                && !projection
                    .pending
                    .iter()
                    .any(|entry| start <= entry.source.start && entry.source.end <= at)
            {
                return None;
            }
            Some(EditorInsertion {
                at,
                text: text.into(),
                owner: SourceSpan { start, end: at },
                expected: record.expected.clone(),
                terminates: false,
            })
        })
        .collect();
    // A skipped statement keeps its text, and TypeScript's list recovery
    // resumes where a statement can start. Lowered code written after it
    // (a later statement's prelude) is not where the user's next statement
    // starts, so a skipped statement before a tt value is ended by a `;`,
    // which aborts every list TypeScript was in
    // (`abortParsingListOrMoveToNextToken`).
    for record in parsed
        .recoveries
        .iter()
        .filter(|record| record.kind == swc_ecma_parser::RecoveryKind::SkippedInput)
    {
        let (Some(start), Some(end)) = (
            source_at(parsed.start.byte(record.owner.lo)),
            source_at(parsed.start.byte(record.span.hi)),
        ) else {
            continue;
        };
        // Skipped expression text ends its statement where the parser
        // resumed at a statement keyword.
        if record.context != swc_ecma_parser::RecoveryContext::Statement {
            let next = source[end..].trim_start();
            let word = next
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
                .next()
                .unwrap_or_default();
            if !matches!(
                word,
                "const" | "let" | "var" | "export" | "return" | "throw"
            ) {
                continue;
            }
        }
        let written = source[start..end].trim_end();
        let at = start + written.len();
        if written.ends_with(';')
            || at >= source.len()
            || !projection
                .pending
                .iter()
                .any(|entry| entry.source.start >= at)
        {
            continue;
        }
        // TypeScript keeps the lists the skipped text left open until
        // their own terminators, so those close first, innermost first.
        let mut open = Vec::new();
        for token in tokens
            .iter()
            .filter(|token| start <= token.span.start && token.span.end <= at)
        {
            match token.kind {
                crate::lexer::TokenKind::Punct(b'(') => open.push(')'),
                crate::lexer::TokenKind::Punct(b'[') => open.push(']'),
                crate::lexer::TokenKind::Punct(b'{') => open.push('}'),
                crate::lexer::TokenKind::Punct(byte @ (b')' | b']' | b'}'))
                    if open.last() == Some(&(byte as char)) =>
                {
                    open.pop();
                }
                _ => {}
            }
        }
        let mut text: String = open.into_iter().rev().collect();
        text.push(';');
        insertions.push(EditorInsertion {
            at,
            text,
            owner: SourceSpan { start, end: at },
            expected: "statement".into(),
            terminates: true,
        });
    }
    insertions.sort_by_key(|insertion| insertion.at);
    Ok(insertions)
}
