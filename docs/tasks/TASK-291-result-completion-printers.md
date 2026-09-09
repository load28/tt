# TASK-291: Repair Result completion in both existing printers

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: See git history (`TASK-291`).

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

### Decision 1: Keep the projection tail out of Result return capture.

- **Context**: Projecting `region.value` requires a synthetic `return` in the
  lexical arrow, but only source-written Result returns are completions that
  codegen must rewrite.
- **Alternatives considered**: Capture the synthetic return as a regular
  `HostExit`; retain the former `0` placeholder; record the synthetic span and
  exclude it from exit collection.
- **Decision and rationale**: Record and exclude the synthetic span. The tail
  keeps its own expression owner, while source-written returns retain exact
  source spans for the two completion printers.

## Work log

- 2026-08-30: Started auditing the two Result printers and their projection exit-call registration.
- 2026-08-30: Registered Result projection calls for exit collection, projected
  `region.value`, and excluded the compiler-written tail return from capture.
- 2026-08-30: Rewrote source-written Result returns to `Ok` completions in the
  expression-boundary and statement-host printers.

## Issues and resolutions

- **Symptom**: A projected `region.value` produced an unmapped evaluation span
  when the collector treated the compiler-written tail return as a user exit.
  **Cause**: The tail return has no source-backed statement span.
  **Resolution**: The projection records that span as synthetic and the
  collector excludes it before mapping exits.

## Verification

Test obligation from the plan: the complete two-row §6.2 matrix, runtime assertions, output snapshots, source maps, and type checking.

Green condition: each host independently satisfies its §6.2 condition; neither a raw arm nor raw body value returns from the user function; in the same commit, the language guide states that a result-owned `return x` completes the block with `Ok(x)` and a bare `return;` with `Ok(undefined)`. Constructor/generator policy is not part of this slice.

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Ships to `main` alone: not stated in the plan.

Changed `src/program_syntax.rs`, `src/codegen/core.rs`, `tests/compile.rs`, and
`docs/ai/tt.md`. Both existing Result printers now convert source-written
`return x` and `return;` into `Ok(x)` and `Ok(undefined)` completions while the
projected tail retains its own expression owner.
