# TASK-757: Complete the refactoring roadmap audit and validation

- **Status**: In progress
- **Started**: 2026-10-04
- **Completed**: —
- **Commit**: —

## Purpose

Review the completed extraction series and remaining first-party ownership boundaries, run all default repository gates, and reconcile roadmap and task evidence.

## Scope

- Included: `docs/design/behavior-preserving-refactoring.md`, `docs/design/refactoring-completion-audit.md`.
- Excluded: Behavior changes, public API changes, baseline updates, and TASK-732.

## Decisions

### Decision 1: Finish the bounded roadmap with one complete validation

- **Context**: The user requested autonomous completion and progression through
  the remaining refactoring. TASK-744 through TASK-756 now form separate local
  implementation commits with their own structural and focused evidence.
- **Alternatives considered**: Stop after each extraction; mechanically split
  every large file; publish unvalidated intermediate changes.
- **Decision and rationale**: Review retained ownership surfaces, validate the
  combined series with all six default gates, and prepare one final review PR.
  Keep stateful phase owners intact when there is no justified new boundary.
  This continuation supersedes the roadmap's intermediate-merge schedule;
  no intermediate merges are claimed. See the
  [detailed brief](../design/refactoring-final-audit.md) and
  [completion audit](../design/refactoring-completion-audit.md).

## Work log

- 2026-10-04: Started from local base `ef9dbeff984a10a75b03b429b916e3b62de60828` on
  `refactor/remaining-roadmap`. Preserved base source under
  `/tmp/tt-refactor/757` before documentation edits.

- 2026-10-04: Reviewed the complete production diff, new module dependencies and retained ownership surfaces; recorded the 417-file inventory method and explicit exclusions in the completion audit. The first full CI run later encountered process/thread exhaustion; details are recorded below.

- 2026-10-04: Pushed the 13 implementation commits and audit commit to
  `origin/refactor/remaining-roadmap` and created draft
  [PR #139](https://github.com/load28/tt/pull/139), targeting `main`.
  The PR states the passed checks and incomplete full local CI explicitly.
  Task-index and whitespace checks passed; reference expectations remain unchanged.

## Issues and resolutions

### Process/thread exhaustion during full CI

Over 32,000 unreaped processes were observed in the long-lived container.
Rust reported OS error 11 while spawning threads, and Node failed its
`uv_thread_create` startup assertion. The first run passed agents, formatting,
clippy, 807 Rust tests across nine completed suites, all 121 npm-stage tests,
and website typecheck/build before the remaining gates were interrupted or
failed. An external child-subreaper/two-CPU retry also stalled and was stopped.
No source behavior, baseline expectation, case filter, or timeout was changed.
The audit records logs and the remaining validation obligation.

### Draft publication authorization

Automatic approval initially rejected uploading repository content because
explicit publication authorization was absent. On 2026-10-04 the user then
explicitly instructed creation of the PR. Prepare the draft with the incomplete
CI status visible; this supersedes preparation-only publication timing, but
allows no merge or false completion claim. The GitHub CLI token was unavailable, but the configured Git transport pushed
the branch successfully; the GitHub connector created the draft PR.


## Regression test (fails before the fix)

Not applicable: Final audit and validation of behavior-preserving extractions; no bug is fixed.

## Verification

- [x] Moved definitions and parent changes reviewed.
- [ ] Focused checks: Run ./scripts/ci, review the complete production diff, verify unchanged reference expectations, and run task-index and whitespace checks.
- [ ] Applicable complete repository gates.
- [x] Reference expectations unchanged; task-index and whitespace checks.

## Result

Implementation and source audit are published in
[draft PR #139](https://github.com/load28/tt/pull/139).
Final local CI and task completion remain pending in a healthy environment.
