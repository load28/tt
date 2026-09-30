# TASK-587: Leave ttc's published outputs out of the TypeScript program

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-587: Leave ttc's published outputs out of the TypeScript program`

## Purpose

After an in-place build (`ttc src` writes `src/m.ts` beside `src/m.tt`,
with its `.m.ts.ttc-output.json` ownership record), `ttc --check-types src`
took the generated `src/m.ts` into the TypeScript program: every error was
reported twice (once at `src/m.ts`, without a snippet), and a script file
got a false TS2451 "Cannot redeclare block-scoped variable" against its own
output. The same held for a `-o build` tree the configuration globs.

## Scope

- Included: the ownership rule's home (`src/ownership.rs`, moved out of the
  CLI), the host's directory listing (`src/typescript/host.mjs`), the
  backend's request loop (`src/typescript/native.rs`), the inferred
  project's source list (`src/engine/project.rs`), `docs/ai/tt.md`, and a
  regression test.
- Excluded: `--types` declaration sidecars, which carry no ownership
  record and are already hidden by name in an output directory (TASK-568);
  the user's own `tsc` run through the content mapper, which does not go
  through the engine.

## Decisions

### Decision 1: One owner for "is this file ttc's output", asked across the host boundary

- **Context**: The ownership record (TASK-383, TASK-393) is what defines a ttc
  output: a record, and the file still holding the bytes it names. The
  compile directory scan used it (`src/main/build.rs`), but it lived in the
  CLI binary, where the typed engine could not reach it. The TypeScript
  program's files come from two places: the engine's own root scan for an
  inferred project (no `tsconfig.json`), and TypeScript's `include`
  globbing through the host's `getAccessibleEntries` for a configured one.
  `tsc` leaves its own outputs out of a default `include`
  (TypeScript handbook, tsconfig reference, `include`/`exclude`/`outDir`).
- **Alternatives considered**: (a) Re-implement the record check in
  `host.mjs`: a second reader of one format that would drift. (b) Have the
  engine precompute the owned files under the project root and send them
  with each request: a configuration may `include` directories outside the
  root, and the host lists directories lazily, so the set would be
  incomplete. (c) Hide `x.ts` whenever `x.tt` is served: a naming heuristic
  that misses `-o` trees and hides an edited output that is the user's
  again.
- **Decision and rationale**: The rule moves into the library as
  `ttc::ownership` (record path, record, `owned_output`); the CLI's writer
  keeps using it. The host, while it lists a directory, asks ttc which of
  the listed files are owned outputs (`{ ownedOutputs: [path] }` →
  `{ owned: [path] }`); the backend's single request loop
  (`read_answer`) answers from `owned_output` before it returns the
  request's own answer. The host is synchronous, so the question is
  answered in line. The inferred project's source list is filtered by the
  same function. An edited output is not owned, so it stays an input.

## Work log

- 2026-09-30: Reproduced with `target/probe5-cli/ip`: the TS2322 was
  reported at `src/m.ts:2:26` and `src/m.tt:1:26`; a script file added
  TS2451 twice.
- 2026-09-30: Moved `record_path`/`record`/`owned_output` from
  `src/main/ownership.rs` to `src/ownership.rs`; added the `ownedOutputs`
  question to `host.mjs` and `read_answer` to `native.rs` (the open
  acknowledgement goes through it too); filtered `Project::update`'s
  inferred sources.
- 2026-09-30: The repro reports one TS2322 at `src/m.tt:1:26` with and
  without `tsconfig.json`, and no TS2451.
- 2026-09-30: Added
  `check_types_leaves_published_outputs_out_of_the_program`
  (`tests/workflow_repairs.rs`): in-place and `-o build` outputs, inferred
  and configured project, and an edited output that is an input again.

## Issues and resolutions

None.

## Verification

- [x] `cargo test --test workflow_repairs` (with `TTC_REQUIRE_TSGO=1`)
- [x] Full gate run once at the end of the TASK-587–592 series; see TASK-592.

## Result

Changed `src/ownership.rs` (new), `src/lib.rs`, `src/main/ownership.rs`,
`src/main/build.rs`, `src/typescript/host.mjs`, `src/typescript/native.rs`,
`src/engine/project.rs`, `docs/ai/tt.md`, and `tests/workflow_repairs.rs`.
