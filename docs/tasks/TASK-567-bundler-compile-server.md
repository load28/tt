# TASK-567: Serve bundler compiles from one persistent ttc server

- **Status**: In progress
- **Started**: 2026-09-29
- **Completed**: —
- **Commit**: —

## Purpose

The bundler adapter spawned one `ttc -p` and one `ttc --dependencies` per
module. With TypeScript installed, each `-p` opens the whole TypeScript project
to refine generated storage annotations
(`docs/design/contextual-type-materialization.md`), and each `--dependencies`
opens and type-checks it, so bundling N modules cost O(N²). The adapter now
asks one persistent `ttc --server` session instead, with output byte-identical
to `-p`.

## Scope

- Included: `--server` methods `print` (exactly what `ttc -p` prints) and
  `dependencies` (what `ttc --dependencies` prints); the unplugin adapter's use
  of one lazily started session with restart-once crash handling and shutdown
  on bundler close; Rust and JS tests; README, `docs/ai/tt.md` and the server
  protocol header.
- Excluded: changing `-p` output, the contextual materialization itself,
  `--emit-std` (a constant module that opens no project), and other consumers
  of `-p`, which keep working unchanged.

## Decisions

### Decision 1: `print` runs the command line's own `-p` compile in the server process

- **Context**: The answer must be byte-identical to `ttc -p` for the same file
  and flags, and the TypeScript project must open once per session.
- **Alternatives considered**: (a) Refine through the workspace `Project`
  (the editor's engine path). That is a second implementation of the
  standalone refinement's inputs (candidate projections, inferred
  configuration, support packages, import rewriting of synthesized types), so
  byte identity would be a property to maintain rather than a consequence.
  (b) Re-run `-p` as a child process per request: no gain.
- **Decision and rationale**: Split `compile_jobs` (`src/main/build.rs`) into
  `compile_outcomes` (load, compile, collect per-job messages and output) and
  the existing print/write tails, and add `print_input`, which runs the same
  `build_jobs` → `support_root` → `compile_outcomes` sequence the `-p` command
  line runs and returns stdout/stderr instead of printing them. The standalone
  refinement (`src/typescript/contextual.rs`) already keeps its TypeScript
  backend, projections and answers per thread, validated by the backend's disk
  generation; the server answers every request on one thread, so the project
  opens on the first request and later requests reuse it. Measured: the first
  `print` in a 400-file project took 4-6 s, later ones 13-75 ms.

### Decision 2: `dependencies` checks the live project again only when a watch path changed

- **Context**: `--dependencies` opens and fully type-checks the project; per
  module that is as expensive as `-p` (5 s per module on 400 files). A warm
  `Project::check` still took about 4 s per request, because every check
  re-checks the whole program.
- **Alternatives considered**: (a) Check on every request: O(N²) again.
  (b) Derive dependencies from the `print` refinement: requires new plumbing
  through the compile API and changes what the set means. (c) Ask the adapter
  to request dependencies once per build: the adapter does not know project
  identities, and a changed file must still refresh the set.
- **Decision and rationale**: The server answers with the watch paths of the
  file's live workspace project after a check of `scan ∪ {file}` (the same
  candidate rule `typedCheck` uses), and keeps, per project root, the checked
  file list and the modification stamps of `Project::watch_paths`. While both
  are unchanged, the previous check stands and the answer is the current watch
  paths. This is the invalidation rule `--check-types --watch` re-checks by
  (`src/main/typed.rs`), including stamping before the check so an edit during
  it still invalidates. Opening, updating or closing a document and
  `reloadProjects` drop the record. The Rust test compares the answer to
  `ttc --dependencies` for several files of one project.

### Decision 3: One session per compiler and working directory, restart once, no fallback

- **Context**: The adapter needs a lifecycle for the child process.
- **Alternatives considered**: Falling back to `-p` after a crash would hide
  the crash and silently change performance characteristics; restarting
  without bound could loop on a compiler that always dies.
- **Decision and rationale**: `integrations/unplugin/compiler-server.js`
  starts the session lazily, keyed by `process.cwd()` (the directory a `-p`
  would have run in, which supplies the TypeScript client), matches answers
  to requests by id, retries a request once on a fresh session when the
  process ends, and otherwise reports the exit status with the tail of
  stderr. Idle sessions are unreferenced so they never keep the bundler alive;
  the server exits at the end of its stdin. `closeBundle` ends the session
  outside watch mode (and for a Vite dev server, whose `closeBundle` is its
  close), `closeWatcher` ends it after watching, webpack/Rspack end it on
  `shutdown`, esbuild on `onDispose`.

## Work log

- 2026-09-29: Reset the worktree to `claude/ecstatic-dijkstra-qw5pf9`, ran
  `npm ci` and `npm ci --prefix integrations/unplugin`, built debug and
  release `ttc`.
- 2026-09-29: Measured the baseline on a generated 400-module project
  (`tsconfig.json`, each module importing the previous one and holding match
  storage that TypeScript refines): `ttc -p` 4.3-5.1 s and
  `ttc --dependencies` 5.1 s per module, release build.
- 2026-09-29: Split `compile_jobs` into `compile_outcomes` and
  `write_outcomes`, added `print_input` (`src/main/build.rs`), and the server
  methods `print` and `dependencies` (`src/server.rs`).
- 2026-09-29: A first `dependencies` checked the project on every request:
  4 s per warm request. Added the watch-path stamp record (Decision 2): 11-50 ms.
- 2026-09-29: Added `integrations/unplugin/compiler-server.js`, moved the
  adapter's `load` and dependency scan to it, added shutdown hooks, updated
  the fake compilers in the adapter tests to speak the server protocol.
- 2026-09-29: Added `tests/cli/server_print.rs` and
  `integrations/unplugin/test/server.test.mjs`; updated the server protocol
  header, the unplugin README and `docs/ai/tt.md`.

### Measurements

Release `ttc`, sequential `load` calls through the adapter, same machine
(shared with other jobs, so single runs vary by roughly ±30%):

| Project | Before (process per module) | After (one session) |
|---------|-----------------------------|---------------------|
| 400 modules, TypeScript installed, 20 loaded | 208.8 s (10.4 s/module) | 10.1 s (505 ms/module, first load included) |
| 400 modules, TypeScript installed, all 400 loaded | ~70 min extrapolated | 39.3 s (98 ms/module) |
| 200 modules, TypeScript installed | 20 loaded: 124.2 s (6.2 s/module) | all 200: 15.0 s (75 ms/module) |
| 400 modules, no TypeScript, all 400 loaded | 158.9 s (397 ms/module) | 8.7 s (22 ms/module) |

Server requests alone on the 400-module project: the first `print` 4-6 s and
the first `dependencies` 5.5-9.7 s (opening and checking the project), then
typically 13-75 ms and 11-50 ms. What remains per request is linear in the
project size and cheap: the refinement re-reads the project's tt sources to
compare them with its cached projections, and `dependencies` stats the watch
paths.

## Issues and resolutions

### Issue 1: A warm project check still cost seconds per module

- **Symptom**: With `dependencies` answered by a fresh check each time, the
  first request took 6.8 s and every later one 3.4-4.6 s on 400 files.
- **Cause**: `Project::check` sends the snapshot and asks TypeScript for the
  whole program's diagnostics; nothing in it is reused between checks of an
  unchanged project.
- **Resolution**: Decision 2 — the previous check stands while no watch path
  changed.

### Issue 2: The Windows path test fakes `process.cwd()`

- **Symptom**: Spawning the session with `cwd: process.cwd()` would fail in
  `test/windows.test.mjs`, which returns `C:\proj` from `process.cwd()`.
- **Cause**: `-p` through `execFile` inherited the real working directory; it
  never passed `process.cwd()` explicitly.
- **Resolution**: The session is spawned with the inherited working directory,
  as `-p` was, and `process.cwd()` only keys the session.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `TTC_REQUIRE_TSGO=1 cargo test`
- [ ] `TTC_BINARY=<worktree>/target/debug/ttc npm --prefix integrations/unplugin test`
- [ ] `TTC_BINARY=<worktree>/target/debug/ttc npm --prefix packages/create-tt run test:e2e`

## Result

Pending.
