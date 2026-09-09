# TASK-314: Split the TypeScript codegen core into responsibility modules

- **Status**: Complete
- **Started**: 2026-09-02
- **Completed**: 2026-09-02
- **Commit**: `TASK-314: split oversized Rust modules`

## Purpose

Reduce the 5,372-line TypeScript target-lowering file into cohesive Rust modules
without changing emitted TypeScript, source mappings, diagnostics, or public APIs.

## Scope

- Included: Split `src/codegen/core.rs` by planning and emission responsibility,
  preserve the existing codegen boundary, and run the full local gate.
- Excluded: Language changes, output-format changes, algorithm rewrites, test
  reorganization, and unrelated large files.

## Decisions

### Decision 1: Preserve behavior through a mechanical module extraction

- **Context**: The largest production file combines host rewrite planning,
  source-preserving emission, Result-region emission, and pattern emission.
- **Alternatives considered**: Rewrite the codegen abstractions while splitting;
  split tests first; or mechanically extract cohesive implementation blocks.
- **Decision and rationale**: Extract existing implementation blocks with only
  the visibility changes required by Rust module privacy. This minimizes semantic
  change and lets the existing snapshot and integration suites prove parity.

## Work log

- 2026-09-02: Fast-forwarded clean `main` to `f4fa4f4`, ran
  `./scripts/doctor`, and confirmed the development environment is ready.
- 2026-09-02: Selected `src/codegen/core.rs` as the first target because it is
  the largest production source file at 5,372 lines.
- 2026-09-02: Created `codex-task-314-split-codegen-core`; the preferred
  `codex/task-314-split-codegen-core` name was unavailable because a `codex`
  branch already occupies that Git ref prefix.
- 2026-09-02: Split the target-lowering entry point, rewrite planner, shared
  emitter state, source traversal, host scheduling, Result regions, expression
  lowering, pattern lowering, and helpers into responsibility modules. Every
  resulting codegen core file is below 1,000 lines.
- 2026-09-02: Ran `./scripts/ci rust`; formatting, Clippy, unit, snapshot,
  source-map, pass-through, runtime integration, native backend, and doctests
  all passed.

## Issues and resolutions

### Issue 1: Preferred branch prefix conflicts with an existing branch

- **Symptom**: Git could not create `codex/task-314-split-codegen-core` because
  `refs/heads/codex` already exists.
- **Cause**: Git cannot use the same ref path as both a branch and a directory
  prefix.
- **Resolution**: Used the repository's established fallback naming style,
  `codex-task-314-split-codegen-core`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Replaced the 5,372-line `src/codegen/core.rs` with a small orchestration module,
a rewrite-planning module, and seven focused emitter modules. Only module
boundaries and codegen-internal visibility changed; the full Rust gate and
emitted-output snapshots pass unchanged.

## Changed files

- `src/codegen/core/mod.rs`
- `src/codegen/core/planning.rs`
- `src/codegen/core/emitter/*.rs`
- `docs/tasks/INDEX.md`
- `docs/tasks/TASK-314-split-codegen-core.md`
