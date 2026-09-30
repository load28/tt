# TASK-664: Keep a binding pattern's implied type off generated storage

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-664`

## Purpose

`const { a = 1 } = { a: try r };` emitted `let $tt_v0: 1 | undefined;`
and `--check-types` reported a false TS2322 (``expected `1 | undefined`,
found `number` ``). The same happened for array patterns
(`const [b = 2] = [try r]`), `match` values, `let`, and nested patterns. The
source is valid TypeScript with `r`'s value in place of the `try`; the
storage annotation came from a type TypeScript never enforces.

## Scope

- Included: the contextual-slot refinement in the TypeScript seam
  (`src/typescript/host.mjs`), `docs/ai/tt.md`, and a runtime case.
- Excluded: annotated declarations (`const { a = 1 }: T = ...`), whose
  type is a real contextual type and a constraint, and parameter or
  binding-element defaults, where tt values are already rejected
  (`match-placement`, `try-placement`).

## Sources

- TypeScript `src/compiler/checker.ts`,
  `getContextualTypeForInitializerExpression`: when the declaration has no
  contextual type of its own and "`!(contextFlags & ContextFlags.SkipBindingPatterns)
  && isBindingPattern(declaration.name)`", the initializer's contextual type
  is `getTypeFromBindingPattern(declaration.name, /*includePatternInType*/
  true, /*reportErrors*/ false)`.
- TypeScript `src/compiler/types.ts`, `ContextFlags.SkipBindingPatterns`:
  "Ignore contextual types applied by binding patterns", which
  `inferTypeArguments` passes when every type parameter has a default,
  because such a type describes the pattern, not what the initializer must
  be.
- TypeScript `src/compiler/checker.ts`, `getWidenedTypeForVariableLikeDeclaration`
  and `checkDeclarationInitializer` (`padObjectLiteralToBindingPattern`,
  `padTupleType`): an unannotated destructuring declaration takes the
  initializer's type, padded with the pattern's defaulted properties; the
  initializer is not checked for assignability to the pattern's type.
- TypeScript `src/compiler/checker.ts`, `getContextualTypeForBinaryOperand`
  and `getContextualTypeForConditionalOperand`: `||` and `??` hand their
  contextual type to both operands, `&&` and `,` to the right one, `? :` to
  both branches.

## Decisions

### Decision 1: A use whose contextual type traces to an unannotated binding pattern supplies no expected type

- **Context**: The refinement annotates storage with the contextual type of
  its uses (`checker.getContextualType`), and the API exposes no
  `ContextFlags`, so it cannot ask for `SkipBindingPatterns` directly.
- **Alternatives considered**: (a) Drop the annotation when TypeScript
  later reports an error on it. A diagnostic-driven fallback, which the
  contract forbids. (b) Annotate with the contextual type widened to the
  pattern's base types. Still a type TypeScript never imposed, and wrong
  for `c = "x"` receiving a template literal type. (c) Recognise the one
  place `getContextualTypeForInitializerExpression` applies the pattern's
  type: the use reaches the initializer of an unannotated declaration whose
  name is a binding pattern through expressions that pass their contextual
  type to the operand (parentheses, object and array literals, property
  assignments, `? :` branches, `||`/`??`, the right of `&&`/`,`). Such a
  use offers no expected type, and the storage is typed as storage without
  a contextual type is: from the join of its values.
- **Decision and rationale**: (c), in the TypeScript seam next to the other
  syntax the refinement reads (`assertionOperand`). An annotated
  declaration, an assertion, or a call on the way is left alone, so every
  real contextual type still reaches the storage.

## Work log

- 2026-09-30: Reproduced with the probe case: seven TS2322 errors under
  `--check-types`, storage `1 | undefined`, `2 | undefined`,
  `"x" | undefined`, `0 | undefined`.
- 2026-09-30: Added `impliedByBindingPattern` to `src/typescript/host.mjs`
  and skipped such uses in the expected-type walk.
- 2026-09-30: Added `tests/cases/compiler/bindingPatternDefaultStorage.tt`
  (object, array, nested, `let`, `match`, `? :`, `&&`, and an annotated
  declaration) with `@run`; updated `docs/ai/tt.md`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/bindingPatternDefaultStorage.tt`
  (`cargo test --test case_baselines`)
- **Observed failure**: The `--check-types` section held 11
  `error[ts2322]: type mismatch: expected \`1 | undefined\`, found
  \`number\`` (and the `2 | undefined`, `"x" | undefined`, `0 | undefined`
  forms), so the case did not compile cleanly and was not run.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`: the case runner here; the full gate over TASK-661 to
  TASK-666 is recorded in TASK-666
- [x] Baseline changes reviewed and committed with the change

## Result

Storage in a destructuring initializer is annotated with the join of its
values (`number`, `string`), the typed check is clean, and the runtime
baseline prints `{"kind":"Ok","value":"7,7,n3,3,7,7,7,7,7,7"}`. Changed:
`src/typescript/host.mjs`, `docs/ai/tt.md`, the case and its baselines.
