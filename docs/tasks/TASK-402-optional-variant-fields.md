# TASK-402: Emit optional variant fields as absent properties and reject required-after-optional

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

A variant case may declare optional fields (`C(opt?: number)`; the parser records `Field::optional`), and the case type declares them as optional properties. The emitted constructor `(opt?: number): V => ({ kind: "C", opt })` always wrote the property, so it failed TypeScript under `exactOptionalPropertyTypes` (TS2322/TS2375: `number | undefined` is not assignable to `number`) and `"opt" in V.C()` was `true`. A case with a required field after an optional one (`C(opt?: number, req: string)`) emitted a parameter list TypeScript rejects (TS1016), while plain `ttc` and `ttc --check` accepted it. Contract 2 requires generated glue to be plain valid TypeScript and tt-level errors to be reported by ttc.

## Scope

- Included: The non-ambient constructor emitted for cases with optional fields (`src/codegen/core/emitter/helpers.rs`), a new sema rule for a required field after an optional one (`src/sema/checker.rs`, `src/diagnostics.rs`, `src/content_mapper.rs`), and the variant section of docs/ai/tt.md, which did not document optional fields.
- Excluded: The emitted union type and the ambient constructor declaration, which were already correct (`opt?: T` in a type literal and in a function type).

## Decisions

### Decision 1: Omit an optional property whose argument is `undefined`

- **Context**: TypeScript's optional property `opt?: T` means the property may be absent. Under `exactOptionalPropertyTypes` (TypeScript 4.4 release notes; tsconfig reference) the property, when present, must have type `T`, not `T | undefined`. An optional parameter `opt?: T` has type `T | undefined` inside the function, so the shorthand property `opt` has type `T | undefined` and does not satisfy the case type.
- **Alternatives considered**: Declaring the case property as `opt?: T | undefined` would change the user's declared type. A type assertion on the object is a type trick contract 2 rules out. Distinguishing an omitted argument from an explicit `undefined` would need `arguments.length` or a rest parameter, which changes the constructor's signature.
- **Decision and rationale**: The constructor spreads a conditional object: `({ kind: "C", ...(opt === undefined ? {} : { opt }) })`. The object has the property exactly when the argument is not `undefined`, so it is assignable under every strictness flag, and the property is absent when the argument is omitted, which is what `?:` declares. Verified with the repository TypeScript 7.1.0-dev.20260826.1 (`node_modules/.bin/tsc --strict`, with and without `--exactOptionalPropertyTypes`) and with the global `tsc` 6.0.2, including generic cases.
- **Behaviour change**: `V.C()` is now `{ kind: "C" }` instead of `{ kind: "C", opt: undefined }`, and an explicit `undefined` argument also leaves the property out. Reads of the property, `JSON.stringify`, and `match` bindings are unchanged; `in`, `Object.keys`, and `hasOwnProperty` now report the property as absent, as the declared type says.

### Decision 2: A required field after an optional one is a tt diagnostic

- **Context**: A case's fields are also its constructor's parameters, in order. TypeScript rejects a required parameter after an optional one (TS1016), because a call could not leave the optional argument out and still pass the required one.
- **Alternatives considered**: Emitting the earlier optional field as a required `opt: T | undefined` parameter would type-check, but it would make the constructor's parameter disagree with the field's declared optionality, and callers would have to pass `undefined` explicitly anyway.
- **Decision and rationale**: sema reports `variant-required-after-optional` at every required field that follows the case's first optional field, with the help "declare the required fields before the optional ones". The code is appended to `DiagnosticCode::ALL`, has a `ttc explain` text, and is appended to the content mapper's append-only wire table.

## Work log

- 2026-09-27: Reproduced with `ttc -o out` and the repository `tsc --strict --exactOptionalPropertyTypes`: TS2322 on each constructor with an optional field, TS1016 and TS2375 for `C(opt?: number, req: string)`; `ttc --check` exited 0 on that declaration.
- 2026-09-27: Changed the constructor object emission, added the sema rule and its code, and updated the explained-rule count in `src/diagnostics/tests.rs`.
- 2026-09-27: Documented optional fields in docs/ai/tt.md.
- 2026-09-27: Added `an_optional_variant_field_is_set_only_when_its_argument_is` and `a_required_variant_field_cannot_follow_an_optional_one` (tests/compile/cases_06.rs) and `optional_variant_fields_are_absent_when_their_argument_is` (tests/integration/cases_05.rs, run with and without `--exactOptionalPropertyTypes`). On the previous compiler the integration test fails (`"opt" in V.C()` printed `true`) and the compile tests do not build (the code did not exist); all pass now.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/codegen/core/emitter/helpers.rs`, `src/sema/checker.rs`, `src/diagnostics.rs`, `src/diagnostics/tests.rs`, `src/content_mapper.rs`, `docs/ai/tt.md`, `tests/compile/cases_06.rs`, `tests/integration/cases_05.rs`. Every accepted variant declaration now emits a constructor that type-checks under all strictness flags, and the one declaration shape that cannot be expressed is a located ttc error.
