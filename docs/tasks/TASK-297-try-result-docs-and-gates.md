# TASK-297: Complete public documentation and release gates

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: See git history

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

### Decision 1: Describe only the committed statement-bodied Result surface

- **Context**: the language guide still described the removed `<-` binding
  and semicolon-free Result tail.
- **Alternatives considered**: retain a parallel legacy section, or replace
  the guide's Result section with the new claim, propagation, completion, and
  migration rules.
- **Decision and rationale**: replace the old surface in place so one guide
  presents one Result syntax.

## Work log

- 2026-08-30: Started the public-language and release-gate audit.
- 2026-08-30: Normalized the VS Code package-install test's temporary-path
  expectation to Node's real-path package resolution on macOS.

## Issues and resolutions

None.

## Verification

Test obligation from the plan: documentation examples compile or diagnose as shown; run `./scripts/ci`, including fmt, clippy, all tests, TypeScript verification, snapshots, server/mapper, and runtime integration.

Green condition: the full gate passes from a clean worktree and the published docs describe no removed rule.

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

The language guide, overview, design status, fixtures, and release gate now
describe and verify the statement-bodied Result surface.
