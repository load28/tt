# TASK-573: Keep a method call around a value a member call on its receiver

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-30
- **Commit**: (see the work log)

## Purpose

A method call whose argument holds a tt value lost TypeScript facts about
the call. `obj?.id(match (k) { A => obj.name, B => "b" })` with
`id<T>(x: T): T` emitted `const $tt_v1 = ($tt_v2?.id); … $tt_v1.call($tt_v2, …)`:
`.call` instantiates `T` as `unknown` (TS2322), and the null test on the
capture `$tt_v2` no longer narrowed `obj` in the arguments (TS18048).
`r.first(match …)` on `first<T>(this: Repo, fallback: T): string | T`
emitted `$f.bind(r)(…)`, and `OmitThisParameter` erases the generic
signature (TS2322 `unknown`). TASK-555 had rejected `.call` for plain calls,
but the optional-call path (TASK-545) still used it, and `bind` fails for a
generic method with a `this` parameter.

## Scope

- Included: the reference parts of a member callee in the protocol (receiver
  and computed key, and which of them are read again at the call), their
  planning, every emission and reading of a member callee (the source walk,
  scheduled and nested readings, consumed-call completions, optional-call
  operations, tagged templates), updated output and order assertions, the
  `contextual-consumed-call` fixture, `docs/ai/tt.md`,
  `docs/design/program-lowering.md` §7.8, notes on TASK-545 and TASK-555,
  and regression tests.
- Excluded: an optional call tested at its callee (`o.m?.(x)`,
  `obj?.cb?.(x)`), whose test is on the method's value. It still captures
  the method, tests it, and calls it through `.call(receiver, …)`, so it
  still loses a generic method's inference, and a receiver whose `?.` sits
  inside the tested callee is not narrowed in the arguments
  (`obj?.cb?.(match (obj.name) …)` reports TS18048). Narrowing it needs the
  chain lowered link by link (test the receiver, then read and test the
  method), which TASK-545 Decision 3 already names as a missing capture
  model; left as a follow-up.

## Decisions

### Decision 1: The call reads its method through the receiver where the call is made

- **Context**: A captured method cannot be called with TypeScript's typing
  of the authored call. `f.call(r, …)` instantiates type parameters with
  `unknown`; `f.bind(r)` keeps a generic signature only when the method
  declares no `this` parameter; and a captured receiver is a different
  reference from the one TypeScript narrows.
- **Alternatives considered**: (a) Keep capturing the method and pick
  `bind` or `call` per method. That depends on type facts plain `ttc` does
  not have, and neither form types every method. (b) Capture the method
  for its `GetValue` and still write `r.m(…)` at the call. That reads the
  member twice. (c) Evaluate only the reference's parts before the
  arguments and let the call read the member.
- **Decision and rationale**: (c). A `MemberReference` input carries its
  receiver and computed key (`HostReferencePart`, `PlannedReceiver`), each
  captured unless inert or an authored identifier or `this`
  (`read_at_call`, the same simple-copiable rule TASK-522 Decision 3 uses for
  assignment targets). The call is written as the author's member call with
  the captured parts replaced by their slots (`member_callee`): `r.first($v)`,
  `$r.m($v)`, `o[$k]($v)`, `obj?.id($v)`, and a consumed call completes in
  each arm as `api.consume(…)`. An optional call tested at its receiver
  tests the receiver as the call reads it (`if (obj != null)`), so the
  arguments see the narrowed receiver.
- **Trade-off**: ECMA-262 `EvaluateCall` performs the member's `GetValue`
  before `ArgumentListEvaluation`. The member is now read after the
  arguments the prelude evaluates (and before those still written in the
  call), so a getter or Proxy `get` on the method runs later than in
  JavaScript. The receiver, a computed key, and every argument keep their
  order, a missing method still throws after the arguments (as TASK-555
  required), and an identifier receiver is re-read at the call, which only
  differs when the arguments rebind it. TypeScript typing of the authored
  call is kept in every case, which a captured method cannot provide;
  `docs/design/program-lowering.md` §7.8 records the exception to the §2
  invariant. Four runtime order assertions were updated to the new order.

### Decision 2: A callee the chain tests keeps its capture

- **Context**: `o.m?.(x)` is skipped when the method is nullish, so the
  test is on the method's value.
- **Decision and rationale**: `callee_tested_step` keeps the previous
  lowering for that form (capture, test, `.call(receiver, …)`); reading the
  member again at the call would run its getter twice. See Scope.

## Work log

- 2026-09-29: Reproduced `target/probe4-compiler/min/b4.tt`, `p/optn.tt`, and
  `p/bindt.tt` with `ttc --check-types` (TS2322, TS18048).
- 2026-09-29: Added `HostReferencePart` and the key to
  `HostEvaluationInput` (`src/program_syntax.rs`), the projected parts and
  `simple_copiable`, `authored_span`, `projected_member_reference`
  (`src/program_syntax/collector.rs`, `protocol.rs`, `visit.rs`); planned
  the parts (`src/evaluation_ir.rs`, `src/evaluation_ir/planning.rs`,
  `validation.rs`, `tests.rs`); replaced `bound_callee` with
  `member_callee` and `callee_tested_step` and changed the replacements and
  completions (`src/codegen/core/planning.rs`); changed the captures,
  readings, and optional-call operations (`src/codegen/core/emitter/host.rs`).
- 2026-09-30: The container restarted mid-task; resumed from a saved patch.
- 2026-09-30: Fixed Issue 1; updated assertions in
  `tests/compile/cases_02.rs`, `cases_05.rs`, `cases_11.rs`, `cases_14.rs`,
  `tests/integration.rs`, `tests/integration/cases_05.rs`,
  `tests/integration/contextual.rs`, and the
  `contextual-consumed-call` fixture (`UPDATE_EXPECT=1`; the diff replaces
  `$f.bind($r)(…)` in each arm by `api.consume(…)` and renumbers slots).
- 2026-09-30: Added `runtime_a_method_call_around_a_value_stays_a_member_call`
  (`tests/integration/cases_05.rs`) and
  `a_method_call_around_a_value_keeps_inference_and_narrowing`
  (`tests/native/cases_10.rs`); documented the lowering.

## Issues and resolutions

### Issue 1: A postfix step's method name was not preserved source

- **Symptom**: `x |> .m(match …)` stopped with
  `validate_source_preservation … (SourceOmitted)`.
- **Cause**: Nested and scheduled readings wrote the member callee as
  literal text, so the authored `.m` never reached the target as source.
- **Resolution**: `captured_reading_rope` writes the callee's authored text
  as mapped source and only the captured parts as slots.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] Both new tests fail without the change (TS2322 and TS18048).

## Result

Changed `src/program_syntax.rs`, `src/program_syntax/collector.rs`,
`src/program_syntax/protocol.rs`, `src/program_syntax/visit.rs`,
`src/evaluation_ir.rs`, `src/evaluation_ir/planning.rs`,
`src/evaluation_ir/validation.rs`, `src/evaluation_ir/tests.rs`,
`src/codegen/core/planning.rs`, `src/codegen/core/emitter/host.rs`,
`docs/ai/tt.md`, `docs/design/program-lowering.md`, the TASK-545 and
TASK-555 records, the tests and fixture listed above,
`docs/tasks/INDEX.md`, and this record.
