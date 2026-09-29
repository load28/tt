# TASK-520: Report each file of a `--types` declaration collision once

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: see `git log --grep TASK-520`

## Purpose

With `src/x.tt` and `lib/x.tt`, `ttc --types -o types src lib --json-report`
exits 3 as it should. But its `failed` list names `types/x.tt.d.ts` and
`types/x.tt.d.ts.map` twice each, and stderr repeats both lines. TASK-509
Decision 1 says every file the run tried to write appears exactly once in the
report.

## Scope

- Included: the collision branch of `write_declarations`
  (`src/main/typed.rs`) and a regression test in `tests/native/cases_07.rs`.
- Excluded: the collision rule itself (any collision fails every planned
  file and writes nothing, TASK-509 Decision 3), its error text, and the
  non-colliding path. That path has distinct targets by construction, so
  each file is already recorded once.

## Decisions

### Decision 1: Fail the planned files, not each module's claim

- **Context**: The collision branch walked `targets`, one entry per emitted
  module, and failed each target and its map. A collision means two modules
  share a target, so the same file was failed once per claimant.
- **Alternatives considered**: making `WriteOutcome` drop a path it has
  already recorded. That would hide any later path that records a file
  twice instead of making the plan correct. It would also have to choose
  between two different results for one file.
- **Decision and rationale**: the collision branch fails the distinct files
  of the output plan. That is the standard-library declarations, then each
  target and its map, in plan order, compared by normalized absolute path,
  the identity `WriteOutcome` reports. Each file appears once in `failed`
  and once on stderr, and the order is unchanged.

## Work log

- 2026-09-29: Reproduced: four `failed` entries and four stderr lines for
  two files.
- 2026-09-29: Changed the collision branch to fail each distinct planned
  file once. Added `types_names_each_file_of_a_declaration_collision_once`
  (two directory inputs `src/a` and `src/b`, each with `x.tt`). It fails
  without the change.

## Issues and resolutions

### Issue 1: A reproduction outside the tsconfig include did not collide

- **Symptom**: with the test project's `tsconfig.json` (which includes
  `src`), a `lib/x.tt` input produced no declaration, and the run exited 0.
- **Cause**: the typed project admits sources through the user's
  configuration, so the test fixture has to keep both inputs under `src`.
- **Resolution**: the test uses `src/a` and `src/b` as the two input roots.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --test native`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/main/typed.rs` and `tests/native/cases_07.rs`.
