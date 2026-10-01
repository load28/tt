# TASK-724: Report `=>` after a later parenthesized item instead of asserting

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-724`

## Purpose

`(a, b() => 1);` in any file stopped a debug build of ttc with an internal
compiler error (`assertion left == right failed`, `left: 2, right: 1`, at
`vendor/swc_ecma_parser/src/parser/expr.rs:2584`). Release builds were not
affected by the assertion, but read the list wrongly. The round-8 probe
found it.

## Scope

- Included: the vendored parser's recovery for an arrow after a
  parenthesized item (`parse_args_or_pats_inner`), its `TT-PATCH.md` note,
  a fuzz regression input, and a case.
- Excluded: TypeScript's TS1200 ("Line terminator not permitted before
  arrow") for `(a⏎=> 1)`, which the vendored parser accepts silently and
  TypeScript reports on the emitted file, as before.

## Decisions

### Decision 1: The recovery reads the item the `=>` follows, and only a lone identifier

- **Context**: Upstream added the recovery for swc-project/swc#433
  (`(x => x)(1)` was "Expected Comma, got Arrow"): after an item, a `=>`
  makes the item an arrow function's parameter. The code ate the `=>`
  first, then `debug_assert_eq!(items.len(), 1)` and tested `items[0]`;
  it then built the parameters from every item and appended the arrow
  after the items, so even `(a⏎=> 1)` became the list `a, a => 1`.
- **Alternatives considered**: (a) Delete the `debug_assert_eq!`: a
  release build's behaviour, which turns `(a, b() => 1)` into an arrow
  over the parameters `a, b()` (or `(a, b⏎=> 1)` into `(a, b) => 1`
  appended after `a, b`), stays wrong. (b) Require `items.len() == 1`
  before eating the `=>`: correct for the error, but `(a, b⏎=> 1)`, which
  TypeScript reads as `(a, (b => 1))` with TS1200, would then become a
  parse error of ttc's instead of passing through. (c) Test the last item
  (the one the `=>` follows) before eating the `=>`, and replace that item
  by the arrow built from it alone.
- **Decision and rationale**: (c). It is the reading TypeScript's
  `parseParenthesizedExpression` gives (the `=>` belongs to the
  `AssignmentExpression` it follows, ECMA-262 §15.3 `ArrowFunction :
  ArrowParameters [no LineTerminator here] => ConciseBody`, which TypeScript
  parses and then reports TS1200 for). When the item is not a lone
  identifier (`b()`), the `=>` stays in place and the list's own
  `expect!(Comma)` reports it at the `=>`, where TypeScript reports TS1005
  ("')' expected", checked with the pinned `tsc`: `(1,9)`).

## Work log

- 2026-10-01: Reproduced the ICE with `ttc` on `(a, b() => 1);`; read
  the recovery and swc-project/swc#433.
- 2026-10-01: Implemented Decision 1 in
  `vendor/swc_ecma_parser/src/parser/expr.rs`; recorded it in
  `vendor/swc_ecma_parser/TT-PATCH.md`. Checked `(a⏎=> 1)` and
  `(a, b⏎=> 1)` still pass through, and `(b() => 1)` now reports at `=>`.
- 2026-10-01: Added
  `fuzz/regressions/compile_any_bytes/arrow-after-a-later-parenthesized-item.tt`
  and `tests/cases/compiler/anArrowAfterALaterParenthesizedItemIsASyntaxError.tt`
  with its baselines.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/anArrowAfterALaterParenthesizedItemIsASyntaxError.tt`
  and `tests/fuzz_regressions.rs` with
  `fuzz/regressions/compile_any_bytes/arrow-after-a-later-parenthesized-item.tt`
- **Observed failure**: with the parser change reverted, the case runner
  panicked at `vendor/swc_ecma_parser/src/parser/expr.rs:2584:17`
  (`assertion left == right failed`), and `fuzz_regressions` reported
  `compile_any_bytes/arrow-after-a-later-parenthesized-item.tt crashes:
  vendor/swc_ecma_parser/src/parser/expr.rs: assertion left == right failed`.

## Verification

- [x] `cargo test --test fuzz_regressions --test passthrough --test
  swc_arrow_asi --test swc_typescript_grammar_gaps`: pass
- [x] `TTC_REQUIRE_TYPESCRIPT_CASES=1 cargo test --test corpus`: pass
- [x] `TT_CASES=anArrowAfterALaterParenthesized cargo test --test case_baselines`: pass
- [x] The full gate, run once for TASK-719 to TASK-725 (see TASK-725)
- [x] Baseline changes reviewed and committed with the change

## Result

Changed `vendor/swc_ecma_parser/src/parser/expr.rs` and
`vendor/swc_ecma_parser/TT-PATCH.md`; added the fuzz input, the case, and
its baselines. A `=>` after a later parenthesized item is a syntax error
reported at the `=>`, in debug and release builds alike.
