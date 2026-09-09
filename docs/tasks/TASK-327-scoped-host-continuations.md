# TASK-327: Generalize scoped match host continuations

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-327: generalize scoped call completions`

## Purpose

Close the host-continuation coverage gaps retained by TASK-324. Its scoped
invocation repair covers discarded, single-argument identifier calls, not
arbitrary contextual expression consumers.

## Scope

- Included: Consumed call results, method and optional calls, explicit generic
  arguments.
- Excluded: Control-flow-bearing arm bodies (TASK-328), multiple scoped
  siblings (TASK-329), and scoped matches inside a larger argument, object, or
  array expression (split to TASK-332 after reproduction).

## Decisions

### Decision 1: Model host evaluation and source ownership structurally

- **Context**: The existing call-completion proof intentionally rejects these
  host forms.
- **Alternatives considered**: Broaden textual matching, cast emitted values,
  suppress diagnostics, or introduce match callbacks; these would weaken the
  language and evaluation contracts.
- **Decision and rationale**: Extend the responsible AST/evaluation/
  continuation model only after reproducing each form. Preserve receivers,
  optional short-circuiting, overload selection, generic context, and
  authored-source ownership.

### Decision 2: Prove the whole-argument requirement by span equality

- **Context**: The syntax proof must guarantee the match is the entire call
  argument before an arm may perform the call. The previous planning-side
  check (a single `CallArgument(0)` step) accepted transparent wrappers, so a
  discarded `consume(match … as Item);` silently dropped its authored cast
  from the emitted output — a semantics defect, not only a typing gap.
- **Alternatives considered**: Keep the step-shape check and special-case
  casts; classify wrappers textually in codegen.
- **Decision and rationale**: `CallCompletionFacts` is proved in the projected
  AST: a non-optional call with a source-backed callee and exactly one
  non-spread argument whose span **equals** the value's span. Planning
  additionally ties the facts to the value's innermost evaluation step. A
  wider argument (cast, operator, containing literal) keeps its authored call
  frame and the join slot stands in for the match — fixing the dropped-cast
  defect for the already-shipped discarded path.

### Decision 3: Deliver consumed results through the value's join slot

- **Context**: A discarded completion removes the whole statement; a consumed
  call's result must keep flowing to the authored position.
- **Alternatives considered**: Replace the entire statement for every
  completion (loses the consumer); annotate the slot with a synthesized type
  (the compiler must not write types the author did not).
- **Decision and rationale**: Each arm assigns `slot = callee(value)`; only
  the call expression's frame is claimed, and the unannotated `let` slot
  stands at the authored call position, so TypeScript's evolving-`let`
  inference types the downstream consumer while the argument keeps the
  callee's contextual parameter type.

### Decision 4: Instantiate explicit type arguments once

- **Context**: Explicit type arguments must reach every arm's call without
  duplicating authored type-argument text across arms (only one copy can own
  the source mapping).
- **Alternatives considered**: Re-emit the type arguments per arm (breaks
  single-mapping ownership); reject type-argument calls entirely.
- **Decision and rationale**: Reserve a generated slot during schedule
  planning and bind `const g = callee<TypeArgs>;` once before the dispatch —
  a standard TypeScript instantiation expression with one source-mapped copy —
  then invoke through it. Optional calls with type arguments retain the
  existing rebuilt-call path.

### Decision 5: Complete single-argument optional calls inside the operation

- **Context**: Optional calls lower as whole conditional operations; the
  match value previously joined through an unannotated slot inside the
  non-null branch.
- **Alternatives considered**: Move the operation to the plain-call path
  (loses short-circuit ownership); leave the gap.
- **Decision and rationale**: When the operation's argument list is exactly
  one whole-value match with completable arms and no type arguments, the
  non-null branch dispatches with the same invocation continuation
  (`result = callee(value)`, or `callee.call(receiver, value)` for member
  callees). The null branch still delivers `undefined`, and the subject is
  only evaluated when the callee is non-null, as authored.

## Work log

- 2026-09-05: Reproduced all five recorded host forms against the TASK-324
  compiler with strict checking: consumed results, `api.consume(...)`,
  `consume?.(...)`, `consume<Item>(...)`, and `consume({item: ...})` each
  fail with TS7006/TS2345/TS2322. Also reproduced the dropped-cast defect:
  `consume(match … as Item);` emitted the call without `as Item`.
- 2026-09-05: Replaced the protocol's `discarded_single` flag with
  `CallCompletionFacts` (whole-call span, consumed flag, type arguments)
  proved by argument-span equality in the projected AST; the schedule carries
  it as `PlannedCallCompletion` with an instantiation-slot reservation.
- 2026-09-05: Generalized codegen planning (`CallCompletionPlan`) and the
  emitter's invocation continuation to carry an invoke prefix and an optional
  result slot; consumed completions claim only the call frame and keep the
  join slot at the authored occurrence. Added the optional-call completion in
  the conditional-operation emitter behind the shared arm proof.
- 2026-09-05: Updated the two compile tests that pinned the old consumed
  emission (semantics unchanged, contextual position improved), rewrote the
  syntax-proof unit test around the new facts, and added: a compile test for
  cast/object-wrapped arguments keeping their frames, a 2×40-cell strict
  typing matrix with TypeScript oracles across tt/ttx, runtime suites for
  consumed results and evaluation order, optional-call short-circuiting, and
  generic instantiation, the `contextual-consumed-call` emitted snapshot, and
  editor cases for consumed/method/optional/generic forms.

## Issues and resolutions

### Issue 1: Scoped values had a bounded host-completion implementation

- **Symptom**: Consumed results, method/optional/generic calls were outside
  the validated scoped contextual forms; each reproduced as TS7006/TS2345/
  TS2322 under strict checking of the emitted TypeScript.
- **Cause**: The implemented structural proof accepted only a discarded,
  single-argument identifier call; it did not model the broader completions.
- **Resolution**: Completion facts now cover discarded and consumed calls,
  identifier and member callees, and explicit type arguments; single-argument
  optional calls complete inside their conditional operation. All reproduced
  forms pass strict checking with runtime order preserved.

### Issue 2: A discarded completion dropped a cast around the match

- **Symptom**: `consume(match … as Item);` emitted `callee(value)` per arm
  with the authored `as Item` deleted from the output.
- **Cause**: The old proof checked the argument's step shape, which is
  identical for a transparent cast wrapper, so the claimed call frame
  swallowed the cast text.
- **Resolution**: The span-equality proof (Decision 2) rejects any argument
  wider than the value; such calls keep their authored frame and consume the
  join slot. Pinned by
  `call_arguments_wider_than_the_match_keep_their_authored_frame`.

### Issue 3: Matches inside a larger argument expression remain unrepaired

- **Symptom**: `consume({item: match …})` still reports TS7006/TS2345: the
  value joins through an unannotated slot inside the object literal.
- **Cause**: The completion moves only the call itself into the arms; moving
  a containing literal would duplicate authored source across arms or change
  property-evaluation order, which no current model proves safe.
- **Resolution**: Recorded as [TASK-332](./TASK-332-wrapped-argument-contextual-values.md)
  with the reproduction; the emitted code keeps authored semantics.

## Verification

- [x] Strict TypeScript/TSX reproductions and native TypeScript oracles for
  each host form (4 match families × 10 hosts × tt/ttx, each with an oracle)
- [x] Receiver/getter, optional short-circuit, exception, shadowing, and
  evaluation-order runtime checks
- [x] Reviewed `contextual-consumed-call` whole-output snapshot; existing
  fixtures byte-identical
- [x] Live `.tt`/`.ttx` editor diagnostics, callback hover/completion, and
  invalid-member source ranges for the new forms
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Changed files: `src/program_syntax.rs`, `src/program_syntax/{collector,protocol,visit,tests}.rs`,
`src/evaluation_ir.rs`, `src/evaluation_ir/planning.rs`,
`src/codegen/core/planning.rs`, `src/codegen/core/emitter/{mod,host,pattern}.rs`,
`tests/compile/cases_02.rs`, `tests/integration/contextual.rs`,
`tests/fixtures/emit/contextual-consumed-call/{input.tt,expected.ts}`,
`editors/vscode/server/src/test/engine.test.ts`,
`docs/design/mixed-source-composition-matrix.md`, and the task records.
Follow-ups: [TASK-328](./TASK-328-control-flow-contextual-arms.md),
[TASK-329](./TASK-329-scoped-sibling-composition.md),
[TASK-332](./TASK-332-wrapped-argument-contextual-values.md).
