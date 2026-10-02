# TASK-627: Keep TypeScript's `unknown` for `try` on an always-failing value, and document it

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-627`

## Purpose

`try` on a value typed `TErr<E>` (`try Result.Err("x")`, `try fail()` with
`fail(): TErr<string>`) is typed `unknown`, so `x + 1` reports TS18046 and
`(try read()) || (try fail())` is `unknown`. The report asked for `never`.
This task establishes where the `unknown` comes from, why the lowering
cannot turn it into `never` without a type assertion, and records the
behaviour and the alternative for users.

## Scope

- Included: the investigation, `docs/ai/tt.md`, and a case file that pins
  the current typing.
- Excluded: any change to the Result ABI, to the standard library's
  types, or to the emitted success test (Decision 1).

## Decisions

### Decision 1: The success of `try` stays typed by TypeScript's narrowing of the ABI test

- **Context**: `try e` lowers to `const $t = e; if (!("value" in $t))
  <exit $t>; … $t.value`, the structural Result ABI (`docs/ai/tt.md`: a
  value is successful exactly when `"value" in result`). TypeScript
  narrows `"value" in $t` (`narrowTypeByInKeyword` in `checker.ts`) by
  filtering the constituents of `$t`'s type when some constituent declares
  `value`: for `TOk<T> | TErr<E>` the success branch is `TOk<T>` and the
  value is `T`. When no constituent declares it, TypeScript 4.9's
  unlisted-property narrowing ("Unlisted Property Narrowing with the `in`
  Operator", TypeScript 4.9 release notes) intersects the type with
  `Record<"value", unknown>`, so for `TErr<E>` the value is `unknown`.
  Checked with the pinned TypeScript 7.1.0-dev.20260826.1: the same test
  in hand-written TypeScript is `unknown`, and even the discriminant form
  `if (t.kind === "Err") return; t` leaves a non-union `TErr<string>`
  un-narrowed (TS2339/TS7053 on reading `value`), since discriminant
  narrowing filters union constituents only.
- **Alternatives considered**: (a) Annotate the temporary with a union
  that declares the success field (`typeof $r | { value: never }`).
  TypeScript narrows a declared union by its initializer
  (`getAssignmentReducedType`), which removes the added constituent
  before the test. (b) Test the discriminant `kind`. It is not part of the
  ABI (a hand-written Result need not carry it), and it does not narrow a
  non-union type either. (c) Narrow through a generic type predicate in
  the runtime module (`r is Extract<R, { value: unknown }>`). `$t` is then
  `never`, and reading `.value` on `never` is TS2339. (d) Have the host
  annotate the success storage `never`. The value written to it is still
  `unknown`, so the write is TS2322. (e) Declare `value?: never` on
  `TErr`. The `in` test then keeps `TErr` in the success branch of every
  `TResult`, whose value becomes `T | undefined`. (f) Write the value as
  `$t.value as never`. It is the only emitted form that types it `never`,
  and it is a type assertion in generated code, which AGENTS.md contract 2
  rules out; it would also hide a real type error if the ABI's promise
  (every failure omits `value`) did not hold for a hand-written value.
  (g) Keep TypeScript's answer and document it.
- **Decision and rationale**: (g). Every assertion-free emitted form keeps
  `unknown` or reports an error, so `never` cannot come from the
  lowering without breaking contract 2. The behaviour is TypeScript's own
  for the ABI test and is now stated in `docs/ai/tt.md`, together with the
  unconditional form (`return Result.Err(e);`) that needs no success
  value.

## Work log

- 2026-09-30: Wrote `tests/cases/compiler/tryAlwaysFailing.tt`
  (`try Result.Err("x")`, a declared `TErr<string>`, a union of two
  `TErr`s, `(try read()) || (try fail())`, and a `result` block) and
  generated its baselines: every success value is `unknown`, with TS18046
  and TS2322 where it is used.
- 2026-09-30: Checked the alternatives of Decision 1 with the pinned
  `tsc` on hand-written TypeScript.
- 2026-09-30: `docs/ai/tt.md`: the typing of `try`'s value under the ABI
  test.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: no compiler behaviour changed; the case file
`tests/cases/compiler/tryAlwaysFailing.tt` pins TypeScript's typing of the
success value so a later change to it is reviewed.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Baseline changes reviewed and committed with the change

## Result

Added `tests/cases/compiler/tryAlwaysFailing.tt` and its baselines and
updated `docs/ai/tt.md`. `try` on a value whose type declares no `value`
stays `unknown`, as TypeScript types the ABI test; `never` would need a
type assertion in emitted code.
