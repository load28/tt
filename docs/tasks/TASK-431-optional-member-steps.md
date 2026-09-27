# TASK-431: Pipe into an optional member as the optional call

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`head() |> o?.m` emitted `$tt_ap(head(), o?.m)`, which calls the method without `this` (`TypeError: Cannot read properties of undefined`), while `1 |> o?.m` emitted `o?.m(1)`. TASK-392 excluded optional chains; docs/ai/tt.md still defines `x |> f` as `f(x)`, so the result depended on the head.

## Scope

- Included: Member-step recognition (`src/program_syntax.rs`) and the member-step emission (`src/codegen/core/emitter/expression.rs`) for value pipelines; `docs/ai/tt.md`.
- Excluded: Optional-chain `flow` steps, which stay function values evaluated at composition (a bound method cannot express the chain's short-circuit).

## Decisions

### Decision 1: Hoist only the object before the first `?.`

- **Context**: `o?.m(x)` short-circuits the whole call when `o` is nullish (ECMA-262 §13.3.9, OptionalChain). Parentheses end an optional chain, so `($tt_r?.m)($tt_v)` would throw instead.
- **Decision and rationale**: For an optional chain, the receiver operand is the object of the leftmost optional link; the rest of the chain stays in the call, written without parentheses: `(($tt_v, $tt_r) => $tt_r?.c?.m($tt_v))(head(), (nested))`. Parentheses around the callee are now written only when its top level is a TypeScript `as`/`satisfies`/type assertion, which is not a left-hand-side expression; for other callees the parentheses were unnecessary.

## Work log

- 2026-09-27: Reproduced; implemented; checked `undefined` short-circuit, nested chains, head-first order, and `this` with Node. Added an integration test (fails before, passes after).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.

## Result

Changed `src/program_syntax.rs`, `src/codegen/core/emitter/expression.rs`, `tests/integration.rs`, and `docs/ai/tt.md`.
