# TASK-565: One contextual TypeScript session per project, shared by workers

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-565: Share one contextual TypeScript session per project across workers`

## Purpose

`ttc -j N` started a node host and its `tsgo` per worker thread (`-j 4`:
four hosts, eight processes), and each worker projected and materialized the
whole project on its own, so `-j 4` was no faster, and often slower, than
`-j 1`. The contextual refinement of a project should be planned by one
session that every worker asks.

## Scope

- Included: the standalone contextual pass
  (`typescript::contextual::standalone`): who owns its backend session and
  reuse state, and the reuse check
- Excluded: what a materialization computes; `-p`'s per-invocation cost (a
  one-shot process holds one project either way); the typed engine, whose
  `Project` already owns one backend

## Decisions

This task reverses TASK-436 Decision 2's choice of a thread-local cache and
backend; TASK-436 now says so at its top.

### Decision 1: The session belongs to the project, not the thread

- **Context**: The backend and the reuse state (projections, last answers)
  were `thread_local!`. A build's workers are threads, so each opened its
  own host and repeated the first materialization.
- **Alternatives considered**: Plan the whole build in the driver before
  dispatching workers (a second entry point beside `compile_report`, which
  the content mapper, `-p`, and the server also reach through the same
  pass); one process-wide slot replaced when the root changes (a build over
  two projects would restart a host at every switch between them).
- **Decision and rationale**: A process-wide registry maps a project root
  to a `ProjectSession` (backend plus reuse state) behind a mutex. Every
  caller for that project uses it, so the project is opened and planned once
  and every later file of it reuses the answers. The registry keeps the most
  recently used sessions, one per available core — the bound the per-thread
  model had — and an evicted session ends its host when its last caller
  releases it. A poisoned session (a caller panicked mid-exchange) is
  replaced, since the host's protocol may be half-read.

### Decision 2: Keep the per-call work outside the session, and O(1) inside it

- **Context**: Sharing the session alone serialized each call's walk, reads,
  and the reuse check, which cloned and compared every served module: `-j 4`
  then only matched `-j 1`.
- **Decision and rationale**: The walk and the reads happen before the
  session is locked. Under the lock, `Reuse::reconcile` compares the texts
  just read with the cached ones, re-projects only what changed (the
  requested file included, so asking for another file does not look like a
  change), and advances a version when anything did. A materialization is
  kept with the modules it was sent and which one was requested. It answers
  a later request when the configuration, root, support modules and version
  are equal, the request's own analysis is the module it was computed with,
  the module requested then still equals its file's projection, and the
  host reports the same disk generation. That is the same question TASK-436
  compared module by module, answered without cloning the project per call.

## Work log

- 2026-09-29: Generated 400 files under `target/probe4-cli/p3` (each a
  variant and a `match` with a join slot; nodenext `tsconfig.json`) and
  measured `ttc -j N -o out src`, sampling the build's child processes.
- 2026-09-29: Instrumented the pass (not committed): with `-j 1` one
  materialization (about 5 s, debug) was followed by about 50 ms per file
  of walk, reads, reconciliation, and module cloning and comparison.
- 2026-09-29: `src/typescript/contextual.rs` — `ProjectSession` and its
  registry, `Reuse::reconcile`, `Answered`, and the reuse check above.
- 2026-09-29: `src/lib/scaling_tests.rs` — the reuse test pinned the
  thread-local model (fresh threads meant fresh state). It now takes its
  reference outputs from projects no other call has seen, and adds parallel
  workers that must neither project nor ask again.
- 2026-09-29: Added
  `parallel_workers_share_one_typescript_session_per_project`
  (`tests/cli.rs`): a `node` wrapper on `PATH` counts the processes started;
  `-j 4` must start as many as `-j 1` and write the same output. It fails
  without the fix (`[("1", 1), ("4", 4)]`).

## Issues and resolutions

### Issue 1: Each worker thread ran its own TypeScript host

- **Symptom**: `-j 4` over 400 files: four host processes (plus their
  `tsgo`), 27–41 s against 36–43 s for `-j 1` (debug; the machine was shared
  with other builds, so the ranges are two rounds each).
- **Cause**: Thread-local backend and reuse state.
- **Resolution**: Decisions 1 and 2. After: one host for any `-j`; `-j 1`
  17–25 s and `-j 4` 13–19 s over the same two rounds. The output of `-j 4`
  is byte-identical to the previous `-j 1` output.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --lib --bins --test cli --test integration --test content_mapper --test engine_cache --test compile --test native`

## Result

Changed `src/typescript/contextual.rs`, `src/lib/scaling_tests.rs`,
`tests/cli.rs`, and the TASK-436 record. A build opens one TypeScript
session per project regardless of `-j`.
