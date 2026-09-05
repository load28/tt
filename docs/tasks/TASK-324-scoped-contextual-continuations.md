# TASK-324: Preserve contextual typing across scoped match continuations

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Complete the contextual-typing audit beyond TASK-323's binding-free expression-arm switches.

## Scope

- Included: Guarded matches, payload bindings, block arms, and sibling match arguments.
- Excluded: Type assertions, suppressed diagnostics, or callback boundaries that change propagation or suspension behavior.

## Decisions

### Decision 1: Preserve lexical scope, narrowing, and evaluation order together

- **Context**: Arm selection cannot be separated from guards that narrow types or bindings consumed by the arm. Sibling arm values cannot be delayed past later subjects.
- **Alternatives considered**: Apply TASK-323's selector indiscriminately; preserve the complete contextual continuation within each scoped control-flow path.
- **Decision and rationale**: Require an explicit continuation model and type/runtime regressions before changing these paths. No known failure is suppressed or treated as a successful compilation test.

## Work log

- 2026-09-05: Recorded four reproducible remaining classes during TASK-323. Strict checking of emitted TypeScript reports TS7006 and TS2345.

## Issues and resolutions

### Issue 1: Scoped and sibling matches still lose the target's contextual type

- **Symptom**: The following complete input compiles as tt but its emitted TypeScript fails strict checking.
- **Cause**: An unannotated value slot severs the consuming call's context; scoped binding/narrowing and sibling sequencing prevent the simple selector transform.
- **Resolution**: Pending.

```typescript
declare const flag: boolean;
type Item = { kind: "item"; run: (x: number) => number };
declare function consume(item: Item): void;
declare function pair(first: Item, second: Item): void;
variant State { Ready(value: number), Empty }
declare const state: State;
consume(match (flag) {
  true if flag => ({ kind: "item", run: x => x }),
  _ => ({ kind: "item", run: x => x }),
});
consume(match (state) {
  Ready(value) => ({ kind: "item", run: x => x + value }),
  Empty => ({ kind: "item", run: x => x }),
});
consume(match (flag) {
  true => { return { kind: "item", run: x => x }; },
  false => { return { kind: "item", run: x => x }; },
});
pair(
  match (flag) {
    true => ({ kind: "item", run: x => x }),
    false => ({ kind: "item", run: x => x }),
  },
  match (flag) {
    true => ({ kind: "item", run: x => x }),
    false => ({ kind: "item", run: x => x }),
  },
);
export {};
```

Save as `remaining.tt`, build with `ttc -o out remaining.tt`, and check with
`tsc --strict --noEmit --skipLibCheck out/remaining.ts`.

## Verification

- [x] Reproduced all four classes against TASK-323's output path
- [ ] Strict checking after the continuation repair
- [ ] Runtime checks for guards, bindings, abrupt exits, and sibling order
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## Result

Pending. TASK-323 does not claim these cases are fixed.
