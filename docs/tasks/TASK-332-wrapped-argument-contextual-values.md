# TASK-332: Preserve contextual typing for matches inside larger argument expressions

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-332: complete calls from arms through literal argument frames`

## Purpose

Close the host form TASK-327 reproduced but did not repair: a scoped match
nested inside a larger argument, object, or array expression, such as
`consume({item: match …})`.

## Scope

- Included: Whole-value positions inside object literals, array literals, and
  multi-argument calls whose surrounding expression must stay at its authored
  evaluation point.
- Excluded: Whole-argument calls (repaired in TASK-327), control-flow-bearing
  arms (TASK-328), and sibling composition (TASK-329).

## Decisions

### Decision 1: Do not duplicate authored source across dispatch arms

- **Context**: Completing the consumer from an arm would have to re-emit the
  containing literal per arm, duplicating authored text (only one copy can own
  the source mapping) or reordering sibling property evaluation.
- **Alternatives considered**: Textual duplication, callback boundaries, and
  slot type synthesis; each violates an existing contract.
- **Decision and rationale**: Require an owner-level structural model (or a
  typed-backend contextual-type query) before changing emission. Record exact
  diagnostics first.

### Decision 2 (supersedes Decision 1): Re-emit the literal frame per arm

- **Context**: With the diagnostics recorded, the alternatives were measured
  against the contracts rather than assumed. Annotating the join slot needs a
  type the compiler would have to invent, and a type the *backend* answers
  would make the emitted output depend on whether TypeScript is installed.
  Deferring the arm values into the authored position — the mechanism that
  already handles nested positions — cannot carry pattern bindings: the only
  places to introduce the authored binding name are an arrow parameter (whose
  own type is then lost) or a hoisted `let` (which leaks the name into the
  enclosing scope and makes it `any`). Nothing else reaches the arm.
- **Alternatives considered**: All three of the above, each rejected on a
  contract; and leaving the form unrepaired, which leaves TS7006/TS2345 on
  correct programs.
- **Decision and rationale**: The remaining route is the one TASK-327 already
  uses for a whole argument — the arm performs the call — extended to carry
  the literal around the value. Decision 1's two objections do not survive
  contact with the code: a source map is a many-to-one mapping, so N copies
  of one authored range are legal and, verified with `--check-types`, a type
  error inside a re-emitted frame is still reported at the byte it was
  written at; and evaluation order is decided by the schedule the compiler
  already computes, not by the duplication. What Decision 1 was right about
  is that the emission must be *proven*, which is what Decisions 3 and 4 do.

### Decision 3: Only whole literal positions may be re-emitted

- **Context**: `consume(match … as number)` and `consume(1 + match …)` also
  have an argument wider than the value. Re-emitting ` as number` around an
  arm's value rebinds the cast to that value: `(x) => x + value as number`
  is not `((x) => x + value) as number`.
- **Alternatives considered**: Parenthesise the arm value (changes the
  authored program's meaning in the general case and hides the real rule);
  test the frame text for a leading `{`/`[` (a string-shaped branch, which
  the repository's third contract forbids).
- **Decision and rationale**: `program_syntax` walks the frames between the
  argument and the value and proves that each one is an object or array
  literal whose position holds the value exactly (`literal_positions`). A
  literal position holds one complete expression, so substituting an arm's
  value leaves the rest of the literal meaning what it meant. Anything else
  between them ends the walk.

### Decision 4: The frame moves only when it observes nothing before the value

- **Context**: The literal moves into the arms, so its own parts run after
  the scrutinee instead of before it.
- **Alternatives considered**: Capture the earlier positions into slots and
  re-emit the literal with slot names (the property names and punctuation
  would then be compiler-synthesised text, not pass-through).
- **Decision and rationale**: Target planning requires every step below the
  call to be an object/array position whose inputs are all `Stable` — the
  schedule's own word for "proven inert, capture elided". A spread is
  refused separately (`spread_free`): it copies its operand at the literal's
  position, running that operand's getters, and the positions list records
  only the operand's effects. Positions *after* the value need no rule: the
  arms re-emit them after the value, which is where they were written.

### Decision 5: Framed completions stay on the expression-arm path

- **Context**: A block arm rewrites each `return` through
  `assignment_prefix`, which builds a `String` and so cannot carry authored
  bytes with their source mapping. Emitting the frame there as literal text
  would put a user's type error on unmapped output — the mislocated
  diagnostic TASK-333 fixed.
- **Alternatives considered**: Make the exit prefix rope-valued everywhere
  (a refactor across `expression.rs`, `result.rs`, and `pattern.rs` for a
  form that can be excluded by proof instead).
- **Decision and rationale**: `all_arms_are_expressions` gates the framed
  form, and `assignment_prefix` reports a compiler bug if a framed
  continuation ever reaches it. Block arms in a literal keep their join slot.

## Work log

- 2026-09-05: Reproduced during TASK-327: `consume({item: match (state) {
  Ready(value) => ({kind: "item", run: x => x + value}), Empty => … }});`
  emits the match through an unannotated join slot inside the literal and
  strict checking reports TS7006 on the callback parameter and TS2345 on the
  argument. Semantics are correct; only contextual typing is lost. No
  compiler change was made.
- 2026-09-05: Re-measured the alternatives (Decision 2). Confirmed with
  `ttc -p` that the deferred-arm mechanism already places arm values in a
  nested authored position and refuses only patterns with bindings, and that
  no binding-carrying form of it survives the type contract.
- 2026-09-05: `src/program_syntax/protocol.rs` — the completion proof now
  accepts an argument that *contains* the value, records the argument's
  extent, and proves the literal-position chain. `spread_free` added to the
  ordered frame in `collector.rs`/`visit.rs`.
- 2026-09-05: `src/codegen/core/planning.rs` — `scoped_call_completion`
  locates the call step among the value's steps, proves the steps below it,
  and computes the head/tail spans. `src/codegen/core/emitter/` — the invoke
  continuation carries them and `emit_value_delivery_control` pushes them as
  source.
- 2026-09-05: Verified with `ttc --check-types` that a type error inside a
  re-emitted frame (`consume({ kind: 42, run: match … })`) is reported once,
  at the authored `kind` — the mapping survives the duplication.

## Issues and resolutions

### Issue 1: A join slot inside a literal severs the consumer's context

- **Symptom**: TS7006/TS2345 for object/array-wrapped scoped matches under
  strict checking.
- **Cause**: The value's slot is an unannotated `let`; the literal around it
  keeps its authored position, so TypeScript cannot propagate the parameter
  context into the arm values.
- **Resolution**: Decisions 2–5. `consume({kind: "item", run: match (state) {
  Ready(value) => x => x + value, Empty => x => x }})` now emits
  `$tt_v2({ kind: "item", run: x => x + value });` inside the `Ready` arm and
  type-checks clean.

### Issue 2: A cast inside the literal position parsed differently once moved

- **Symptom**: `consume({item: match … as number})` would have emitted
  `{item: x => x + value as number}`, binding the cast to the arm's body.
- **Cause**: The first containment proof accepted any argument enclosing the
  value, including one whose innermost frame was not a literal position.
- **Resolution**: Decision 3 — the frame walk requires the value to *be* a
  literal position, so a cast between them ends the chain. Pinned by
  `call_arguments_wider_than_the_match_keep_their_authored_frame`.

## Verification

- [x] Strict reproductions and native oracles per wrapped form across
  `.tt`/`.ttx` — `literal_wrapped_call_arguments_keep_their_context`
- [x] Property/element evaluation order and effect-bearing sibling checks —
  `literal_wrapped_completions_keep_evaluation_order`
- [x] Refusals pinned for a cast, an operator, an effectful earlier position,
  an object spread, an array spread, and a block arm
- [x] Emitted output fixed by `tests/fixtures/emit/contextual-literal-argument`
- [x] A type error inside a re-emitted frame is reported at the authored byte
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Changed files: `src/program_syntax.rs`,
`src/program_syntax/{protocol,collector,visit,tests}.rs`,
`src/codegen/core/planning.rs`,
`src/codegen/core/emitter/{mod,host,pattern}.rs`,
`tests/compile/cases_02.rs`, `tests/integration/contextual.rs`,
`tests/fixtures/emit/contextual-literal-argument/{input.tt,expected.ts}`,
`docs/design/mixed-source-composition-matrix.md`, and this record.

This was the last outstanding form of
[TASK-324](./TASK-324-scoped-contextual-continuations.md), which is closed
with it.
