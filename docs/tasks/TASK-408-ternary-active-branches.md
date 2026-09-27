# TASK-408: Own the calls around a value in either branch of a conditional expression

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

docs/ai/tt.md says a value under `&&`/`||`/`??`/`? :`/`f?.()` lowers the whole operation as one region, and the value form of `try` preserves conditional evaluation. `c && f(match ...)` and `c && Ok(try r)` compiled, but `c ? f(match ...) : 0`, `c ? Ok(try r) : Ok(0)`, and `c ? (try r) + 1 : 0` were rejected with `match-placement` / `try-placement`.

## Scope

- Included: Conditional-operation planning (`src/evaluation_ir/planning.rs`), the plan's active-branch model, and its emission.
- Excluded: Optional-call operations, which keep requiring the value to be a direct argument.

## Decisions

### Decision 1: Record an active branch per consumed value

- **Context**: A logical operation already owned its complete active branch (`active_branch`, `active_steps`), so the calls between the value and the branch were emitted inside the branch. A ternary accepted only a value that was the whole branch (`conditional_index != 0` rejected the operation), and the member check required every value of the operation to have the same number of inner steps.
- **Alternatives considered**: A ternary-only special path would duplicate the logical branch emission.
- **Decision and rationale**: The plan carries `active: Vec<PlannedActiveBranch { value, branch, steps }>`. Logical operations record their one branch as before. A ternary records a branch for each side whose value sits inside other evaluation steps, and the member check compares only the steps outside the operation. The emitter uses the same active-branch emission for both, so the skipped side is still never evaluated.

## Work log

- 2026-09-27: Reproduced the rejections with `ttc --check`. Generalised the plan and emitter; verified values and evaluation order with Node (skipped branches never call `f`/`g`) and types with the repository TypeScript. Added an integration test that fails on the previous compiler.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.
- [x] `a_conditional_branch_owns_the_calls_around_its_value` fails with the previous `src/` (`match-placement`) and passes now.

## Result

Changed `src/evaluation_ir.rs`, `src/evaluation_ir/planning.rs`, `src/evaluation_ir/tests.rs`, `src/codegen/core/planning.rs`, `src/codegen/core/emitter/host.rs`, and `tests/integration.rs`.
