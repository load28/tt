# TASK-414: Own an exported `try` declaration as one statement

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`export const a = try f();` at module top level reported the correct `try-placement` and, in addition, `lowering-plan-failed: ... does not parse: Expected '{', got '('` spanning the file's start. docs/ai/tt.md (`try`) specifies one located `try-placement`.

## Scope

- Included: The statement owner of a declaration-form `try` in `src/parser/parse.rs`.
- Excluded: Placement rules and their messages.

## Decisions

### Decision 1: Include declaration modifiers in the statement the construct owns

- **Context**: In ECMA-262 an `ExportDeclaration` (`export VariableStatement`) is one statement. The parser recorded the owner from `const`, so the program-syntax projection replaced `const a = try f();` with a placeholder statement and left `export` in front of it, producing `export ($tt_syntax_expr_0);`, which TypeScript cannot parse.
- **Alternatives considered**: Dropping the resulting `lowering-plan-failed` when a placement error exists would hide the malformed projection instead of fixing it.
- **Decision and rationale**: Contiguous `export`/`declare` modifiers before the declaration keyword belong to the owner span. A declaration-form `try` with `export` is only possible at module or namespace top level, where it is a placement error, so no valid lowering changes.

## Work log

- 2026-09-27: Reproduced; extended the owner span; added a compile test for module and namespace forms (fails before, passes after).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.

## Result

Changed `src/parser/parse.rs` and `tests/compile/cases_11.rs`.
