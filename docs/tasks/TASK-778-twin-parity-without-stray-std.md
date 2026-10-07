# TASK-778: Pin the TypeScript-twin parity of an `@tt/std` case as a clean checkout answers it

- **Status**: Complete
- **Started**: 2026-10-07
- **Completed**: 2026-10-07
- **Commit**: —

## Purpose

CI's `fmt / clippy / test` job fails on `main` (`1cc08aa9`) and on this
branch: `every_editor_case_matches_its_baseline` reports that
`tests/baselines/reference/editor/recoveryTryOperandBeforeStatement.baseline`
is out of date. The baseline was recorded where the TypeScript twin could
resolve `@tt/std`, which a clean checkout cannot.

## Scope

- Included: the two editor baselines a clean checkout produces.
- Excluded: any change to the compiler, the editor, or the parity rules.

## Decisions

### Decision 1: Keep the twin without `@tt/std` and record what it answers

- **Context**: The twin project is written under the repository, so
  `tsgo --lsp` resolves packages by walking up to the repository's
  `node_modules`. A local checkout held a stray, ignored
  `node_modules/@tt/std` (not in `package.json`), so the twin resolved the
  standard library there; CI's checkout has none and reports `TS2307` for
  both imports, `width` and `height` as `unknown`, and a different hover.
  `recoveryTryOperandBeforeStatement` is the only twin that imports
  `@tt/std`.
- **Alternatives considered**: (a) Materializing the standard library into
  the twin's project: `parity_view` documents `@tt/std` as "a package a
  twin's project does not have", so this would change the parity contract
  rather than the record of it. (b) Recording what a clean checkout
  answers.
- **Decision and rationale**: (b). With the stray directory moved out of
  the repository, `UPDATE_EXPECT=1 cargo test --test editor_cases`
  produces the baseline CI's `fix_baselines.patch` carries byte for byte,
  and `failingParity.txt` gains `recoveryTryOperandBeforeStatement hover
  width`. CI's patch deleted `failingParity.txt` only because the failing
  assertion stops the run before it writes that file.

## Work log

- 2026-10-07: Read the failing job of runs `37376692369` (`main`) and
  `37600030738` (this branch) and the `fix_baselines.patch` artifact;
  found the stray `node_modules/@tt` in the local checkout; moved it out
  of the repository and regenerated the editor baselines.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: no compiler behaviour changes. The failing test is
`tests/editor_cases.rs::every_editor_case_matches_its_baseline` on a clean
checkout, as CI ran it.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --test editor_cases` without the stray directory
- [x] Baseline changes reviewed: they equal CI's patch for the `.baseline`
  file, plus the one parity line it implies

## Result

Complete. Changed `tests/baselines/reference/editor/recoveryTryOperandBeforeStatement.baseline`
and `tests/baselines/reference/editor/failingParity.txt`.
