# TASK-696: Name the incompatible part of a union as TypeScript names it

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-696`

## Purpose

TASK-673 Issue 1: `const partly: number = either()`, with
`either(): number | boolean`, rendered ``found `false | true` `` with a
redundant ``required type: `number` ``, where TypeScript elaborates
`Type 'boolean' is not assignable to type 'number'.` TASK-695 made that
line TypeScript's own diagnostic, but its message lost the elaboration, and
the same union on generated code (a pipeline step's input, a match arm's
value) still read `false | true`.

## Scope

- Included: the backend host (`src/typescript/host.mjs`): the diagnostic
  message with its chain (`messageText`) and the literal generalization of
  a reported leaf (`relationPair`, `isLiteralType`,
  `couldHaveTopLevelSingletonTypes`); the `required type` rule of
  `mismatch_pair` (`src/engine/semantics/translate.rs`); the case
  `tests/cases/compiler/partialUnionMismatch.tt`; the TASK-673 note.
- Excluded: related information's own chains (the labels keep their head
  message, as before).

## Decisions

### Decision 1: Carry TypeScript's elaboration chain in the message

- **Context**: The host sent `d.text`, the head of the diagnostic. The
  TypeScript 7 API's `DiagnosticResponse` carries the elaboration as
  `messageChain` (`dist/api/proto.generated.d.ts`), and TypeScript's own
  formatter prints it as the head followed by each chained message on its
  own line, indented two spaces per level (`flattenDiagnosticMessage` in
  `dist/api/diagnosticFormatter.js` of the pinned `typescript`
  7.1.0-dev.20260826.1).
- **Alternatives considered**: keep the head only (the user loses the line
  that names the incompatible part, and a `.tt` file shows less than the
  same `.ts` file); render the chain in Rust from a structured list (a
  second formatter of TypeScript's text).
- **Decision and rationale**: The host flattens the chain exactly as the
  formatter does, so the message a diagnostic carries is the one `tsc`
  prints. With TASK-695, `partly` now reads `Type 'number | boolean' is not
  assignable to type 'number'.` followed by `Type 'boolean' is not
  assignable to type 'number'.`, byte-identical to `tsc` on the emitted
  file.

### Decision 2: Generalize a literal leaf where TypeScript does

- **Context**: On generated code ttc restates the mismatch from the
  backend's leaves. TypeScript names the part `boolean` because
  `reportRelationError` (`tsc/internal/checker/relater.go`,
  microsoft/TypeScript at `5739027c`) shows a literal source through
  `getBaseTypeOfLiteralType` when the target is not `never` and
  `typeCouldHaveTopLevelSingletonTypes(target)` is false; the relater
  itself reports the first failing constituent (`eachTypeRelatedToType`).
  The TypeScript 7 API exposes `getBaseTypeOfLiteralType`,
  `getConstraintOfTypeParameter`, and `getBaseConstraintOfType`, so the
  rule can be applied to the checker's own types; TASK-673's note that the
  API cannot form the union of chosen constituents still holds, and is not
  needed.
- **Alternatives considered**: folding `false | true` into `boolean` in
  the renderer (a type-name special case, contract 3, and not TypeScript's
  rule: `"a" | "b"` against `number` is shown as `string`).
- **Decision and rationale**: Every leaf the backend forms from two types
  goes through `relationPair`, which applies TypeScript's condition with
  `isLiteralType` and `couldHaveTopLevelSingletonTypes` transcribed from
  `checker.go` and `relater.go` (an instantiable target uses its
  constraint: `getConstraintOfTypeParameter` for a type parameter, else
  `getBaseConstraintOfType`, as `getConstraintOfType` does). `false` and
  `true` each become `boolean` and the existing deduplication leaves one
  pair.

### Decision 3: `required type` only when the expected type was reduced

- **Context**: `mismatch_pair` added the complete contextual type whenever
  either side was reduced, so a reduced found type printed
  ``required type: `number` `` under ``expected `number` ``, the redundant
  line TASK-673 removed for a whole union.
- **Decision and rationale**: The line names the complete contextual type
  when the expected type shown was reduced from it; a reduction of the
  found side alone adds nothing on the expected side. No other baseline
  changed.

## Work log

- 2026-10-01: Read `reportRelationError`, `eachTypeRelatedToType`,
  `typeCouldHaveTopLevelSingletonTypes`, and `isLiteralType` at the pinned
  commit, and the API's `messageChain` and formatter.
- 2026-10-01: Added `messageText`; `booleanMismatchRendering` and
  `tryAlwaysFailing` baselines gained TypeScript's elaboration lines, now
  equal to the `tsc` section of the same files.
- 2026-10-01: Added `partialUnionMismatch.tt` (a plain line, a pipeline
  step, a pipeline argument, a match arm); its restated lines read
  `false | true` with `required type` before `relationPair`.
- 2026-10-01: Ran case, editor, snapshot, practical, CLI, native,
  workflow, integration, and library tests with `UPDATE_EXPECT=1`; only the
  three case baselines changed.

## Issues and resolutions

### Issue 1: The first test run was killed

- **Symptom**: `cargo test` with five suites at once exited 137.
- **Cause**: Memory, with another agent building in the same container.
- **Resolution**: Suites run one or a few at a time with
  `RUST_TEST_THREADS=2`.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/partialUnionMismatch.tt`;
  `tests/cases/compiler/booleanMismatchRendering.tt`.
- **Observed failure**: Without the changes the `partialUnionMismatch`
  baseline is modified: the elaboration line of `direct` is missing and the
  three restated lines read ``found `false | true` `` (or ``receives
  `false | true` ``) with ``required type: `number` ``;
  `booleanMismatchRendering` loses the elaboration lines of `named` and
  `partly`.

## Verification

- [x] `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`.
- [x] `cargo test --test case_baselines --test editor_cases --test snapshot
  --test practical_diagnostics --test cli_outputs --test native --test cli
  --test workflow_repairs --test integration --lib`: passed. The full gate
  is recorded in TASK-699.
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed files: `src/typescript/host.mjs`,
`src/engine/semantics/translate.rs`,
`tests/cases/compiler/partialUnionMismatch.tt` and its baselines,
`tests/baselines/reference/{booleanMismatchRendering,tryAlwaysFailing}.errors.txt`,
the TASK-673 note, `docs/tasks/INDEX.md`, and this record.
