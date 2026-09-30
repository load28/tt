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
//!    code parses as TypeScript's parser reads it; the rules TypeScript
//!    checks after parsing stay TypeScript's). Disabled with `--no-verify`.
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

use swc_common::Spanned;

use crate::host_input::HostInput;

fn parse_ts_module(code: &str, source_kind: crate::SourceKind) -> Result<(), (String, usize)> {
    match parse_errors(code, source_kind, &mut None)
        .into_iter()
        .next()
    {
        None => Ok(()),
        Some(error) => Err((error.message, error.at)),
    }
}

/// One error swc reported for a module, over bytes `at..end` of it.
struct ParseError {
    message: String,
    at: usize,
    end: usize,
    /// swc built the syntax tree past this error, and TypeScript enforces
    /// its rule after parsing ([`checked_after_parsing`]).
    after_parsing: bool,
}

fn parse_errors(
    code: &str,
    source_kind: crate::SourceKind,
    tokens: &mut Option<Vec<crate::lexer::Token>>,
) -> Vec<ParseError> {
    let (error, lexed) = crate::lexer::host_syntax_check(code, source_kind);
    *tokens = lexed;
    if let Some((span, message)) = error {
        return vec![ParseError {
            message: message.to_string(),
            at: span.start,
            end: span.end,
            after_parsing: false,
        }];
    }
    let input = HostInput::new(code);
    let mut parser = input.parser(source_kind);
    let result = parser.parse_program();
    let describe = |error: swc_ecma_parser::error::Error, recovered: bool| {
        let span = error.span();
        ParseError {
            at: input.byte(span.lo()),
            end: input.byte(span.hi()),
            after_parsing: recovered && checked_after_parsing(error.kind()),
            message: error.into_kind().msg().to_string(),
        }
    };
    let mut errors: Vec<ParseError> = parser
        .take_errors()
        .into_iter()
        .map(|error| describe(error, true))
        .collect();
    if let Err(error) = result {
        errors.push(describe(error, false));
    }
    errors
}

/// Whether TypeScript enforces the rule behind an swc error after it has
/// parsed the file: in its binder or its checker, not its parser.
///
/// TypeScript's parser reads a file by its productions alone; the static
/// semantics that ECMA-262 lists as early errors, and TypeScript's own
/// grammar rules, are reported later under TypeScript's codes. The kinds
/// listed here are those whose diagnostic the pinned TypeScript
/// (`5739027c`, `tsc/internal`) reports only from `binder/binder.go`,
/// `checker/checker.go`, or `checker/grammarchecks.go`, never from
/// `parser/parser.go` or `scanner/scanner.go`. Every other kind, the ones
/// the parser shares included, is read as a failure to parse.
pub(crate) fn checked_after_parsing(kind: &swc_ecma_parser::error::SyntaxError) -> bool {
    use swc_ecma_parser::error::SyntaxError as E;
    matches!(
        kind,
        E::InvalidIdentInStrict(_)
            | E::EvalAndArgumentsInStrict
            | E::TS1100
            | E::WithInStrict
            | E::TS1102
            | E::PrivateConstructor
            | E::LabelledFunctionInStrict
            | E::TS1009
            | E::TS1014
            | E::NonLastRestParam
            | E::TS1015
            | E::TS1029(..)
            | E::TS1030(_)
            | E::TS1031
            | E::TS1038
            | E::TS1042
            | E::TS1047
            | E::TS1048
            | E::TS1089(_)
            | E::TS1092
            | E::TS1093
            | E::TS1096
            | E::TS1098
            | E::TS1105
            | E::TS1107
            | E::TS1115
            | E::TS1116
            | E::TS1123
            | E::TS1162
            | E::TS1171
            | E::TS1172
            | E::TS1173
            | E::TS1174
            | E::TS1175
            | E::TS1183
            | E::TS1184
            | E::TS1242
            | E::TS1243(..)
            | E::TS1244
            | E::TS1273(_)
            | E::TS1274(_)
            | E::TS1277(_)
            | E::TS2206
            | E::TS2207
            | E::TS2483
            | E::TS18010
            | E::AwaitParamInAsync
            | E::AwaitInFunction
            | E::PropertyNamedConstructor
            | E::GetterParam
            | E::SetterParam
            | E::ReadOnlyMethod
            | E::IllegalLanguageModeDirective
            | E::ConstDeclarationsRequireInitialization
            | E::PatVarWithoutInit
            | E::TaggedTplInOptChain
            | E::EmptyJSXAttr
            | E::PrivateNameInInterface
            | E::TS1114
            | E::DuplicateLabel(_)
            | E::TS1164
            | E::TS1245
            | E::TS1267
            | E::TS2369
            | E::TS2371
            | E::TS2406
            | E::TS2410
            | E::TS2414
            | E::TS2427
            | E::TS2452
            | E::TS2491
            | E::TS2499
            | E::TS2680
            | E::TS2703
            | E::TS4112
            | E::ArgumentsInClassField
            | E::InvalidNewTarget
            | E::NotSimpleAssign
            | E::InvalidAssignTarget
            | E::InvalidPat
            | E::DuplicateConstructor
            | E::ReturnNotAllowed
            | E::MultipleDefault { .. }
            | E::TopLevelAwaitInScript
            | E::InvalidSuperCall
            | E::InvalidSuper
            | E::TsRequiredAfterOptional
            | E::NullishCoalescingWithLogicalOp
    )
}

/// Validates a variant field's type annotation. Returns a plain message on error.
pub(crate) fn check_type_fragment(ty: &str) -> Result<(), String> {
    let wrapped = format!("type __Tt = {};", ty);
    parse_ts_module(&wrapped, crate::SourceKind::TypeScript).map_err(|(msg, _)| msg)
}

/// A failed self-check: its message and the byte of the *generated*
/// module it stopped at.
pub(crate) struct Failure {
    pub message: String,
    pub at: usize,
    pub kind: FailureKind,
}

/// Which self-check failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailureKind {
    /// swc did not parse the generated module.
    Parse,
    /// The generated module parses, but a statement the source ends by
    /// automatic semicolon insertion runs into the text after it.
    StatementBoundary,
}

/// Validates the final generated TypeScript.
pub(crate) fn verify_output(code: &str, source_kind: crate::SourceKind) -> Result<(), Failure> {
    verify_output_lexed(code, source_kind, &[], &mut None)
}

/// The first error that makes the module fail the self-check.
///
/// Bytes the emission copied from the source are the user's TypeScript
/// ([`crate::EmitMapping`]). There the self-check restates only what
/// TypeScript's parser reports. An error swc recovered from inside one
/// copied range whose rule TypeScript enforces after parsing
/// ([`checked_after_parsing`]) is left to TypeScript's binder and checker,
/// which report it under their own codes. An error that touches generated
/// text is ttc's own and fails the check either way.
fn verify_output_lexed(
    code: &str,
    source_kind: crate::SourceKind,
    mappings: &[crate::EmitMapping],
    tokens: &mut Option<Vec<crate::lexer::Token>>,
) -> Result<(), Failure> {
    let copied = |error: &ParseError| {
        mappings.iter().any(|mapping| {
            mapping.out <= error.at
                && error.end <= mapping.out + mapping.len
                && error.at < mapping.out + mapping.len
        })
    };
    match parse_errors(code, source_kind, tokens)
        .into_iter()
        .find(|error| !(error.after_parsing && copied(error)))
    {
        None => Ok(()),
        Some(error) => Err(Failure {
            message: error.message,
            at: error.at,
            kind: FailureKind::Parse,
        }),
    }
}

/// Validates the final generated TypeScript, and that it keeps the
/// source's statement boundaries ([`verify_statement_boundaries`]).
pub(crate) fn verify_emit(
    code: &str,
    source_kind: crate::SourceKind,
    automatic_semicolons: &[crate::lexer::AutomaticSemicolon],
    mappings: &[crate::EmitMapping],
) -> Result<(), Failure> {
    let mut tokens = None;
    verify_output_lexed(code, source_kind, mappings, &mut tokens)?;
    verify_statement_boundaries(code, source_kind, automatic_semicolons, mappings, tokens)
}

/// Checks that every statement the source ends by automatic semicolon
/// insertion still ends in the generated module.
///
/// A module can parse and still mean something else: when generated text
/// that starts with `(`, `[`, a template, or an operator follows a statement
/// the source ended by a line break, the two parse as one statement. For
/// each such source boundary whose ending token the output copies together
/// with the separation after it, the output's own token facts must show a
/// boundary after that token too. A copy of the ending token alone is part
/// of a lowering's text, which ends no statement. Output that copies the
/// source across the boundary unchanged is the source's statement pair and
/// needs no lexing, and output the parse check already lexed is not lexed
/// again.
fn verify_statement_boundaries(
    code: &str,
    source_kind: crate::SourceKind,
    automatic_semicolons: &[crate::lexer::AutomaticSemicolon],
    mappings: &[crate::EmitMapping],
    mut output_tokens: Option<Vec<crate::lexer::Token>>,
) -> Result<(), Failure> {
    let mut by_source: Vec<&crate::EmitMapping> = mappings.iter().collect();
    by_source.sort_unstable_by_key(|mapping| mapping.src);
    let to_output = |src: usize| {
        let index = by_source.partition_point(|mapping| mapping.src <= src);
        let mapping = by_source.get(index.checked_sub(1)?)?;
        (src < mapping.src + mapping.len).then(|| mapping.out + (src - mapping.src))
    };
    let bytes = code.as_bytes();
    for boundary in automatic_semicolons {
        let Some(last) = boundary.end.checked_sub(1).and_then(to_output) else {
            continue;
        };
        let end = last + 1;
        if to_output(boundary.end) != Some(end) {
            continue;
        }
        let (next, _) = crate::scanner::skip_trivia(bytes, end, bytes.len());
        if next < bytes.len() && to_output(boundary.next) == Some(next) {
            continue;
        }
        let tokens = output_tokens
            .get_or_insert_with(|| crate::lexer::lex_with_kind(code, 0, code.len(), source_kind));
        if crate::lexer::statement_continues_after(tokens, end) {
            return Err(Failure {
                message: "a statement the source ends by automatic semicolon insertion \
                          continues into the generated code after it"
                    .to_string(),
                at: next,
                kind: FailureKind::StatementBoundary,
            });
        }
    }
    Ok(())
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
    let out = failure.at.min(code.len());
    let generic = || match failure.kind {
        FailureKind::Parse => format!(
            "generated TypeScript failed to parse: {}. This is either invalid TypeScript passed \
             through from the source or a ttc bug; use --no-verify to bypass.",
            failure.message,
        ),
        FailureKind::StatementBoundary => format!(
            "generated TypeScript changed the meaning of this code: {}. This is a ttc bug; use \
             --no-verify to bypass.",
            failure.message,
        ),
    };
    let (message, span) = match crate::typescript::mapper::to_source(mappings, out) {
        // Copied from the source: the offending text is the user's own. A
        // parser-owned rollback fact may identify the exact tt candidate.
        Some(src) if failure.kind == FailureKind::StatementBoundary => {
            (generic(), Some((src, src)))
        }
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
        } => match error {
            crate::evaluation_ir::EvaluationError::DiscardedResult { source: result } => {
                crate::error::TtError::span(
                    result.start.min(source.len()),
                    result.end.min(source.len()),
                    "`result` is used as a statement, so its `Err` would be discarded".to_string(),
                )
                .code(crate::DiagnosticCode::ResultValueDiscarded)
                .help("assign, return, or otherwise consume this Result value")
            }
            _ => crate::error::TtError::span(
                span.start.min(source.len()),
                span.end.min(source.len()),
                format!("tt host lowering could not plan this construct: {error}"),
            )
            .code(crate::DiagnosticCode::LoweringPlanFailed),
        },
        crate::codegen::LoweringFailure::HostProjection {
            error,
            source: span,
        } => crate::error::TtError::span(
            span.start.min(source.len()),
            span.end.min(source.len()),
            format!("tt host lowering could not plan this construct: {error}"),
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
    fn a_failure_after_a_wide_character_is_at_the_byte_swc_stopped_on() {
        let code = "const e = \"한글\"; const x = ;\n";
        let failure =
            verify_output(code, crate::SourceKind::TypeScript).expect_err("does not parse");
        assert_eq!(failure.at, code.find(" ;").unwrap() + 1);
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

    #[test]
    fn generated_text_that_continues_a_source_statement_is_rejected() {
        let source = "const v = 1\nv |> o.m\nconst w = 2\n";
        let tokens =
            crate::lexer::lex_with_kind(source, 0, source.len(), crate::SourceKind::TypeScript);
        let boundaries = crate::lexer::automatic_semicolons(&tokens);
        let check = |code: &str, lowered: usize| {
            let tail = source.find("\nconst w").unwrap();
            let mappings = [
                crate::EmitMapping {
                    src: 0,
                    out: 0,
                    len: 12,
                },
                crate::EmitMapping {
                    src: tail,
                    out: 12 + lowered,
                    len: source.len() - tail,
                },
            ];
            verify_emit(code, crate::SourceKind::TypeScript, &boundaries, &mappings)
        };
        let joined = "const v = 1\n(o.m)(v)\nconst w = 2\n";
        let failure = check(joined, "(o.m)(v)".len()).expect_err("the statements run together");
        assert_eq!(failure.kind, FailureKind::StatementBoundary);
        assert_eq!(failure.at, joined.find("(o.m)").unwrap());
        let work = crate::work::measure(|| check(joined, "(o.m)(v)".len()));
        assert_eq!(work["whole-text lexes"], 1);
        let separated = "const v = 1\n;(o.m)(v)\nconst w = 2\n";
        assert!(check(separated, ";(o.m)(v)".len()).is_ok());
        let named = "const v = 1\no.m(v)\nconst w = 2\n";
        assert!(check(named, "o.m(v)".len()).is_ok());
    }

    #[test]
    fn malformed_jsx_is_a_validation_error() {
        let source = "<>&>&w=<>&>&w=2&(&#;;\\w\u{1}";
        let result = verify_output(source, crate::SourceKind::Tsx);
        let failure = result.expect_err("malformed JSX must be reported");
        assert!(failure.at > 0);
        assert!(failure.message.contains("unbalanced TypeScript delimiter"));
    }
}
