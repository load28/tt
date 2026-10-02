# TASK-545: Test an optional call at the link its chain short-circuits at

> Superseded in part by TASK-573: a call tested at its receiver no longer
> captures the callee and calls it through `.call`; it is written as the
> member call on the tested receiver (Decision 2 below).

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

`o?.m(<tt value>)` returned `undefined` when `o` was present but had no `m`,
where JavaScript throws a `TypeError`. The lowering captured the callee as
`$tt_v1 = ($tt_v2?.m)` and tested `if ($tt_v1 != null)`, which is the
lowering of `o.m?.(x)`. The same happened for `o?.m(try r())` and
`o |> ?.m(match …)`. `docs/ai/tt.md` says a value under `f?.()` lowers the
whole operation, preserving short-circuiting, evaluation order, and `this`;
ECMA-262 §13.3.9.1 skips an optional chain only at a `?.` whose base is
nullish, so `o?.m(x)` is skipped exactly when `o` is.

## Scope

- Included: the optional-call protocol frame (which link of the chain skips
  the call), the conditional facts and the planned optional-call operation,
  both target emitters of an optional-call argument, `docs/ai/tt.md`, and
  compile and runtime regression tests.
- Excluded: lowering an optional chain link by link, which a call skipped
  at a `?.` inside its callee would need (Decision 3).

## Decisions

### Decision 1: The protocol records the link an optional call is skipped at

- **Context**: `visit_opt_call` recorded every SWC `OptCall` as
  `optional: true`, but an `OptCall` is any call inside an optional chain.
  In `o?.m(x)`, SWC parses `OptChainExpr { optional: false, Call(OptCall {
  callee: OptChainExpr { optional: true, Member(o.m) } }) }`: the `?.` is on
  the member link, not on the call.
- **Alternatives considered**: (a) Always test the receiver for a member
  callee. That is wrong for `o.m?.(x)`, which is skipped when `o.m` is
  nullish. (b) Test both the receiver and the callee. That still turns a
  missing method into `undefined`.
- **Decision and rationale**: The frame carries an `OptionalCallTest`
  (`src/program_syntax.rs`), decided from the call's own `OptChainExpr` and
  its callee: `Callee` when the call is written `?.(` (a `?.` inside the
  callee then also yields a nullish callee, so the callee test covers
  `o?.m?.(x)`), `Receiver` when the callee's member access is written
  `?.name` or `?.[key]`, and `Inner` when the chain is skipped at a `?.`
  further inside the callee (`a?.b.m(x)`, `a?.b.m?.(x)`, `f?.()(x)`). The
  facts pass the test to planning, and `PlannedConditionalKind::OptionalCall`
  keeps it.

### Decision 2: A receiver test reads the callee inside the branch

- **Context**: With the test moved to the receiver, the callee captured
  before it kept the type `F | undefined` of `$r?.m`, which `tsc` rejected at
  `$c.call(...)`.
- **Decision and rationale**: Both emitters (`emit_conditional_operation`
  and `emit_scheduled_step`) capture the receiver before the test and the
  callee as the first statement of the branch, where TypeScript narrows the
  receiver. This is also the chain's order: the member is read only when the
  receiver is not nullish, and before the arguments. A missing method then
  throws at the call, after the arguments ran, as in JavaScript.

### Decision 3: A call skipped inside its callee is a placement error

- **Context**: For `a?.b.m(x)`, the receiver `a?.b` is `undefined` both when
  `a` is nullish (skip) and when `a.b` is `undefined` (JavaScript throws
  reading `m`), so no captured input of the call tells the two apart. The
  previous lowering read `$tt_v2.m` on the captured receiver and threw a
  `TypeError` where JavaScript returns `undefined`, and its output failed
  `tsc` (`'$tt_v2' is possibly 'undefined'`).
- **Alternatives considered**: Lowering the whole chain link by link, as
  TypeScript's down-level emit does, needs a capture model for chain bases
  the plan does not have.
- **Decision and rationale**: `target_capability` treats an `Inner` optional
  call as a conditional operation that cannot be owned whole, so it is
  reported as `match-placement` or `try-placement`, the documented answer
  for a host operation that cannot be owned soundly (`docs/ai/tt.md`).

## Work log

- 2026-09-29: Reproduced on `c719cf6`: `missing?.m(match …)`,
  `viaTry(missing)` and `missing |> ?.m(match …)` printed `undefined`;
  `nul?.b.m(match …)` threw `TypeError` and failed `tsc`.
- 2026-09-29: Added `OptionalCallTest`, `optional_call_test`, the frame and
  facts field (`src/program_syntax.rs`, `src/program_syntax/{collector,
  protocol,visit}.rs`), the planned test (`src/evaluation_ir.rs`,
  `src/evaluation_ir/planning.rs`), and the emitters
  (`src/codegen/core/emitter/host.rs`, `src/codegen/core/mod.rs`).
- 2026-09-29: Added `an_optional_call_tests_the_link_its_chain_is_skipped_at`
  (`tests/compile/cases_14.rs`) and
  `runtime_an_optional_member_call_is_skipped_only_when_its_receiver_is_nullish`
  (`tests/integration/cases_05.rs`); updated `docs/ai/tt.md`.

## Issues and resolutions

### Issue 1: The receiver test left the callee typed as possibly undefined

- **Symptom**: `tsc` reported TS2684 and TS18048 at `$tt_v1.call($tt_v2, …)`.
- **Cause**: The callee was captured before the receiver's test.
- **Resolution**: Decision 2.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] Both new tests fail without the change (the callee test for `o?.m`,
  and `undefined` for a present receiver without `m`).

## Result

Changed `src/program_syntax.rs`, `src/program_syntax/collector.rs`,
`src/program_syntax/protocol.rs`, `src/program_syntax/visit.rs`,
`src/evaluation_ir.rs`, `src/evaluation_ir/planning.rs`,
`src/codegen/core/emitter/host.rs`, `src/codegen/core/mod.rs`,
`docs/ai/tt.md`, `tests/compile/cases_14.rs`,
`tests/integration/cases_05.rs`, `docs/tasks/INDEX.md`, and this record.
