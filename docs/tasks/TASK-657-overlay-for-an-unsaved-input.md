# TASK-657: Check an input that exists only as an `--overlay` buffer

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-657`

## Purpose

`ttc --check-types --tt-only --overlay <p> <p>`, the argv the editor's
one-shot fallback uses, failed with `No such file or directory` when `<p>`
was a new buffer never saved to disk. Input collection looked the path up
on disk before the overlay could stand in for it, although the overlay is
the text the check is asked about.

## Scope

- Included: `open_typed_project` in `src/main/typed.rs`, and a test in
  `tests/cli.rs`.
- Excluded: the untyped modes, which have no `--overlay`.

## Decisions

### Decision 1: An input an overlay names needs no file on disk

- **Context**: The server's `typedCheck` already opens the project of an
  unsaved document (`Engine::open_document_project`), so the two
  consumers of the engine disagreed about the same question.
- **Alternatives considered**: (a) Make the editor write a temporary file:
  the checked path would differ from the buffer's, and every diagnostic
  would name the wrong file. (b) Teach `Inputs::collect` about overlays:
  input collection is a filesystem walk shared by every mode, and only
  the typed modes have overlays. (c) In the typed driver, set aside the
  inputs that do not exist on disk and are named by an overlay (compared
  by `normalize_document_path`, the identity overlays are stored under):
  when no other input remains, open the first one's project with
  `open_document_project`, as the server does; otherwise open the project
  of the inputs on disk. The overlays are then opened as documents as
  before.
- **Decision and rationale**: (c). The responsible layer is the typed CLI
  driver, which owns how inputs and overlays combine, and the project it
  opens is the one the server opens for the same document.

## Work log

- 2026-09-30: Reproduced with `target/probe7-cli/cases/04`.
- 2026-09-30: Changed `open_typed_project`; added
  `an_overlay_checks_a_buffer_whose_file_is_not_saved_yet`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cli.rs`,
  `an_overlay_checks_a_buffer_whose_file_is_not_saved_yet`.
- **Observed failure**: With `open_typed_project` reversed: exit status 2
  (expected 1) and `ttc: <dir>/src/new.tt: No such file or directory (os
  error 2)` on stderr.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (the full gate, see TASK-654's record for the run)
- [x] Baseline changes reviewed and committed with the change (none)

## Result

An unsaved buffer is checked under its own path with the diagnostics a
saved one gets, and nothing is written to disk. Changed files:
`src/main/typed.rs`, `tests/cli.rs`.
