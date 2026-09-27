# TASK-415: Give numeric literal patterns their ECMAScript values

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

Valid numeric literal patterns were rejected as ``tt `match` could not be parsed``: a radix literal wider than 128 bits (`0x100000000000000000000000000000000`, with or without `n`) and a decimal literal whose value is `Infinity` (`1e400`). docs/ai/tt.md lists these literal forms and compares literal arms by value. Messages also printed numbers in Rust notation (`inf`, `1000000000000000000000`).

## Scope

- Included: `numeric_value` in `src/parser/literals.rs` and the display of number literals in diagnostics.
- Excluded: String and boolean literals.

## Decisions

### Decision 1: Compute the literal's mathematical value without a width limit

- **Context**: ECMA-262 §12.9.3 (NumericValue) defines radix literals of any length; a Number literal's value is the mathematical value rounded to the nearest Number, `+∞` beyond the largest finite value; a BigInt literal's value is exact.
- **Decision and rationale**: Radix digits are converted to an exact decimal string with base-10⁹ limbs. A BigInt keeps that string; a Number parses it with Rust's correctly rounded `f64` parser. Infinity is a value like any other, so `1e400` and `2e400` are duplicate arms.

### Decision 2: Print numbers the way JavaScript does

- **Context**: The literal appears in duplicate-arm and missing-arm messages and suggestions, which must read as JavaScript.
- **Decision and rationale**: `ast::js_number_string` implements Number::toString (ECMA-262 §6.1.6.1.20) over the shortest round-trip digits; both literal displays use it.

## Work log

- 2026-09-27: Reproduced the rejections; implemented both decisions; added a compile test covering wide radix literals, BigInt equality with the decimal spelling, `Infinity`, `1e+21`, and `1e-7` (fails before, passes after).

## Issues and resolutions

None.

## Verification

- [x] `cargo test --test compile numeric_literal_patterns_take_their_ecmascript_values`

## Result

Changed `src/parser/literals.rs`, `src/ast.rs`, `src/engine/semantics/declarations.rs`, and `tests/compile/cases_11.rs`.
