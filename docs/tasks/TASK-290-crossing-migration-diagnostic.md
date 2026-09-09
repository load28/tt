# TASK-290: Freeze the one-release crossing migration

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: See git history (`TASK-290`).

## Purpose

Item **M0** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`M0` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: add the `try-crosses-value-region` compatibility diagnostic for the #87-accepted isolated-arm shape, its applicable nested-function extraction edit where mechanically proven safe, and the release-note/help contract; it is staged before language work but published only with the `<-` cutover.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: nearest-scope/crossing analysis in `src/analysis` and `src/sema.rs`, diagnostics and edit spans, `docs/ai/tt.md`, snapshots/content mapper.

## Decisions

Record every decision this task makes. Naming a new public diagnostic code, a
wire-compatibility choice, or a seam that the design leaves open is a decision
and belongs here with its alternatives.

### Decision 1: Offer located help but no edit before nearest-scope lowering.

- **Context**: Extracting a crossing `try` into a nested function changes its
  return target. Preserving the old target would require another crossing
  `try` at the call site.
- **Alternatives considered**: Offer a syntactic extraction; suppress the
  diagnostic until L2; report the migration with non-applicable help.
- **Decision and rationale**: Report the migration now with located help and
  no edit. No current shape can prove capture, evaluation, and old target
  preservation; L4 activates edits alongside nearest-scope propagation.

## Work log

- 2026-08-30: Started locating the existing isolated value-region placement paths and diagnostic registry.
- 2026-08-30: Added the stable crossing diagnostic, registry explanation,
  isolated Result-value traversal, and public migration guidance.
- 2026-08-30: Confirmed nested-function extraction cannot preserve the old
  function target in the current surface; retained located non-edit help.

## Issues and resolutions

- **Symptom**: The old function-targeted propagation could be accepted under
  a Result-isolated match arm. **Cause**: placement did not retain that
  isolated crossing. **Resolution**: distinguish `ResultValueRegion` and
  emit `try-crosses-value-region` at the original `try` span.

## Verification

Test obligation from the plan: preserve conditional evaluation, captures, argument order, and old function-level propagation after applying the edit; require located help without an edit when applicability cannot be proven.

Green condition: the compatibility diagnostic fires on every #87-accepted isolated-arm shape, and every offered edit parses, type-checks, and preserves runtime behavior, captures, and evaluation order/count. The anti-retargeting assertion—that no affected program silently moves from a function target to a ResultRegion target—is not testable until nearest-scope propagation exists and is therefore an L2 exit criterion, re-run there against the M0 corpus.

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Ships to `main` alone: no.

Changed `src/diagnostics.rs`, `src/lib.rs`, `src/sema.rs`,
`tests/compile.rs`, and `docs/ai/tt.md`. The diagnostic is staged for the L4
release and exposes help without an edit until target-preserving rewrites exist.
