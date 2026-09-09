# TASK-323: Preserve contextual typing through composed matches

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-323: fix(compiler): preserve composed match type contexts`

## Purpose

Audit strict TypeScript checking of composed match results, beyond output parsing.
Preserve the host expression's contextual typing when lowering match values.

## Scope

- Included: Binding-free expression-arm switch matches with one TT value per host owner; object, array, and call contexts; contextual callbacks and literal types; type/runtime regressions.
- Excluded: Type assertions, diagnostic suppression, and changes to TypeScript inference rules.

## Decisions

### Decision 1: Compare against equivalent TypeScript expressions

- **Context**: A parseable target can lose the type context of its source expression.
- **Alternatives considered**: More parse-only fuzzing; explicit annotations on generated slots; strict checking against a TypeScript conditional-expression oracle.
- **Decision and rationale**: Use TypeScript's own checker on equivalent programs. Context flows through expression structure, so target planning must account for this contract without inventing type-level computations.

### Decision 2: Separate switch selection from contextual value evaluation

- **Context**: Hoisting arm values loses the host's contextual typing. Moving guards or bindings away from their values would also lose narrowing and lexical scope.
- **Alternatives considered**: Contextual helper callbacks; synthesized type annotations; distributing the complete host continuation across every control-flow path.
- **Decision and rationale**: For binding-free expression-arm switches with one TT value in the owner, preserve the scheduled captures and switch dispatch, assign an arm ordinal, and emit each value once in a conditional expression at its authored host. Guards, bindings, block arms, and sibling TT values retain their scoped plans. Their broader context-transfer problem is tracked in TASK-324.

### Decision 3: Model function creation separately from function execution

- **Context**: Capturing a preceding arrow/function expression discards its contextual parameter type.
- **Alternatives considered**: Capture with inferred annotations; retain creation at its authored argument position using the existing effect protocol.
- **Decision and rationale**: Function-expression creation has no observable execution effects; its body and defaults run only when called. Mark both arrow and ordinary function expressions as inert, retaining one evaluation in their original type context.

## Work log

- 2026-09-05: Doctor passed. Reproduced strict-check failures for match results in object properties, array elements, and function arguments. The same directly annotated initializer succeeds.
- 2026-09-05: Added target-plan eligibility and selector/value emission for safe switch matches. Updated three output-shape assertions while preserving capture, spread, and JSX contracts.
- 2026-09-05: Added 68 strict TypeScript/TSX composition cells with matching TypeScript host oracles. Added runtime checks for receiver identity, evaluation order, lazy function defaults/bodies, and unexpected scrutinees, plus narrowing and incompatible-callback regressions.
- 2026-09-05: Added and reviewed the complete emitted-output fixture `tests/fixtures/emit/contextual-composed-match/`.
- 2026-09-05: Reproduced remaining contextual failures for guarded, payload-binding, block-arm, and sibling matches. Recorded the exact shared trigger in TASK-324 rather than extending selection across unproven scope boundaries.
- 2026-09-05: Corrected the test generators to use broad number/string/boolean scrutinee types where multiple literal cases are intended; singleton scrutinees correctly trigger TypeScript's incomparable-case diagnostics.
- 2026-09-05: The selector revision passed a 60-second generated-program fuzz run (22,014 executions). After refining function-creation effects, the final revision passed another 10,000 executions. Counts include generator inputs, not unique valid programs.
- 2026-09-05: Final `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --quiet`, and locked all-target fuzz compilation passed. The integration suite ran 120 tests; the compile suite ran 401. Reviewed the fixture output and `git diff --check`.

## Issues and resolutions

### Issue 1: Match temporaries lose contextual typing

- **Symptom**: TS7006 on callbacks and widened object discriminants causing TS2322/TS2345.
- **Cause**: Composed lowering assigns the arm result to an unannotated temporary before it reaches its contextual host.
- **Resolution**: Preserve arm values in their contextual host for the structurally supported switch class. Remaining scoped cases are tracked in TASK-324.

### Issue 2: Preceding function arguments lose contextual parameter types

- **Symptom**: `callbackFirst(x => x + 1, match (...) { ... })` emits TS7006 on the first callback.
- **Cause**: Effect analysis treated function creation like execution, forcing an untyped capture before the call.
- **Resolution**: Classify arrow/function creation as inert. The existing capture-elision protocol now retains the function's contextual host, while runtime tests keep defaults and bodies lazy.

## Verification

- [x] Strict TypeScript contextual-composition matrix (68 tt/ttx cells plus TypeScript host oracles)
- [x] Runtime evaluation-order, receiver, unexpected-value, and lazy function execution regressions
- [x] Guard narrowing and incompatible-callback regressions
- [x] Emitted-output snapshot, generated and reviewed
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `cargo check --manifest-path fuzz/Cargo.toml --all-targets --locked`
- [x] Final sanitizer-enabled generated-program fuzz run: 10,000 executions

## Result

Repaired contextual arm values for the structurally proven switch class and
contextual function arguments before a match. Changed target planning and
pattern/host/source emission, refined function-creation effects in
`program_syntax`, added contextual integration tests and an emitted fixture,
and updated the composition-matrix documentation and task index. No type
assertions, diagnostic suppression, or callback execution boundaries were
introduced. The compiler's broader contextual-continuation work remains
explicitly open in [TASK-324](./TASK-324-scoped-contextual-continuations.md).
