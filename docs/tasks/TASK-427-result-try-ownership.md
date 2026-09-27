# TASK-427: Decide `result` ownership of `try` from the parsed construct boundaries

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

Two defects in how a `result` block decides which `try` it owns (docs/ai/tt.md, `result` block):

- A `try` in a concise-arrow body inside the block (`const f = (n) => Result.Ok(try get(n))`) targeted the block, emitting `break $tt_v0;` inside the arrow ("A 'break' statement can only jump to a label"), although "a `try` in a nested user function still targets that function".
- A block whose only `try` sits in a match arm was not claimed, so the user got ``source-not-typescript`` at the `{` instead of the specified `try-crosses-value-region`.

## Scope

- Included: The function-depth question `nearest_result_try_spans` asks (`src/parser/results.rs`, `src/flow/syntax.rs`).
- Excluded: The crossing rule itself and `try` placement.

## Decisions

### Decision 1: Separate tt delimiters from user functions with the parse the block already has

- **Context**: The depth came from `function_depth_at`, a token-level count of braced function bodies. It did not count concise arrow bodies, and it read `match (o) {` as a function body (`ident(...) {`) and an arm's `=> {` as an arrow body. The token stream alone cannot tell an arm's `=>` from an arrow function's.
- **Alternatives considered**: Adding concise arrows to `function_depth_at` would also count every arm arrow as a function.
- **Decision and rationale**: The speculative body parse knows every match. Its body braces and arm arrows are collected as tt-owned tokens, and `user_function_depth_at` counts braced function bodies and concise arrow bodies except those delimiters. A `try` is the block's when its user-function depth equals the block's.

## Work log

- 2026-09-27: Reproduced both cases. Added `user_function_depth_at` and the tt-owned token collection; the arrow's `try` returns from the arrow (verified by running the output) and the arm case reports `try-crosses-value-region`. Added two compile tests (both fail before).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.

## Result

Changed `src/flow/syntax.rs`, `src/flow/mod.rs`, `src/parser/results.rs`, and `tests/compile/cases_11.rs`.
