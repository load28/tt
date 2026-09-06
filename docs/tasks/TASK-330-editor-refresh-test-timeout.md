# TASK-330: Investigate intermittent editor buffer-refresh test timeout

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-330: record the buffer-refresh timeout investigation`

## Purpose

Retain the unresolved editor test timeout observed during TASK-324 without
misclassifying it as a confirmed editor product defect.

## Scope

- Included: Reproduction and lifecycle/timing investigation of
  `unsaved tt changes refresh untouched ttx diagnostics` in the extension
  integration suite.
- Excluded: Increasing timeouts, ignoring cancellations, or claiming a
  production race without evidence.

## Decisions

### Decision 1: Keep the observation separate from its unproven cause

- **Context**: One run timed out while a separate extension rebuild/test was
  active. Subsequent isolated and full runs passed.
- **Alternatives considered**: Declare a race fixed after a passing rerun, or
  add retries; neither establishes the cause.
- **Decision and rationale**: Preserve the observation and investigate
  reproducibility, request completion, process lifecycle, and shared build
  artifacts before changing behavior.

### Decision 2: Close the investigation without a behavior change

- **Context**: The recorded conditions were reproduced deliberately and the
  test passed every time; the request lifecycle was read and cannot hang.
- **Alternatives considered**: Leave the task open indefinitely; add a retry
  or a longer timeout to make the observation moot.
- **Decision and rationale**: Record the attempts, their counts and
  conditions, and the lifecycle evidence, and close the task with the
  uncertainty retained. No production defect is claimed and no test behavior
  is relaxed, so a recurrence would still fail loudly.

## Work log

- 2026-09-05: Recorded TASK-324's earlier 60,000 ms timeout: 122 tests passed
  and one was cancelled. The isolated rerun passed all 123 tests.
- 2026-09-05: Reviewed the request lifecycle in `editors/vscode/server/src/engine.ts`.
  A spawn failure, a child `error`/`exit`, a stdin write error and a
  per-request timeout (15,000 ms) each resolve the pending request with
  `null`, and `shutdownEngineServer` drains the pending map, so no engine
  request can outlive its timeout. The 60,000 ms figure is the test's own
  bound on an unbounded `waitFor`, not a server-side wait, so a recurrence
  means one expected `publishDiagnostics` never arrived rather than that the
  server hung.
- 2026-09-05: Reproduction attempts, all on this 4-core checkout, all with
  zero failed and zero cancelled tests:
  - the named test alone, 3 runs (~2.3 s each);
  - the named test with `cargo build` concurrently rewriting
    `target/debug/ttc`, 6 runs (2–3 s each);
  - two concurrent full `server.test.js` suites (39 tests each);
  - three concurrent full suites with concurrent rebuilds (39 tests each);
  - three full `./scripts/ci` runs earlier the same day (127 extension tests
    each).
  The timeout did not recur in any of them.

## Issues and resolutions

### Issue 1: One editor buffer-refresh integration test did not complete

- **Symptom**: `unsaved tt changes refresh untouched ttx diagnostics` reached
  its 60,000 ms timeout in one full validation run.
- **Cause**: Not established. Concurrent extension rebuilding was observed,
  not proved causal, and deliberately reproducing that condition (and
  harsher ones) did not reproduce the timeout.
- **Resolution**: Closed as unreproduced. The engine cannot hold a request
  past its own timeout, so the mechanism would have to be a missing
  publication rather than a stalled server; no such path was demonstrated.
  The test keeps its bound and its strict zero-cancelled expectation, so a
  recurrence still fails the suite and reopens this record with new evidence.

## Verification

- [x] Reproduction commands, run counts, outcomes, and lifecycle evidence
  recorded (Work log)
- [x] Product behavior distinguished from test/build isolation: the deliberate
  build-contention and concurrent-suite reproductions passed
- [x] Targeted repeated tests and full `./scripts/ci`, without retries masking
  failures
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

No production defect established and no code changed. Original observation:
[TASK-324](./TASK-324-scoped-contextual-continuations.md). Changed files: this
record and `docs/tasks/INDEX.md`.
