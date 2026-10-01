# TASK-710: Report a malformed postfix pipeline and a crossing `yield` once

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-710`

## Purpose

TASK-700 found two faults reported twice. Issue 1: a pipeline whose
optional postfix tail is malformed
(``if let Ok(value: n) = read(1) { const v = n |> ?.toFixed`1`; }``, or the
same in a match arm) reports `malformed-pipeline-postfix` at the tail and
`source-not-typescript` at the `|>`. Issue 6: a `yield` crossing a `result`
block inside a generator in a template literal or a JSX attribute reports
`result-yield-crossing` and `source-not-typescript` at the same `yield`.
One cause must give one diagnostic.

## Scope

- Included: `DiagnosticCode::blocks_projection` (`src/diagnostics.rs`); the
  token stream Core lowering and the semantic checks read
  (`src/lib/compile.rs`, `src/lib/mapped.rs`, `src/lexer.rs`); the
  function-target index (`FunctionTargets` in `src/flow/syntax.rs`), its
  users in `src/core_ir/lower.rs` and `src/sema.rs` /
  `src/sema/checker.rs`, and the tt-owned token query it takes
  (`Hir::match_owned_tokens`, `src/hir/mod.rs`); three compiler cases; the
  listed defects in `tests/oracle-failures.txt` and
  `tests/editor-diagnostic-differences.txt`.
- Excluded: `missing-pipeline-step`, whose pipeline is claimed and lowered.

## Sources

- `DiagnosticCode::blocks_projection` (`src/diagnostics.rs`): "text the
  parser could not claim (a stray `|>` passes through verbatim and is not
  TS)" leaves the file without a TypeScript projection.
- `docs/ai/tt.md` (result block): "A `break`, `continue`, or `yield` cannot
  leave the block: a jump ... reports exactly one ...
  `result-yield-crossing` ... wherever the block stands, including a
  template interpolation", and (try) a `try` whose target is a generator is
  rejected.
- TASK-510 (Decision on the token stream): Core lowering and the checks
  "always lexed as TypeScript", which for a `.ttx` file was a second,
  non-JSX lexing kept "as before", not a decision with a reason.

## Decisions

### Decision 1: A malformed postfix pipeline blocks the projection, as a stray `|>` does

- **Context**: The parser does not claim a pipeline whose optional postfix
  tail is malformed (`pipes::Attempt::MalformedOptional`): it records the
  diagnostic and a recovery node and leaves the text, `|>` included, in
  place. When the file has another tt construct, the lowering projection
  copies that text and SWC stops at the `|>`.
- **Alternatives considered**: (a) Mask the parser's recovery node in the
  lowering projection, so the rest of the file still lowers: the projection
  is built from Core statements and copies an opaque statement whole, so
  the mask would have to be threaded through every copy, for a file that
  cannot be emitted anyway. (b) Drop `source-not-typescript` when another
  diagnostic overlaps it: a diagnostic filter, which AGENTS.md (contract 3)
  rules out. (c) Classify the code with the stray `|>`.
- **Decision and rationale**: (c). It is the same fact as a stray `|>`:
  text the parser could not claim stays as written and is not TypeScript.
  `blocks_projection` now lists it, so the lowering plan is not built for
  the file; the typed projection still substitutes the parser's recovery
  node, so the editor keeps the file's other type information, and
  `leaves_tt_text` withholds no emission that would hold the `|>`.

### Decision 2: Core lowering and the checks read the file's own tokens

- **Context**: For a `.ttx` file, `lexer::TypeScriptTokens` lexed the
  source a second time as plain TypeScript. JSX is not TypeScript's
  grammar: `<div data-value={...} />` lexes there as operators and a `/>`
  regular expression, so the brace structure the function-target index
  reads is wrong, `function* () {` in the attribute is not found, and the
  `result` block's projection region was written as an ordinary arrow,
  where `yield` does not parse.
- **Alternatives considered**: (a) Keep the second lexing and teach it
  JSX: that is the parse's own lexing. (b) Use the parse's tokens.
- **Decision and rationale**: (b). The parse's token stream for a `.ttx`
  file has every JSX tag and text run as one opaque token and the
  expression containers as ordinary tokens, which is what both consumers
  ask about; `TypeScriptTokens` is removed. For a `.tt` file nothing
  changes (it was already the parse's stream).

### Decision 3: The function-target index answers inside template interpolations

- **Context**: A template literal is one token whose interpolations hold
  their own token streams (`lexer::TplPart::Interp`). The index
  (`FunctionTargets`) walked the file's stream only, so for an offset in an
  interpolation it answered with the function around the template: Core
  wrote a `result` block in a generator in an interpolation as an ordinary
  region (Issue 6), and `check_try` treated any function written in an
  interpolation as ordinary (its comment said so), so a `try` in a
  generator there was accepted (Issue 2).
- **Alternatives considered**: (a) Record "in a generator" on each parsed
  construct, as `in_function` is recorded: a second answer to the question
  the index answers. (b) Make the index descend into interpolations.
- **Decision and rationale**: (b). `FunctionTargets::new` builds an index
  for each interpolation's stream, and `at_offset` asks the innermost one
  that holds the offset, falling back to the function around the template.
  The tt-owned braces and arrows (match bodies, arm arrows) are asked per
  stream (`new` takes the query), and `match_owned_tokens` answers only for
  a match whose tokens are in the stream asked (it used to mark the next
  `{` after the template for a match inside one). Core's
  `node_in_generator` and sema's `check_try` both ask `at_offset`.

## Work log

- 2026-10-01: Reproduced both: Issue 1 also at module level beside another
  tt construct; Issue 6 in a template literal and a JSX attribute, not in a
  JSX child (its tokens lexed as TypeScript happened to balance).
- 2026-10-01: Decision 1; Decision 2 (removing `TypeScriptTokens` fixed
  the JSX attribute); Decision 3, with sema's `check_try` (Issue 2).
- 2026-10-01: Added `tests/cases/compiler/malformedPostfixPipelineInConstructBodyIsReportedOnce.tt`,
  `yieldCrossingResultInTemplateOrJsxIsReportedOnce.ttx`, and
  `tryInGeneratorWrittenInTemplate.tt` (`@expectDiagnostic`); removed the
  seven lines from `tests/oracle-failures.txt` and the two from
  `tests/editor-diagnostic-differences.txt`.

## Issues and resolutions

### Issue 1: The two double reports

- **Symptom**: as in Purpose.
- **Cause**: Decisions 1, 2, and 3.
- **Resolution**: Decisions 1, 2, and 3.

### Issue 2: A `try` in a generator written in a template was accepted

- **Symptom**: ``return `${[...(function* () { const n = try read(x); yield n; })()]}`;``
  compiled, while the same generator outside the template reports
  `try-placement` ("`try` cannot be used in a constructor or generator").
- **Cause**: Decision 3: `check_try` answered "a function written in the
  `try`'s region is ordinary".
- **Resolution**: Decision 3.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/malformedPostfixPipelineInConstructBodyIsReportedOnce.tt`,
  `tests/cases/compiler/yieldCrossingResultInTemplateOrJsxIsReportedOnce.ttx`,
  `tests/cases/compiler/tryInGeneratorWrittenInTemplate.tt`
  (`cargo test --test case_baselines`), and the matrix cases
  `malformed-pipeline-postfix_taggedTemplate_{ifLetBody,matchArm}` and
  `result-yield-crossing_yieldInside_{templateLiteral,jsxAttribute,jsxComponentProp,jsxNested,jsxSpreadAttribute}`.
- **Observed failure**: Without the changes in `src/`, the oracles
  reported: the first case `source-not-typescript at ...:9:46` besides the
  three `malformed-pipeline-postfix`; the second
  `source-not-typescript at main.ttx:11:79` besides the three
  `result-yield-crossing`; the third "was to report try-placement at
  tryInGeneratorWrittenInTemplate.tt:9:43 and reports nothing".

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib` (412 passed); `TTC_REQUIRE_TSGO=1
  TT_MATRIX_CASES=all cargo test --test case_baselines` and
  `TTC_REQUIRE_TSGO=1 TT_REQUIRE_EXTENSION=1 TT_MATRIX_CASES=all cargo test
  --test editor_cases`: every case agreed with its oracle; the baselines
  that moved were the seven listed matrix cases (each losing only its
  `source-not-typescript`), the editor baseline of
  `result-yield-crossing_yieldInside_jsxAttribute` (its `restates:
  source-not-typescript` line), and two TASK-709 baselines committed
  there. The full gate runs once at the end of the batch (see TASK-712).
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed: `src/diagnostics.rs`, `src/lexer.rs`, `src/lib/compile.rs`,
`src/lib/mapped.rs`, `src/flow/syntax.rs`, `src/flow/tests.rs`,
`src/hir/mod.rs`, `src/core_ir/lower.rs`, `src/sema.rs`,
`src/sema/checker.rs`, `tests/oracle-failures.txt`,
`tests/editor-diagnostic-differences.txt`, the three cases with their
baselines, seven matrix baselines, and one editor baseline. Both faults are reported once; a `try` in a generator written in
a template is rejected as it is anywhere else.
