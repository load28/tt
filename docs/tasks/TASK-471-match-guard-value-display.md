# TASK-471: Show any unexpected value in a match's runtime guard

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

A `_`-less match compiles to a runtime guard, `throw new Error("tt match: unexpected literal " + JSON.stringify($tt_m))`. `JSON.stringify` throws a `TypeError` for a BigInt (ECMA-262 SerializeJSONProperty, step 10), for a cyclic structure, and for any `toJSON` or getter that throws. A bigint match that met an unexpected value, the case TASK-470 makes checkable, therefore failed with `TypeError: Do not know how to serialize a BigInt` instead of the tt error. The guard must raise its own error for every scrutinee value.

## Scope

- Included: The three guard shapes in codegen (`unexpected_throw` for switch and if-chain matches, and the inline conditional-operator form) in `src/codegen/core/emitter/pattern.rs`, the generated helper in `src/codegen/core/mod.rs`, its reserved name and the `String` host global in `src/evaluation_ir.rs` and `src/evaluation_ir/evaluation.rs`, the fields that carry them (`src/codegen/core/planning.rs`, `src/codegen/core/emitter/mod.rs`), pinned tests and snapshots, `docs/ai/tt.md`, and the two design documents that quote the guard.
- Excluded: A variant switch reads `$tt_m.kind` before it reaches the guard, so a `null` or `undefined` scrutinee fails there with a `TypeError`. That value violates the scrutinee's declared type, and the access is the dispatch, not the guard.

## Decisions

### Decision 1: Format the value in one generated helper

- **Context**: Every guard is an expression concatenated into the message. No expression form can catch the exceptions `JSON.stringify` raises for objects, and objects are what a variant guard reports (`{"kind":"Nope"}`, which runtime tests pin).
- **Alternatives considered**:
  - An inline `typeof` dispatch around `JSON.stringify`. It fixes bigint and symbol, but still throws for a cyclic object or a throwing `toJSON`.
  - A `never`-returning helper that throws the whole error. Replacing `throw` with a call changes the statement forms' control flow as TypeScript reads it, and the `switch` default would no longer end the function's paths by itself.
- **Decision and rationale**: Each guard keeps its `throw new Error("tt match: unexpected <kind> " + ...)` and calls a per-file helper, `$tt_show(value)`, emitted once at the end of the file when a guard uses it, like `$tt_raise`. Its name is reserved by the same generated-name allocation, so it cannot collide with a user binding.

### Decision 2: What the helper shows for each type

- **Context**: Existing messages for strings and objects must not change, and every branch must be free of user code that can throw, per ECMA-262.
- **Alternatives considered**: `String(value)` for everything. It runs `toString`/`valueOf` on objects, which can throw, and prints `[object Object]` for the variant case the tests pin.
- **Decision and rationale**:
  - A string goes through `JSON.stringify`, which consults no `toJSON` for a primitive string (SerializeJSONProperty, step 2 applies only to objects and BigInts) and so cannot throw. The message stays `"zz"`.
  - A bigint is `String(value) + "n"`, the tt literal spelling (`2n`, `-5n`). `String` on a BigInt runs BigInt::toString and no user code.
  - An object or function goes through `JSON.stringify` inside `try`. The helper uses the result when it is a string, and otherwise uses `typeof value` (`object`, `function`). Cyclic structures, throwing `toJSON` methods, getters, and revoked proxies all end there.
  - Any other value (number, boolean, symbol, undefined) is `String(value)`. For a Symbol that is SymbolDescriptiveString (`Symbol(s)`), where `+` concatenation would throw. `String` also shows `NaN` and `Infinity`, which `JSON.stringify` rendered as `null`.
  - The tuple guard concatenates `"[" + $tt_show(a) + "," + $tt_show(b) + "]"`, which keeps the former `[{"kind":"A"},{"kind":"Nope"}]` text for JSON-safe elements.
  - `JSON` and `String` go through `host_global`. `String` joins `Error` and `JSON` in the shadowed-global list, so a module that declares its own `String` gets `globalThis.String`.

### Decision 3: The helper is written one statement per line

- **Context**: `generated_control_flow_uses_statement_lines_and_expanded_blocks` requires generated statements on their own lines. A one-line helper broke that contract.
- **Decision and rationale**: The helper body is emitted in the expanded block layout the rest of the generated code uses.

## Work log

- 2026-09-28: Reproduced with node: a `_`-less literal match with a `1n` arm threw `TypeError: Do not know how to serialize a BigInt` for `2n`. A Symbol printed `undefined`, and a cyclic object threw a `TypeError`.
- 2026-09-28: Found the guard text in `unexpected_throw` and in the inline form in `src/codegen/core/emitter/pattern.rs`. Nothing else emits a guard, and the stdlib's `expect` messages do not format values.
- 2026-09-28: Added `$tt_show` (allocation in `src/evaluation_ir/evaluation.rs`, accessor in `src/evaluation_ir.rs`, fields in `src/codegen/core/planning.rs` and `src/codegen/core/emitter/mod.rs`, and emission in `src/codegen/core/mod.rs`), and routed all three guard shapes through it.
- 2026-09-28: A first version used a one-line `switch`. That broke the `switch (` counts in compile tests and the statement-line layout contract, so the helper became an expanded `if` chain.
- 2026-09-28: Regenerated snapshots with `UPDATE_EXPECT=1 cargo test --test snapshot`. I reviewed the 18 fixture diffs: each replaces `JSON.stringify(x)` with `$tt_show(x)` (the tuple guard with the concatenated form) and appends the helper once. Updated the five compile tests that pinned the old text (`tests/compile/cases_01.rs`, `cases_02.rs`, `cases_06.rs`, `cases_07.rs`).
- 2026-09-28: Type-checked the helper alone with `tsc --strict --noUnusedLocals --noImplicitReturns --noFallthroughCasesInSwitch --exactOptionalPropertyTypes` for es2015, es2022, and esnext targets.
- 2026-09-28: Added `an_unexpected_value_guard_reports_every_scrutinee_type` and `an_unexpected_value_guard_survives_shadowed_globals` to `tests/integration/cases_05.rs`. They cover switch, if-chain, and inline guards and variant and tuple guards, run by node over bigints, a Symbol, a string, numbers including `NaN`, `undefined`, `null`, a boolean, a throwing `toJSON`, a cyclic object, a function, a plain object, and a module that shadows `String` and `JSON`.
- 2026-09-28: Updated the guard line in `docs/design/match-literal-patterns.md` and `docs/design/variant-and-error-layers.md`, and the guard description in `docs/ai/tt.md`.
- 2026-09-28: After merging onto TASK-468, the new `variant-comments` emit fixture predated this guard change. Regenerated it with `UPDATE_EXPECT=1 cargo test --test snapshot`; the reviewed diff only swaps `JSON.stringify($tt_m)` for `$tt_show($tt_m)` and appends the helper.

## Issues and resolutions

### Issue 1: The first helper broke compile tests

- **Symptom**: Tests that count `switch (` in the output, and the statement-line layout test, failed.
- **Cause**: The helper used a `switch (typeof value)` on one line.
- **Resolution**: The helper uses `if` statements, one statement per line.

## Verification

- [x] `cargo fmt --check` (exit 0)
- [x] `cargo clippy --all-targets -- -D warnings` (exit 0)
- [x] `TTC_REQUIRE_TSGO=1 cargo test` (exit 0)

## Result

Changed `src/codegen/core/emitter/pattern.rs`, `src/codegen/core/emitter/mod.rs`, `src/codegen/core/mod.rs`, `src/codegen/core/planning.rs`, `src/evaluation_ir.rs`, `src/evaluation_ir/evaluation.rs`, `tests/compile/cases_01.rs`, `tests/compile/cases_02.rs`, `tests/compile/cases_06.rs`, `tests/compile/cases_07.rs`, `tests/integration/cases_05.rs`, 18 `tests/fixtures/emit/*/expected.ts(x)` snapshots, `docs/ai/tt.md`, `docs/design/match-literal-patterns.md`, `docs/design/variant-and-error-layers.md`, this record, and `docs/tasks/INDEX.md`. A match's runtime guard now raises its own `tt match: unexpected ...` error for any value.
