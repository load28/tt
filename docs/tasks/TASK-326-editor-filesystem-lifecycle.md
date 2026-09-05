# TASK-326: Keep editor projects current across filesystem changes

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-326: fix(editor): reload projects on filesystem changes`

## Purpose

Audit real developer workflows beyond open-buffer edits: changes from generators,
new or deleted modules, and project configuration updates.

## Scope

- Included: Real extension-host filesystem lifecycle tests and fixes at client, server, and project boundaries.
- Excluded: TASK-324's contextual continuation work and claims of exhaustive enterprise readiness.

## Decisions

### Decision 1: Observe real filesystem events

- **Context**: Buffer synchronization alone does not cover changes from outside the editor.
- **Alternatives considered**: Simulated notifications only; real file operations with actual VS Code watchers.
- **Decision and rationale**: Reproduce through the extension host first, then add protocol regressions for headless CI.

### Decision 2: Reload graphs at an explicit filesystem barrier

- **Context**: File existence, module resolution, and compiler options can change independently of open-buffer versions. The persistent backend had no external graph invalidation contract.
- **Alternatives considered**: Adapter-side dependency guessing; restarting the entire compiler for every source event; an ordered graph-reload request with buffer replay.
- **Decision and rationale**: Add `reloadProjects` to the compiler server protocol. It releases project graphs; the adapter replays every open tt and host buffer on the same ordered pipe. Invalidate diagnostic generations before asynchronous configuration lookup. Ordinary keystrokes retain incremental sessions. External events use conservative whole-graph reloads until an engine-owned affected-graph API exists.

### Decision 3: Distinguish executable replacement from source changes

- **Context**: A process already running at a compiler path continues using the old executable after that path is rebuilt.
- **Alternatives considered**: Rebuild only project caches; explicitly replace the compiler process for binary events.
- **Decision and rationale**: Stop the engine server for compiler-binary events and settle its pending requests immediately. Normal sources/configuration changes only reload project state.

## Work log

- 2026-09-05: Doctor passed; inspected filesystem watching and persistent TypeScript project caches.
- 2026-09-05: Added a real extension-host filesystem suite. Before repairs, all 11 checks failed (`target/editor-tests/run-CsHRq8/results.json`). Closed dependencies were never opened as buffers, so the cases exercise actual VS Code filesystem events.
- 2026-09-05: Added source/configuration watchers, excluded support-package/Git metadata events, and wired ordered project invalidation and open-buffer replay. All 11 cases passed in `target/editor-tests/run-PjbJww/results.json`; the existing 64-check editing matrix also passed in `target/editor-tests/run-4Ap2q7/results.json`.
- 2026-09-05: Added eight headless lifecycle regressions covering create/change/delete/recreate and compiler-option toggles while retaining unsaved consumer text. Added a pending-request shutdown regression and immediate validation-generation invalidation.
- 2026-09-05: Rewrote the extension README in English against the current implementation. Removed incorrect zero-configuration/toolchain-equivalence promises and outdated whole-file diagnostic-suppression descriptions; documented actual setup, diagnostic codes, missing-tool behavior, filesystem lifecycle, and unresolved TASK-324 failures.

## Issues and resolutions

### Issue 1: Filesystem changes leave stale project answers

- **Symptom**: External dependency repairs do not clear errors; module creation/deletion and tsconfig edits do not reliably update consumers.
- **Cause**: The client watched compiler artifacts but not source/config files. Rechecking alone also left persistent backend graphs and configuration caches alive.
- **Resolution**: Watch source/configuration files, invalidate previous generations, reload graphs through the compiler protocol, replay open buffers, and revalidate consumers. Support-package writes do not trigger reload loops.

### Issue 2: A rebuilt compiler keeps its old process and pending requests

- **Symptom**: A binary rebuilt at the same path can leave editor requests on the previously launched compiler. Explicit shutdown leaves pending requests waiting for their full timeout.
- **Cause**: Re-arming the session did not terminate a live process; shutdown marked it dead before its failure callback could drain pending requests.
- **Resolution**: Restart for binary events, resolve pending requests during shutdown, and replay buffers when the new session starts.

### Issue 3: Setup and diagnostic documentation overstates guarantees

- **Symptom**: The README claims no editor configuration is necessary and editor/CLI compilers cannot differ, while also documenting an explicit compiler-path override and preview settings.
- **Cause**: Earlier implementation descriptions and absolute promises had accumulated beside newer configuration instructions.
- **Resolution**: Replace the outdated README with a single English description of the current setup, ownership boundaries, test commands, and known limitations. No claim of complete compiler/editor readiness is made.

## Verification

- [x] Final real VS Code filesystem suite: 11/11 checks (`target/editor-tests/run-8ByWXj/results.json`).
- [x] Final real VS Code combined-extension editing matrix: 64/64 checks over 16 directed edges (`target/editor-tests/run-Ey4QA9/results.json`).
- [x] Final headless extension suite: 121/121 tests, no skips.
- [x] Documentation links and pinned TypeScript installation contracts: 10/10 tests.
- [x] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`.
- [x] Final `./scripts/ci`: agents, rust, npm, website, native, extension passed. Run outside the sandbox to permit package downloads and the website prerender server.
- [x] `git diff --check` and syntax checks of the editor test runner and filesystem suite.

## Result

Added filesystem/configuration graph reloads with open-buffer replay, explicit
compiler replacement and pending-request cleanup, nine headless regressions,
and an 11-check real filesystem suite. Rewrote the extension README against the
current setup and error-reporting contracts. The changed implementation spans
the VS Code client, adapter/session, and compiler server protocol. User-installed
extensions and settings were not changed. TASK-324 remains pending; this task
does not claim complete language, multi-root, or large-project readiness.
