# TASK-798: Project only the candidates the program contains

- **Status**: Complete
- **Started**: 2026-10-09
- **Completed**: 2026-10-09
- **Commit**: `TASK-798: Project only the candidates the program contains`

## Purpose

K9 (TASK-794 Decision 10): every `.tt` file under the project root was
lowered and served on each typed compile, including files the
configuration leaves out and nothing imports. The maintainer chose to fix
it.

## Scope

- Included: which scanned candidates a snapshot projects.
- Excluded: the membership rules themselves; TypeScript's program still
  decides them.

## Decisions

### Decision 1: The program decides membership over placeholders

- **Context**: Membership cannot be read off the configuration's file list
  (TASK-796 Decision 6): a hand-written `.ts` file may import a `.tt` file
  outside `include`, and a `.tt` file may reach one through a `.ts` module.
  But membership does not depend on a candidate's own text unless it is
  already a member: `files` and `include` match paths, and a module joins
  through an import only from a member.
- **Alternatives considered**: Projecting on TypeScript's first read (the
  host asking ttc for a module from its `readFile` callback) needs the
  projections made mid-ask to flow back into an immutable snapshot, and
  their contextual storage to be settled in another pass. It changes the
  host protocol and the snapshot model for the same result.
- **Decision and rationale**: With a configuration and a checker, a
  snapshot projects the files named, opened, requested and imported by
  projected `.tt` files, and the candidates the last membership answer
  contained. Every other candidate is served as a placeholder
  (`export {};`, as a blocked file already is), and the host is asked which
  modules the programs contain (`Project::program_members`, a contextual
  ask with no slots, which answers `projectModules`). A placeholder the
  programs contain is projected and the question is asked again, until no
  placeholder joins. Placeholders that stay out are kept in the snapshot
  (`Snapshot::deferred`) and served by every later ask, so the programs see
  the same files. Without a configuration or a checker every candidate is
  projected, as before.

## Work log

- 2026-10-09: Measured with 40 files of 100 matches each in `other/`
  (outside `include`) and `src/main.tt` (release build): `--check-types
  src/main.tt` took 1.2 s, against 0.45 s without `other/`.
- 2026-10-09: Changed `src/engine/project.rs` (`update_scoped`,
  `program_members`, `known_members`), `src/engine/projection.rs`
  (`assemble` serves deferred placeholders), `src/engine/snapshot.rs`
  (`Snapshot::deferred`). Added a native test.
- 2026-10-09: The same check takes 0.46 s with `other/`. A `.tt` file
  outside `include` imported by a `.ts` file, or reached through a `.ts`
  module from `src/main.tt`, is still projected and checked.
- 2026-10-09: Marked TASK-794 Decision 10 and the K9 part of TASK-796
  Decision 6 as superseded.

## Issues and resolutions

### Issue 1: A file a check names as a root was served as a placeholder

- **Symptom**: `tests/cli/server_print.rs::server_dependencies_check_a_file_its_configuration_leaves_out_as_a_root`
  failed: a server session opened for `app/src/in.tt`, asked for the
  dependencies of `app/out.tt`, no longer listed `out.tt` and the
  `shared/helper.ts` it imports.
- **Cause**: The check makes the inputs it names roots by request
  (`check_requested`'s `requested`), but the update before it did not know
  them, so `out.tt` was deferred and its program never read it.
- **Resolution**: `update_with_roots` takes the roots a check will name;
  they are projected, and the membership question names them as roots too.
  `check_for_dependencies` and `literal_members` pass theirs.

### Issue 2: A hand-written TypeScript file in the update was deferred

- **Symptom**: `tests/cases/editor/referencesReachNestedAndBuiltinCases.tt`
  gained a reference at `11:1-11:1`.
- **Cause**: The editor's references update passes the file it asks about
  (`node_modules/@tt/std/option.ts`) with the candidates; it was deferred
  and served as an `option.ts.ts` placeholder.
- **Resolution**: Only `.tt` and `.ttx` candidates are deferred; any other
  file in the update is projected as before.

## Regression test (fails before the fix)

- **Path**: `tests/native/cases_10.rs::only_the_candidates_the_program_contains_are_projected`
- **Observed failure**: the snapshot projected
  `["ext/y.tt", "ext/z.tt", "other/unused.tt", "src/main.tt"]`;
  `other/unused.tt` is in no program.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` with `TTC_REQUIRE_TSGO=1`
- [x] Baseline changes reviewed and committed with the change (none)

## Result

With a configuration, a snapshot projects only the candidates the
project's TypeScript programs contain; the rest are placeholders. The
example check went from 1.2 s to 0.46 s. Without a configuration, nothing
changes (`benches/compile.rs` opens such projects).
