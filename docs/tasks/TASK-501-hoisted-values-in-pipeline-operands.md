# TASK-501: Compose hoisted values inside pipeline operands like any other expression owner

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

A `match` or a value-form `try` inside a pipeline head or step, below the operand's top level, stopped the compiler with an internal error: `declare function f(a: any): any;⏎export const r = f(match (1) { _ => 1 }) |> String;` exited 101 with `validate_source_preservation ... SourceEmittedTwice`, through `-p`, `--check`, and the server's `check`/`emitMap`. So did `match ... + 1 |> String`, `1 + match ... |> String`, `[match ...] |> String`, `-match ... |> String`, `(match ...).toFixed() |> String`, `1 |> f(match ...)`, and `function h() { return f(try g()) |> String; }`. `tt.md` says a match is a value, the value form of `try` works inside a larger expression, and a lowering failure is a located diagnostic, never an internal error.

## Scope

- Included: Evaluation IR planning of values that an enclosing tt value owns (a pipeline, a match subject, an expression arm body), the target emission of an operand that contains such values, the design note, compile and runtime regression tests.
- Excluded: A pipeline member step whose tail holds a hoisted value (`x |> .m(match ...)`), a pipeline accumulator whose checker-inferred annotation does not fit a later step's value, and the callee capture of a `try` inside a template inside a pipeline; see Result for each.

## Decisions

### Decision 1: Where the source was emitted twice

- **Context**: `Emitter::emit_apply_continued` lowers a pipeline with a hoisted value as `do { acc = <head>; acc = step(acc); … } while (false)`. A head that is a `Sequence` reached `emit_sequence_continued`, which declared the child's slot, called `emit_continued_expr` on the child, and then delivered the head's source with the child replaced by its slot. The Evaluation IR had recorded the child's schedule inside the pipeline (`owned_child_schedules`: the callee capture and the call step of `f(...)`), and `emit_continued_expr` took that schedule's path, which delivers the child's *whole parent* (`$tt_v1 = $tt_v2($tt_v1)`, with a second `let $tt_v1`). The head's delivery then printed `f($tt_v1)` again: two emitters for the bytes of `f(`, which `Rope::flatten`'s preservation check rejects.
- **Alternatives considered**: (a) Stop the child's schedule at the operand so it delivers only itself. That loses the capture of `f` before the match, so `f` would be read after the match's arms ran. (b) Keep the child path and skip the enclosing delivery. That only works for one child and a known parent; `[match, match]`, `f(match) + f(match)`, and a template around the value each need the other children.
- **Decision and rationale**: The operand owns the delivery. `emit_operand` (`src/codegen/core/emitter/source.rs`) collects every value the operand structurally owns (through sequences and template interpolations), runs each value's region behind the captures its schedule steps take, and delivers the operand's source once, with values, conditional operations, and captured inputs replaced (`source_range_with_scheduled_values`). `captured_source` also composes captures of earlier siblings' schedules, so `f(m1) + f(m2)` captures `$tt_f($tt_m1)` rather than re-reading `f`. Pipeline heads and call steps, template values, and expression arm bodies all use it; the child path that delivered its own parent was then unreachable (checked by making it an internal error across the whole suite) and was removed with `source_range_with_nested_schedule`.

### Decision 2: Plan owned values as an owner plans its values

- **Context**: With one emitter, `g() && f(match ...) |> String` emitted the capture of `f` inside `if (cond) { … }` and read it outside, which TypeScript rejects. The same shape in an arm body did this before the task. An owner plans such a value as a conditional operation (TASK-160 decision 17) and rejects one it cannot own whole; owned values skipped both.
- **Alternatives considered**: Rejecting any conditional step in an operand, which refuses `g() && match ...`, a shape owners accept.
- **Decision and rationale**: `EvaluationFile::lowering_plan` groups owned values (and nested `try` regions) by the value that owns them, computes each one's `target_capability` from its bounded schedule, and runs `plan_conditional_operations` over the group. Operations go to `LoweringPlan::nested_operations`; a value that cannot be owned is reported as `match-placement` or `try-placement` at its position, like an owner's. The target emits a nested operation with the owner-level `emit_conditional_operation` and replaces its parent with the join slot. This supersedes TASK-379 Decision 1 (noted at the top of that record), and `a_match_under_a_conditional_operation_in_an_arm_body_is_a_region` now expects the operation's join slot.

### Decision 3: Evaluation order

- **Context**: The task required head before steps and the documented step order.
- **Decision and rationale**: Heads are evaluated into the accumulator before any step; a call step's operand (`make(match ...)`) runs its captures and regions after the head and calls the resulting function with the accumulator, so the callee expression is evaluated after the piped value, as `$tt_ap(head, step)` does. A step that is directly a structured value keeps its slot; a step that contains one uses the operand path instead of reusing the child's slot for the step function. Verified under node by `a_hoisted_value_inside_a_pipeline_operand_runs_once_in_source_order`.

## Work log

- 2026-09-28: Reproduced the eight reported shapes on `43d603f`; found template heads (`SourceOmitted`), multiple values in an arm or head, and `g() && f(match)` in an arm (TypeScript scope error) in the same family.
- 2026-09-28: Added the operand emitter, the sibling-capture composition, and the nested conditional operations; removed the child-delivers-parent path. Fixed a nested pipeline in a head declaring its accumulator twice (`emit_apply_continued` now leaves the declaration to the consumer that assigns to it).
- 2026-09-28: Added `tests/compile/cases_14.rs` (a table of 18 operand shapes in 5 pipeline positions for `match` and `try`, each compiled and analyzed with no diagnostic and exactly one region per value; callee order; a conditional operand; the placement diagnostic) and the node runtime test in `tests/integration/cases_05.rs`; documented the model in `docs/design/program-lowering.md` §7.5.

## Issues and resolutions

### Issue 1: A step's function overwrote the child's typed slot

- **Symptom**: The runtime test failed `tsc`: `Type '(value: number) => number' is not assignable to type 'number'` for `mark("head", 3) |> make(match ...)`.
- **Cause**: The step path reused the child's slot (typed `number` by the contextual pass) to hold the step's function value.
- **Resolution**: A call step that contains a value (rather than being one) uses the operand path and calls the delivered expression.

## Verification

- [x] `cargo fmt --check`: exit 0.
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0.
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: exit 0.
- [x] `./scripts/ci extension`: exit 0.
- [x] The reported CLI reproduction: `ttc -p` and `ttc --check` exit 0.

## Result

Changed `src/evaluation_ir.rs`, `src/evaluation_ir/evaluation.rs`, `src/codegen/core/planning.rs`, `src/codegen/core/mod.rs`, `src/codegen/core/emitter/{mod,source,host,expression,pattern}.rs`, `tests/compile.rs`, `tests/compile/cases_11.rs`, `tests/integration/cases_05.rs`, `docs/design/program-lowering.md`, `docs/tasks/TASK-379-complete-lowering-ownership.md`, `docs/tasks/INDEX.md`, and this record; added `tests/compile/cases_14.rs`.

Follow-ups found and left out of scope:

- `x |> .m(match ...)` (a hoisted value in a member step's tail) still fails `verify-failed`, as before; the projection has no protocol for a tail applied to the piped value, so the method reference cannot be captured before the argument the way `x.m(match ...)` captures it.
- A pipeline accumulator is annotated by the contextual pass with the first value's type, so `match (1) { _ => [1] } |> (p => p.length)` fails `tsc` (`number` is not assignable to `number[]`); this predates the task.
- In `` `${f(try g())}` |> String `` the nested `try` gets no schedule from the projection, so `f` is read after `g()` runs.
