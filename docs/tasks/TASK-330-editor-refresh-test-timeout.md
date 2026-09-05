# TASK-330: Investigate intermittent editor buffer-refresh test timeout

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Retain the unresolved editor test timeout observed during TASK-324 without misclassifying it as a confirmed editor product defect.

## Scope

- Included: Reproduction and lifecycle/timing investigation of `unsaved tt changes refresh untouched ttx diagnostics` in the extension integration suite.
- Excluded: Increasing timeouts, ignoring cancellations, or claiming a production race without evidence.

## Decisions

### Decision 1: Keep the observation separate from its unproven cause

- **Context**: One run timed out while a separate extension rebuild/test was active. Subsequent isolated and full runs passed.
- **Alternatives considered**: Declare a race fixed after a passing rerun, or add retries; neither establishes the cause.
- **Decision and rationale**: Preserve the observation and investigate reproducibility, request completion, process lifecycle, and shared build artifacts before changing behavior.

## Work log

- 2026-09-05: Recorded TASK-324's earlier 60,000 ms timeout: 122 tests passed and one was cancelled. The isolated rerun passed all 123 tests. The latest full `./scripts/ci` run passed all 127 extension tests with zero skips/cancellations. No causal relationship with concurrent rebuilding was established.

## Issues and resolutions

### Issue 1: One editor buffer-refresh integration test did not complete

- **Symptom**: `unsaved tt changes refresh untouched ttx diagnostics` reached its 60,000 ms timeout in one full validation run.
- **Cause**: Unknown. Concurrent extension rebuilding was observed, not proved causal.
- **Resolution**: Pending investigation. Compare serial runs with controlled overlapping build/test execution in disposable outputs; capture pending requests and server/process lifecycle. Fix the responsible implementation or test isolation only when evidence supports it. If unreproduced, document the run count and retain the uncertainty.

## Verification

- [ ] Reproduction commands, run counts, outcomes, and lifecycle evidence recorded
- [ ] Product behavior distinguished from test/build isolation
- [ ] Targeted repeated tests and full `./scripts/ci`, without retries masking failures
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## Result

Pending investigation; not a confirmed remaining production bug. Original observation: [TASK-324](./TASK-324-scoped-contextual-continuations.md).
