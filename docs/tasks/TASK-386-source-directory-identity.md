# TASK-386: Track canonical directory identity during source discovery

- **Status**: Complete
- **Started**: 2026-09-23
- **Completed**: 2026-09-23
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

### Decision 2: Preserve error reporting and caller-owned output selection

- **Context**: Review proposed silently skipping identity failures and passing an output directory to every source collector.
- **Alternatives considered**: Ignoring unreadable inputs would accept incomplete builds and contradict existing CLI error tests. Changing every collector signature would move mode-specific input selection into a generic enumerator.
- **Decision and rationale**: Keep I/O errors visible. Ordinary builds exclude canonical output paths in `main/build.rs::build_jobs` after enumeration. Typed mode collects only tt roots and emits `.tt.d.ts`/`.ttx.d.ts` sidecars, which are not tt inputs; the project candidate scan separately excludes its configured output tree. Document this boundary, make alias assertions explicit, and test file-level aliases because directory visitation alone does not deduplicate those files. Hosted CI run 35842691544 passed for the first commit; this does not replace the required local gate.

## Work log

- 2026-09-23: Inspected main at 392489581cf7c7b6a5d61fe00e2a3aa020ff0a3c and ran `./scripts/doctor`. Rust, Bun, and the project TypeScript installation are absent. Created a working branch and this task before modifying compiler code.

- 2026-09-23: Installed the pinned Rust 1.98.0 toolchain and ran `npm ci`. Added `SourceDirectories` and two filesystem regression tests. Fixed-source `cargo test --test engine_cache`: 7 passed. Running the same tests with the original `project.rs`: exactly the two new tests failed (OS symlink-loop error and generated sources included through output aliases); the original five passed. Restored the fix immediately afterward.
- 2026-09-23: The first build encountered an LLVM archive error. Removed only generated ttc artifacts with `cargo clean -p ttc` and reran with `CARGO_INCREMENTAL=0`; the build and focused tests passed. Installed temporary TypeScript 6 and rolldown test executables outside the repository for the full gate.

- 2026-09-23: Reopened verification for PR #127 review. Re-ran doctor; the pinned Rust and TypeScript dependencies are available. Reviewed existing unreadable-entry tests, build output filtering, and typed sidecar naming before changing behavior.

- 2026-09-23: Clarified collector/error ownership, strengthened alias assertions, added a file-symlink regression, and extended the CLI output test with an alias. Focused engine tests: 8 passed; output-alias and unreadable-entry CLI tests: 2 passed.
- 2026-09-23: Re-ran `./scripts/ci rust` with `CARGO_TARGET_DIR=/tmp/tt-review-build`, `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=2`, and temporary TypeScript 6/rolldown executables on PATH. The fresh build passed formatting, clippy, all 1,249 tests (zero failed or ignored), and the standalone fuzz target check. The previous local build errors did not recur; no repository build settings were changed.

## Issues and resolutions

### Issue 1: Source discovery treats a filesystem graph as a tree

- **Symptom**: Directory cycles repeat traversal until an OS error; aliases repeat sources; output aliases enter the candidate set.
- **Cause**: `project_sources` compares lexical directory paths with a canonical output path, and neither collector tracks directory identity.
- **Resolution**: Both collectors now use `SourceDirectories` to admit canonical directory identities once. Output exclusion covers the canonical output subtree, including aliases of descendants. Project results deduplicate canonical file paths.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`: 1,249 passed, zero failed or ignored.

- [x] `CARGO_INCREMENTAL=0 cargo test --test engine_cache`: 7 passed.
- [x] Baseline reproduction with the same tests and original `project.rs`: 5 passed, 2 failed exactly at the new regressions.
- [x] `node scripts/check-task-index` and `git diff --check`.
- [x] `./scripts/ci rust`: passed in a fresh target directory, including the standalone fuzz target check. Earlier attempts failed before test execution with linker/archive errors; those historical failures are superseded by this completed local run.
- [x] Review regression suite: 8 engine tests plus 2 targeted CLI tests passed.

## Result

Implemented the shared directory identity model in `src/engine/project.rs`, added two regression tests in `tests/engine_cache.rs`, and recorded this task in `docs/tasks/INDEX.md`. Focused tests establish failure before the change and success after it. Review follow-up adds ownership comments and file-alias coverage, strengthens watch/scan alias assertions, and extends the existing CLI output test in `tests/cli.rs`. The required full local Rust gate now passes, so the task is Complete. No runtime behavior was changed during review, and no repository toolchain pins or build flags were changed. PR review and merge remain separate from this implementation record.

