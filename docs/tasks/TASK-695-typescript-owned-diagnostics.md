# TASK-695: Report a checker diagnostic on user-written text as TypeScript reports it

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-695`

## Purpose

For plain TypeScript inside a `.tt` file, the typed pass reworded and moved
TypeScript's assignability errors: `const unused: string = 1;` was published
as ``type mismatch: expected `string`, found `1` `` at the initializer, where
TypeScript reports `Type 'number' is not assignable to type 'string'.` at the
declared name. The editor cases `plainTypeScript`, `plainTsx`, and
`twoErrorsAtOneCall` differed from their TypeScript twins for this reason
(TASK-675 Issue 1). AGENTS.md contract 2 makes user TypeScript errors
TypeScript's to report.

## Scope

- Included: the ownership rule in `src/engine/semantics/report.rs` and
  `src/engine/semantics/translate.rs` (`typescript_owned`), the hand-written
  file path of the same report, the tests and baselines that pinned the
  restated form, `docs/ai/tt.md`, and reversal notes on TASK-574, TASK-585,
  and TASK-675.
- Excluded: TypeScript's elaboration lines, which the backend dropped
  (TASK-696); the wording of restated diagnostics on generated code, which
  is unchanged.

## Decisions

### Decision 1: TypeScript's own span decides who reports a checker diagnostic

- **Context**: The report computed every assignability diagnostic's origin
  from the mismatch fact's span (the expression whose type was compared) and
  rendered it from that fact, whatever TypeScript's own span covered. Which
  diagnostics ttc may restate had to be decided by the diagnostic's
  structure, not by its code or text (contract 3).
- **Alternatives considered**:
  - Restate only diagnostics in files that contain tt syntax: a `.tt` file
    with one `match` would still reword its plain TypeScript, and the rule
    would be about the file, not the diagnostic.
  - Decide by the mismatch fact's span: the initializer of
    `const x: number = match (...) {...}` is generated, but TypeScript
    reports at `x`, which the user wrote; TypeScript's statement there is
    correct and points at user text.
  - Decide by TypeScript's own span, mapped through the projection: when one
    verbatim source mapping owns the whole span
    (`DiagnosticOrigin::Exact`, the same test that already places a
    diagnostic exactly), TypeScript's location is user text and its message
    is about that text.
- **Decision and rationale**: The third. A diagnostic whose own span maps
  exactly is TypeScript's: its code, its message (with tt's case names
  appended only where the message prints a structural case type tt
  generated, as before), and its range, also on the hand-written-file path.
  Every other diagnostic, whose span lands on generated code where
  TypeScript's location and wording would point at text the user never
  wrote, keeps the restatement over the construct. Such a diagnostic is
  also left out of the set of structured causes that suppress consequences
  on the same lowering, since it is not reported as one. The rule follows
  how TypeScript places the error node: the checker reports an
  initializer's assignability on the declaration and a returned value's on
  the return statement, and `GetErrorRangeForNode`
  (`tsc/internal/scanner/scanner.go`, microsoft/TypeScript at `5739027c`,
  the pinned package) narrows a `VariableDeclaration` to its name and a
  `ReturnStatement` to its keyword, so a `.tt` file shows what a `.ts` twin
  shows.

### Decision 2: A pipeline's first operand is TypeScript's argument

- **Context**: `42 |> format` lowers to `format(42)`; TypeScript reports
  TS2345 at `42`, user text.
- **Decision and rationale**: Under Decision 1 it reads `Argument of type
  'number' is not assignable to parameter of type 'string'.` at `42`.
  `docs/ai/tt.md` defines `x |> f` as `f(x)`, so the sentence is accurate.
  A later step's input (`f(a)` inside `g(...)`) spans generated text and
  keeps the step wording.

## Work log

- 2026-10-01: Read `report`, `diagnostic_span`, `diagnostic_message`, and
  `projection::diagnostic_origin`; added `typescript_owned` and used
  TypeScript's span and message for an exactly mapped diagnostic.
- 2026-10-01: Added `tests/cases/compiler/typescriptOwnedDiagnostics.tt`;
  regenerated the editor, case, and practical baselines with
  `UPDATE_EXPECT=1` and read every diff; updated the native, CLI, workflow,
  and extension tests that asserted the restated form or the initializer
  column, and the practical manifests.
- 2026-10-01: `failingParity.txt` became empty and was removed by the
  runner, which expects it absent when no case differs.

## Issues and resolutions

### Issue 1: Tests pinned the restated form for plain TypeScript

- **Symptom**: 23 native and CLI tests, 3 workflow tests, and 4 extension
  tests failed: they searched for ``type mismatch: expected `string` `` or
  asserted the initializer's column.
- **Cause**: They described the old rule for user-written code.
- **Resolution**: Each now asserts TypeScript's message and position. Tests
  whose names stated the old rule were renamed
  (`plain_typescript_in_a_tt_file_is_reported_in_typescripts_words`,
  `a_mismatch_in_user_code_keeps_typescripts_words`,
  `types_reports_plain_typescript_diagnostics_in_typescripts_words`).
  `a_type_error_is_reported_at_its_position_in_the_tt_source` keeps its
  UTF-16 point with a multi-byte comment before the reported name.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/typescriptOwnedDiagnostics.tt`; the editor
  cases `plainTypeScript`, `plainTsx`, `twoErrorsAtOneCall`.
- **Observed failure**: Without the report change the case's
  `.errors.txt` is modified (``error[ts2322]: type mismatch: expected
  `string`, found `number` `` at 7:34 instead of TypeScript's message at
  7:14), and `editor_cases` reports `plainTypeScript.baseline` out of date
  with `diagnostics plainTypeScript.tt: differs`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --test editor_cases --test case_baselines --test snapshot
  --test practical_diagnostics --test cli_outputs --test cli --test native
  --test integration --test workflow_repairs --test corpus --test public_api
  --lib`: passed. The full gate is recorded in TASK-699.
- [x] Extension tests touched by the change passed (27).
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed files: `src/engine/semantics/report.rs`,
`src/engine/semantics/translate.rs`, `docs/ai/tt.md`,
`tests/cases/compiler/typescriptOwnedDiagnostics.tt` and its baselines,
`tests/baselines/reference/{booleanMismatchRendering,tryAlwaysFailing}.errors.txt`,
`tests/baselines/reference/editor/{plainTypeScript,plainTsx,twoErrorsAtOneCall}.baseline`,
the removed `tests/baselines/reference/editor/failingParity.txt`, the
practical-diagnostics fixtures, `tests/cli.rs`, `tests/cli/cases_01.rs`,
`tests/native.rs`, `tests/native/cases_0{1,2,3,5,7,9}.rs`,
`tests/native/cases_10.rs`, `tests/workflow_repairs.rs`, the extension's
`compiler.test.ts`, `server.test.ts`, and `typedcheck.test.ts`, the notes
on TASK-574, TASK-585, and TASK-675, `docs/tasks/INDEX.md`, and this record.
