# TASK-333: Preserve contextual types through scheduled captures

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-333: leave inert host expressions in their contextual position`

## Purpose

Repair the contextual typing lost when the evaluation schedule captures an
authored expression into an unannotated generated binding.

## Scope

- Included: The effect model's inertness proof, and the one lowering that
  cannot leave an elided input in place — a completed call re-emitted inside
  a dispatch.
- Excluded: The unannotated join slot a tt value itself uses in a position no
  completion covers; that is a property of the value, not of a capture.

## Decisions

### Decision 1: Do not synthesize types the author did not write

- **Context**: The obvious repair — annotating the generated binding — would
  require the compiler to name a type it inferred, which the error-layer
  contract assigns to TypeScript, not to ttc.
- **Alternatives considered**: Emit `const $tt_v: <inferred> = …`; cast the
  binding at its use; suppress the resulting diagnostics.
- **Decision and rationale**: Rejected all three. The repair keeps the
  authored expression in a position TypeScript already types, by proving the
  capture unnecessary rather than by describing its type.

### Decision 2: Extend inertness to literals built from inert parts

- **Context**: The expressions that need a contextual type are exactly the
  ones that have no type of their own — object and array literals, arrows and
  function expressions. Arrows and functions were already inert, with
  "preserves contextual parameter inference" given as part of the reason;
  object and array literals were not, so they were captured and widened.
- **Alternatives considered**: Treat allocation as observable and leave the
  model alone (but function creation allocates too and is already inert);
  special-case object literals only in call arguments.
- **Decision and rationale**: Defining a property does not call a setter and
  defining an accessor or method does not run its body, so an object literal
  is exactly as observable as its computed keys and property values; an array
  literal, as its elements. A spread iterates or copies its operand and
  shorthand reads a binding, so both stay observable. The allocation itself is
  not observable here because eliding a capture changes only *where* the
  expression is evaluated, never how often, and nothing holds the value to
  compare identities against.

### Decision 3: A completed call captures what it cannot leave in place

- **Context**: A call completion re-emits the call inside the match's
  dispatch, where the authored position of an elided input no longer exists.
  The first implementation copied the input's source text into the invoke
  prefix, so it was repeated once per arm and carried no source mapping.
- **Alternatives considered**: Copy the source into each arm with mapping
  (repeats the input's diagnostics once per arm); refuse the completion
  whenever an input was elided (loses the repair for every call with a
  literal argument); disable elision for any schedule whose syntax admits a
  completion (regressed `callbackFirst(x => x + 1, match …)`, because the
  facts exist even where lowering chooses the deferred-arm plan instead).
- **Decision and rationale**: A schedule that carries completion facts also
  reserves a generated name for each elided input. The elision itself still
  stands, so every other lowering is unchanged; the completion binds the
  reserved name once, from mapped source, before the dispatch and calls
  through it. Authored source is never repeated and never loses its mapping.

## Work log

- 2026-09-05: Reproduced during TASK-329 and confirmed against the TASK-328
  binary that the widening predates both tasks.
- 2026-09-05: Extended `expression_effects` to object and array literals
  built from inert parts, with a 20-case classification test. Existing
  snapshots stayed byte-identical, so no previously covered case relied on
  those captures.
- 2026-09-05: Found that a completed call inlined its elided inputs as
  unmapped text. Verified the consequence with `ttc --check-types`: a type
  error inside such an argument was reported on the *match*, 23 columns
  away, and re-explained as `ts2339: match on a tag pattern needs a value
  with a kind discriminant` — a TypeScript error mislabelled as a tt one,
  which the error-layer contract forbids. This predates this task: TASK-329
  already inlined inert arguments.
- 2026-09-05: First fix attempt disabled elision whenever completion facts
  existed; that regressed `composed_match_values_preserve_typescript_contextual_typing`,
  because the facts also exist for owners where lowering uses the
  deferred-arm plan and the arrow argument must stay in place. Replaced with
  the reservation in Decision 3.
- 2026-09-05: Added the classification test, a compile test covering elided,
  captured and completed hosts, a runtime test for order/identity/binding
  capture across all three, a typing test for elided literal positions, and
  extended the sibling snapshot with an elided-literal case.

## Issues and resolutions

### Issue 1: An unannotated capture erased the authored contextual type

- **Symptom**: TS7006 on unannotated callback parameters and TS2345/TS2322
  on literal-typed properties, for any inert host expression the schedule
  captured in a contextually typed position — an array element, an object
  property, an operand, a JSX attribute, or an earlier call argument.
- **Cause**: Object and array literals were classified as observable, so the
  schedule captured them into unannotated `const` bindings and TypeScript
  inferred them in isolation.
- **Resolution**: Decision 2. Verified against the TASK-328 binary: the array
  and object-argument reproductions lose their TS7006 and the literal's
  authored position is restored.

### Issue 2: A completed call copied authored source into every arm

- **Symptom**: A type error inside an elided argument of a completed call was
  reported at the wrong column and under a tt rule name.
- **Cause**: The invoke prefix inlined the input's source text with no
  mapping, once per arm.
- **Resolution**: Decision 3. Re-verified with `ttc --check-types`: the same
  program now reports `ts7006` on the argument's own parameter and `ts2345`
  on the match, each at its authored position.

### Issue 3: A tt value's own join slot is still unannotated

- **Symptom**: A match in a position no completion covers — a non-final call
  argument, an array element, an object property — still reports TS7006 on
  its arm callbacks.
- **Cause**: The value crosses the Core/TypeScript boundary through a
  generated `let` with no annotation. That is a property of the value's
  lowering, not of a capture, so it is outside this task.
- **Resolution**: Not addressed here. It remains the residue behind
  [TASK-332](./TASK-332-wrapped-argument-contextual-values.md).

## Verification

- [x] Reproductions and native TypeScript oracles per captured position
  (`inert_literal_arguments_keep_their_contextual_position`)
- [x] Evaluation order, count and identity preserved for elided and captured
  inputs, including a closure over a binding the arm mutates and an
  effectful sibling
  (`elided_and_captured_inputs_preserve_order_identity_and_bindings`)
- [x] Effect classification pinned over 20 shapes, including spreads,
  shorthand, computed keys, accessors and nesting
- [x] Emitted shapes pinned for elided, captured and completed hosts
  (`inert_arguments_are_not_captured_out_of_their_contextual_position`)
- [x] Reviewed the `contextual-sibling-completion` snapshot; it type-checks
  under the pinned TypeScript, and every other fixture is byte-identical
- [x] Diagnostic mapping re-verified through `ttc --check-types`
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Changed files: `src/program_syntax.rs`, `src/program_syntax/tests.rs`,
`src/evaluation_ir.rs`, `src/evaluation_ir/{planning,evaluation,tests}.rs`,
`src/codegen/core/planning.rs`, `src/codegen/core/emitter/host.rs`,
`tests/compile/cases_02.rs`, `tests/integration/contextual.rs`,
`tests/fixtures/emit/contextual-sibling-completion/{input.tt,expected.ts}`,
`docs/design/mixed-source-composition-matrix.md`, and the task records.
Split from [TASK-329](./TASK-329-scoped-sibling-composition.md); the
remaining join-slot residue stays with
[TASK-332](./TASK-332-wrapped-argument-contextual-values.md).
