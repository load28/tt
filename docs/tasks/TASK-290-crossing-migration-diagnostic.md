# TASK-290: Freeze the one-release crossing migration

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

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

### Decision 1: <one-line summary>

- **Context**:
- **Alternatives considered**:
- **Decision and rationale**:

## Work log

- YYYY-MM-DD: ...

## Issues and resolutions

None.

## Verification

Test obligation from the plan: preserve conditional evaluation, captures, argument order, and old function-level propagation after applying the edit; require located help without an edit when applicability cannot be proven.

Green condition: the compatibility diagnostic fires on every #87-accepted isolated-arm shape, and every offered edit parses, type-checks, and preserves runtime behavior, captures, and evaluation order/count. The anti-retargeting assertion—that no affected program silently moves from a function target to a ResultRegion target—is not testable until nearest-scope propagation exists and is therefore an L2 exit criterion, re-run there against the M0 corpus.

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Ships to `main` alone: no.

Summarize the changed files and the outcome, then set this record and the index
row to `Complete`.
