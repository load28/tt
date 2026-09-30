# TASK-623: Widen only fresh literal types in a value's storage

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-623`

## Purpose

With `a, b: "north" | "south"`, `const d = match (o) { A => a, B => b }`
was stored in `let $tt_v0: string`, so `take(d)` for a `take(d: Dir)`
reported TS2345 where the same TypeScript (`const d = c ? a : b`)
type-checks. The same happened for `as const` arms, template `as const`
arms, block arms, `return match` in an unannotated function, and
`n && match (…)`.

## Scope

- Included: how the TypeScript host types each value written to a join
  slot and to asserted storage (`src/typescript/host.mjs`), the design note
  and `docs/ai/tt.md`, and a typed regression test.
- Excluded: a `const` whose value's arms are literal expressions
  (Decision 2), and the `0` TypeScript gives the left part of `n && …`
  (Issue 1, TASK-626).

## Decisions

### Decision 1: Only a fresh literal type widens, as in the checker

- **Context**: The join computed each incoming type as
  `getWidenedType(getBaseTypeOfLiteralType(type))`, which widens every
  literal type. TypeScript widens a value written to a mutable location
  with `getWidenedLiteralType` (`checker.ts`, used by
  `checkExpressionForMutableLocation` and for `let` initializers and
  inferred return types), which widens only a *fresh* literal type, the
  type of a literal expression, and keeps a *regular* one: a declared
  literal type, `as const`, a narrowing (TypeScript 2.1 release notes,
  "Better inference for literal types": "The type of a `const` variable or
  `readonly` property without a type annotation is the type of the
  initializer as-is"; "a fresh literal type widens"). The asserted storage
  of TASK-596 used the same widening where `T` has no literal of the kind.
- **Alternatives considered**: (a) Read freshness from the type the API
  returns. `getTypeAtLocation` answers `getRegularTypeOfExpression` for an
  expression, so every literal it returns is regular; checked with the
  pinned TypeScript (every arm then stayed a literal, and `let w = match …
  { A => "x" … }; w = "z"` failed). (b) Rely on an unannotated storage's
  evolving type. That exists only under `noImplicitAny`, and the storage
  is read through narrowing, not at an assignment. (c) Read freshness where
  the checker keeps it: a literal token is fresh by
  `checkExpression`'s rules; for a name or a property,
  `getTypeOfSymbolAtLocation` answers `getTypeOfExpression`, which keeps
  freshness (`const c = "a"` is fresh, a declared `"a"` is not); for a call,
  the resolved signature's return type keeps it (`id("p")`); and
  parentheses, `satisfies`, `!`, a conditional's branches, a comma's right
  operand, and `&&`/`||`/`??` keep their operands' constituents.
  `Type.getRegularType()` tells a fresh literal from its regular type.
- **Decision and rationale**: (c), in one function (`freshLiterals`) used
  by both the join (`widenedAtMutable`) and asserted storage (`widenedIn`),
  so the two storage kinds widen as TypeScript does. The API builds no
  unions, so a union with a fresh constituent is answered as its widened
  constituents, which join like any incoming types.

### Decision 2: Storage stays a mutable location

- **Context**: `const d = match (o) { A => "x", B => "y" }` has the fresh
  type `"x" | "y"` in TypeScript, and a `const` keeps it.
- **Alternatives considered**: Annotate the storage with the literals when
  its one read initializes a `const`. A written annotation is a regular
  type, and a regular `"x"` is not widened again: `let e = d; e = "z";`,
  valid TypeScript, would then be rejected.
- **Decision and rationale**: Keep widening fresh literals: no written
  annotation carries freshness, and the widened storage never rejects a
  program TypeScript accepts through later mutable uses. Recorded as a
  limitation in `docs/design/contextual-type-materialization.md` and
  `docs/ai/tt.md`.

## Work log

- 2026-09-30: Reproduced with `ttc --check-types`: TS2345 for `d`, `c`,
  `t`, `k`, `f()`, and TS2322 for `n && match` assigned to
  `0 | null | string`.
- 2026-09-30: First version compared `getRegularType()` on the answered
  type; every literal was regular (Decision 1 (a)). Replaced it with
  `freshLiterals`.
- 2026-09-30: `src/typescript/host.mjs`: `freshLiterals`,
  `widenedAtMutable`, the join and asserted-storage paths.
  `docs/design/contextual-type-materialization.md` ("Inferred joins") and
  `docs/ai/tt.md`.
- 2026-09-30: Test `a_values_storage_widens_only_fresh_literal_types`
  (`tests/native/cases_10.rs`, `--check-types` clean): declared, `as const`,
  template `as const`, block, call, `!`, `satisfies`, and returned arms keep
  `Dir`; literal, `const`-of-literal, generic-call, negative-number,
  conditional, enum, and boolean arms widen as `let` does. It reports seven
  TS2345s with the previous host. `cargo test --test native --test
  integration --test snapshot --test cli --test cli_outputs` passes.

## Issues and resolutions

### Issue 1: `n && match …` is still `number | null | string`

- **Symptom**: `const mm: 0 | null | string = n && match (o) { … }` with
  `n: number | null` reports TS2322 (`number`).
- **Cause**: Not literal widening: the left operand's storage is read in
  the branch where `n` is falsy, and TypeScript's narrowing keeps `number`
  there (`NaN` is falsy too), while the checker's `&&` result type takes the
  left part from `extractDefinitelyFalsyTypes`, which maps `number` to `0`.
- **Resolution**: Taken up with the other left-part differences of logical
  operations in TASK-626, which leaves this one open (its Issue 2).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/typescript/host.mjs`, `tests/native/cases_10.rs`,
`docs/design/contextual-type-materialization.md`, `docs/ai/tt.md`,
`docs/tasks/INDEX.md`, and this record. A tt value whose values have
declared, `as const`, or narrowed literal types keeps them in its storage.
