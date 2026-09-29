# TASK-429: Keep missing-arm suggestions free of duplicate bindings across tuple positions

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

For a tuple match over `R { Ok(value: O), Err(error: string) }`, the suggested arm was `(Ok(value), Ok(value), Err(error)) => undefined,`. Pasting it produced `pattern-duplicate-binding`, although docs/ai/tt.md says a hole is reported as a pattern you can paste back.

## Scope

- Included: The binding text of a coverage witness (`src/analysis/usefulness.rs`, `src/analysis/coverage.rs`).
- Excluded: The witness search and the message's unquoted pattern text.

## Decisions

### Decision 1: Allocate binding names per suggested arm

- **Context**: Each tuple position rendered its witness independently, so the same field bound the same name in several positions; a tuple pattern may not bind a name twice.
- **Decision and rationale**: One set of bound names is shared across the positions of a suggested arm. A repeated field keeps its field name and binds an alias (`Ok(value: value_1)`, the documented alias form), allocated with the same `base`, `base_1` rule as `crate::generated_names::allocate`.

## Work log

- 2026-09-27: Reproduced with `--check` and `--check-types`; both paths use the same witness text and now suggest `Ok(value: value_1)`. Added a test that applies the suggested edit and re-analyzes the file (fails before, passes after).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.

## Result

Changed `src/analysis/usefulness.rs`, `src/analysis/coverage.rs`, and `tests/compile/cases_11.rs`.
