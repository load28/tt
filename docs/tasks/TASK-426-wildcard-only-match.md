# TASK-426: Dispatch a wildcard-only match without reading its subject

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`match (v) { _ => "any" }` lowered to `switch ($tt_m.kind) { default: ... }`. With `v === null` it threw `TypeError: Cannot read properties of null (reading 'kind')`, and for `v: number` TypeScript reported TS2339 in generated code, although `_` matches every value (docs/ai/tt.md: "`_` works in every family").

## Scope

- Included: Match dispatch selection in `src/core_ir/lower.rs`.
- Excluded: Tag and literal dispatch for matches that test something.

## Decisions

### Decision 1: A match without tag tests switches on the value

- **Context**: Dispatch was `VariantSwitch` unless some arm tested a literal, so a match whose arms test nothing read the `kind` discriminant.
- **Decision and rationale**: A single-subject switch uses `LiteralSwitch` (`switch (value)`) when every arm is `_` or a literal test; `VariantSwitch` needs at least one tag test. A mix of tag and literal arms remains a `match-mixed-patterns` error.

## Work log

- 2026-09-27: Reproduced with Node; changed the dispatch rule; added an integration test over `1`, `null`, and `undefined` subjects (fails before, passes after).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.

## Result

Changed `src/core_ir/lower.rs` and `tests/integration.rs`.
