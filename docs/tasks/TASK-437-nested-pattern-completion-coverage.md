# TASK-437: Stop marking a case covered in completion when only a nested pattern handles it

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

TASK-419 stopped `ttCompletions` from marking a case covered by a guarded arm. An arm with a nested pattern (`Has(o: Some(value)) => ...`) still marked `Has` covered, although it matches only part of the case; the coverage analysis counts a row as covering only when it is unguarded and nested-free.

## Scope

- Included: `arm_tags` in `src/engine/completions.rs`.
- Excluded: Exhaustiveness analysis, which already descends into payloads.

## Decisions

### Decision 1: Recognise a nested pattern by its grammar

- **Context**: TASK-419 recorded that detecting nesting needs the declaration table. docs/ai/tt.md fixes the pattern grammar instead: inside a case's parentheses, `field: Tag(...)` is a nested pattern (a unit case needs `Tag()`), while `field: name` is an alias.
- **Decision and rationale**: While scanning a pattern, `:` followed by an identifier and `(` inside the case's parentheses marks the arm as nested; such an arm still identifies the variant but does not count as covering its case.

## Work log

- 2026-09-27: Added the grammar check and a unit test for a nested arm (not covered) and an alias arm (covered); the test fails without the check.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib`

## Result

Changed `src/engine/completions.rs`.
