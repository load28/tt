# TASK-291: Repair Result completion in both existing printers

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Item **L0** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`L0` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: implement ResultRegion call registration and `region.value` projection together, plus a real expression-host continuation, so result-owned return always wraps `Ok`.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: `src/program_syntax.rs::expected_exit_calls`, `src/program_syntax.rs::emit_result_region` (the ProgramSyntax projection at `:1078`, not `src/codegen/core.rs::emit_result_region`, the expression-boundary printer at `:1832`), `src/codegen/core.rs::{emit_result_region,emit_result_region_continued,emit_body_with_exits,wrap_result_ok}`, and `docs/ai/tt.md` (the result-block `return` rule, cited by content rather than line number).

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

Test obligation from the plan: the complete two-row §6.2 matrix, runtime assertions, output snapshots, source maps, and type checking.

Green condition: each host independently satisfies its §6.2 condition; neither a raw arm nor raw body value returns from the user function; in the same commit, the language guide states that a result-owned `return x` completes the block with `Ok(x)` and a bare `return;` with `Ok(undefined)`. Constructor/generator policy is not part of this slice.

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Ships to `main` alone: not stated in the plan.

Summarize the changed files and the outcome, then set this record and the index
row to `Complete`.
