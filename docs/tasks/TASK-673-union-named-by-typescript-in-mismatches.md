# TASK-673: Name a found union as TypeScript names it in a type mismatch

> **Issue 1 resolved by TASK-695 and TASK-696**: the plain line is
> TypeScript's own diagnostic with its elaboration, and a restated leaf
> generalizes a literal as TypeScript's `reportRelationError` does, so the
> partly incompatible union reads `boolean`.

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-673`

## Purpose

`const x: number = flag()`, where `flag(): boolean`, rendered as
``type mismatch: expected `number`, found `false | true` `` with a
redundant ``required type: `number` `` line, in the CLI and the editor.
TypeScript says `Type 'boolean' is not assignable to type 'number'`. The
same held for an alias (`Dir`, rendered as its literals) and for a
`boolean` value imported from another module.

## Scope

- Included: `incompatibleLeaves` in `src/typescript/host.mjs`, the case
  `tests/cases/compiler/booleanMismatchRendering.tt` and its baselines.
- Excluded: `mismatch_pair` in `src/engine/semantics/translate.rs`, which
  renders whatever leaves the backend reports and needs no change; a
  union only part of which is incompatible (Issue 1).

## Decisions

### Decision 1: The backend reports the whole union when no member reduces

- **Context**: The backend reduces a mismatch to its minimal incompatible
  leaves by descending through the found type's union constituents
  (`incompatibleLeaves`). TypeScript represents `boolean` as the union
  `false | true` (and an alias of literals as their union), so each
  constituent became a leaf, `mismatch_pair` joined them as
  `false | true`, and because that differed from the found type it added
  the complete expected type as `required type`.
- **Alternatives considered**:
  - Recognize `false | true` in `mismatch_pair`: a type-name special case
    in the renderer (AGENTS.md contract 3), and it would not cover an
    alias such as `Dir`.
  - Stop descending through unions: loses the reduction where it helps
    (`number | boolean` against `number` names only the incompatible
    part).
- **Decision and rationale**: When every constituent of the found union is
  incompatible and none reduces further (each leaf is the constituent
  against the whole expected type), the reduction found nothing smaller
  than the found type, so the found type itself is the leaf, printed by
  `checker.typeToString`: `boolean`, `Dir`, or `string | boolean`, as
  TypeScript names the union in its own messages (the checker prints a
  union through `typeToString`, which folds `true | false` into `boolean`
  and a union behind an alias into the alias name). `mismatch_pair` then
  finds the leaf equal to the complete pair and renders no
  `required type`. The rule lives in the backend, which owns the
  reduction; the renderer is unchanged.

## Work log

- 2026-09-30: Added the probe's `booleanMismatchRendering.tt` as a
  compiler case and generated its baseline with the unfixed backend:
  three ``found `false | true` `` with ``required type: `number` ``.
- 2026-09-30: Changed `incompatibleLeaves`; extended the case with an
  imported `boolean`, an alias, and a partly incompatible union; ran
  `case_baselines`, `snapshot`, `practical_diagnostics`, `cli_outputs`
  (no other baseline changed) and the native suite.

## Issues and resolutions

### Issue 1: A partly incompatible union still lists its boolean literals

- **Symptom**: `const partly: number = either()`, with
  `either(): number | boolean`, renders ``found `false | true` `` and
  ``required type: `number` ``, where TypeScript elaborates `Type
  'boolean' is not assignable to type 'number'`.
- **Cause**: The incompatible part is a subset of the found union.
  TypeScript prints it as `boolean` by folding the literals while
  formatting a union (`formatUnionTypes` in `checker.ts`), but the
  TypeScript 7 API the backend uses has no way to form or print a union
  of chosen constituents (`Checker` offers `getBooleanType` and
  `typeToString`, no `getUnionType`).
- **Resolution**: Left as is and pinned by the case's `partly` line, so a
  later change is a reviewed diff. Folding the literals in tt would
  re-implement TypeScript's printer.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/booleanMismatchRendering.tt`
  (`tests/baselines/reference/booleanMismatchRendering.errors.txt`).
- **Observed failure**: Without the fix the three `flag()` errors read
  ``type mismatch: expected `number`, found `false | true` `` followed by
  ``required type: `number` ``; the committed baseline has
  ``found `boolean` `` and no `required type`, so the unfixed run reports
  a modified baseline.

## Verification

- [x] `cargo test --test case_baselines --test snapshot --test
  practical_diagnostics --test cli_outputs`: passed, no other baseline
  changed.
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native`: 163 passed.
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed files: `src/typescript/host.mjs`,
`tests/cases/compiler/booleanMismatchRendering.tt`,
`tests/baselines/reference/booleanMismatchRendering.{ts,errors.txt,map.txt,types}`,
`docs/tasks/INDEX.md`, and this record.
