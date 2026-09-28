# TASK-478: Decide a guarded all-wildcard arm by its guard alone

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`match (a, b) { (A, _) => 1, (_, _) if cond => 2, _ => 3 }` emitted `if () { if (cond) { ... } }` and failed the output self-check with "Expression expected" (found during TASK-469, Issue 1). An arm whose pattern tests nothing must be decided by its guard alone.

## Scope

- Included: The Core IR arm predicates (`src/core_ir/mod.rs`), the arm test and conditional chains in the pattern emitter (`src/codegen/core/emitter/pattern.rs`, `helpers.rs`), the inline-selection plan (`src/codegen/core/planning.rs`), and regression tests.
- Excluded: Language changes. A single-subject `_ if cond` arm is still rejected by the parser (`_` must be the last arm), and literal elements stay out of tuple patterns.

## Decisions

### Decision 1: One arm-level predicate for "tests nothing" and "always matches"

- **Context**: A tuple pattern of wildcards lowers to `PatternPlan::AllOf([Any, Any])`, which tests nothing but is not `PatternPlan::Any`. The three emitters that turn an arm into a condition disagreed on how to recognize such an arm: the statement if-chain used `pattern_has_test` but still opened `if (<empty test>) {` when a guard was present, and the selected-value and inline conditional-expression emitters recognized only a literal `PatternPlan::Any`, so they wrote `() ? ...` and `( && guard) ? ...`. The inline-plan admission check used the same literal comparison for totality.
- **Alternatives considered**:
  - Normalize an all-wildcard tuple to `PatternPlan::Any` during Core lowering. That fixes only the tuple spelling; any other test-free plan (a future shape) would reach the same broken emitters, and the emitters would keep three different notions of "unconditional".
  - Patch the if-chain alone. Leaves the expression emitters writing `()` for the same pattern.
- **Decision and rationale**: `PatternPlan::has_test` and `DecisionArm::always_matches` in Core IR state the property once. The emitter builds every arm condition through `emit_arm_test`, which joins the pattern's tests and the guard with `&&` only when both exist, yields the guard alone for a test-free pattern, and yields nothing for an arm that always matches, which ends the conditional chain. The statement if-chain opens no pattern `if` for a test-free arm, so its guard (already emitted by `emit_arm_action`) is the whole test. The inline-plan admission and subject-storage checks use `always_matches`, so a final `(_, _)` is total like `_`.

## Work log

- 2026-09-28: Reproduced with `ttc` on the shape above: verification failed; `--no-verify` showed `if () { if (cond) { ... } }` in the statement chain, and the same match as a call argument or sibling argument produced `() ? ...` / `( && cond) ? ...`.
- 2026-09-28: Added `PatternPlan::has_test` and `DecisionArm::always_matches` (`src/core_ir/mod.rs`), removed the emitter-local `pattern_has_test` (`helpers.rs`), added `emit_arm_test` and used it in `emit_selected_arm_values`, `emit_inline_match` (`pattern.rs`); fixed the open/close of test-free arms in `emit_if_chain`; used `always_matches` in `inline_subject_needs_storage` and the inline-plan admission (`planning.rs`).
- 2026-09-28: Tests: `a_guarded_all_wildcard_tuple_arm_is_tested_by_its_guard_alone` (`tests/compile/cases_11.rs`) pins the statement output; `runtime_guarded_all_wildcard_arm_is_decided_by_its_guard` (`tests/integration/cases_05.rs`) type-checks and runs the statement, final `(_, _)`, selected-value, inline sibling, and guarded literal forms.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/core_ir/mod.rs`, `src/codegen/core/emitter/pattern.rs`, `src/codegen/core/emitter/helpers.rs`, `src/codegen/core/planning.rs`, `tests/compile/cases_11.rs`, `tests/integration/cases_05.rs`, this record, and `docs/tasks/INDEX.md`. A guarded arm whose pattern tests nothing is tested by its guard in every match emission form.
