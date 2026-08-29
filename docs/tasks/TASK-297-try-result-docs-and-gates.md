# TASK-297: Complete public documentation and release gates

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Item **L6** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`L6` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: update the AI language guide, design status, reference/examples, migration notes, and task records, then run the full repository gate.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: `docs/ai/tt.md`, this design, user-facing English documentation, `docs/tasks`, fixtures.

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

Test obligation from the plan: documentation examples compile or diagnose as shown; run `./scripts/ci`, including fmt, clippy, all tests, TypeScript verification, snapshots, server/mapper, and runtime integration.

Green condition: the full gate passes from a clean worktree and the published docs describe no removed rule.

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Ships to `main` alone: not stated in the plan.

Summarize the changed files and the outcome, then set this record and the index
row to `Complete`.
