# TASK-516: Exclude only an output root strictly inside a directory input

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: see `git log --grep TASK-516`

## Purpose

`ttc -o . src`, `ttc -o src src` and `ttc -o gen gen/src` reported
`no sources found`. An `-o` directory that equals or encloses a directory
input removed every source of that input from the build.

## Scope

- Included: the output-subtree exclusion in `build_jobs`
  (`src/main/build.rs`) and a CLI regression test.
- Excluded: the exclusion itself for an output root inside the input
  (TASK-321 Issue 3), which is unchanged, and named file inputs.

## Decisions

### Decision 1: Exclude the output tree only when it is a proper subtree of the input

- **Context**: TASK-321 Issue 3 introduced the exclusion so that
  `-o src/generated src` does not read its previous outputs back as inputs.
  `docs/ai/tt.md` (Modules) states the contract: "When `-o` is inside a
  directory input, that output subtree is excluded from source collection."
  The implementation dropped every collected file inside the output root
  without first asking whether that root was inside the input. When the root
  is the input or an ancestor of it, every source is inside the root.
- **Alternatives considered**: keep the per-file test and exempt files that
  are also under the input root. Every source is under the input root, so
  this would disable the exclusion for the case it exists for.
- **Decision and rationale**: `output_tree_inside` decides once per
  directory input whether the output root is strictly inside it. Lexical and
  canonical containment both count, matching `path_is_within`, so a symlinked
  output root is still recognized. The same directory, by either identity, is
  not strictly inside. When the root is the input or encloses it, its earlier
  outputs are already skipped by their ownership records (`owned_output`),
  as they are in a build without `-o`.

## Work log

- 2026-09-29: Reproduced `-o . src`, `-o src src`, `-o gen gen/src`: each
  exits 1 with `no sources found`.
- 2026-09-29: Added `output_tree_inside` and gated the retain on it. Added
  `an_output_directory_that_is_or_encloses_the_input_keeps_its_sources` to
  `tests/cli.rs`; it fails before the change and passes after.

## Issues and resolutions

### Issue 1: An output root enclosing the input excluded every source

- **Symptom**: `ttc -o . src` → `ttc: no sources found`, exit 1.
- **Cause**: `files.retain(|file| !path_is_within(file, dir))` ran for every
  directory input, whatever the relation between the output root and the
  input.
- **Resolution**: the retain runs only when the output root is strictly
  inside the directory input (Decision 1).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --test cli` (the new test fails without the change)
- [x] `node scripts/check-task-index`

## Result

Changed `src/main/build.rs` and `tests/cli.rs`. The existing
`an_output_directory_inside_the_input_is_not_recompiled` case still passes.
