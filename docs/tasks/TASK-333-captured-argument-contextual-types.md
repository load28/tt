# TASK-333: Preserve contextual types through scheduled captures

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Repair the contextual typing lost when the evaluation schedule captures an
authored expression into an unannotated generated binding. This is the
remaining cause behind the sibling and nested-argument failures that
TASK-329 and TASK-332 could not close.

## Scope

- Included: Host expressions the schedule must capture to preserve evaluation
  order — call arguments before a tt value, operator operands, array and
  object elements — and tt values in non-final argument positions, whose join
  slot is the same unannotated binding.
- Excluded: The repaired final-argument completion (TASK-329) and the
  arm-completion proofs (TASK-327, TASK-328).

## Decisions

### Decision 1: Do not synthesize types the author did not write

- **Context**: The obvious repair — annotating the generated binding — would
  require the compiler to name a type it inferred, which the error-layer
  contract assigns to TypeScript, not to ttc.
- **Alternatives considered**: Emit `const $tt_v: <inferred> = …`; cast the
  binding at its use; suppress the resulting diagnostics.
- **Decision and rationale**: Reject all three. A repair must keep the
  authored expression in a position TypeScript already types — or establish,
  through the responsible layer, that the capture is unnecessary because
  evaluating the expression at its authored position is unobservable.

## Work log

- 2026-09-05: Reproduced during TASK-329, and confirmed against the TASK-328
  binary that the behavior predates both tasks.

  ```typescript
  type Item = { kind: "item"; run: (x: number) => number };
  declare function pair(first: Item, second: Item): number;
  variant State { Ready(value: number), Empty }
  declare const state: State;

  const result = pair({ kind: "item", run: x => x }, match (state) {
    Ready(value) => ({ kind: "item", run: x => x + value }),
    Empty => ({ kind: "item", run: x => x }),
  });
  export { result };
  ```

  Build with `ttc -o out repro.tt` and check with
  `tsc --strict --noEmit --skipLibCheck out/repro.ts`. The first argument is
  captured as `const $tt_v2 = ({ kind: "item", run: x => x });`, so its
  callback parameter reports TS7006 and its `kind` widens to `string`,
  reported as TS2345 at the call. Reversing the arguments moves the same
  failure onto the match's own join slot.

## Issues and resolutions

### Issue 1: An unannotated capture erases the authored contextual type

- **Symptom**: TS7006 on unannotated callback parameters and TS2345 on
  literal-typed properties, for any host expression the schedule captures in
  a contextually typed position.
- **Cause**: The capture is a generated `const` with no annotation, so
  TypeScript infers it in isolation instead of against the parameter type the
  authored position supplied.
- **Resolution**: Pending. Two directions to evaluate before choosing:
  extending the effect model so provably unobservable expressions (an object
  or array literal whose members are themselves inert) stay at their authored
  position, and modelling a multi-value completion that delivers every
  participating value inside the authored call.

## Verification

- [ ] Reproductions and native TypeScript oracles per captured position
  (call argument, operand, array and object element, non-final tt value)
- [ ] Evaluation order, count, and identity preserved for every relocated or
  retained capture, including effect-bearing siblings
- [ ] Mixed `.tt`/`.ttx`/`.ts`/`.tsx` fixtures and reviewed whole-output
  snapshots
- [ ] Live `.tt`/`.ttx` editor diagnostics, hover, completion, and
  invalid-member source ranges
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Pending. Split from [TASK-329](./TASK-329-scoped-sibling-composition.md);
shares its root cause with
[TASK-332](./TASK-332-wrapped-argument-contextual-values.md).
