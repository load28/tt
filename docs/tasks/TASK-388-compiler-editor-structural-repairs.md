# TASK-388: Repair compiler and editor defects structurally

> Review correction (2026-09-25): The initial open-file watcher change was
> removed. Open documents are authoritative overlays; rearming a project
> reopens their held text and cannot ingest a different disk version.

- **Status**: Complete
- **Started**: 2026-09-24
- **Completed**: 2026-09-24
- **Commit**: —

## Purpose

Identify and repair multiple reproducible compiler or editor defects in their owning layers without input-specific workarounds.

## Scope

- Included: Reproduction, structural fixes, and regression coverage for confirmed defects.
- Excluded: Unrelated feature changes and release tooling.

## Decisions

### Decision 1: Require a minimal failing case for each repair

- **Context**: The request covers multiple possible defects across separate layers.
- **Alternatives considered**: Speculative edits based on code inspection alone could change established contracts without evidence.
- **Decision and rationale**: Confirm each symptom with an executable regression before changing its owning implementation.

### Decision 2: Preserve open-document authority during watched changes

- **Context**: A changed disk file can differ from an open editor buffer, but the engine receives the buffer again after project rearming.
- **Alternatives considered**: Compare disk text and rearm, which only repeats work against the same overlay; replace the held text from disk, which violates the editor's unsaved-buffer ownership.
- **Decision and rationale**: Keep the existing open-file change classification. The editor's document-change notification supplies new text when it actually reloads the file.

### Decision 3: Use path containment and process outcomes directly

- **Context**: Separator concatenation rejects filesystem roots, and a signal-terminated compiler has no successful exit code.
- **Alternatives considered**: Special-case `/` and signal names in the caller.
- **Decision and rationale**: Use `path.relative` for all workspace roots and accept sidecar output only on the compiler's documented exit codes 0 and 1.

## Work log

- 2026-09-24: Ran `./scripts/doctor`, confirmed the development environment, and created a branch from the current `main`.
- 2026-09-24: Added regressions for external writes to open buffers, filesystem-root workspaces, and a signal-terminated sidecar compiler; repaired the owning watcher, root, and process-result logic.
- 2026-09-24: The full gate exposed a preexisting path-alias assertion in `tests/engine_cache.rs`; aligned its expected scan paths with the project's canonical-path contract.
- 2026-09-24: `./scripts/ci rust` passed after that correction. `./scripts/ci npm website` passed with network and local-port access. The full extension suite passed with 194 tests.
- 2026-09-25: Reviewed PR #129 against the engine's open-document overlay lifecycle and removed the watcher change because it could only cause needless project rebuilds.
- 2026-09-25: Re-ran `./scripts/ci agents extension`; all 193 extension tests passed after the review correction.

## Issues and resolutions

### Issue 1: The initial watcher repair rebuilt against stale held text

- **Symptom**: A disk change differing from an open buffer triggered a full rebuild, but no new disk text reached the engine.
- **Cause**: Project rearming reopens every document using its held editor text. An open document remains the authoritative overlay until the editor sends a document change.
- **Resolution**: Removed the proposed watcher change and its regression test during PR review.

### Issue 2: A filesystem-root workspace contained no files

- **Symptom**: A file below `/` did not resolve to `/` as its workspace folder.
- **Cause**: The prefix check appended another separator to an already terminated root path.
- **Resolution**: Use the platform path library's relative-path containment rule, retaining the deepest matching root.

### Issue 3: A killed sidecar compiler reported success

- **Symptom**: A compiler terminated by SIGTERM produced no declarations but `refreshSidecar` returned `written`.
- **Cause**: Missing process exit codes were converted to exit code 1, a valid declaration-emitting result.
- **Resolution**: Preserve the actual process exit code; only 0 and 1 count as written.

### Issue 4: A canonical-path scan test failed on macOS temp aliases

- **Symptom**: The scan returned `/private/var/...` while the test expected `/var/...`.
- **Cause**: The project contract canonicalizes scan paths, but the assertion compared them against lexical temp paths.
- **Resolution**: Canonicalize the test's expected scan paths. The source collector's separate lexical-path assertion remains unchanged.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci rust` (including fuzz compilation)
- [x] `./scripts/ci npm website` with network and local-port access
- [x] `./scripts/ci agents extension` after review correction (193 tests)

## Result

Changed `editors/vscode/server/src/{roots,sidecar}.ts`, their corresponding regression tests, `tests/engine_cache.rs`, and the task index and record. Two editor failures now follow their owning path and process contracts; the compiler test now asserts its documented canonical-path result. The open-file watcher proposal was removed during review.
