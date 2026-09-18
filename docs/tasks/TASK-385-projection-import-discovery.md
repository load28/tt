# TASK-385: Reuse projection imports during graph discovery

- **Status**: Complete
- **Started**: 2026-09-18
- **Completed**: 2026-09-18
- **Commit**: PR #126 performance repair commit

## Purpose

Repair PR #126's first-snapshot performance regression without weakening the performance gate or external-import discovery.

## Scope

- Included: Project graph discovery, projection metadata reuse, regressions, benchmarks, PR update.
- Excluded: Benchmark thresholds and unrelated compiler changes.

## Decisions

### Decision 1: Discover graph edges from projected document metadata

- **Context**: TASK-384 introduced an eager source read and full parse during project opening and scans, before projection reads and parses the same files.
- **Alternatives considered**: Raising the budget, bypassing import discovery, or adding a text heuristic. These do not fix duplicated work.
- **Decision and rationale**: Reuse imports already collected by projection, and expand the snapshot work queue from those edges. Keep unchanged documents and their imports cached together.

### Decision 2: Reuse directory-entry metadata during fresh scans

- **Context**: TASK-384 correctly rescans host files on updates, but the walker called `stat` for every entry just to distinguish directories.
- **Decision and rationale**: Use `DirEntry::file_type` and inspect the target only for symbolic links. Keep fresh membership discovery and existing link traversal behavior.

## Work log

- 2026-09-18: Doctor passed. CI run 35350840724 failed only performance: first snapshot 58.37 ms to 65.22 ms (+11.7%, fastest +11.8%, budget 10%). Other required jobs passed.

- 2026-09-18: Added an engine regression for out-of-root transitive imports, re-exports, cycles, unchanged Arc reuse, overlay import removal, and an unsaved external dependency.
- 2026-09-18: Initial local comparison passed: single file +2.6%, first snapshot +2.7%, one-file recheck +4.4%. Final exact-source comparison follows the complete Rust gate.

- 2026-09-18: The final comparison exposed a second cost: first snapshot passed (+3.7%), but one-file recheck failed (+12.6%, fastest +10.7%). Host-source discovery performed a metadata syscall for every directory entry on each update. Reused directory-entry file types, retaining target metadata lookups for symbolic links.

## Issues and resolutions

### Issue 1: Duplicate discovery parse on every initial project

- **Symptom**: First-snapshot benchmark exceeded the unchanged 10% noise floor.
- **Cause**: Graph discovery performed a separate full module parse before projection, which already collected the same imports.
- **Resolution**: Removed eager import discovery from project opening and filesystem scans. Snapshot construction now expands a deduplicated work queue from each projected document's already-collected imports. The same metadata is retained across unchanged snapshots; blocked files retain their own cached import facts.

### Issue 2: Repeated metadata lookups during one-file rechecks

- **Symptom**: One local comparison exceeded the recheck budget after duplicate parsing was removed.
- **Cause**: The host-source rescan requested directory/file metadata that directory enumeration already supplies.
- **Resolution**: Reuse directory-entry types. A regression covers nested directories and linked directories.

## Verification

- `./scripts/ci rust`: passed 1,246 tests, formatting, clippy, and fuzz-target compilation.
- Default `./scripts/bench-compare`: passed after the walker change. Recheck variance warranted a higher-sample comparison.
- `TT_BENCH_ITERS=100 ./scripts/bench-compare`: passed against `069a5fb`; single file +4.8% (fastest +4.7%), first snapshot +1.7% (fastest +0.3%), one-file recheck +2.2% (fastest +6.2%). Budgets were unchanged (10%, 10%, measured 11.4%).
- Raw benchmark summaries: [head](./evidence/TASK-385/bench-head-100.json), [base](./evidence/TASK-385/bench-base-100.json).
- Regression coverage: transitive out-of-root imports, re-exports, cycles, unchanged document identity, overlay edge removal, unsaved external dependencies, directory and symlink traversal.

## Changed files

- `src/engine/mod.rs`
- `src/engine/project.rs`
- `src/engine/projection.rs`
- `tests/engine_cache.rs`
- `docs/tasks/INDEX.md`
- `docs/tasks/TASK-384-developer-workflow-repairs.md`
- This record and `docs/tasks/evidence/TASK-385/`.

## Result

Local Rust and performance gates pass with unchanged benchmark budgets. Import discovery now reuses projection-owned metadata, and fresh filesystem scans avoid redundant per-file metadata calls. The fix updates PR #126; remote verification follows the push.
