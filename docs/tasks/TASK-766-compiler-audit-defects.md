# TASK-766: Fix defects found by a compiler audit

- **Status**: Complete
- **Started**: 2026-10-05
- **Completed**: 2026-10-05
- **Commit**: `35852ff2`

## Purpose

An audit of the compiler's lowering and checks found false `val`
diagnostics, a missed exhaustiveness error, and emitted storage that strict
TypeScript rejects where the hand-written equivalent passes. Fix each in the
layer that owns it.

## Scope

- Included: `val` shadowing by unparenthesized arrow parameters and match
  arm bindings, `val-pass` through TypeScript wrappers, lexical visibility
  of variants for subject identification, storage typing for the right
  operand of `||`/`&&`/`??`, storage typing for hoisted unique-symbol keys,
  and the language reference's statement about nested conditional
  operations.
- Excluded: lowering a value nested in a second conditional operation
  (Decision 6); a later same-named variant still replaces an earlier one
  file-wide in the resolver's name table.

## Decisions

### Decision 1: Arrow and arm bindings open scopes in the `val` walk

- **Context**: `x => { x.a = 1 }` and `Some(value: x) => x.a = 1` reported
  `val-mutation` against an outer `val const x`; the walk opened parameter
  scopes only at `(`.
- **Alternatives considered**: Read arm patterns from tokens (a bare unit
  tag reads as a binding); take arm bindings from the parser AST.
- **Decision and rationale**: An arrow whose parameter is a bare
  identifier opens a scope over its body, as a parenthesized list does. The
  arm bindings come from the parser's patterns (`parser::arm_scopes`, built
  like `pipeline_shapes`), which name exactly the bound identifiers, and
  are in scope over the guard and the body.

### Decision 2: A call argument is a path through the same wrappers as a write

- **Context**: `g(x!)` was judged by `val-pass`, `g((x))` and `g(x as T)`
  were not, while mutation reads a path through every wrapper.
- **Decision and rationale**: A non-plain argument is read by the access
  path definition the mutation check already uses
  (`reference::access_path`), with the plain-path rule's exclusion of a
  computed last step.

### Decision 3: A variant is visible in the block that declares it

- **Context**: `variant Q { X, Y }` inside a function made a later
  top-level `match (v: A) { X, Y }` exhaustive against `Q`, so the build
  accepted a non-exhaustive match.
- **Alternatives considered**: Prefer the earliest or the largest
  candidate; scope candidates lexically.
- **Decision and rationale**: The documented rule selects a *visible*
  variant. A declaration that is not exported is block scoped, as
  TypeScript's `type` and `const` are; the parser records the innermost
  enclosing block, and the resolver and the coverage table consider only
  variants visible at the site.

### Decision 4: A logical right operand is typed by what TypeScript checks

- **Context**: `const b: true = flag || try r()` annotated the right
  operand's storage `true`, a check TypeScript never makes.
- **Alternatives considered**: Never use a contextual type there (loses
  literal typing such as `cond && match ... { => "a" }`); follow the
  checker's rule.
- **Decision and rationale**: typescript-go includes the right operand in
  the result type only when the left can take the branch that evaluates it
  (`hasTypeFacts(leftType, TypeFactsFalsy)` for `||`, `EQUndefinedOrNull`
  for `??` in `internal/checker/checker.go`). In the lowered branch the
  stored left is narrowed; when it is `never`, the use is not a source of
  the storage's contextual type. A left operand whose storage is not
  settled yet defers the decision to a later round.

### Decision 5: A hoisted unique-symbol key keeps its type

- **Context**: `{ *[Symbol.iterator]() {...}, v: match ... }` captured the
  key in `const $tt_v1 = (Symbol.iterator)`, which TypeScript widens to
  `symbol`, so the object lost its iterator member.
- **Decision and rationale**: A `const` widens only a unique symbol among
  its initializer types, so a capture whose initializer is a unique symbol
  is annotated with that type (`typeof Symbol.iterator`). The annotation
  check now resolves a qualified `typeof` name through a value's property
  as TypeScript does when its namespace exports have no such member.

### Decision 6: Nested conditional operations stay unsupported, documented

- **Context**: `a ? 1 : b ? match ... : 2` reports `match-placement`; the
  reference said every conditional operation is lowered as one region.
- **Alternatives considered**: Extend the planner to a tree of conditional
  regions; document the restriction.
- **Decision and rationale**: The planner admits exactly one conditional
  boundary per value (`evaluation_ir/planning.rs`) by design. Extending it
  is a lowering feature, not a fix; the reference now states the rule and
  the rewrite.

## Work log

- 2026-10-05: Reproduced each defect, compared against an unmodified
  build of `main`, read the matching typescript-go checker code, and fixed
  `src/val.rs`, `src/val/checker.rs`, `src/val/reference.rs`,
  `src/parser/{parse,mod,variants}.rs`, `src/ast.rs`, `src/hir/`,
  `src/resolve/mod.rs`, `src/analysis/{patterns,coverage}.rs`,
  `src/lib/{compile,mapped}.rs`, `src/typescript/host.mjs`, and
  `docs/ai/tt.md`.

## Issues and resolutions

### Issue 1: The literal left operand kept its annotation

- **Symptom**: `true || try r()` still failed after the `never` rule.
- **Cause**: The right operand's slot settled in the round where the left
  operand's storage was still unannotated, so the left read as `boolean`.
- **Resolution**: A use whose left operand reads unsettled storage defers
  the slot to a later round.

### Issue 2: `typeof Symbol.iterator` was not accepted as an annotation

- **Symptom**: The node was built but the annotation check rejected it.
- **Cause**: The check resolved `Symbol.iterator` through namespace exports
  only; `Symbol` is a variable whose type has the property.
- **Resolution**: Decision 5.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/anArrowParameterWithoutParenthesesShadowsAnOuterVal.tt`,
  `aMatchArmBindingShadowsAnOuterVal.tt`,
  `aValBindingPassedThroughAWrapperIsJudged.tt`,
  `aVariantDeclaredInAFunctionIsNotVisibleOutsideIt.tt`,
  `aLogicalRightOperandIsTypedByItsResult.tt`,
  `aHoistedComputedSymbolKeyKeepsItsUniqueSymbolType.tt`.
- **Observed failure**: With a build of `main`, `ttc --check` reported 4,
  2, 2, and 1 errors for the first four (expected 1, 1, 3, 2), and
  `ttc --check-types` reported 6 and 1 errors for the last two (expected
  none).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (every suite passed once the API baseline was accepted)
- [x] Baseline changes reviewed and committed with the change: six new
  cases; `tests/baselines/reference/api/ttc.api.txt` records that
  `hir::VariantItem` and `resolve::VariantDef` now hold a crate-private
  scope, so code outside the crate can no longer build them with a struct
  literal

## Result

Six defects fixed and pinned by cases; nested conditional operations are
documented as unsupported (Decision 6).
