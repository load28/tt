# TASK-711: Report an incomplete fragment in a construct where TypeScript stops

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-711`

## Purpose

TASK-700 (Issue 7) found that `match (n) { 1 => n +, _ => 0 }` reports
`source-not-typescript` at the `+`, while TypeScript reports TS1109
"Expression expected" at the `,` and the editor, which publishes
TypeScript's position, shows it there. The command line and the editor
must name the same place, and that place is TypeScript's.

## Scope

- Included: the source position a projection boundary carries
  (`ProjectionBuilder::push_source_boundary` and
  `ProjectionSegmentKind::SourceBoundary` in
  `src/program_syntax/projection.rs`), which every fragment a match embeds
  goes through (a scrutinee, a guard, an expression arm, a block arm), the
  unit test that pinned the old position, a compiler case, the listed
  defects in `tests/oracle-failures.txt` and
  `tests/editor-diagnostic-differences.txt` and the 42 matrix baselines
  that held the old column, and `docs/ai/tt.md`.
- Excluded: a `result` block's body, whose end has no boundary at all
  (TASK-707).

## Sources

- TypeScript's parser (`src/compiler/parser.ts`, `parseExpected`,
  `createMissingNode` via `parseErrorAtCurrentToken`): an expected token or
  expression that is missing is reported at the current token, the first
  one that cannot continue the construct. With the pinned TypeScript,
  `tsc --noEmit` on `[n +, 0]` reports TS1109 at the `,`, on
  `() => { return x + }` at the `}`, and on `[x., 0]` TS1003 at the `,`
  (checked in this task).
- `src/program_syntax/projection.rs`: a `SourceBoundary` is the
  compiler-written delimiter after a copied fragment; SWC stopping on it
  proves the fragment was incomplete.

## Decisions

### Decision 1: A boundary stands for the source token after the fragment

- **Context**: The boundary recorded the last byte of the fragment it
  closes (`source.end - 1`). An arm body's span includes the trivia after
  it, so that byte was the operator, a space, or the end of a comment,
  never where TypeScript stops.
- **Alternatives considered**: (a) Map the boundary to the fragment's last
  significant byte (the `+`): still not TypeScript's position, which is
  what the editor publishes. (b) Re-parse the construct's source text with
  TypeScript's grammar to find the stop: a second parser for a fact the
  projection already has. (c) Map the boundary to the first significant
  source byte at or after the fragment's end.
- **Decision and rationale**: (c). The boundary is the projection's
  stand-in for the source token that ends the fragment (the `,` between
  arms, the `}` of the match or of a block arm, the `=>` after a guard, the
  `)` after a scrutinee), so TypeScript reading the source stops on that
  token where SWC stops on the boundary. Trivia is skipped with the
  scanner's `skip_ws_comments`, which judges ASCII bytes and the ECMA-262
  white space code points. The boundary's span is used only to place this
  error (every other projection lookup skips `SourceBoundary`), so nothing
  else moves.

## Work log

- 2026-10-01: Confirmed TypeScript's positions with the pinned `tsc` on the
  equivalent TypeScript.
- 2026-10-01: Changed `push_source_boundary` to record the next token
  (`token_after`); updated
  `an_incomplete_source_expression_owns_the_generated_closing_boundary`
  (`.` to `,`); removed the defect lines; `UPDATE_EXPECT=1
  TT_CASES=source-not-typescript cargo test --test case_baselines` moved
  the column of the 41 `armBody` cases and `explain1` by one, to the `,`.
- 2026-10-01: Added
  `tests/cases/compiler/incompleteFragmentInMatchIsReportedWhereTypeScriptStops.tt`
  (six units: an arm before `,`, the last arm before `}`, a comment before
  the `,`, a block arm, a guard, a scrutinee). It uses `@expectErrors` and
  its `.errors.txt`, not `@expectDiagnostic`: the server's `typedCheck`
  answers for the whole project on each unit's request, so a range per
  unit is reported once per unit.

## Issues and resolutions

### Issue 1: The old position could be a space

- **Symptom**: Before the change, `1 => n * /* the factor */` followed by a
  line with `,` was reported at the space before the `,`, and
  `1 if n > => 0` at the space after `>`.
- **Cause**: The fragment's span ends after its trailing trivia, and the
  boundary took its last byte.
- **Resolution**: Decision 1.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/incompleteFragmentInMatchIsReportedWhereTypeScriptStops.tt`
  (`cargo test --test case_baselines`), the matrix cases
  `source-not-typescript_armBody_*` and `source-not-typescript_explain1`
  (and their editor surface comparison in `tests/editor_cases.rs`), and
  `program_syntax::tests::an_incomplete_source_expression_owns_the_generated_closing_boundary`.
- **Observed failure**: Without the change in `src/program_syntax/projection.rs`,
  the case's `.errors.txt` differed in every unit by one column or more
  (`armThenComma.tt:1:58` for `1:59`, `commentBeforeComma.tt:4:4` for
  `4:5`, `guard.tt:1:59` for `1:60`, `scrutinee.tt:1:48` for `1:49`, ...),
  and the unit test found `.` where `,` was expected.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib program_syntax`, `cargo test --test emit_map --test
  sidecar`, `TTC_REQUIRE_TSGO=1 TT_CASES=source-not-typescript cargo test
  --test case_baselines` and `... TT_REQUIRE_EXTENSION=1 ... --test
  editor_cases`: passed. The full gate runs once at the end of the batch
  (see TASK-712).
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed: `src/program_syntax/projection.rs`, `src/program_syntax/tests.rs`,
`docs/ai/tt.md`, `tests/oracle-failures.txt`,
`tests/editor-diagnostic-differences.txt`, the compiler case and its
baseline, and the 42 `source-not-typescript` matrix baselines. An
incomplete fragment in a match is reported where TypeScript reports it, on
the command line as in the editor.
