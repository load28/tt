# TASK-563: Leave ttc's output directory out of the TypeScript program

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-563: Leave the output directory out of the TypeScript program`

## Purpose

`ttc --types -o <dir>` failed on its own sidecars from the second run: with
a `tsconfig.json` that has no `include`, the default glob reaches `<dir>`,
so the program checked the sources against the `.tt.d.ts` files the first
run wrote (TS2451 "Cannot redeclare block-scoped variable" for a script
file; TS2307 was reported for an importing module). Watch mode hit the same
after the first pass.

## Scope

- Included: the typed engine's TypeScript program for a project opened with
  an output directory (`--types`, including `--watch`)
- Excluded: the standalone contextual pass of a plain build, which does not
  know the build's `-o` and asks only for contextual types (no
  diagnostics); the engine's own source scan, which already excluded the
  output directory (`project_sources`)

## Decisions

### Decision 1: Hide the output directory from the program's file globbing

- **Context**: `tsc` leaves its own `outDir`/`declarationDir` out of a
  default `include`. Only file discovery is affected: an explicit `files`
  entry or an import still reaches a file there. ttc's sidecar directory is
  the same kind of directory, but TypeScript does not know about it.
- **Alternatives considered**: Add the directory to the served root
  configuration's `exclude` (setting `exclude` removes TypeScript's own
  default exclusion of `outDir` when the user has none, and an inherited
  `exclude` from `extends` would have to be merged by hand); set
  `declarationDir` in the served configuration (changes declaration emit
  paths and requires `declaration`, TS5069).
- **Decision and rationale**: The layered filesystem the host gives the
  compiler lists an output directory as empty. That is exactly what an
  `exclude` does — no glob finds a file there — while `files`, imports, and
  the user's own `exclude` and `outDir` behave as before. The engine passes
  the directory when it opens a project with `out_dir`
  (`NativeBackend::excluding_output`); the host receives it in the open
  request (`outputs`). The directory is named by the canonical path it will
  have once it exists (`engine::paths::prospective`), so a watch that
  creates it after the session opened still hides it.

## Work log

- 2026-09-29: Reproduced under `target/probe4-cli/p2b` (module `esnext`,
  resolution `bundler`, `src/g.tt` holding `const globalThing: number = 1;`):
  run 1 exits 0, runs 2 and 3 exit 1 with TS2451 at `src/g.tt` and
  `types/g.tt.d.ts`. The nodenext two-module layout did not reproduce
  TS2307 on this head, and is kept in the regression test as a second
  layout.
- 2026-09-29: `src/typescript/host.mjs` — an `outputs` list in the open
  request, and `getAccessibleEntries` answers an output directory as empty.
  `src/typescript/native.rs` — `outputs` on the backend, sent at open.
  `src/engine/mod.rs` — the project's `out_dir` becomes an output of its
  backend. `src/engine/paths.rs` — `prospective`.
- 2026-09-29: Added `types_output_directory_is_not_a_program_input`
  (`tests/workflow_repairs.rs`): two consecutive `--types -o types src`
  runs and a watch edit for both layouts. Without the fix the second run
  fails with TS2451.

## Issues and resolutions

### Issue 1: Later `--types` passes read the sidecars as inputs

- **Symptom**: `error[ts2451]: Cannot redeclare block-scoped variable
  'globalThing'` at `src/g.tt:1:7` and `types/g.tt.d.ts:2:15`, exit 1.
- **Cause**: The engine excluded `out_dir` from its own scan, but the
  configured TypeScript program globbed the directory through the
  configuration's default `include`.
- **Resolution**: Decision 1.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test cli --test native --test workflow_repairs --test engine_cache --test sidecar --test content_mapper`

## Result

Changed `src/typescript/host.mjs`, `src/typescript/native.rs`,
`src/engine/mod.rs`, `src/engine/paths.rs`, and `tests/workflow_repairs.rs`.
Repeated `--types -o <dir>` runs and watch passes check the same program as
the first run.
