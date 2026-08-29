# TASK-283: Close host projection and source-preservation crashes

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Item **P1** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`P1` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: make C-style declaration-init Legal by hoisting only its initializer value while retaining `let i` in the header; make repeated for-test a located rejection; make discarded Result source preservation diagnose rather than ICE.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: `src/program_syntax.rs::{owner_reach,HostContinuation}`, `src/program_syntax.rs::emit_result_region` (the ProgramSyntax projection at `:1078`, not the codegen printer of the same name at `src/codegen/core.rs:1832`), `src/evaluation_ir.rs`, `src/codegen/core.rs::lowering_plan`, and relevant diagnostics.

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

Test obligation from the plan: statement and expression-boundary host cases, `for (let i = try n();;)`, `for (; try ready();)`, assignment-init guard, and `result { ... };`, all wrapped around `analyze` and verified as parseable output or located diagnostics.

Green condition: Legal init executes once and retains declaration semantics; repeated test and discard never panic.

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Ships to `main` alone: yes.

Summarize the changed files and the outcome, then set this record and the index
row to `Complete`.
