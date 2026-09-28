# TASK-502: End an import type at a line break before a `<`

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

Valid TypeScript `declare const y: any;⏎let x: typeof import("x")⏎<any>y` was rejected with `verify-failed: Expected a semicolon`, and so were the TSX sources `let x: typeof import("x")⏎<b>hi</b>` and `let x: import("x").A⏎<b>hi</b>`. The `--no-verify` output was byte-identical to the input, so only the vendored SWC parser was wrong. That breaks contract 1 (every valid TypeScript file is a valid `.tt` file).

## Scope

- Included: `parse_ts_import_type` in the vendored SWC parser, its patch notes, a parser-level test file, a passthrough test, and a review of SWC's other type-argument sites against `tsc`.
- Excluded: Sites where TypeScript itself reads type arguments across a line break (below), which SWC already matches.

## Decisions

### Decision 1: Follow TypeScript's `parseTypeArgumentsOfTypeReference`

- **Context**: TypeScript's `parseImportType` reads the import type's type arguments with `parseTypeArgumentsOfTypeReference`, which requires `!scanner.hasPrecedingLineBreak()` and rescans `<<` as `<`. SWC's `parse_ts_import_type` (`src/parser/typescript.rs`) read a `<` unconditionally, so a type assertion or a JSX element on the next line became the type's argument list.
- **Alternatives considered**: Handling the shape in tt's own layers. The input is valid TypeScript and the defect is in the parser the verifier uses, so the fix belongs there, as TASK-497's did.
- **Decision and rationale**: Read type arguments only when no line break precedes the `<`, and accept `<<` the way `parse_ts_type_ref` does (TypeScript's rescan). Recorded in `vendor/swc_ecma_parser/TT-PATCH.md`. A search of the SWC issue tracker found no report of this shape.

### Decision 2: The other type-argument sites already agree with `tsc`

- **Context**: The task asked to check type references, `typeof x` instantiation, and heritage clauses for the same rule.
- **Decision and rationale**: Checked each against `tsc --noEmit` of the pinned TypeScript and against `ttc -p` after the patch. Type references (`parse_ts_type_ref`) and type queries (`parse_ts_type_query`) already require no preceding line break, as `parseTypeArgumentsOfTypeReference` and `parseTypeQuery` do; `let x: A⏎<any>y` and `let x: typeof y⏎<any>y` parse as two statements in both. Heritage clauses (`implements I⏎<any>`, `extends B⏎<any>`) take the type arguments across a line break in TypeScript (`parseExpressionWithTypeArguments` calls `tryParseTypeArguments` with no line-break test), and SWC does the same. Decorator and JSX element type arguments likewise have no line-break rule in TypeScript. Only the import type differed.

## Work log

- 2026-09-28: Reproduced the three sources; confirmed with `tsc --noEmit` that each parses (only TS2307 for the missing module), and that `let x: import("x")⏎<any>;` is a syntax error (TS1109) in both `tsc` and `ttc`.
- 2026-09-28: Patched `parse_ts_import_type`; added `tests/swc_import_type_arguments.rs` (TypeScript and TSX, four import-type forms: the next line is a type assertion or a JSX element statement; `<any>`, `<⏎any>`, and `<<T>() => T>` on the same line stay the type's arguments) and `a_line_after_an_import_type_is_not_its_type_arguments` in `tests/passthrough.rs`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`: exit 0.
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0.
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: exit 0.
- [x] `./scripts/ci extension`: exit 0.
- [x] `scripts/check-task-index`: the index and the records agree.

## Result

Changed `vendor/swc_ecma_parser/src/parser/typescript.rs`, `vendor/swc_ecma_parser/TT-PATCH.md`, `tests/passthrough.rs`, `docs/tasks/INDEX.md`, and this record; added `tests/swc_import_type_arguments.rs`. An import type followed by a line starting with `<` now parses the way `tsc` parses it.
