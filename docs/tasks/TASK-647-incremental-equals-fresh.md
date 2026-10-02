# TASK-647: Hold an edited project to the answers of a fresh one

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-647`

## Purpose

The engine keeps state between edits: projections per content version, the
semantic cache, the pattern-analysis cache, and a TypeScript language
service fed one change at a time. Tests pin single answers
(`tests/engine_cache.rs` pins hit counts), but nothing asserts that the
state an editor builds up by typing ends where a freshly opened project
starts. TypeScript asserts exactly that for its incremental parser. This
task adds the same assertion for the engine, as a deterministic property
test.

## Scope

- Included: `tests/incremental.rs`, the shared case parser
  `tests/common/cases.rs` (moved out of `tests/case_baselines.rs`, which now
  uses it), `ProjectedDocument::code` (the emission, readable from outside the
  crate), the nightly step in `.github/workflows/ci.yml`, and
  `CONTRIBUTING.md` ("Incremental answers equal fresh ones").
- Excluded: edits to more than one document of a sample, file creation and
  deletion on disk, and the `ttc --server` transport (TASK-639 already
  requires the server to give the engine's answers).

## Sources modelled

- microsoft/TypeScript `release-6.0` at
  `050880ce59e30b356b686bd3144efe24f875ebc8`,
  `src/testRunner/unittests/incrementalParser.ts`: `compareTrees` (lines
  47 to 77) parses the new text from scratch and incrementally from the old
  tree and the change range, and asserts structural equality and the same
  diagnostics (`assertStructuralEquals`, `assertSameDiagnostics`);
  `insertCode` and `deleteCode` (lines 96 to 121) type or delete one
  character at a time, comparing after each keystroke with the previous
  incremental tree as the next old tree.

## Decisions

### Decision 1: Compare observations of the whole engine, not a tree

- **Context**: TypeScript compares parse trees. The engine has no single
  tree; what can go stale is every cache between a keystroke and an answer.
- **Alternatives considered**: (a) Compare projections only (cheap, but
  misses the semantic cache and the language service, the two stateful
  layers most likely to drift). (b) Drive `ttc --server` (adds the
  transport, which TASK-639 already holds equal to the engine).
  (c) Drive `ttc::engine::Workspace` as the server does, and compare
  everything the editor reads.
- **Decision and rationale**: (c). The final observation is the typed check
  (`CheckRequest { emit_declarations: true, tt_only: false }`: every
  diagnostic, each emitted declaration module, the backend error), the
  emitted TypeScript of every projected file (`ProjectedDocument::code`, a
  new read-only accessor), blocked files, service diagnostics and semantic
  tokens of every tt unit, hover and definition at up to eight identifier
  starts and at the cursor, and completion at the cursor and after the first
  `.`. Completion items are compared as a sorted multiset and the probe id
  only as present or absent: TASK-639 Issue 1 showed that two TypeScript
  processes order the same items differently, and the probe id is a
  process counter. Nothing else is normalized.

### Decision 2: Edit scripts shaped like editing, with questions between keystrokes

- **Context**: The state under test is built by the requests an editor
  makes while typing, not by the edits alone.
- **Decision and rationale**: Each sample's seed picks one of three scripts
  on one tt unit: retype (delete up to twelve characters one backspace at a
  time, then type them back, TypeScript's `deleteCode` and `insertCode`),
  replace (delete a span, then type another span of the file character by
  character), or paste (insert a copy of one line, then delete a span).
  After each edit the seed picks nothing (40%), a typed check (20%),
  service diagnostics, hover at the cursor, completion at the cursor, or
  semantic tokens (10% each). The first two scripts leave broken
  intermediate text; the retype script's final text is the original, so it
  also shows whether state from broken text survives a return to valid
  text.

### Decision 3: A fixed sample per pull request, every sample nightly

- **Context**: Each sample starts two projects with their TypeScript
  processes.
- **Alternatives considered**: Every sample in `cargo test` (174 samples,
  288 seconds in a debug build on four workers); a random sample (not
  reproducible).
- **Decision and rationale**: The corpus is every tt unit of
  `tests/cases/{compiler,conformance}` and of `tests/fixtures/{emit,diagnostic}`,
  three seeds each (174 samples). `cargo test` runs eight, chosen by a
  fixed-seed Fisher-Yates draw (splitmix64, seed `0x7474696e6372656d`):
  12 to 13 seconds on four workers. `TT_INCREMENTAL=all` runs every sample
  and is a step of the scheduled `exhaustive` job; `TT_INCREMENTAL=<n>` and
  `TT_INCREMENTAL_SEED=<n>` choose another sample, and a failure prints
  `TT_INCREMENTAL_ONLY=<input>#<unit>#<seed>` to rerun it alone.

### Decision 4: One case parser

- **Context**: The new suite reads case files as the case runner does.
- **Decision and rationale**: `directive`, the unit split, and the default
  `tsconfig.json` moved to `tests/common/cases.rs`; `tests/case_baselines.rs`
  interprets the directives it returns. The case runner's baselines are
  unchanged.

## Work log

- 2026-09-30: Read `incrementalParser.ts` at `050880c`.
- 2026-09-30: Moved the case parser to `tests/common/cases.rs`; the case
  runner passes unchanged (`TTC_REQUIRE_TSGO=1 cargo test --test
  case_baselines`).
- 2026-09-30: Added `ProjectedDocument::code` and `tests/incremental.rs`;
  ran the sample and every sample.
- 2026-09-30: Negative check (below); added the nightly step and the
  `CONTRIBUTING.md` section.

## Issues and resolutions

None. Every one of the 174 samples answered after its edits as a fresh
project does, so no defect was found and there is no expected-failure
list.

## Regression test (fails before the fix)

Not applicable: this task adds a property test and fixes no bug. Its
ability to fail is shown under Verification by a deliberately broken
projection cache.

## Verification

- [x] `TTC_REQUIRE_TSGO=1 cargo test --test incremental`: 8 samples pass,
  12 to 13 seconds.
- [x] `TT_INCREMENTAL=all TTC_REQUIRE_TSGO=1 cargo test --test incremental`:
  all 174 samples pass, 288 seconds in a debug build.
- [x] Negative check: with the projection cache reusing an open document's
  projection regardless of its text (`Some(cached) if (open ||
  !cached.unparsed)` in `Project::update`), the sample fails 5 of 8 samples,
  each with the edit script, the rerun command, and the differing
  diagnostics; the change was reverted.
- [x] `cargo fmt --check`; `cargo clippy --test incremental --test
  case_baselines -- -D warnings`.
- [x] The full gate ran over the final tree of TASK-647 to TASK-651; its
  results are in TASK-651.

## Result

Changed files: `tests/incremental.rs`, `tests/common/cases.rs`,
`tests/common/mod.rs`, `tests/case_baselines.rs`,
`src/engine/projection.rs`, `.github/workflows/ci.yml`, `CONTRIBUTING.md`,
`docs/tasks/INDEX.md`, and this record.
