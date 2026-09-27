# TASK-394: Check unsaved overlays and rebuild importers of deleted files in watch

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

Two CLI paths identified files only by canonicalizing them, which requires the
file to exist. `--overlay` therefore refused a never-saved buffer, and `-w`
never rebuilt the importers of a deleted or renamed file, so a watch session
reported a different result than a one-shot build of the same tree.

## Scope

- Included: `--overlay` path identity and first-pass membership
  (`src/main/command.rs`, `src/main/typed.rs`); untyped watch change detection
  and dependent selection (`src/main/output.rs`).
- Excluded: Removing stale outputs of deleted sources (a one-shot build does
  not remove them either), the typed watch (`--check-types -w`/`--types -w`),
  which already rescans the engine project, and positional inputs that do not
  exist.

## Defects

1. `ttc --check-types --overlay src/new.tt src` with the buffer on stdin
   failed with `ttc: --overlay src/new.tt: No such file or directory` when
   `src/new.tt` had never been saved. The CLI used `Path::canonicalize`, while
   the engine already defines `ttc::engine::normalize_document_path`
   (TASK-360) for document identity independent of file existence.
2. With `ttc -w -o out src`, deleting `src/s.tt` (or renaming it) did not
   rebuild `src/u.tt`, which imports it. The watch kept showing the previous
   `match-not-exhaustive` error, whereas a one-shot build of the same tree
   succeeds. The watch only treated files present in the new job list as
   changed, and `with_dependents` matched import targets through
   `canonicalize`, which fails for a deleted file.

## Decisions

### Decision 1: Use the engine's document identity for overlays

- **Context**: The overlay is keyed by the path the buffer occupies in the
  project. TASK-360 decided that a document's identity is its canonical parent
  plus its leaf name, so a new file has the same identity before and after its
  first save.
- **Alternatives considered**: Creating a temporary file writes user data;
  keeping the canonicalize requirement leaves the CLI behind the editor
  server.
- **Decision and rationale**: Call `ttc::engine::normalize_document_path`,
  the single engine boundary for this identity. A path whose directory does
  not exist is still an error.

### Decision 2: An overlaid tt document is a root of the first pass

- **Context**: With the identity fixed, the unsaved file was still not
  checked, because the first pass layers the project scan and the named
  inputs, both of which come from disk. The server's `typedCheck` already adds
  the document to the files it snapshots.
- **Decision and rationale**: Add overlaid `.tt`/`.ttx` paths
  (`ttc::SourceKind::from_tt_path`) to the first pass's files, matching
  `Engine::open_document_project`, which admits only tt documents as requested
  roots. For an already saved file the set is unchanged after deduplication.

### Decision 3: A removed job is a change, and import identity does not require existence

- **Context**: A one-shot build compiles the importer against the current
  tree. For the watch to agree, a deletion must select the same importers that
  an edit selects.
- **Alternatives considered**: Rebuilding every job on any deletion is correct
  but discards the incremental selection. Tracking a reverse dependency graph
  duplicates the import scan that `with_dependents` already performs.
- **Decision and rationale**: Treat files present in the previous round's
  stamps but absent from the current job list as changed, and compare import
  targets with the same document identity as Decision 1 (falling back to the
  lexical absolute path when even the parent is gone). A rename is a deletion
  plus an addition, so both halves are handled. A round prints only when it
  selected jobs, so deleting a file nothing imports stays silent as before.

## Work log

- 2026-09-27: Reproduced both defects (`w2` investigation material). Added
  `an_overlay_checks_a_file_that_was_never_saved` and
  `watch_rebuilds_importers_of_a_deleted_or_renamed_file` to
  `tests/cli_outputs.rs`. Before the change, the first failed with "No such
  file or directory" and the second observed `1 file(s) ok` after a rename
  instead of `2 file(s) ok`.
- 2026-09-27: Switched the overlay to `normalize_document_path`. The overlay
  test then exited successfully without checking the buffer, which exposed
  Decision 2; after adding overlaid tt documents to the first pass, it passed.
- 2026-09-27: Updated watch change detection and `with_dependents`; the watch
  test passed for rename, re-creation, and deletion.
- 2026-09-27: Updated `overlay_reports_a_missing_value_and_a_missing_file` in
  `tests/cli.rs`, which pinned the defect (a missing leaf was an error), to
  `overlay_reports_a_missing_value_and_a_missing_directory`. Updated the watch
  bullet in `docs/ai/tt.md`.

## Issues and resolutions

### Issue 1: The first overlay regression needed a TypeScript toolchain

- **Symptom**: In a workspace outside the repository the check reported that
  no TypeScript compiler was found, and the `val` diagnostic did not appear.
- **Cause**: The `val` mutation check belongs to the typed layer, as in the
  existing `overlay_checks_the_buffer_rather_than_the_saved_file` test.
- **Resolution**: The test runs in a repository workspace and is gated on the
  pinned toolchain like the other typed CLI tests.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Changed `src/main/command.rs`, `src/main/typed.rs`, `src/main/output.rs`,
`tests/cli_outputs.rs`, `tests/cli.rs`, and `docs/ai/tt.md`. An editor can
check a buffer that has never been saved, and a watch round after a deletion
or rename rebuilds the importers and matches a one-shot build.
