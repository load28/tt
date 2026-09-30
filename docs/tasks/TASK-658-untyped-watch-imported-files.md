# TASK-658: Rebuild an untyped watch when a `.tt` file an input imports changes

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-658`

## Purpose

`ttc -w -o out src` did not rebuild `src/u.tt` when `shared/s.tt`, which
`u.tt` imports and which is not an input, changed: the watch stamped only
the input files. A variant gaining a tag in `s.tt` left `u.tt`'s output
and its missing exhaustiveness error stale until `u.tt` itself was saved.

## Scope

- Included: `watch_mode` in `src/main/output.rs`, `compile_reads` and
  `extern_module` in `src/main/loading.rs`, and a test in
  `tests/cli_outputs.rs`.
- Excluded: the typed watch, which already observes the engine's
  dependencies (`docs/ai/tt.md`, `ttc --dependencies`).

## Decisions

### Decision 1: Watch exactly what a one-shot compile reads

- **Context**: The untyped compile of a file reads the file and, for each
  import that brings names in, the exported variants of the module beside
  it (`collect_extern_variants`); that is its whole dependency closure,
  since `exported_variants` reads one file.
- **Alternatives considered**: (a) Watch every `.tt` under the inputs'
  directories: it misses `../shared/` and rebuilds for unrelated files.
  (b) Ask the engine's `dependencies`: it opens a TypeScript
  project, which the untyped mode exists to avoid. (c) Factor the module an
  import reads into `extern_module`, used by both `collect_extern_variants`
  and a new `compile_reads(file)`, and stamp the union over the inputs.
- **Decision and rationale**: (c). The compile and the watch share one
  definition, so they cannot drift. A changed non-input reaches its
  importers through the existing `with_dependents`. `compile_reads` is
  recomputed only when an input's own stamp changes.

## Work log

- 2026-09-30: Reproduced with `target/probe7-cli/cases/08`.
- 2026-09-30: Added `extern_module` and `compile_reads`; the watch round
  stamps their union; added
  `watch_rebuilds_an_input_when_a_tt_file_it_imports_changes`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cli_outputs.rs`,
  `watch_rebuilds_an_input_when_a_tt_file_it_imports_changes`.
- **Observed failure**: With `loading.rs` and `output.rs` reversed, no
  round followed the edit of `shared/s.tt`: the wait for "1 file(s)
  rebuilt, with errors" ended in `Err(Timeout)` after 10 s.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (the full gate, see TASK-654's record for the run)
- [x] Baseline changes reviewed and committed with the change (none)

## Result

The untyped watch rebuilds an input when any file its compile reads
changes. Changed files: `src/main/loading.rs`, `src/main/output.rs`,
`tests/cli_outputs.rs`.
