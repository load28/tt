# TASK-388: Repair compiler and editor defects structurally

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

### Decision 2: Classify open-file changes by content identity

- **Context**: A watched change for an open file can come from the editor's save or an external writer.
- **Alternatives considered**: Treat every change as external, which rebuilds on ordinary saves; ignore every change to an open file, which loses external edits.
- **Decision and rationale**: Compare the held buffer with the file's current bytes decoded as text. Equal content is already in the engine; different or unreadable content requires project rearming.

### Decision 3: Use path containment and process outcomes directly

- **Context**: Separator concatenation rejects filesystem roots, and a signal-terminated compiler has no successful exit code.
- **Alternatives considered**: Special-case `/` and signal names in the caller.
- **Decision and rationale**: Use `path.relative` for all workspace roots and accept sidecar output only on the compiler's documented exit codes 0 and 1.

## Work log

- 2026-09-24: Ran `./scripts/doctor`, confirmed the development environment, and created a branch from the current `main`.
- 2026-09-24: Added regressions for external writes to open buffers, filesystem-root workspaces, and a signal-terminated sidecar compiler; repaired the owning watcher, root, and process-result logic.
- 2026-09-24: The full gate exposed a preexisting path-alias assertion in `tests/engine_cache.rs`; aligned its expected scan paths with the project's canonical-path contract.
- 2026-09-24: `./scripts/ci rust` passed after that correction. `./scripts/ci npm website` passed with network and local-port access. The full extension suite passed with 194 tests.

## Issues and resolutions

### Issue 1: External writes to open files were discarded

- **Symptom**: `CHANGED` was ignored for any open path even when the disk content differed from the held buffer.
- **Cause**: `isExternalChange` classified events using only open-path membership.
- **Resolution**: Compare disk content with the open buffer and rearm when it differs or cannot be read.

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
- [x] Extension suite: 194 tests passed

## Result

Changed `editors/vscode/server/src/{watch,server,roots,sidecar}.ts`, their corresponding regression tests, `tests/engine_cache.rs`, and the task index and record. Three editor failures now follow their owning file, path, and process contracts; the compiler test now asserts its documented canonical-path result.
