# TASK-386: Track canonical directory identity during source discovery

- **Status**: Blocked
- **Started**: 2026-09-23
- **Completed**: —
- **Commit**: —

## Purpose

Source discovery follows directory symlinks but currently models a tree rather than a graph. Cycles repeat traversal, aliases duplicate work, and aliases of the output directory bypass exclusion.

## Scope

- Included: Directory identity in project scanning and CLI source collection; filesystem regression tests.
- Excluded: Language lowering, import resolution, and TypeScript configuration admission.

## Decisions

### Decision 1: Share a canonical directory visitation model

- **Context**: Both source collectors follow symlinks without remembering visited directories.
- **Alternatives considered**: Rejecting all symlinks breaks supported linked sources. Limiting recursion depth hides valid sources and leaves aliases unresolved. Separate fixes would allow the collectors to diverge.
- **Decision and rationale**: Canonicalize each directory before admission, compare output exclusion against that identity, and visit each identity once per walk. Preserve logical CLI file paths and deterministic sorted child traversal. Project sources remain canonical and deduplicated.

## Work log

- 2026-09-23: Inspected main at 392489581cf7c7b6a5d61fe00e2a3aa020ff0a3c and ran `./scripts/doctor`. Rust, Bun, and the project TypeScript installation are absent. Created a working branch and this task before modifying compiler code.

- 2026-09-23: Installed the pinned Rust 1.98.0 toolchain and ran `npm ci`. Added `SourceDirectories` and two filesystem regression tests. Fixed-source `cargo test --test engine_cache`: 7 passed. Running the same tests with the original `project.rs`: exactly the two new tests failed (OS symlink-loop error and generated sources included through output aliases); the original five passed. Restored the fix immediately afterward.
- 2026-09-23: The first build encountered an LLVM archive error. Removed only generated ttc artifacts with `cargo clean -p ttc` and reran with `CARGO_INCREMENTAL=0`; the build and focused tests passed. Installed temporary TypeScript 6 and rolldown test executables outside the repository for the full gate.

## Issues and resolutions

### Issue 1: Source discovery treats a filesystem graph as a tree

- **Symptom**: Directory cycles repeat traversal until an OS error; aliases repeat sources; output aliases enter the candidate set.
- **Cause**: `project_sources` compares lexical directory paths with a canonical output path, and neither collector tracks directory identity.
- **Resolution**: Both collectors now use `SourceDirectories` to admit canonical directory identities once. Output exclusion covers the canonical output subtree, including aliases of descendants. Project results deduplicate canonical file paths.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

- [x] `CARGO_INCREMENTAL=0 cargo test --test engine_cache`: 7 passed.
- [x] Baseline reproduction with the same tests and original `project.rs`: 5 passed, 2 failed exactly at the new regressions.
- [x] `node scripts/check-task-index` and `git diff --check`.
- [ ] `./scripts/ci rust`: formatting and clippy pass, but the full test build fails before test execution. First attempt: undefined hidden symbols from rust-lld. Clean single-job retry: an archive object has zero length. Therefore the full suite and downstream fuzz check are not claimed as passed.

## Result

Implemented the shared directory identity model in `src/engine/project.rs`, added two regression tests in `tests/engine_cache.rs`, and recorded this task in `docs/tasks/INDEX.md`. Focused tests establish failure before the change and success after it. The task remains Blocked because the required full Rust gate has not completed; submit only as a draft for review, with no merge. No compiler flags or toolchain pins in the repository were changed to conceal the build failure.

