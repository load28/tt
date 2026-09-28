# TASK-511: Give test temporary directories one owner per language

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: see `git log --grep TASK-511`

## Purpose

Test runs leaked temporary directories until the disk filled: about 1,500
`/tmp/tt-*` and `create-tt-*` directories, 1,900 `ttc-host-*` directories,
and 2,700 editor case directories under `target/tt-tests` had accumulated on
the development machine. Every suite created directories its own way and
removed them its own way, or not at all.

## Scope

- Included: one helper per language that owns every test temporary
  directory (`scripts/test-dirs.cjs` for the node:test suites,
  `src/test_workspace.rs` for Rust unit and integration tests); routing
  every test creator through it; the `ttc-host-*` directories the typed
  backend created per session.
- Excluded: `editors/vscode/scripts/test-editor.mjs`, which is a manual VS
  Code harness that deliberately keeps its run directory under
  `target/editor-tests` and prints its path; release and publish scripts
  that already remove their scratch directory in `finally`
  (`promote-nightly-latest.mjs`, `build-ts-preview-vsix.mjs`,
  `publish-local-registry.mjs`); `benches/compile.rs`, whose workspace
  already removes itself on drop; archived evidence scripts under
  `docs/tasks/evidence/`.

## Decisions

### Decision 1: One helper per language, with a run-scoped parent

- **Context**: The creators found were:
  - editor server tests: `caseDir` in `server/src/test/workspace.ts` (no
    removal at all, 2,700 directories under `target/tt-tests`), and
    `mkdtempSync(os.tmpdir())` in `session`, `compiler`, `compilerfor` and
    `install` tests (no removal, or removal only after the last assertion);
    `sidecar` and `server` tests removed with `rmSync` at the end of the
    test body or in `finally`;
  - `packages/create-tt/test/installer.test.mjs`: 20 `mkdtemp` calls, none
    removed; `e2e/scaffold.test.mjs` removed in `finally`;
  - `integrations/unplugin/test`: `mkdtemp` plus `t.after(rm)` per case;
  - `npm/scripts/*.test.mjs`: `tt-release-*` and `tt-publish-*` never
    removed, `tt-platform-package-*` removed in `finally`;
  - `tools/deliberation-bot/test/state.test.mjs`: never removed;
  - Rust: `tests/common::Workspace` (removed on drop, but deliberately kept
    when the test panicked), and eight unit or integration tests that built
    `temp_dir().join(...)` by hand and removed it only after the last
    assertion passed (`content_mapper/tests.rs`, `engine/names.rs`,
    `engine/language/tests.rs`, `typescript/toolchain.rs`,
    `typescript/service.rs`, `main/output.rs`, `lib/scaling_tests.rs`,
    `tests/stdlib.rs`, `tests/cli.rs`, `tests/engine_cache.rs`).
  Removal after the last assertion is skipped whenever an assertion fails
  or the test times out, and a test with no removal leaks every run.
- **Alternatives considered**: fixing each call site's cleanup (the
  approach that produced the leak: every new test is one more place to
  forget); a periodic sweep of `/tmp` (hides the cause and can delete
  another run's live directory); one helper per package (the JS suites
  would carry five copies of the same lifecycle).
- **Decision and rationale**: `scripts/test-dirs.cjs` exports `testDir`
  (under the system temporary directory) and `repoTestDir` (under
  `target/tt-tests`, for a case that must inherit the repository's
  `node_modules`). Both create under one run directory per test process
  (`tt-test-run-<pid>-XXXXXX`) and a root-level node:test `after` hook
  registered when the helper loads removes the run directories. node:test
  runs root hooks after every test, whether it passed, failed, or timed
  out. It is CommonJS with a `.d.cts` so the compiled CommonJS editor tests
  and the ES-module suites import the same file. `src/test_workspace.rs`
  holds the Rust `Workspace`, included by the library and binary crates
  under `#[cfg(test)]` and by `tests/common` through `#[path]`, so unit and
  integration tests share one type. Each workspace lives under
  `tt-test-run-<pid>-<nonce>`; `Drop` removes the workspace and then the
  run directory once it is empty, under a mutex so a concurrent creation
  cannot lose its parent. Every ad-hoc `mkdtemp`, `temp_dir().join`, and
  removal at the end of a test was deleted.

### Decision 2: A failing Rust test no longer keeps its workspace

- **Context**: `Workspace::drop` kept the directory when the thread was
  panicking and printed its path. That is exactly the "cleanup only on
  success" behavior that lets a failing or flaky suite fill the disk.
- **Alternatives considered**: an opt-in environment variable to keep
  directories. None existed in the repository (no `TT_KEEP_TEST_DIRS` or
  equivalent), and inventing one was out of scope.
- **Decision and rationale**: removal happens on success and on failure.
  A person debugging a case can still stop the test before it ends.

### Decision 3: The typed backend's host directory is content addressed

- **Context**: `NativeBackend` wrote `host.mjs` to
  `ttc-host-<pid>-<session>` for every session and removed it in
  `Session::drop`. When the process is killed, which is how the editor
  retires an engine server (`retireEngineServer` calls `child.kill()`), or
  when the host dies before its first answer, no drop runs, so every
  killed `ttc --server` left one directory per session. This is a leak for
  users as well as for test runs.
- **Alternatives considered**: signal handling in `ttc` (needs a new
  dependency or `unsafe`, and SIGKILL still leaks); having the host remove
  its own directory when its input closes (a second owner next to
  `Session::drop`, and still leaks when both processes are killed).
- **Decision and rationale**: the host script is a constant of the build,
  so its directory is too: `ttc-host-<FNV-1a digest of host.mjs>`, shared
  by every session and process of the same build. `prepare_host` rewrites
  `host.mjs` only when the file differs, through a staging file and a
  rename, so a concurrent reader never sees a partial file. The identity
  content-mapper package the host writes on demand is named by a digest of
  `process.execPath` and published the same way (`publishFile`). The
  directory has no lifecycle to own, and one exists per distinct host
  script instead of one per session.

### Decision 4: Link the compiler instead of copying it

- **Context**: `session.test.ts` copied the debug `ttc` binary (about
  124 MB) into two temporary directories to get a second compiler path;
  those copies are what made each leaked `tt-two-compilers-*` directory
  124 MB.
- **Decision and rationale**: the tests need a distinct path, not a
  distinct file, and the engine keys sessions by path. A symlink keeps the
  behavior and costs no space. The extension suite runs on Linux only.

## Work log

- 2026-09-28: Created the task record. Surveyed `/tmp` (2,655 entries: 26
  `create-tt-*` prefixes, 1,907 `ttc-host-*`, 6 each of the `tt-session-*`,
  `tt-install-*`, `tt-two-compilers-*` families) and `target/tt-tests` in
  the main checkout (2,727 editor case directories). Measured sizes:
  `tt-two-compilers-*` 124 MB (a copy of `ttc`), `create-tt-*` 12 to 16 KB,
  `ttc-host-*` 48 KB.
- 2026-09-28: Added `src/test_workspace.rs`, moved `Workspace` there from
  `tests/common/mod.rs`, and routed the Rust creators through it.
- 2026-09-28: Replaced per-session host directories with
  `prepare_host` in `src/typescript/native.rs` and `publishFile` in
  `src/typescript/host.mjs`.
- 2026-09-28: Added `scripts/test-dirs.cjs` and `scripts/test-dirs.d.cts`,
  deleted `editors/vscode/server/src/test/workspace.ts`, and routed every
  node:test creator through the helper; unwrapped `try`/`finally` blocks
  whose only purpose was removal.
- 2026-09-28: Ran the gates and the /tmp counts below; ran deliberately
  failing and timing-out probes in both languages, then deleted them.

## Issues and resolutions

### Issue 1: The npm stage failed in the fresh worktree

- **Symptom**: `typescript-version.test.mjs` failed with `ENOENT` for
  `node_modules/typescript/package.json`.
- **Cause**: the worktree had no root `npm ci`.
- **Resolution**: ran `npm ci` at the worktree root, as AGENTS.md
  prescribes. Not related to this change.

## Verification

`/tmp` is shared with other sessions, so each run was measured as a
before/after listing diff as well as a count.

- [x] `cargo fmt --check`: exit 0
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: exit 0, 22 test binaries, no
  failures. First run: `/tmp` 2,667 to 2,656 entries; the only new entry was
  the shared `ttc-host-f67633f7adca5fe6`. Second run: 2,656 to 2,656, no
  new entries; `target/tt-tests` empty.
- [x] `./scripts/ci npm`: exit 0 (pass 69, 11, 1, 15; fail 0). `/tmp`
  2,656 to 2,656, no new entries.
- [x] `./scripts/ci extension`: exit 0 (pass 219, fail 0, skipped 0). No
  new entries from this run (the only additions were another session's
  `tt-bench-*` and valgrind pipes); `target/tt-tests` empty.
- [x] A deliberately failing node:test (assertion failure, plus a
  `repoTestDir`) and a test failing with `testTimeoutFailure` left no
  `tt-test-run-*` directory in `/tmp` or `target/tt-tests`. The probe files
  were deleted.
- [x] A deliberately panicking Rust test holding `Workspace::new` and
  `Workspace::in_repo` left no directory in `/tmp` or `target/tt-tests`.
  The probe was deleted.
- [x] `node scripts/check-task-index`

## Result

Test temporary directories now have one owner per language:
`scripts/test-dirs.cjs` (with `scripts/test-dirs.d.cts`) and
`src/test_workspace.rs`. Changed files: `src/lib.rs`, `src/main.rs`,
`src/test_workspace.rs`, `src/typescript/native.rs`,
`src/typescript/host.mjs`, `src/content_mapper/tests.rs`,
`src/engine/names.rs`, `src/engine/language/tests.rs`,
`src/typescript/toolchain.rs`, `src/typescript/service.rs`,
`src/main/output.rs`, `src/lib/scaling_tests.rs`, `tests/common/mod.rs`,
`tests/cli.rs`, `tests/stdlib.rs`, `tests/engine_cache.rs`, the editor
server tests (`compiler`, `compilerfor`, `completion`, `emitmap`, `engine`,
`install`, `server`, `session`, `sidecar`, `typedcheck`; `workspace.ts`
deleted), `integrations/unplugin/test/{plugin,windows}.test.mjs`,
`packages/create-tt/{test/installer,e2e/scaffold}.test.mjs`,
`npm/scripts/{platform-packages,release-version,tt-lang-publish}.test.mjs`,
`tools/deliberation-bot/test/state.test.mjs`,
`docs/design/tsgo-native-backend.md`, and the task index.

The directories already on the machine are not removed by this change;
they can be deleted once, by hand.
