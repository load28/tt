# TASK-325: Verify real editor editing workflows

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-325: fix(editor): synchronize mixed-source editing state`

## Purpose

Verify developer-facing diagnostics, completion, and navigation through a real
VS Code extension host, including edits to dependencies in mixed-source projects.

## Scope

- Included: An isolated extension-host test harness and repairs for reproduced editor failures.
- Excluded: Reinstalling the user's extension, changing the user's settings, and unrelated compiler features.

## Decisions

### Decision 1: Exercise the actual editor API

- **Context**: Existing language-server tests do not prove client activation and editor provider integration.
- **Alternatives considered**: More direct LSP tests; manual-only UI checks.
- **Decision and rationale**: Run the development extension in an isolated VS Code profile and use document edits, diagnostic events, and provider commands. Keep fixtures and logs separate from user projects.

### Decision 2: Preserve one source of truth for host buffers

- **Context**: Host buffers were not synchronized by the tt client, and the engine's buffer protocol rejected TypeScript-only document identities.
- **Alternatives considered**: Saving files before checks; registering tt providers for TypeScript; forwarding buffers while retaining TypeScript's own providers.
- **Decision and rationale**: Forward host open/change/close notifications without changing provider ownership. Resolve editor projects from any document. Freeze host overlays into snapshots and synchronize them at original paths in the live language service.

### Decision 3: Revalidate open consumers conservatively

- **Context**: The editor adapter has no engine-provided affected-file list.
- **Alternatives considered**: Guess dependencies in the adapter; refresh only the changed file; refresh all open tt documents.
- **Decision and rationale**: Revalidate all open tt documents with new generations after buffer changes and closes. Dependency semantics remain in the engine; no adapter-level import heuristic is introduced. Large-project latency remains unmeasured.

## Work log

- 2026-09-05: Ran `./scripts/doctor` successfully; inspected the editor validation lifecycle and existing test coverage.
- 2026-09-05: Added an isolated local VS Code runner. Initial expectations were corrected to assert structured diagnostic codes/source spans and TypeScript's legitimate import-alias rename semantics, rather than English message wording or a forced export rename.
- 2026-09-05: Reproduced stale consumer diagnostics for tt dependencies, then host-buffer failures. Added generation invalidation, host synchronization, document-based project routing, and snapshot/live-service host overlays.
- 2026-09-05: A native regression exposed that releasing a disk-backed overlay was reported as deletion. Changed the backend delta to report a text change when the underlying file still exists.
- 2026-09-05: Combined-extension tests exposed contradictory member completions because the tt session rejected host-buffer identities while the native extension saw their new types. Routing host documents into the engine project resolved the disagreement.
- 2026-09-05: Assigned distinct fixture basenames to avoid TypeScript's `.ts`/`.tsx` same-stem root selection contaminating the directed-edge matrix.
- 2026-09-05: The final combined-extension run passed all 64 checks across 16 edges (`target/editor-tests/run-Ph8eAR/results.json`). No user extension was reinstalled.
- 2026-09-05: Added eight direct LSP regressions, including host-first opening and close-to-disk restoration, plus a native test for immutable snapshots and live completion overlay restoration.

## Issues and resolutions

### Issue 1: Consumer diagnostics remain stale after dependency edits

- **Symptom**: An untouched consumer retains a clean diagnostic list after an unsaved dependency changes from string to number.
- **Cause**: Validation was scheduled only for the edited document.
- **Resolution**: Invalidate and reschedule open tt documents on buffer changes and closes.

### Issue 2: Unsaved host dependencies do not reach semantic consumers

- **Symptom**: Host changes are absent from tt diagnostics/completion; with the native extension active, stale and current member suggestions appear together.
- **Cause**: The tt client did not sync host buffers; server project lookup filtered them out; snapshots and the language service did not serve their overlays.
- **Resolution**: Sync host buffers without claiming their providers, resolve editor project identity independently of tt input filtering, and preserve host overlays in both semantic paths.

### Issue 3: Closing a host overlay makes a real module disappear

- **Symptom**: A native regression reports TS2307 after closing an unsaved host buffer.
- **Cause**: The backend marked every removed virtual overlay as a file deletion even when a disk file remained.
- **Resolution**: Report a change when the disk file exists; only truly absent modules are deleted. Close the live service overlay explicitly.

## Verification

- [x] Real VS Code with the development tt extension and TypeScript 7.1 extension: 64/64 checks, 16 directed edges (`target/editor-tests/run-Ph8eAR/results.json`).
- [x] Real VS Code with only the development tt extension: 32/32 checks, 8 directed edges (`target/editor-tests/run-v501Qs/results.json`).
- [x] Headless extension suite: 112/112 tests, no skips, including eight new dependency lifecycle regressions.
- [x] Native host-overlay regression: frozen snapshot contents, changed completion types, close-to-disk restoration.
- [x] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`.
- [x] All `./scripts/ci` stages: agents, rust, native, and extension passed initially; npm and website passed with `./scripts/ci npm website` outside the sandbox after download and local-listener restrictions caused the initial failures.
- [x] `git diff --check`.

## Result

Repaired three editor-state failures across the client, server adapter, engine
project/snapshot, and TypeScript backend. Added host-buffer synchronization,
consumer diagnostic invalidation, correct host overlay release, a reusable real
editor harness, eight LSP regressions, and a native snapshot/service regression.
Updated the extension README and task index. The installed user extension and
settings were not changed. Multi-root behavior and large-project latency are
outside the validated matrix.
