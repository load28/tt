# TASK-376: Isolate externally terminated mapper profiles

- **Status**: Complete
- **Started**: 2026-09-12
- **Completed**: 2026-09-12
- **Commit**: —

## Purpose

Repair the coverage merge failure reported on PR #124 without discarding malformed aggregate profiles or lowering the coverage baseline.

## Scope

- Included: Coverage ownership of the TypeScript-controlled mapper process tree and regression verification.
- Excluded: Product behavior, TypeScript dependencies, and coverage thresholds.

## Decisions

### Decision 1: The external process owner determines profile lifetime

- **Context**: TASK-362 isolated directly terminated children but omitted mappers spawned and terminated by TypeScript.
- **Alternatives considered**: Retrying CI cannot repair ownership; ignoring corrupt profiles would hide errors; disabling all subprocess coverage would exclude normally exiting children.
- **Decision and rationale**: Apply the existing workspace-owned disposable profile contract to the TypeScript mapper process tree. Keep the direct mapper protocol test, which closes stdin and waits for normal exit, in aggregate coverage.

## Work log

- 2026-09-12: Doctor passed. Read run 34687486210: every test passed, but llvm-profdata rejected a corrupt raw-profile header. The raw artifact was not retained, so the failed PID cannot be identified retrospectively.
- 2026-09-12: The unchanged local coverage command passed at 89.19% lines. Audited subprocess ownership and found the mapper test launcher still inherited aggregate profiles. TypeScript's childProcess.Close closes stdin and immediately kills the mapper, racing its normal exit and profile write (upstream cmd/tsgo/sys.go).

## Issues and resolutions

### Issue 1: Externally terminated mappers inherit aggregate profiles

- **Symptom**: An intermittent invalid raw-profile header prevents coverage merging.
- **Cause**: The mapper suite owns the TypeScript launcher, but TypeScript owns mapper termination. The direct-child isolation audit did not cover this descendant lifecycle.
- **Resolution**: Redirect the launcher's inherited profile path to its test workspace, covering every mapper descendant. Direct, normally exiting mapper protocol coverage remains measured.

## Verification

- `./scripts/ci rust` passed: formatting, clippy with warnings denied, and all Rust tests.
- `TTC_REQUIRE_TSGO=1 cargo llvm-cov --workspace --summary-only --fail-under-lines 86.9` passed with 89.19% line coverage, unchanged from the pre-fix local run.
- Task index validation and `git diff --check` passed.

## Result

The mapper launcher now shares its workspace-owned profile with externally terminated descendants. Changed `tests/content_mapper.rs`, the task index, and TASK-362's audit scope note.
