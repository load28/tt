# TASK-430: Explain let-else placement as the compiler enforces it

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`ttc explain let-else-placement` said a let-else belongs "not to a `match` arm, a `result` block, or another construct's value region", but the compiler accepts a let-else in a statement-bodied `result` block, where its `else` exit completes the block.

## Scope

- Included: The explanation text in `src/diagnostics.rs`.
- Excluded: The placement rule.

## Decisions

### Decision 1: The behavior is the specified one; the explanation was wrong

- **Context**: docs/ai/tt.md gives let-else the same position limits as `try`, and a `try` inside a statement-bodied `result` block exits that block; a `return` there completes the block with `Ok`. The integration test `result_allows_let_else_when_each_else_path_completes_the_result` pins the accepted behavior.
- **Decision and rationale**: The explanation now names only match arms and other value regions as excluded, and says that inside a `result` block the `else` exits complete that block.

## Work log

- 2026-09-27: Corrected the text; added a unit test on the explanation.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib`, `--test cli`, `--test snapshot`

## Result

Changed `src/diagnostics.rs` and `src/diagnostics/tests.rs`.
