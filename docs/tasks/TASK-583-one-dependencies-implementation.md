# TASK-583: Answer `--dependencies` and the server's `dependencies` through one implementation

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: the `TASK-583` commit on this branch

## Purpose

`ttc help workflow` says the server's `dependencies` answers what
`ttc --dependencies` prints, but the two were separate code: with a
`tsconfig.json` TypeScript cannot parse, the command line listed 86 paths and
the server 3, and for a missing module the server said
`cannot read: No such file or directory` where the command line says
`ttc: src/x.tt: No such file or directory`.

## Scope

- Included: one engine implementation, `Project::dependencies_of`, with the
  command's own input collection (`Inputs::collect`, which
  `Engine::open_project` now uses too); the server's check record keyed on
  what the recorded check covered; Rust tests; `docs/ai/tt.md`.
- Excluded: the server's check record itself (TASK-567 Decision 2), which
  stays a memo in front of the shared implementation.

## Decisions

### Decision 1: The files a request names are roots by request on both sides

- **Context**: The command line opened a project for its inputs, whose named
  files are roots by request ("a named file is a root by request, so it is
  checked even when the project's own configuration does not list it",
  `Engine::open_collected`). The server checked the file's workspace project
  with the file only among its candidates. When the configuration admits the
  file the difference is invisible; when it does not (a configuration
  TypeScript cannot parse admits nothing, or the file is outside `include`),
  the command line checked the file in its default program and reported what
  that program read, and the server reported almost nothing.
- **Alternatives considered**: (a) Make the command line stop treating named
  files as roots for `--dependencies`: every other command treats them as
  roots, and the server's `typedCheck` makes its document a root the same
  way (an open document is one). (b) Record the file as a root of the
  workspace project for good: every later check of that project would carry
  it, and the server's record would have to change with every new file.
- **Decision and rationale**: `Project::dependencies_of(&Inputs)` checks the
  candidates (`scan` plus every collected source) with the inputs' named
  files as roots for that check only (`check_requested`, which `check`
  calls with none), rejects an internal backend failure, and answers
  `Project::dependencies()`. The command line calls it on the project it
  opens for its inputs; the server calls it on the file's live project. The
  inputs of both are collected by `Inputs::collect`, the function
  `open_project` uses, so a missing file, a file that is not a tt source, or
  an input with no sources fails with the same sentence naming the input.

### Decision 2: The server's record stands only for a file its check covered

- **Context**: The server skips the check while the project's candidates and
  watch-path stamps are unchanged (TASK-567 Decision 2). With roots by
  request, a record made for one file does not answer for a file that check
  did not contain: that file's own program was never checked.
- **Alternatives considered**: Keying the record on the requested file
  rechecks the whole project once per module (O(N²), what TASK-567 removed).
- **Decision and rationale**: `check` records the sources the TypeScript
  programs of that check contained (the `projectModules` answer, which
  includes a root by request in its default program), and
  `Project::checked(path)` answers whether the last check covered a path (or
  had no TypeScript program to leave it out of). The server's record stands
  while the candidates and stamps are unchanged and every named file was
  covered. For a 40-module project, the first request took 743 ms and the
  other 39 at most 6 ms each, as before.

## Work log

- 2026-09-29: Reproduced with the finder's `probe4-cli/bc` (broken
  `tsconfig.json`): 86 paths from the command line, 3 from the server; and
  the two error texts for a missing module.
- 2026-09-29: Added `Inputs`, `Engine::open_inputs`, `Project::candidates`,
  `Project::dependencies_of`, `Project::checked`, `check_requested`
  (`src/engine/mod.rs`, `src/engine/project.rs`); moved `dependencies_mode`
  (`src/main/modes.rs`) and the server's `dependencies` (`src/server.rs`)
  onto them. The probe's answers are equal byte for byte, including for a
  second request of the same file and for files of `probe4-cli/vp`.
- 2026-09-29: Tests; `docs/ai/tt.md`.

## Issues and resolutions

### Issue 1: The existing server/CLI comparison never reached TypeScript

- **Symptom**: `server_dependencies_answer_what_dependencies_prints` passed
  in 0.07 s.
- **Cause**: Its project lives in the system temp directory
  (`Workspace::new`), where no TypeScript resolves, so both sides answered
  without a program.
- **Resolution**: The new tests use `Workspace::in_repo`, which resolves the
  repository's TypeScript; the existing test is unchanged.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` (full, `--test-threads=4`): 1659 passed, 0 failed, with TASK-580..583 applied
- [x] New `server_dependencies_check_a_file_its_configuration_leaves_out_as_a_root`
  (a broken configuration, and a file outside `include` asked after one
  inside it) and `server_dependencies_refuse_what_dependencies_refuses_in_its_words`
  (`tests/cli/server_print.rs`): both fail with the TASK-582 sources; the
  first also fails when the server's record ignores what its check covered.
- [x] `TTC_BINARY=<worktree>/target/debug/ttc npm --prefix integrations/unplugin test`: 18 passed
- [x] `TTC_BINARY=<worktree>/target/debug/ttc npm --prefix packages/create-tt run test:e2e`: 1 passed

## Result

Changed files: `src/engine/mod.rs`, `src/engine/project.rs`,
`src/main/modes.rs`, `src/server.rs`, `tests/cli/server_print.rs`,
`docs/ai/tt.md`, `docs/tasks/INDEX.md`, this record.

`--dependencies` and the server's `dependencies` run one implementation, from
the same input collection, and answer the same object or the same error.
