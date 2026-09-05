# TASK-321: Reject ambiguous build output paths

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-321: reject ambiguous build outputs`

## Purpose

Prevent mixed-source builds from silently overwriting one input's output when
two distinct source paths map to the same emitted TypeScript path.

## Scope

- Included: CLI build planning, conflicting `.tt`/`.ts` and `.ttx`/`.tsx`
  stems, overlapping input roots, compiler support-module outputs, nested
  output directories, deterministic diagnostics, and regression coverage
- Excluded: New output naming conventions and changes to type-only modes

## Decisions

### Decision 1: Reject conflicts before reading or writing the project

- **Context**: Serializing contested writes makes the winner deterministic but
  still loses one source file.
- **Alternatives considered**: Keep last-writer-wins behavior; invent alternate
  output names; reject distinct sources that claim one output.
- **Decision and rationale**: Reject the build during planning and name the
  output and both inputs. Existing import rewriting assumes the current output
  names, so automatic renaming would produce a different and ambiguous module
  graph.

## Work log

- 2026-09-05: Reproduced the existing last-writer-wins implementation in
  `compile_jobs`; it explicitly serialized contested writes without reporting
  data loss.
- 2026-09-05: Replaced serialized contested writes with a planning-time check.
  Distinct sources that claim one output now produce a deterministic diagnostic
  naming the output and both inputs before any source is loaded or output is
  written.
- 2026-09-05: Found that generated support modules could independently collide
  with source outputs. Added the same pre-write rejection for those paths.
- 2026-09-05: Reproduced recursive `generated/generated` growth when `-o` was
  below a directory input and a prior output tree already existed. Excluded the
  output subtree from that input's collected files.
- 2026-09-05: Ran the complete local gate with registry and loopback access.
  The Rust, npm, create-tt E2E, website, native-backend, and extension stages
  passed.

## Issues and resolutions

### Issue 1: Distinct sources silently overwrite one emitted path

- **Symptom**: `x.tt` and `x.ts`, or two separate input roots containing the
  same relative output name, both write one `x.ts`; the later job wins.
- **Cause**: Build planning counted output claims only to serialize their
  writes, not to reject an ambiguous output graph.
- **Resolution**: Reject distinct source claims before loading the project and
  remove the former last-writer-wins path.

### Issue 2: A source output could replace a compiler support module

- **Symptom**: `tt/runtime.tt` emitted to the same `tt/runtime.ts` path as the
  pipeline runtime and silently replaced it.
- **Cause**: The support-module guard compared its target with input paths but
  not with each job's planned output path.
- **Resolution**: Compare every required support target with both source and
  planned output paths, then reject the build before creating the support tree.

### Issue 3: Nested output directories became inputs on the next build

- **Symptom**: With `-o src/generated src`, a previous
  `src/generated/stale.ts` was copied to
  `src/generated/generated/stale.ts`; each build could add another level.
- **Cause**: Directory collection skipped fixed generated-directory names but
  did not know the build's explicit output path.
- **Resolution**: Exclude the selected output subtree from each directory input
  while preserving explicitly named file inputs.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci` — agents, rust, npm, website, native, and extension passed

## Result

Build planning now rejects every confirmed ambiguous output before writing and
does not re-ingest an explicit nested output tree. Regressions cover both tt/TS
source-kind collision pairs, separate roots with one relative output, compiler
support modules, and repeated builds with an output below the input.

Changed files: `src/main/build.rs`, `tests/cli.rs`, `docs/ai/tt.md`, this task
record, and the task index.
