# TASK-446: Let `try` bind to a private-member operand

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`try` binds to the following primary expression, which includes member access. A private member was not recognized as one. `try this.#v;` was left unclaimed and reported as `source-not-typescript` ("Expected '{', got 'this'"), and `const a = try this.#v;` bound only `this`. A private name spelled like a keyword (`this.#case`, `this.#match()`) also stopped the operand scan because the name was read as the keyword.

## Scope

- Included: The primary-expression scanner (`is_primary_expression` in `src/scanner.rs`), which `try` operands and codegen receiver grouping share, and `dotted_at` in `src/parser/cursor.rs`, which every parser scan uses to tell a property name from a keyword or binding.
- Excluded: `#name in obj` (an ergonomic brand check, not a primary expression) keeps its current handling.

## Decisions

### Decision 1: Model the private name as the lexer splits it

- **Context**: ECMA-262 defines `MemberExpression . PrivateIdentifier` and `OptionalChain ?. PrivateIdentifier`, where `PrivateIdentifier :: # IdentifierName` is one token with no gap after `#`. ttc's lexer emits `Punct('#')` followed by an `Ident`.
- **Alternatives considered**: Lexing `#name` as a single token would change every consumer of the token stream, including the passthrough mapper, for no other benefit. Special-casing `this.#` in the `try` scanner would leave the keyword and receiver cases wrong.
- **Decision and rationale**: `is_primary_expression` accepts `#` immediately followed by an identifier as a member name after `.` and `?.` (`member_name_end`). `dotted_at` treats an identifier whose previous token is an adjacent `#` as a property name, so no scan reads a private name as a keyword (`#case`, `#match`, `#result`) or as a binding. Codegen's `push_receiver` uses the same scanner, so `this.#v` is now also recognized as a receiver that needs no grouping parentheses.

## Work log

- 2026-09-27: Reproduced `try this.#v;` failing as `source-not-typescript` and traced it to `scan_primary_operand` ending the operand at `this`.
- 2026-09-27: Added `member_name_end` to the scanner and extended `dotted_at` to private names.
- 2026-09-27: Added `try_binds_to_a_private_member_operand` to `tests/compile/cases_11.rs`, covering the statement and value forms, `?.#`, and keyword-spelled private names. It fails with either change reverted. A companion `val` test was dropped because it passed without the change; `val` never confused a private name with a binding.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/scanner.rs`, `src/parser/cursor.rs`, `tests/compile/cases_11.rs`, and `docs/tasks/INDEX.md`.
