# TASK-470: Check exhaustiveness of a bigint literal union

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`ttc --check-types` reported nothing for `export function c(n: 1n | 2n) { return match (n) { 1n => "a" }; }`, while the same match over `1 | 2` or `boolean` reported the missing arm. `docs/ai/tt.md` lists bigint literal patterns and says the typed check covers any finite literal union, and TypeScript has had bigint literal types since 3.2, so a bigint union is one.

## Scope

- Included: The literal probe query (`src/engine/projection.rs`), the literal JSON exchanged with the host (`src/typescript/native.rs`, `src/typescript/host.mjs`), the `Literal::BigInt` documentation (`src/probe.rs`), `docs/ai/tt.md`, and a regression test (`tests/native/cases_03.rs`).
- Excluded: The untyped compile path, which has no types and keeps its runtime guard. The guard's own message is TASK-471.

## Decisions

### Decision 1: Ask about bigint matches instead of skipping them

- **Context**: `projection.rs` dropped every literal probe whose arms covered a `Literal::BigInt`, on the premise, repeated in `probe.rs` and `native.rs`, that no finite literal union TypeScript reports holds a BigInt. The premise is false: `1n | 2n` is a union of two `BigIntLiteral` types. A match with only number arms over `1n | 2` was asked, but the host's `literalValue` returned `undefined` for the bigint constituent, so it too came back unanswered.
- **Alternatives considered**: Answer bigint matches in Rust from the declared annotation. That ignores narrowing and aliases, which is why the literal question is asked of the checker at all.
- **Decision and rationale**: Remove the skip and let the host answer bigint constituents like any other literal. The checker's answer includes narrowing, so `if (n === 1n) return; match (n) { 2n => ... }` stays silent.

### Decision 2: Carry a bigint in JSON as `{ "bigint": "<decimal>" }`

- **Context**: JSON has no bigint. ttc sent a covered bigint as its digit string, which is indistinguishable from a string literal with the same text (`"1"` versus `1n`), and the host's comparison is by JSON text.
- **Alternatives considered**: A string with an `n` suffix (`"1n"`). It still collides with the string literal `"1n"`.
- **Decision and rationale**: Both directions use the object `{ "bigint": "-1" }`, the decimal digits with a leading `-` when negative. `probe.rs` already normalizes every spelling (`0x10n`, `-0n`, `1_0n`) to that form, and the TypeScript API decodes a `BigIntLiteral` type's `value` into a JavaScript `bigint` (`api/sync/api.js`: `BigInt(value)` when `TypeFlags.BigIntLiteral`), whose `toString()` is the same form. The comparison in `missingLiterals` therefore matches spellings by value, and `json_literal` reads the object back into `Literal::BigInt`, which `display_literal` renders as `2n` for the message and the missing-arm edit.

## Work log

- 2026-09-28: Reproduced with a debug `ttc --check-types` on a scratch project: the `1 | 2` match reported `missing 2`, the `1n | 2n` match reported nothing.
- 2026-09-28: Read the TypeScript API (`node_modules/typescript/dist/api/sync/types.d.ts`, `api.js`): `BigIntLiteralType.value` is a `bigint`.
- 2026-09-28: Removed the skip in `src/engine/projection.rs`, changed `literal_json` and `json_literal` in `src/typescript/native.rs` and `literalValue` in `src/typescript/host.mjs`, and corrected the `Literal::BigInt` doc comment in `src/probe.rs`.
- 2026-09-28: Checked the scratch project again: `1n | 2n` reports `missing 2n`, `-1n | 2n | 0x10n` reports `missing -1n, 16n`, `1n | 2` with a `2` arm reports `missing 1n`, and a full `-1n | 2n` match is silent.
- 2026-09-28: Added `bigint_literal_unions_are_checked_for_exhaustiveness` to `tests/native/cases_03.rs` (positive, negative, hexadecimal, mixed number/bigint, fully covered, and narrowed scrutinees, plus the missing-arm help text). Updated `docs/ai/tt.md`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check` (exit 0)
- [x] `cargo clippy --all-targets -- -D warnings` (exit 0)
- [x] `TTC_REQUIRE_TSGO=1 cargo test` (exit 0)

## Result

Changed `src/engine/projection.rs`, `src/probe.rs`, `src/typescript/native.rs`, `src/typescript/host.mjs`, `tests/native/cases_03.rs`, `docs/ai/tt.md`, this record, and `docs/tasks/INDEX.md`. `--check-types` now checks `_`-less matches over bigint literal unions, including negative and mixed number/bigint unions.
