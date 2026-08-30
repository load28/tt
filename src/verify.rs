//! swc-based validation.
//!
//! tt syntax is not valid TypeScript, so swc cannot parse a `.tt` file as a
//! whole — construct detection stays in the hand-rolled scanner. swc is used
//! where real TypeScript exists:
//!
//! 1. `check_type_fragment` — variant field types are pure TS type syntax;
//!    parsing them at compile time rejects bad annotations with an exact
//!    position in the `.tt` file.
//! 2. `verify_output` — the fully generated TypeScript module is parsed as a
//!    self-check that the compiler emitted valid code (and that passthrough
//!    code was valid TS to begin with). Disabled with `--no-verify`.
//!
//! This SWC check is intentionally in the syntax pipeline. The compiler
//! already uses a whole-program SWC AST to model TypeScript owners and
//! evaluation contexts (`crate::program_syntax`), so parsing the final module
//! here checks the target against the same in-process syntax boundary. It does
//! not ask or approximate any type-semantic question.
//!
//! The TypeScript 7 backend remains the authority for inferred types,
//! narrowing, resolution, diagnostics, and declaration emit. Even when that
//! backend is a required toolchain component, using it for this self-check
//! would broaden an external semantic adapter into the compiler's syntax
//! layer, add a process/protocol dependency to a local invariant, and repeat
//! work before the typed pass. A valid TypeScript form accepted by TypeScript
//! but rejected here is therefore an SWC compatibility/configuration bug to
//! reproduce and fix at this boundary, not evidence that syntax verification
//! belongs to the type backend.

use swc_common::input::StringInput;
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap, Spanned};
use swc_ecma_parser::lexer::Lexer;
use swc_ecma_parser::{Parser, Syntax, TsSyntax};

fn ts_syntax(source_kind: crate::SourceKind) -> Syntax {
    Syntax::Typescript(TsSyntax {
        tsx: source_kind.is_tsx(),
        decorators: true,
        ..Default::default()
    })
}

/// Parses `code` as a TypeScript module; returns the first syntax error as
/// `(message, line, col)` (1-based, positions in `code`).
fn parse_ts_module(
    code: &str,
    source_kind: crate::SourceKind,
) -> Result<(), (String, usize, usize)> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(Lrc::new(FileName::Anon), code.to_string());
    let lexer = Lexer::new(
        ts_syntax(source_kind),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let result = parser.parse_module();
    let mut errors = parser.take_errors();
    if let Err(e) = result {
        errors.push(e);
    }
    match errors.into_iter().next() {
        None => Ok(()),
        Some(e) => {
            let pos = cm.lookup_char_pos(e.span().lo());
            let msg = e.into_kind().msg().to_string();
            Err((msg, pos.line, pos.col_display + 1))
        }
    }
}

/// Validates a variant field's type annotation. Returns a plain message on error.
pub(crate) fn check_type_fragment(ty: &str) -> Result<(), String> {
    let wrapped = format!("type __Tt = {};", ty);
    parse_ts_module(&wrapped, crate::SourceKind::TypeScript).map_err(|(msg, _, _)| msg)
}

/// A failed self-check: swc's message and where it stopped in the
/// *generated* module (1-based line and column).
pub(crate) struct Failure {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

/// Validates the final generated TypeScript.
pub(crate) fn verify_output(code: &str, source_kind: crate::SourceKind) -> Result<(), Failure> {
    parse_ts_module(code, source_kind).map_err(|(message, line, col)| Failure {
        message,
        line,
        col,
    })
}

/// The self-check's failure as an error in the `.tt` file the user has
/// open.
///
/// swc stopped at a byte of the *generated* module, which is a file no one
/// wrote — reporting that position as-is gives the user nothing to look
/// at, and an editor nowhere to put the squiggle but line 1. The position
/// travels back the way every other diagnostic does: through the emit
/// mappings to the source byte it was copied from, and — when it landed on
/// glue instead — to the construct that wrote the glue
/// ([`crate::EmitAnchor`]), whose own text is then the error's span.
///
/// A construct that almost parsed as tt may be passed through verbatim by
/// contract. The parser carries those rolled-back candidates explicitly;
/// when the mapped failure belongs to one, this layer names that parser
/// fact rather than rediscovering intent from source strings.
pub(crate) fn at_source(
    unclaimed: &[crate::ast::UnclaimedTtCandidate],
    mappings: &[crate::EmitMapping],
    anchors: &[crate::EmitAnchor],
    code: &str,
    failure: &Failure,
) -> crate::error::TtError {
    let out = byte_of(code, failure.line, failure.col);
    let generic = || {
        format!(
            "generated TypeScript failed to parse: {}. This is either invalid TypeScript passed \
             through from the source or a ttc bug; use --no-verify to bypass.",
            failure.message,
        )
    };
    let (message, span) = match crate::typescript::mapper::to_source(mappings, out) {
        // Copied from the source: the offending text is the user's own. A
        // parser-owned rollback fact may identify the exact tt candidate.
        Some(src) => match unclaimed_candidate_at(unclaimed, src) {
            Some(candidate) => {
                let word = match candidate.kind {
                    crate::ast::UnclaimedTtKind::Try => "try",
                };
                (
                    format!(
                        "`{word}` here did not parse as a tt `{word}`, so it was passed through as \
                     TypeScript and the generated module no longer parses: {}",
                        failure.message,
                    ),
                    Some((candidate.keyword.start, candidate.keyword.end)),
                )
            }
            None => (generic(), Some((src, src))),
        },
        // Glue: ttc's own output, and the construct that wrote it is the
        // only thing the user can act on.
        None => (
            generic(),
            anchors
                .iter()
                .find(|a| a.out <= out && out < a.end)
                .map(|a| (a.src, a.src_end)),
        ),
    };
    let code = crate::DiagnosticCode::VerifyFailed;
    match span {
        Some((start, end)) if end > start => {
            crate::error::TtError::span(start, end, message).code(code)
        }
        Some((start, _)) => crate::error::TtError::at(start, message).code(code),
        None => crate::error::TtError::positionless(message).code(code),
    }
}

/// The byte offset of a 1-based line/column pair in `text`. swc counts
/// columns in characters, so the walk does too.
fn byte_of(text: &str, line: usize, col: usize) -> usize {
    let mut at = 0;
    for (n, text) in text.split_inclusive('\n').enumerate() {
        if n + 1 == line {
            return at
                + text
                    .char_indices()
                    .nth(col.saturating_sub(1))
                    .map_or(text.len(), |(byte, _)| byte);
        }
        at += text.len();
    }
    text.len()
}

/// The file's TypeScript, at byte `at`, is not TypeScript — established
/// before host lowering rather than after emission.
///
/// The projection built for target lowering is the source with tt values
/// replaced by placeholders, so a parse failure inside text copied from the
/// source is the user's own syntax error, at a source byte with no mapping
/// hops in between ([`crate::codegen::lowering_plan`]). Emission cannot run
/// without the owner model that parse would have produced, so this is not a
/// bypassable self-check but the reason the file has no output.
///
/// The message states only what the projection proves: which byte stopped
/// the parse, and why that ends the compile. At this boundary the claimed
/// constructs are known, and constructs that failed to claim have
/// diagnostics or parser-owned rollback facts of their own
/// ([`crate::DiagnosticCode::blocks_projection`]).
pub(crate) fn in_source(
    source: &str,
    failure: &crate::codegen::LoweringFailure,
) -> crate::error::TtError {
    match failure {
        crate::codegen::LoweringFailure::SourceNotTypeScript {
            message,
            source: at,
        } => {
            let at = (*at).min(source.len());
            let message = format!(
                "the TypeScript here does not parse: {message}. tt lowering models this file's TypeScript, \
                 so no output is emitted (`--no-verify` does not apply).",
            );
            crate::error::TtError::at(at, message).code(crate::DiagnosticCode::SourceNotTypeScript)
        }
        crate::codegen::LoweringFailure::Evaluation {
            error,
            source: span,
        } => crate::error::TtError::span(
            span.start.min(source.len()),
            span.end.min(source.len()),
            format!("tt host lowering could not plan this construct: {error:?}"),
        )
        .code(crate::DiagnosticCode::LoweringPlanFailed),
        crate::codegen::LoweringFailure::HostProjection {
            error,
            source: span,
        } => crate::error::TtError::span(
            span.start.min(source.len()),
            span.end.min(source.len()),
            format!("tt host lowering could not plan this construct: {error:?}"),
        )
        .code(crate::DiagnosticCode::LoweringPlanFailed),
    }
}

fn unclaimed_candidate_at(
    candidates: &[crate::ast::UnclaimedTtCandidate],
    at: usize,
) -> Option<&crate::ast::UnclaimedTtCandidate> {
    candidates
        .iter()
        .filter(|candidate| candidate.extent.start <= at && at < candidate.extent.end)
        .min_by_key(|candidate| candidate.extent.end - candidate.extent.start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_column_pair_becomes_the_byte_it_names() {
        let text = "const a = 1;\nconst 한글 = 2;\nconst c = 3;\n";
        assert_eq!(byte_of(text, 1, 1), 0);
        assert_eq!(byte_of(text, 2, 7), text.find('한').unwrap());
        // Columns are counted in characters, as swc reports them.
        assert_eq!(byte_of(text, 2, 10), text.find("= 2").unwrap());
        // Past the end clamps rather than panicking.
        assert_eq!(byte_of(text, 99, 1), text.len());
    }

    #[test]
    fn the_innermost_structural_candidate_owns_the_failure() {
        use crate::ast::{Span, UnclaimedTtCandidate, UnclaimedTtKind};

        let outer = UnclaimedTtCandidate {
            kind: UnclaimedTtKind::Try,
            keyword: Span { start: 0, end: 3 },
            extent: Span { start: 0, end: 30 },
        };
        let inner = UnclaimedTtCandidate {
            kind: UnclaimedTtKind::Try,
            keyword: Span { start: 10, end: 13 },
            extent: Span { start: 10, end: 20 },
        };
        assert_eq!(unclaimed_candidate_at(&[outer, inner], 15), Some(&inner));
        assert_eq!(unclaimed_candidate_at(&[outer, inner], 31), None);
    }
}
