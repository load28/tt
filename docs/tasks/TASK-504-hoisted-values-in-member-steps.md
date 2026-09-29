# TASK-504: Evaluate a hoisted value in a member step after the piped value and its method

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`declare const x: { m(a: number): number };⏎export const r = x |> .m(match (1) { _ => 1 });` failed with `verify-failed` ("Expression expected"): the emitter printed `$tt_v1 = .m($tt_v1); $tt_v0 = $tt_v0$tt_v1;`. `x |> .a(match ...) |> .b(match ...)` failed with `lowering-plan-failed` ("the construct has two TypeScript hosts"). `tt.md` defines a postfix step as a chain applied to the piped value, evaluated as JavaScript evaluates `x.m(arg)`: the piped value, then the method reference, then the argument, then the call.

## Scope

- Included: The projection of postfix pipeline steps, the pipeline's ownership of values in optional postfix steps, the target delivery of a postfix step's operand and captures, the design note, compile, runtime, and regression tests.
- Excluded: The accumulator's type annotation across steps (TASK-505), templates in pipeline operands (TASK-506), and the optional-call receiver that is not narrowed by the call's own nullish check (see Result).

## Decisions

### Decision 1: Project the piped value, not the head and earlier steps

- **Context**: `ProjectionBuilder::emit_apply` projected a postfix step's shadow as the head followed by every earlier step and the step itself (`x.a(<m1>).b(<m2>)`). An earlier step's tt value was then projected twice (the duplicate-host failure), and the protocol of a value in the step described a callee (`x.m`) whose source span began at the head and crossed ` |> `. The head's bytes are delivered by the pipeline's `acc = head`, so a capture of that span evaluated the head a second time; only an optional step had a special case (`push_pipeline_member_reference`) that printed the receiver and the step's suffix instead.
- **Alternatives considered**: (a) Keep the old projection and teach the target to recognize a capture whose span starts at the head. This is string-shape repair in the target and does not fix the duplicate host. (b) Rewrite postfix steps into call steps in Core IR. That changes the language's Core model to fix an evaluation-structure question the projection owns.
- **Decision and rationale**: The projection writes one placeholder for the piped value in front of the step's own shadow (`$tt_syntax_piped.m(<match>)`) and maps it to the empty source span at the step's start. The piped value has no source bytes: the pipeline already holds it. The SWC collector then reads TypeScript's own structure for the tail: a call argument behind a member callee with a receiver, a computed member property behind its object, an optional call as a conditional operation. Every input that contains the piped value starts at the step, inside the pipeline, so the bounded schedule of TASK-501 covers it without special cases, and `push_pipeline_member_reference` was removed.

### Decision 2: The target prints the piped value as the accumulator

- **Context**: A capture or delivery that starts at a postfix step's first byte contains the piped value.
- **Decision and rationale**: `Emitter::piped_value_at` answers, for a position, the accumulator of the pipeline whose postfix step begins there, anchored like the existing piped input (`AnchorKind::Pipe` with the producing step as context). `captured_source` and `source_range_with_scheduled_values` add it as a zero-width part at that position, ordered after any wider capture that already contains it; the tail after a member receiver is printed with `captured_tail`, which never adds it, because the receiver already did. `emit_apply_continued` delivers a postfix step that contains values through the operand path of TASK-501 (`acc = <operand>`), as it already did for call steps.

### Decision 3: A pipeline owns values in its optional steps

- **Context**: `CoreFile::has_statement_form` excluded values in an optional postfix step from the pipeline's region, because the region used to have no model for the step's conditional reach.
- **Decision and rationale**: TASK-501 plans values an enclosing value owns as conditional operations. With the step's own structure projected, `x |> ?.m(match ...)` is an optional-call operation inside the pipeline, emitted by the operand path. The exclusion is removed; a conditional shape that cannot be owned whole is still a placement diagnostic through the TASK-501 planning.

## Work log

- 2026-09-28: Reproduced `x |> .m(match ...)` (`verify-failed`), `f() |> .m(match ...)`, `x |> .a(1).b(match ...)` (`verify-failed`), and `x |> .a(match ...) |> .b(match ...)` (`lowering-plan-failed`) on `e19c22c`. Checked `x |> obj.m(match ...)`: it is a call step whose function is `obj.m(match ...)`, and it already evaluated the piped value, `obj`, `obj.m`, the match, and both calls in order.
- 2026-09-28: Changed `src/program_syntax/projection.rs` (piped-value placeholder), `src/core_ir/mod.rs` (optional steps), `src/codegen/core/emitter/host.rs` (`piped_value_at`, `pipe_input`, `captured_tail`, piped parts; removed `push_pipeline_member_reference`), and `src/codegen/core/emitter/expression.rs` (postfix operand path).
- 2026-09-28: Added `a_hoisted_value_in_a_member_step_is_emitted_once_after_its_method` and `a_member_step_captures_its_method_from_the_piped_value_before_the_argument` to `tests/compile/cases_14.rs`, and the node runtime order test `a_hoisted_value_in_a_member_step_runs_after_the_piped_value_and_its_method` to `tests/integration/cases_05.rs`; documented the model in `docs/design/program-lowering.md` §7.5.

## Issues and resolutions

### Issue 1: A typed check of a member step failed on the accumulator's annotation

- **Symptom**: `export const a: number = new Box(1) |> .add(match ...) |> .n;` failed `--check-types` with `expected number, found Box`.
- **Cause**: The accumulator is one slot for every step and the contextual pass annotates it with the pipeline's result type. This is TASK-505.
- **Resolution**: The pinned-TypeScript check for member steps is added with TASK-505.

### Issue 2: `this` of an optional call's captured receiver is not narrowed

- **Symptom**: The runtime test's first version declared `addTo(this: Box, ...)`; `tsc` rejected `$tt_v25.call($tt_v26, value)` with `Box | undefined` for `this`.
- **Cause**: The optional-call operation captures the receiver before its nullish check and calls through it; TypeScript does not narrow the receiver from the check on the method. The same happens without a pipeline (`b?.addTo(match ...)` with `b: Box | undefined`), so it predates this task.
- **Resolution**: The runtime test uses a method without a `this` parameter; the owner-level issue is left as a follow-up.

## Verification

- [x] `cargo fmt --check`: exit 0.
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0.
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: exit 0.
- [x] `./scripts/ci extension`: exit 0.
- [x] The reported reproduction: `ttc -p` exits 0 and evaluates `x`, `x.m`, the match, then the call.

## Result

Changed `src/program_syntax/projection.rs`, `src/core_ir/mod.rs`, `src/codegen/core/emitter/host.rs`, `src/codegen/core/emitter/expression.rs`, `tests/compile/cases_14.rs`, `tests/integration/cases_05.rs`, `docs/design/program-lowering.md`, `docs/tasks/INDEX.md`, and this record.

Follow-up left out of scope: an optional call whose receiver is `T | undefined` and whose method declares a `this` parameter fails `tsc` after lowering, because the captured receiver is not narrowed by the operation's nullish check.
