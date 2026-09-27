# TASK-407: Describe value-region placement without the retired IIFE lowering

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

The `try-placement` and `let-else-placement` messages for a construct inside an isolated value region said its exit "would exit this construct's own IIFE". `match` and `result` no longer lower to an IIFE (docs/ai/tt.md, `match`: "match never emits an IIFE, callback, or `$tt_expr` helper"); a `return` written there delivers the construct's value. The message described a lowering that does not exist.

## Scope

- Included: The two messages in `src/sema/checker.rs` and their tests.
- Excluded: Placement rules.

## Decisions

### Decision 1: State the effect in terms of the construct's value

- **Context**: The value-region rule is that an exit written in a match arm or `result` block completes that construct's value (tt.md, let-else and `try` sections).
- **Decision and rationale**: The messages now say the exit "would complete this construct's value instead of returning from / leaving the enclosing function".

## Work log

- 2026-09-27: Updated the messages, the stale comment in `try_inside_match_arm_is_an_error`, and added assertions that the messages describe the value and do not mention an IIFE. Both tests fail with the previous messages.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.

## Result

Changed `src/sema/checker.rs`, `tests/compile/cases_03.rs`, and `tests/compile/cases_04.rs`.
