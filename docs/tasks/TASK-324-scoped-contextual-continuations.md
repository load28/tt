# TASK-324: Preserve contextual typing across scoped match continuations

- **Status**: In progress
- **Started**: 2026-09-05
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

### Decision 2: Inline total, binding-free conditional dispatch

- **Context**: Expression arms without bindings can keep their guards and values in the consuming TypeScript expression when the final arm is an unconditional wildcard.
- **Alternatives considered**: Assign values to an unannotated slot; introduce a callback; move guards away from arm values.
- **Decision and rationale**: Capture the subject once, then emit a conditional expression with each guard next to its arm value. Retain the existing single-value scheduling requirement. Do not allocate an unused selector slot for conditional dispatch. Scoped bindings, block arms, and sibling values remain outside this repair.

## Work log

- 2026-09-05: Recorded four reproducible remaining classes during TASK-323. Strict checking of emitted TypeScript reports TS7006 and TS2345.
- 2026-09-05: Resumed after confirming TASK-326 is committed and the worktree is clean. Investigating inline conditional dispatch for binding-free guarded expression arms with a terminal unconditional fallback; this retains guard narrowing and contextual arm values in one host expression.
- 2026-09-05: Reproduced strict TypeScript failures for guarded boolean and variant matches across the contextual host matrix, then repaired the conditional-dispatch emission path. The matrix now covers 102 tt/ttx cells, with independent TypeScript oracle expressions.
- 2026-09-05: Added runtime coverage for receiver and argument order, subject/guard evaluation, skipped arms, and thrown guards. Added an emitted-output snapshot and strict unused-local/parameter checking.
- 2026-09-05: Updated the for-of output assertion to permit the conditional expression in the once-evaluated iterable position; subject capture remains before the loop. The ordinary for initializer retains its existing slot path.
- 2026-09-05: Added language-server regressions for saved-file/unsaved-buffer separation, contextual parameter hover, and numeric member completion in both tt and ttx.
- 2026-09-05: Confirmed invalid contextual member accesses produce TS2339 on the original `missing` token in both source kinds.
- 2026-09-05: The first full CI run passed agents, Rust, npm/create-tt, website, and native stages. One existing editor buffer-refresh test timed out while an additional extension rebuild/test ran concurrently; the causal relationship is not established. Re-running the extension stage without concurrent builds before committing.
- 2026-09-05: The isolated `./scripts/ci extension` rerun passed all 123 tests with zero skipped/cancelled tests. No production fix is claimed for the one-off timeout.

## Issues and resolutions

### Issue 1: Scoped and sibling matches still lose the target's contextual type

- **Symptom**: The following complete input compiles as tt but its emitted TypeScript fails strict checking.
- **Cause**: An unannotated value slot severs the consuming call's context; scoped binding/narrowing and sibling sequencing prevent the simple selector transform.
- **Resolution**: Partial. The first guarded example below is repaired by inline conditional dispatch. Payload-binding arms, block arms, and sibling match arguments remain reproducible failures; guarded matches without a terminal unconditional wildcard are not covered by this repair.

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
- [x] 102 contextual host cells and their TypeScript oracles
- [x] Guard runtime ordering, short-circuiting, abrupt completion, and strict unused checks
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci rust`, including the fuzz harness build check
- [x] `npm --prefix editors/vscode test`: 123 passed, zero skipped
- [x] Focused editor regression rerun after adding invalid-member range assertions: 2 passed
- [x] Isolated VS Code with the native TypeScript extension: 64/64 mixed-source checks, report at `target/editor-tests/run-tWQJWD/results.json`
- [x] Full local stage coverage: agents/Rust/npm/website/native passed in the full run; extension passed on the isolated rerun

## Result

In progress. The binding-free total conditional-dispatch subset is repaired; the broader scoped-continuation task is not complete.

Changed files in this partial repair: `src/codegen/core/planning.rs`, `src/codegen/core/emitter/{host,pattern}.rs`, `tests/integration/contextual.rs`, `tests/compile/cases_01.rs`, `tests/fixtures/emit/contextual-guarded-match/{input.tt,expected.ts}`, `editors/vscode/server/src/test/engine.test.ts`, and this task record/index.
