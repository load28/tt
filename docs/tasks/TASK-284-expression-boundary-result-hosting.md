# TASK-284: Repair expression-boundary Result hosting

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Item **P2** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`P2` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: replace the #87 `MissingHost` path for a current Result containing value-form `try` with the current language's located placement result, without changing passthrough.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: `src/program_syntax.rs::expected_exit_calls`, `src/program_syntax.rs::emit_result_region` (the ProgramSyntax projection at `:1078`, not `src/codegen/core.rs::emit_result_region`, the expression-boundary printer at `:1832`), `src/evaluation_ir.rs` target capability, and `src/codegen/core.rs::{lowering_plan,emit_result_region}`.

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

Test obligation from the plan: the same Result input in statement-capable and expression-boundary owners through analyze/compile/server/Engine; include the already-correct expression-host match-tail guard.

Green condition: no `MissingHost`, no unwind, and a stable located code at every host.

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Ships to `main` alone: yes.

Summarize the changed files and the outcome, then set this record and the index
row to `Complete`.
