# TASK-653: Reuse a project's contextual materialization while its inputs are unchanged

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-653`

## Purpose

TASK-652 (Issue 2) found that every engine request re-runs the contextual
materialization: `Project::serve` calls `Project::update`, which calls
`typescript::contextual::materialize` whenever a projected file has a
contextual value slot. A hover on an unchanged project therefore paid the
whole checker round trip again, which made the `.types` baselines of
`shortCircuitTtOperands.tt` take about a minute and slowed editor hovers
the same way.

## Scope

- Included: a materialization cache on `Project`
  (`src/engine/project.rs`), keyed by everything the materialization is
  asked plus the host's session and disk generation; a work-count
  regression test (`src/lib/scaling_tests.rs`); the design notes
  (`docs/design/engine-architecture.md`,
  `docs/design/contextual-type-materialization.md`).
- Excluded: the materialization algorithm and the host protocol (both
  unchanged; the disk-generation probe already exists since TASK-436), the
  standalone path's own reuse (`contextual::standalone`, TASK-436/565), and
  keeping more than one materialization per project.

## Sources modelled

- microsoft/TypeScript `release-6.0` at `050880c`,
  `src/services/services.ts`: `synchronizeHostDataWorker` returns at once
  when `host.getProjectVersion()` equals the last one it built a program
  for, and otherwise reuses the program when `isProgramUptoDate` finds the
  same root names, settings, and script versions. Every language service
  request (`getQuickInfoAtPosition` among them) starts with
  `synchronizeHostData()`, so a hover on an unchanged project reuses the
  program instead of rebuilding it.
- microsoft/typescript-go `main` at `89d5d5b`,
  `internal/project/session.go`, `Session.getSnapshot`: when no file change,
  ATA change, or configuration change is pending and the requested
  document's project is not dirty, the current immutable snapshot and its
  programs answer the request; `internal/project/project.go` rebuilds a
  program only for a dirty project (`dirty`, `dirtyFilePath`,
  `CreateProgram`).

## Decisions

### Decision 1: Key the reuse by the whole question and the host's disk generation

- **Context**: The materialization's answer depends on the lowered
  modules, the modules served beside them (host overlays of hand-written
  TypeScript, the std support package, blocked files), the listed
  hand-written sources of a configuration-less project, the roots by
  request, the project's configuration and root, and the files the
  compiler reads from disk that nobody serves (`tsconfig.json`, `.ts`
  dependencies, `node_modules` declarations, failed lookups).
- **Alternatives considered**:
  - Key by source texts and overlay texts only: misses a disk-only
    dependency edit, the defect TASK-436's Issue 1 recorded for the
    standalone path.
  - Fingerprint dependency files from Rust: duplicates what the host
    already tracks, and misses what the compiler reads between rounds.
  - Clear the cache from every mutation entry point (`open_document`,
    `update_document`, `close_document`) as an eager dirty flag: disk
    changes still need the probe, and a mutation that leaves the text
    equal would discard a valid answer.
- **Decision and rationale**: The key holds the question itself
  (`ContextualQuestion`: modules as `(module path, lowered emit)`, support
  modules, sources, roots). Configuration and root are the project's fixed
  identity. Disk is covered the way TASK-436 Decision 3 covers it for the
  standalone path: an answer is kept only when every round of it saw the
  same host session and disk generation (`observe_generations`,
  `stable_generation`), and reused only when a `diskGeneration` probe
  reports that session and generation again (`current_generation`). This
  is TypeScript's project-version check (`getProjectVersion` then
  `isProgramUptoDate`) at tt's granularity: the question is tt's project
  version, the generation is its disk half.

### Decision 2: Ask the question in path order

- **Context**: The projected files come in discovery order, which depends
  on which files a request starts from (`serve` starts from the project's
  candidates and the requested file's imports; a check starts from the
  scan; `sees` from the candidates), and host overlays come from a hash
  map. The same project state could therefore be asked in different
  orders and miss the cache.
- **Alternatives considered**: Keep discovery order and compare an
  order-insensitive key; keep several entries.
- **Decision and rationale**: Modules, support modules, and roots are
  sorted by path before the materialization is asked, as the standalone
  path already sends them (TASK-436 Decision 3: the host keys served files
  by path and the program's roots come from the configuration, so order
  carries no meaning), and the answers are written back to the projected
  documents by index. One order means one question per state, and the
  literal comparison is the key.

### Decision 3: Keep one materialization per project

- **Context**: A project could keep answers for several questions.
- **Alternatives considered**: A map of questions to answers, bounded by
  count or memory.
- **Decision and rationale**: A TypeScript language service keeps one
  program per project and replaces it when the project version changes;
  typescript-go keeps one current snapshot. Every editor request of one
  project asks the same question after Decision 2, so the last answer is
  the one the next request needs, and an edit makes the old one useless.
  One entry bounds the memory to one copy of the project's emits.

## Work log

- 2026-09-30: Opened the task from TASK-652's Issue 2. Read
  `Project::update`, `contextual::materialize`, the standalone reuse
  (`contextual::standalone`, `Reuse`, `Answered`, TASK-436 and TASK-565),
  and the host's disk-generation probe (`host.mjs` `detectDisk`,
  `native.rs` `current_generation`).
- 2026-09-30: Timed `tests/case_baselines.rs` on the base commit
  (`cc53a54`), then added
  `project_requests_materialize_once_per_state_of_their_inputs` to
  `src/lib/scaling_tests.rs` and ran it against the unfixed engine: it
  failed (below).
- 2026-09-30: Added `ContextualQuestion`, `Materialized`,
  `Project::materialized`, and `Project::contextual_emits` in
  `src/engine/project.rs`; `Project::update` asks through them in path
  order. The test passes, and so do
  `project_files_share_one_projection_each_and_one_checker_materialization`
  and `the_editor_and_the_typed_pass_share_one_semantic_cache`.
- 2026-09-30: Counted requests against materializations with a temporary,
  uncommitted `eprintln!` in the two places, and timed the case runner
  again without it.
- 2026-09-30: Updated the design notes; ran the full gate.

## Issues and resolutions

### Issue 1: Every engine request materialized again

- **Symptom**: `shortCircuitTtOperands.tt` alone took 54 s in the case
  runner; its `.types` baseline asks 163 snapshots, and each ran the whole
  contextual materialization (two checker asks per hover in the test
  project).
- **Cause**: `Project::update` called `contextual::materialize`
  unconditionally when a projected file had a contextual slot. The
  projection cache kept the lowered emits across snapshots, but the
  refined emits were recomputed each time; nothing held the answer.
- **Resolution**: Decisions 1 to 3.

### Issue 2: The extension capabilities baseline failed in the first gate run

- **Symptom**: `tests/public_api.rs`,
  `the_extension_capabilities_match_their_baseline`: "TT_REQUIRE_EXTENSION
  is set but the extension's language server is not built", and
  `scripts/check-baselines` then reported
  `api/lsp-capabilities.json` unused.
- **Cause**: The fresh worktree had `editors/vscode` dependencies
  installed but not compiled when `cargo test` ran; the extension compile
  ran afterwards.
- **Resolution**: Environment only. After `npm run compile`, the test
  passed into the same tracking directory and `check-baselines` reported
  250 compared, none unused.

## Regression test (fails before the fix)

- **Path**: `src/lib/scaling_tests.rs`,
  `project_requests_materialize_once_per_state_of_their_inputs`. It opens
  an engine project with a tsconfig, a hand-written `dep.ts`, and two tt
  files with contextual slots, and counts `contextual checker asks`
  (`crate::work`): the first hover asks; five more hovers and a snapshot
  ask nothing; an edited document, a disk edit of `dep.ts`, and a host
  overlay of `dep.ts` each ask again; after each, the snapshot's emits
  equal a freshly opened engine's.
- **Observed failure**: With `src/engine/project.rs` reverted to
  `cc53a54`: `assertion 'left == right' failed: unchanged hovers
  materialize again`, `left: 10`, `right: 0`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 TTC_REQUIRE_TYPESCRIPT_CASES=1 TT_REQUIRE_EXTENSION=1 TT_BASELINE_TRACKING_DIR=<dir> cargo test --no-fail-fast`
      (includes `tests/incremental.rs`, the TASK-647 edited-equals-fresh
      contract, and `tests/engine_cache.rs`)
- [x] `node scripts/check-baselines --tracking <dir>`
- [x] Extension tests (`editors/vscode`: `npm run compile`, then
      `node --test "server/out/test/*.test.js" "client/out/test/*.test.js"`)
- [x] `./scripts/ci agents`
- [x] Baseline changes reviewed and committed with the change: none; every
      `.types`, `.ts`, `.errors.txt`, and `.map.txt` baseline is unchanged.

Measurements (debug build, same machine, `tests/case_baselines.rs`, one
case at a time with `TT_CASES`, wall time of the test binary):

| Case | Engine requests with slots | Materializations before | Materializations after | Before | After |
| --- | --- | --- | --- | --- | --- |
| `shortCircuitTtOperands` | 163 | 163 | 1 | 54.2 s | 4.1 s |
| `conditionalOperationNarrowing` | 155 | 155 | 1 | 23.5 s | 3.4 s |
| `literalLeftOperandNarrowing` | 108 | 108 | 1 | 28.8 s | 4.0 s |
| `letElseIfLetDivergence` | 0 | 0 | 0 | 1.8 s | 2.5 s |
| Whole suite | | | | 185.2 s | 49.7 s |

Before the fix every request with a slot materialized, so the before
column equals the request count; the after column was counted.
`letElseIfLetDivergence` has no contextual slot and is noise.

## Result

Changed files: `src/engine/project.rs`, `src/lib/scaling_tests.rs`,
`docs/design/engine-architecture.md`,
`docs/design/contextual-type-materialization.md`, `docs/tasks/INDEX.md`,
and this record. A project now materializes once per state of its inputs:
editor requests on an unchanged project reuse the last answer, and a
document edit, a host overlay, or a disk change the host observes asks
again. Output and editor answers are unchanged.
