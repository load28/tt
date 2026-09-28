# TASK-485: The `@tt/std` CommonJS entry points are declaration files

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

TASK-473 made `@tt/std` and `@tt/runtime` dual-format packages whose `cjs/` directory held byte-identical copies of the ES-module sources under `{"type": "commonjs"}`. Those copies use `export const` and `export function`. Under `verbatimModuleSyntax`, that is TS1287 in a CommonJS module. A `nodenext` ES-module project with `verbatimModuleSyntax` and a `.cts` file that requires the package reported 51 TS1287 errors in `node_modules/@tt/std/cjs/*.ts` under `tsc -p . --runExternalCode` (second run, after the mapper materialized the package). `skipLibCheck` does not help, because the copies are `.ts` sources, not declaration files. The CommonJS entry points must type-check cleanly under every option, without changing what worked before.

## Scope

- Included:
  - `src/stdlib.rs` (`StdModule::declaration`, `StdPackage::commonjs_file_name`, the manifest, the upgrade of earlier layouts).
  - The declaration texts in `src/stdlib/commonjs/`.
  - Tests in `tests/stdlib.rs`, `tests/content_mapper.rs`, and `tests/native/cases_03.rs`.
  - `docs/design/content-mapper.md`, and the correction in the TASK-473 record.
- Excluded:
  - The root ES-module sources, which stay byte-identical. The typed engine reads the compiler's declaration emit for them (`semantics/declarations.rs`).
  - Errors in the user's own files. A CommonJS project under `verbatimModuleSyntax` still gets TS1287/TS1295 for ESM syntax in its own `.ts` and `.tt` files, exactly as `tsc` reports them.

## Decisions

### Decision 1: Serve the CommonJS condition TypeScript's declaration emit of each module

- **Context**: The `require` condition needs files that TypeScript reads as CommonJS and that are clean under `verbatimModuleSyntax`.
  - TypeScript's modules reference: under `verbatimModuleSyntax`, ESM syntax in a file that emits CommonJS is an error, and a CommonJS module uses `export =` and `import = require` ([Modules reference: `verbatimModuleSyntax`](https://www.typescriptlang.org/tsconfig/#verbatimModuleSyntax); [Modules — Reference: module format detection](https://www.typescriptlang.org/docs/handbook/modules/reference.html#module-format-detection)).
  - That rule is about emit. A declaration file emits nothing. Measured with the pinned `tsc` 7.1.0-dev.20260826.1, without `skipLibCheck`: `export declare const` in a `.d.ts` under `{"type": "commonjs"}`, required by a `.cts` in a `"type": "module"` project under `nodenext` and `verbatimModuleSyntax`, reports nothing. The program lists `cjs/{index,option,result}.d.ts`.
- **Alternatives considered**:
  - CommonJS-syntax sources (`export =` with a namespace for the types): each module would need a second, hand-written shape. The two formats would then name different declarations (`typeof import("@tt/std/option")` versus the object behind `export =`), and a mixed project's `.cts` and `.mts` values would no longer be the same structural declarations TASK-473 measured. Named imports from an `export =` object also depend on `esModuleInterop`.
  - `.d.ts` files that re-export the root ES modules: under `node16`, a CommonJS file importing an ES module is TS1479, as TASK-473 measured for its first version.
  - Keep `.ts` copies next to the declarations: `./result.js` inside a declaration resolves to `result.ts` before `result.d.ts`, so the copies would be checked again.
  - Declarations for every entry point: TASK-473 Decision 1 rejected that for the root, which the typed engine needs as sources. It still holds there.
- **Decision and rationale**:
  - **Files.** `cjs/` holds `index.d.ts`, `option.d.ts`, and `result.d.ts` (`index.d.ts` for `@tt/runtime`), plus its `{"type": "commonjs"}` manifest. Each is the pinned compiler's declaration emit of the matching source (`--declaration --emitDeclarationOnly --strict --target es2022 --module esnext --moduleResolution bundler`), checked in under `src/stdlib/commonjs/`. `std_commonjs_declarations_are_the_compilers_declaration_emit` fails when a source changes without its declaration.
  - **Exports.** `"require": { "types": "./cjs/<module>.d.ts", "default": "./<module>.ts" }`. The `types` condition comes first, so TypeScript reads the declaration. `default` names the one source, whose bytes are those the `cjs/` copy had, so a bundler resolving the `require` condition bundles the same text as before.
  - **Types.** The declarations are what TypeScript infers for the sources, so a CommonJS importer sees the same types. The ES-module side is untouched.

### Decision 2: Upgrade the package TASK-473's build wrote

- **Context**: Materializers never overwrite an existing package, and the `hunt6/mx` project already had the TASK-473 layout. Without an upgrade it would keep its 51 errors.
- **Decision and rationale**: `StdPackage::earlier_layouts` lists the exact files each earlier release wrote beside the root modules: the legacy manifest, and the TASK-473 manifest with its `cjs/` source copies and `cjs/package.json`. When every file of one layout is present with exactly those bytes, `materialize` removes the files the current layout does not have and writes the current manifest and `cjs/` files. A package where any of those files differs, such as an edited copy, is left alone, as before. Root modules are never touched.

## Work log

- 2026-09-28: Reproduced on a copy of `hunt6/mx` (`nodenext`, `"type": "module"`, `verbatimModuleSyntax`, `src/b.cts` with `import Option = require("@tt/std/option")`): 51 TS1287 errors in `node_modules/@tt/std/cjs/{option,result}.ts`, with and without `skipLibCheck`.
- 2026-09-28: Measured the declaration alternative by hand: replaced the `cjs/*.ts` copies with `tsc --declaration` output and pointed the `require` condition at them. The program was clean without `skipLibCheck`.
- 2026-09-28: Implemented in `src/stdlib.rs` and `src/stdlib/commonjs/*.d.ts`.
- 2026-09-28: On a fresh copy of `hunt6/mx` with a `.tt` file added so the mapper runs, the first `tsc --runExternalCode` run upgraded the TASK-473 package and the second reported 0 errors.
- 2026-09-28: Matrix with a scratch script, the TASK-473 build (the hunter's `ttc`) against this build. Configurations: `node16`/`nodenext` CommonJS and ESM ± `verbatimModuleSyntax`, `esnext`/`bundler` ± `verbatimModuleSyntax`, `preserve`/`bundler`, and `commonjs`/`bundler`. Each project has a `.tt` file using `@tt/std`, `@tt/std/option`, `@tt/std/result`, and a pipeline, plus a hand-written `.ts` file using `@tt/std`. Counts are `tsc --runExternalCode` (second run) and `ttc --check-types`.
  - **`skipLibCheck: true`, no `.cts` (TASK-473's matrix).** Identical output in every configuration except `node16`/`nodenext` CommonJS with `verbatimModuleSyntax`. There `tsc` went from 60 to 9 errors and `ttc --check-types` lost the 2 in `node_modules`. The 9 that remain are the user's own ESM syntax in CommonJS files, the same before and after.
  - **`skipLibCheck: false`, no `.cts`.** The same result: identical except that the 51 `node_modules` errors are gone in the two CommonJS `verbatimModuleSyntax` configurations.
  - **`skipLibCheck: false`, with the `.cts` from the report.** `node16`/`nodenext` ESM with `verbatimModuleSyntax` went from 51 errors (2 under `ttc --check-types`) to 0. `esnext`/`bundler` reports TS1202 on the user's `import … = require` in `.cts` before and after, plus the 51 before in its `verbatimModuleSyntax` configuration. Every other configuration is 0 before and after.
- 2026-09-28: Tests:
  - `tests/stdlib.rs`: `materialized_packages_are_dual_format_with_an_exports_map` now checks the declaration entries, the `require` condition, and the absence of source copies. `a_package_with_commonjs_source_copies_gets_declarations_in_their_place` covers the upgrade and an edited copy that is kept.
  - `tests/content_mapper.rs`: `a_commonjs_file_requires_the_std_package_cleanly_under_verbatim_module_syntax` starts from the TASK-473 layout and runs `tsc --runExternalCode` twice, with `skipLibCheck` on and off. `std_commonjs_declarations_are_the_compilers_declaration_emit` guards the checked-in declarations.
  - `tests/native/cases_03.rs`: `a_commonjs_file_requiring_the_standard_library_under_verbatim_module_syntax_is_clean` checks that `ttc --check-types` reports exactly the one deliberate error under `node16` and `nodenext`.
- 2026-09-28: Corrected the TASK-473 record's claim about verbatim CommonJS and noted the supersession at its top. Updated `docs/design/content-mapper.md`.

## Issues and resolutions

### Issue 1: The `.tt` mapper never materialized the package in the reported project

- **Symptom**: `hunt6/mx` has no `.tt` file, so a fresh `tsc --runExternalCode` never spawns the mapper, and the package stays as the earlier build wrote it.
- **Cause**: The content mapper materializes the package for each transformed file's package root (`src/content_mapper.rs`).
- **Resolution**: None needed in the compiler. The reproduction adds a `.tt` file, as any project using the mapper has.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `./scripts/ci extension` (208 server tests) and `./scripts/ci npm` (64 tests; run with the repository's `node_modules` linked into the worktree, because the pinned-TypeScript checks read it)

## Result

Changed `src/stdlib.rs`, added `src/stdlib/commonjs/{types,option,result,runtime}.d.ts`, and changed `tests/stdlib.rs`, `tests/content_mapper.rs`, `tests/native/cases_03.rs`, `docs/design/content-mapper.md`, `docs/tasks/TASK-473-std-package-exports.md`, this record, and `docs/tasks/INDEX.md`. A CommonJS importer of `@tt/std` or `@tt/runtime` now reads declaration files, which are clean under `verbatimModuleSyntax` with or without `skipLibCheck`. Every configuration that was clean before is unchanged, and packages TASK-473's build wrote are upgraded.
