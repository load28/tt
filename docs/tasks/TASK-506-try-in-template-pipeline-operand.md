# TASK-506: Schedule a `try` inside a template in a pipeline operand

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

In `` `${f(try g())}` |> String `` the lowered code ran `g()` and its failure exit before reading `f`, although ECMA-262 evaluates a call's callee before its arguments; with a callee that has effects, the order of observable events changed, and on failure the callee was never evaluated at all. A `try` in the same template outside a pipeline was scheduled correctly.

## Scope

- Included: The projection's test for whether a pipeline operand must be shown to the collector, a design note, compile, runtime, and pinned-TypeScript tests.
- Excluded: Nothing else; `match` in a template was already scheduled (TASK-501).

## Decisions

### Decision 1: A template contains a propagation when an interpolation does

- **Context**: `ProjectionBuilder::emit_apply` projects a pipeline as one placeholder and adds a shadow of each operand that contains a tt value, so the SWC collector sees the operand's evaluation structure and builds the value's protocol. It asked `expr_contains_propagation` and `expr_contains_value_region`. The first answered `false` for every `Expr::Template`, and the second descends into interpolations but only finds value regions (a `try`'s input `g()` is not one). A template head holding a `try` therefore had no shadow, the `try` got no schedule inside the pipeline, and the operand delivery printed `f` in place after the propagation had run.
- **Alternatives considered**: Capture callees of template operands in the target when a value has no schedule. That re-derives evaluation structure in the target, which TASK-501 made the projection's and the planner's job, and would only cover the shapes the target recognized.
- **Decision and rationale**: `expr_contains_propagation` descends into template interpolations, as it already did into sequences, pipelines, decisions, and result regions, and as `expr_contains_value_region` already did for templates. The template operand is then shadowed like any other, the collector records the call's callee as an input evaluated before the `try`, and the TASK-501 operand path captures it. Templates nested in templates, in call arguments, and in steps follow from the same recursion.

## Work log

- 2026-09-28: Reproduced the order on `36adff9` (`const $tt_t0 = g(); … $tt_v0 = \`${f($tt_v2)}\``); found the head, a call step's argument, and a template inside a call in a head all affected.
- 2026-09-28: Changed `src/program_syntax/projection.rs`; added `a_try_in_a_template_in_a_pipeline_operand_keeps_its_callee_before_it` (`tests/compile/cases_14.rs`), the node runtime order test `a_try_in_a_template_in_a_pipeline_runs_after_the_callee_it_is_an_argument_of` (`tests/integration/cases_05.rs`; it fails on `36adff9` with `try,callee,call` and without the callee on failure), and `a_try_in_a_template_in_a_pipeline_type_checks` (`tests/native/cases_06.rs`); noted the rule in `docs/design/program-lowering.md` §7.5.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`: exit 0.
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0.
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: exit 0.
- [x] `./scripts/ci extension`: exit 0.
- [x] The reported reproduction: `ttc -p` captures `f` before `g()`.

## Result

Changed `src/program_syntax/projection.rs`, `tests/compile/cases_14.rs`, `tests/integration/cases_05.rs`, `tests/native/cases_06.rs`, `docs/design/program-lowering.md`, `docs/tasks/INDEX.md`, and this record.
