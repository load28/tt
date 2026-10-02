# TASK-589: Let an input take over the unedited output its vanished predecessor left

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-589: Let an input take over the unedited output its vanished predecessor left`

## Purpose

Migrating a file to tt broke the build: `ttc -o out src` with `src/a.ts`,
then renaming it to `src/a.tt` and rebuilding, refused with "output is not
owned by this input or has been edited" and kept the stale `out/a.ts`. A
watch could never recover, since every round refused the same write.

## Scope

- Included: the ownership check before a write (`src/main/ownership.rs`),
  `docs/ai/tt.md`, regression tests.
- Excluded: the record format and what counts as a ttc output
  (`ttc::ownership`, TASK-587).

## Decisions

### Decision 1: An unedited output whose recorded source input is gone has no owner

- **Context**: A record names the output's owner: a source path
  (canonical) or a support module (`@tt/std/...`). The protection exists
  so that ttc never replaces an authored or edited file, and never lets one
  input overwrite another input's output (TASK-383, TASK-393).
- **Alternatives considered**: (a) Let any input overwrite any unedited
  output: two live inputs mapping to one output (`a/k.ts` and `b/k.tt`
  under one `-o`) would overwrite each other silently. (b) Delete outputs
  whose source was deleted, in the build and the watch: a new policy
  (cleaning) that would remove files between runs the user did not ask
  about, and still would not help a one-shot run after a rename. (c) Ask
  the user to delete the output and its record: the current behaviour,
  which a watch cannot recover from.
- **Decision and rationale**: An output is taken over when it is still
  byte-identical to its record and the record names a source whose path no
  longer exists (`symlink_metadata` reports `NotFound`, so an unreadable
  path is not treated as gone). A support record (current `support` key,
  or the legacy `source` spelling ending in `@tt/std/<file>`, which
  `owns` already accepts) always has its owner. Edited outputs and outputs
  of a still-existing input keep the refusal.

## Work log

- 2026-09-30: Reproduced with `target/probe5-cli/mig`: the rebuild and the
  watch both refused `out/a.ts`.
- 2026-09-30: Added `orphaned` to `src/main/ownership.rs` and used it in
  `check_output_owner`.
- 2026-09-30: Added
  `a_renamed_input_takes_over_the_output_its_predecessor_left` (rename,
  edited output, still-existing input) and
  `build_watch_recovers_from_a_renamed_input`
  (`tests/workflow_repairs.rs`); both fail with `orphaned` disabled and
  pass with it.

## Issues and resolutions

None.

## Verification

- [x] `cargo test --test cli_outputs --test workflow_repairs` (with `TTC_REQUIRE_TSGO=1`)
- [x] Full gate run once at the end of the TASK-587–592 series; see TASK-592.

## Result

Changed `src/main/ownership.rs`, `docs/ai/tt.md`, and
`tests/workflow_repairs.rs`.
