# TASK-505: Give each value a structured pipeline pipes its own slot

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`export const r = match (1) { _ => [1] } |> (p => p.length);` compiled to `let $tt_v0: number[]; … $tt_v0 = (p => p.length)($tt_v0);`, which `tsc` rejects (`number` is not assignable to `number[]`); `export const a: number = new Box(1) |> .add(match ...) |> .n;` failed `--check-types` the other way (`expected number, found Box`). A structured pipeline stored the head and every step's result in one accumulator, and one annotation cannot describe values whose type each step changes.

## Scope

- Included: Evaluation IR slot planning for structured pipelines, the target emission of `emit_apply_continued` and of the piped value in postfix steps, the design notes, and typed and runtime regression tests.
- Excluded: Pipelines lowered as expressions (`$tt_ap(...)` or direct calls); they have no slot.

## Decisions

### Decision 1: One slot per piped value, planned by the Evaluation IR

- **Context**: The contextual pass (`docs/design/contextual-type-materialization.md`) annotates a generated slot with the contextual type at its references, or else with the join of the values assigned to it. For the accumulator, the reference is the pipeline's consumer, so it received the pipeline's result type (`number`) while holding the head (`Box`); without a context, the join could not type a step's result that depended on the accumulator itself, so only the head's type survived (`number[]`).
- **Alternatives considered**: (a) Leave the accumulator unannotated when the steps change its type. TypeScript then infers an implicit `any` or evolving type for an uninitialized `let` under `strict`, and deciding "the type changes" needs the checker the annotation pass is supposed to feed. (b) Annotate the accumulator with the union of every step's type. Assignment narrowing makes it work for some pipelines, but the steps' types still depend on the accumulator's own annotation, the consumer's contextual type is no longer applied, and the union is a type trick the source never wrote. (c) Write each step's result to a fresh slot.
- **Decision and rationale**: (c). `EvaluationFile::lowering_plan` allocates, for every structured pipeline, one generated slot per step for the value piped into it (`LoweringPlan::piped_slots`, collision-free like every other slot and never globalized because they are block-scoped in the pipeline's `do` block). The head's value goes to the first; each step's result goes to the next step's slot, and the last step's result goes to the pipeline's value slot. Each slot is written once, so the existing contextual pass types it with the context of the step that consumes it (`pick(slot)` gives the object literal its union type) or with the one value written to it; no new typing rule was needed. A value that is not a structured value is written with `const slot = value;` (`Rope::push_value_definition`, which keeps the contextual annotation site of `push_value_capture` without adding grouping parentheses); a structured head keeps `let slot;` and its arms' assignments.

### Decision 2: The piped value of a postfix step is the step's own input slot

- **Context**: TASK-504 printed a postfix step's piped value as the accumulator.
- **Decision and rationale**: `Emitter::piped_value_at` now prints the slot planned for that step, so a method reference is taken from the value the previous step produced, typed as that value.

## Work log

- 2026-09-28: Reproduced `match (1) { _ => [1] } |> (p => p.length)` failing `tsc` and the member-step typed check failing `--check-types` on `b34a123`.
- 2026-09-28: Changed `src/evaluation_ir.rs` and `src/evaluation_ir/evaluation.rs` (`piped_slots`), `src/codegen/core/planning.rs`, `src/codegen/core/mod.rs`, `src/codegen/core/emitter/mod.rs`, `src/codegen/core/emitter/expression.rs`, `src/codegen/core/emitter/host.rs`, and `src/codegen/rope/builder.rs`.
- 2026-09-28: Updated `a_materialized_pipeline_accumulator_uses_a_direct_call` and `pipeline_head_reclaims_a_lifted_match` in `tests/compile/cases_05.rs` to the head's own slot; added the pinned-TypeScript checks `each_pipeline_step_holds_the_type_of_its_own_value` and `a_hoisted_value_in_a_member_step_type_checks` (`tests/native/cases_06.rs`) and the runtime test `a_pipeline_whose_steps_change_the_value_type_compiles_and_runs` (`tests/integration/cases_05.rs`); documented the slots in `docs/design/program-lowering.md` §7.5 and `docs/design/contextual-type-materialization.md`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`: exit 0.
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0.
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: exit 0.
- [x] `./scripts/ci extension`: exit 0.
- [x] The reported reproduction: the emitted TypeScript passes the pinned `tsc --noEmit --strict`.

## Result

Changed `src/evaluation_ir.rs`, `src/evaluation_ir/evaluation.rs`, `src/codegen/core/planning.rs`, `src/codegen/core/mod.rs`, `src/codegen/core/emitter/{mod,expression,host}.rs`, `src/codegen/rope/builder.rs`, `tests/compile/cases_05.rs`, `tests/integration/cases_05.rs`, `tests/native.rs`, `docs/design/program-lowering.md`, `docs/design/contextual-type-materialization.md`, `docs/tasks/INDEX.md`, and this record; added `tests/native/cases_06.rs`.
