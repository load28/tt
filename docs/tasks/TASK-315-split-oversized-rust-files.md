# TASK-315: Split every oversized Rust source and test file

- **Status**: Complete
- **Started**: 2026-09-02
- **Completed**: 2026-09-02
- **Commit**: `TASK-314: split oversized Rust modules`

## Purpose

Ensure no Rust production or test file grows beyond 1,000 lines by extracting
cohesive responsibility modules without changing behavior, output, diagnostics,
or public APIs.

## Scope

- Included: Every `.rs` file under `src/` and `tests/` above 1,000 lines after
  TASK-314, module-level responsibility documentation, and the full local gate.
- Excluded: Language features, algorithms, emitted output, diagnostic wording,
  public API changes, and non-Rust source trees.

## Decisions

### Decision 1: Use 1,000 physical lines as the repository-wide size ceiling

- **Context**: “Large” needs an objective completion condition that applies to
  production and test code alike.
- **Alternatives considered**: Split only production files; use an informal
  judgment per file; or enforce a uniform physical-line ceiling.
- **Decision and rationale**: Split every Rust file above 1,000 lines. This
  includes integration tests because test maintainability is part of the same
  repository structure problem.

### Decision 2: Preserve implementation text and test bodies during extraction

- **Context**: Broad structural work has a high regression surface.
- **Alternatives considered**: Redesign abstractions while splitting or perform
  mechanical responsibility extraction first.
- **Decision and rationale**: Keep function and test bodies unchanged wherever
  possible, limiting edits to module wiring, imports, and internal visibility.
  Existing snapshot, source-map, runtime, and integration gates remain the
  behavior oracle.

### Decision 3: Keep integration-test helpers in their existing crate scope

- **Context**: The four oversized integration-test files share helpers across
  many test groups, so child modules would either duplicate those helpers or
  broaden their visibility.
- **Alternatives considered**: Create child modules with re-exported helpers;
  duplicate helpers per module; or extract item-boundary case files with
  `include!`.
- **Decision and rationale**: Keep each integration test as one test crate and
  extract only complete test-item ranges into bounded `cases_NN.rs` files. This
  preserves helper scope, test names, and test bodies while meeting the same
  physical-file ceiling.

## Work log

- 2026-09-02: Ran `./scripts/doctor`; all required tools and linked artifacts
  are ready.
- 2026-09-02: Enumerated 19 remaining Rust files above 1,000 lines across
  `src/` and `tests/` after TASK-314.
- 2026-09-02: Extracted inline unit tests from production modules before
  partitioning their implementation responsibilities.
- 2026-09-02: Split program syntax, evaluation IR, analysis, flow, language
  services, semantic translation, public APIs, semantic and val checking,
  parser support, diagnostics, rope construction, and the CLI into focused
  child files.
- 2026-09-02: Split the four oversized integration-test crates into bounded
  item-range case files while retaining their shared helpers at crate scope.
- 2026-09-02: Confirmed all 147 Rust files under `src/` and `tests/` are at or
  below 1,000 physical lines; the largest is 997 lines.

## Issues and resolutions

- **Symptom**: The first attempted `analysis` extraction started at a
  test-only helper attribute instead of the inline test module.
  **Cause**: The boundary search selected the first `#[cfg(test)]` occurrence.
  **Resolution**: Restored the file from `HEAD` and extracted from the actual
  `mod tests` item boundary.
- **Symptom**: Mechanical CLI partitions left documentation attributes at the
  end of preceding files and made sibling-owned fields private.
  **Cause**: Initial physical cut points preceded documented items without
  accounting for their attributes or Rust sibling privacy.
  **Resolution**: Moved each documentation block with its item and limited the
  required fields and constructors to `pub(super)`.

## Verification

- [x] No `.rs` file under `src/` or `tests/` exceeds 1,000 lines (147 files;
  maximum 997 lines)
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci rust`

## Result

All oversized Rust production and test files were split without changing
public APIs, implementation algorithms, diagnostics, emitted output, or test
bodies. The repository-wide Rust gate passes.

## Changed files

- Production module roots under `src/analysis`, `src/codegen`,
  `src/content_mapper`, `src/diagnostics`, `src/engine`, `src/evaluation_ir`,
  `src/flow`, `src/lib`, `src/main`, `src/parser`, `src/program_syntax`,
  `src/render`, `src/sema`, and `src/val`, plus their extracted child files.
- Integration-test roots and extracted cases under `tests/cli`,
  `tests/compile`, `tests/integration`, and `tests/native`.
- `docs/tasks/INDEX.md` and this task record.
