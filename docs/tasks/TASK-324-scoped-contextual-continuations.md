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

### Decision 3: Preserve syntax-proven single-return block values

- **Context**: A block containing only `return expression` has no local bindings or preceding/following effects to move with its value.
- **Alternatives considered**: Treat every block as an expression; identify a return by matching text; emit an immediately invoked callback.
- **Decision and rationale**: Register each projected arm block with its Core `BodyId`. The host AST visitor proves that its `BlockStmt.stmts` contains exactly one `ReturnStmt` with an argument, and carries that body identity on `HostExit` through evaluation planning. Codegen consumes this proof without source-text classification. Keep the value in the contextual host, preserving comments and erasing only the return keyword and optional terminator. Blocks with additional statements, nested tt constructs, bare returns, or scope-sensitive control flow keep their existing lowering.

### Decision 4: Preserve native evaluation in multi-value hosts

- **Context**: Hoisting all match selections runs later subjects before earlier arm values. Hoisting earlier values instead loses their contextual types.
- **Alternatives considered**: Delay arm evaluation across sibling subjects; annotate unknown value slots; preserve each match as an inline conditional expression.
- **Decision and rationale**: Where every value in an owner has expression-compatible arms, retain the native host and its conditional operators. Allocate hygienic subject storage, assigning it at the authored match occurrence. Keep failure behavior with a named throw-only helper that executes no authored callbacks. Omit storage when no pattern or failure path needs the captured subject.

### Decision 5: Carry a scoped invocation completion

- **Context**: Payload/local bindings must remain in their original scopes while callback parameters need the consumer's contextual type.
- **Alternatives considered**: Lift bindings, introduce a callback boundary, copy the complete consumer into arbitrary scopes, or model a restricted invocation completion in the existing continuation protocol.
- **Decision and rationale**: The AST proves a discarded single-argument identifier call; the schedule proves the match occupies the entire argument. Expression or linear-return arms may invoke the captured callee before exiting their dispatch, preserving scope and parameter context. This proof rejects handler/finalizer crossings rather than catching or suppressing resulting errors. The target consumes the original call frame exactly once through explicit source ownership.

## Work log

- 2026-09-05: Continuing the remaining failures. Multi-value hosts need inline subject/branch evaluation rather than pre-evaluating every match before the consuming call; developing an explicit multi-value expression plan that preserves the host's native evaluation order and contextual types.
- 2026-09-05: Resumed the remaining block-arm failures after the guarded-dispatch checkpoint. Investigating a syntax-proven single-return block value using the existing host return-statement spans; no lexical binding, side effect, or abrupt completion may be dropped.
- 2026-09-05: Recorded four reproducible remaining classes during TASK-323. Strict checking of emitted TypeScript reports TS7006 and TS2345.
- 2026-09-05: Resumed after confirming TASK-326 is committed and the worktree is clean. Investigating inline conditional dispatch for binding-free guarded expression arms with a terminal unconditional fallback; this retains guard narrowing and contextual arm values in one host expression.
- 2026-09-05: Reproduced strict TypeScript failures for guarded boolean and variant matches across the contextual host matrix, then repaired the conditional-dispatch emission path. The matrix now covers 102 tt/ttx cells, with independent TypeScript oracle expressions.
- 2026-09-05: Added runtime coverage for receiver and argument order, subject/guard evaluation, skipped arms, and thrown guards. Added an emitted-output snapshot and strict unused-local/parameter checking.
- 2026-09-05: Updated the for-of output assertion to permit the conditional expression in the once-evaluated iterable position; subject capture remains before the loop. The ordinary for initializer retains its existing slot path.
- 2026-09-05: Added language-server regressions for saved-file/unsaved-buffer separation, contextual parameter hover, and numeric member completion in both tt and ttx.
- 2026-09-05: Confirmed invalid contextual member accesses produce TS2339 on the original `missing` token in both source kinds.
- 2026-09-05: The first full CI run passed agents, Rust, npm/create-tt, website, and native stages. One existing editor buffer-refresh test timed out while an additional extension rebuild/test ran concurrently; the causal relationship is not established. Re-running the extension stage without concurrent builds before committing.
- 2026-09-05: The isolated `./scripts/ci extension` rerun passed all 123 tests with zero skipped/cancelled tests. No production fix is claimed for the one-off timeout.
- 2026-09-05: Reproduced TS7006/assignability failures for simple and guarded single-return blocks in the contextual matrix before changing emission. The expanded 136-cell matrix now passes, including TypeScript/TSX host contexts and independent TypeScript oracles.
- 2026-09-05: Added runtime regressions for block return evaluation order and excluded statementful blocks with local bindings/effects/finally; added a complete emitted snapshot with retained comments. Extended the tt/ttx editor regression to block callbacks: clean diagnostics, numeric parameter hover/completion, and exact invalid-member ranges all pass.
- 2026-09-05: After the user's structural-fix requirement, replaced the initial codegen-side trivia/span classification with an explicit host-AST statement-list proof linked to `BodyId`. Added 13 AST cases covering comments, optional terminators, ASI, empty statements, effects, declarations, nested blocks/functions, conditionals, and finally. No fallback, type assertion, or diagnostic suppression was introduced.
- 2026-09-05: Rechecked payload bindings, declaration-bearing blocks, and sibling arguments against the repaired compiler. Each still reports TS7006 and TS2345 under strict checking; reproduction sources/output are in `/tmp/tt324-remaining-audit/`.
- 2026-09-05: Repaired all three retained reproducers. Multi-value expression plans keep subjects and arm values at their original argument/operator positions. Scoped call continuations deliver directly to a captured callee while retaining payload/local bindings in their original blocks.
- 2026-09-05: Added 112 sibling family/host cells, runtime checks for receiver/callee/argument order, later-subject mutation, failure short-circuiting, local shadowing, and thrown consumers. Added strict unused-local and generated-name collision checks, plus tt/ttx editor checks for all three repaired forms.
- 2026-09-05: AST proofs restrict scoped invocation to discarded, single-argument identifier calls and expression/linear-return arms without nested structured tt values. Handlers, finalizers, resource disposal, method/optional/type-argument calls, and arbitrary surrounding expressions are not moved across scopes. Existing unsupported contextual paths are not silently asserted or suppressed.

## Issues and resolutions

### Issue 1: Scoped and sibling matches still lose the target's contextual type

- **Symptom**: The following complete input originally compiled as tt but its emitted TypeScript failed strict checking.
- **Cause**: An unannotated value slot severs the consuming call's context; scoped binding/narrowing and sibling sequencing prevent the simple selector transform.
- **Resolution**: All four recorded examples below now pass strict checking. Supported multi-value owners use inline expression plans; supported discarded single-argument calls use scoped invocation continuations. The remaining architectural boundaries are recorded below rather than treated as successful contextual compilation.

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
- [x] Strict checking of the four recorded reproducers after the continuation repairs
- [x] Runtime checks for guards, bindings, abrupt exits, and sibling order
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

### Single-return-block follow-up verification

- [x] Reproduced the block-arm contextual failures before the repair
- [x] Host-AST single-return proof: 13 structural cases
- [x] Contextual matrix: 136 tt/ttx host cells and their TypeScript oracles
- [x] Runtime order, retained scope/effects/finally, and emitted-output snapshot
- [x] Editor suite: 125 passed, zero skipped/cancelled, including both block callback source kinds
- [x] Final AST-based implementation: `./scripts/ci` passed agents, Rust (fmt/clippy/tests/fuzz), npm/create-tt, website, native, and extension in one run
- [x] Payload/declaration-bearing/sibling limitations rechecked without suppressing TS7006 or TS2345

### Scoped/sibling follow-up verification

- [x] Previously failing payload/local-declaration/sibling CLI outputs now pass strict TypeScript checking
- [x] 112 sibling family/host cells, plus scoped-call contextual and runtime regressions
- [x] AST call/linear-completion proofs distinguish nested argument frames, optional/method/generic calls, handlers, finalizers, and disposal
- [x] Generated-name hygiene, strict unused checks, and unexpected-value abrupt completion
- [x] Reviewed whole-output snapshots for scoped invocation and inline siblings
- [x] Editor suite: 127 passed, zero skipped/cancelled; tt/ttx buffer diagnostics, hover, completion, and invalid-member source ranges cover the repaired forms
- [x] `./scripts/ci`: agents, Rust (fmt/clippy/tests/fuzz), npm/create-tt, website, native, and extension all passed in one run

## Result

Outstanding work is tracked separately as Pending: [TASK-327](./TASK-327-scoped-host-continuations.md) for broader host continuations, [TASK-328](./TASK-328-control-flow-contextual-arms.md) for control-flow/cleanup-bearing arms, [TASK-329](./TASK-329-scoped-sibling-composition.md) for scoped sibling/nested composition, and [TASK-330](./TASK-330-editor-refresh-test-timeout.md) for the unresolved test-timeout observation. The compiler follow-ups distinguish unvalidated forms from individually reproduced failures; the timeout follow-up does not assert a production defect. These records were added on 2026-09-05 without further compiler changes.

In progress. The recorded guarded, payload-binding, local-declaration, and sibling failures are repaired. Arbitrary scope-sensitive expression continuations (including handler/finalizer-bearing arms and scoped values inside larger argument/object expressions) remain outside the validated forms; this is not a claim that every contextual program compiles.

Changed files in this partial repair: `src/codegen/core/planning.rs`, `src/codegen/core/emitter/{host,pattern}.rs`, `tests/integration/contextual.rs`, `tests/compile/cases_01.rs`, `tests/fixtures/emit/contextual-guarded-match/{input.tt,expected.ts}`, `editors/vscode/server/src/test/engine.test.ts`, and this task record/index.

The single-return-block follow-up additionally updates `src/program_syntax.rs`, `src/program_syntax/{collector,projection,visit,tests}.rs`, `tests/fixtures/emit/contextual-return-block/{input.tt,expected.ts}`, and `docs/design/mixed-source-composition-matrix.md`; it does not modify `host.rs`, `cases_01.rs`, or the task index.

The scoped/sibling follow-up additionally updates the evaluation schedule/name allocation, target continuation protocol, whole-call source ownership, `tests/compile.rs`, `tests/compile/cases_09.rs`, and the `contextual-scoped-call` / `contextual-sibling-match` output fixtures.
