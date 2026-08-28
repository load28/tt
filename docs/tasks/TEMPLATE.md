# TASK-NNN: <Title>

- **Status**: Pending | In progress | Complete | Blocked | Cancelled
- **Started**: YYYY-MM-DD
- **Completed**: —
- **Commit**: —

## Purpose

Explain why this task is needed in one or two sentences.

## Scope

- Included: What this task changes
- Excluded: Explicit boundaries that prevent scope expansion

## Decisions

Record every meaningful decision made during the task. For each decision:

### Decision 1: <One-line decision summary>

- **Context**: What choice was required and why it had to be made now
- **Alternatives considered**: Alternatives and their tradeoffs
- **Decision and rationale**: What was selected, why it was preferable, and any
  measurable evidence such as benchmarks, `cargo metadata`, or test results

## Work log

Record the work chronologically with enough detail to reproduce it. Include
the files changed and the commands used for verification.

- YYYY-MM-DD: ...

## Issues and resolutions

Write `None.` when there were no issues. For each issue:

### Issue 1: <One-line symptom summary>

- **Symptom**: What failed, including relevant errors or logs
- **Cause**: How the root cause was identified
- **Resolution**: How it was fixed, including any workaround or remaining debt

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## Result

Summarize the changed files and final outcome. Link any follow-up task.
