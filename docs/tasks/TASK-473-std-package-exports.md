# TASK-473: `@tt/std` and `@tt/runtime` are dual-format packages that resolve in every module mode

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

The standard-library packages did not resolve under `module`/`moduleResolution` `node16`/`nodenext` when the importing file was an ES module:

- **On disk.** The content mapper (`ttc --content-mapper`) writes `node_modules/@tt/{std,runtime}` with a `package.json` that has only `name`, `version`, and `types`. Under `node16`/`nodenext`, `tsc --runExternalCode` reported `Cannot find module '@tt/std/option'`. With `"type": "module"` and `verbatimModuleSyntax`, it also reported 54 TS1287 errors in `node_modules/@tt/*`: the package's own manifest made those ES-module sources CommonJS.
- **In memory.** The typed engine served `node_modules/@tt/std/*.ts` with no `package.json`. `ttc --check-types` reported `ts2307` for `@tt/std` and `@tt/std/option` in ESM projects.

The fix must not change anything that worked before. In particular, CommonJS files, including under `module: node16`, must keep resolving the package with the answers they got before.

## Scope

- Included:
  - One package definition, `StdPackage` with `STD_PACKAGE_COMMONJS_DIR` and `GENERATED_BANNER` (`src/stdlib.rs`, exported from `src/lib.rs`).
  - Its users: the content mapper (`src/content_mapper.rs`), the language service (`src/engine/language/service.rs`), the contextual pass (`src/typescript/contextual.rs`), the typed engine's projection (`src/engine/projection.rs`), and the `--emit-std` banner (`src/main/command.rs`).
  - Tests in `tests/stdlib.rs`, `tests/content_mapper.rs`, and `tests/native/cases_03.rs`, plus `docs/design/content-mapper.md`.
- Excluded:
  - Resolution of relative `.tt` specifiers under ESM. That is TASK-472.
  - The bundler plugin (`integrations/unplugin`) and `create-tt`. Checked: the plugin serves each module as a virtual module id through `ttc --emit-std` and writes no package, and `create-tt` never writes the standard library.
  - The build's `tt/` support directory (`src/main/build.rs`). It is not a package: the build rewrites specifiers to its files.

## Decisions

### Decision 1: A dual-format package with `import`/`require` conditions

- **Context**: The sources are ES modules with explicit `./x.js` relative specifiers. TypeScript's modules reference sets these rules:
  - A `.ts` file's format follows the nearest `package.json` `"type"` ([Module format detection](https://www.typescriptlang.org/docs/handbook/modules/reference.html#module-format-detection)).
  - Under `node16`/`nodenext`/`bundler`, a bare subpath resolves only through `package.json` `"exports"`. Conditions match in object order, with `import` for ES-module importers and `require` for CommonJS importers, and `types` should come first in each ([`package.json` `"exports"`](https://www.typescriptlang.org/docs/handbook/modules/reference.html#packagejson-exports), [conditional exports](https://www.typescriptlang.org/docs/handbook/modules/reference.html#conditional-exports)).
  - A package serving both formats points each condition at files of that format ([Modules — Choosing compiler options: dual packages](https://www.typescriptlang.org/docs/handbook/modules/guides/choosing-compiler-options.html)).

  Before this task, the in-memory package inherited the user's package format, because it had no manifest. That is why CommonJS projects worked, including `node16`, and why ES-module projects could not resolve subpaths.
- **Alternatives considered**:
  - ES-module only (`"type": "module"` plus a single-target `exports`): This was the first version of this change. It regressed `module: node16` CommonJS files. A value import from `@tt/std/option` became TS1479, and `import type` from `@tt/std` became TS1541, both measured. The rework was rejected on that ground.
  - Declaration-only entry points (`.d.mts`/`.d.cts`): TypeScript would have to emit them first. The typed engine relies on the real sources: `semantics/declarations.rs` reads the compiler's own declaration emit for them. Match exhaustiveness also needs the variant values.
  - `.mts`/`.cts` twins at the package root: The sources' relative `./result.js` specifiers would cross formats. A `.cts` would import the ESM `result.ts`, which is TS1541 again unless the sources were rewritten.
- **Decision and rationale**:
  - **Layout.** The package root is `"type": "module"` and holds the unchanged sources. `cjs/` holds byte-identical copies under a `{"type": "commonjs"}` manifest, so each copy's relative imports stay within its own format.
  - **Exports.** Each `"exports"` entry (`.`, `./option`, `./result`; `.` for `@tt/runtime`) is `{ "import": { "types", "default" } → root file, "require": { "types", "default" } → cjs/ copy }`. The top-level `"types": "./index.ts"` remains for resolvers that ignore `"exports"`.
  - **Mixed projects.** A project whose `.cts` and `.mts` files both import the package loads both copies. Their declarations are structurally identical, so values flow between them, as measured with a `typeof legacy = Option.None` assignment.

### Decision 2: One package definition for every materializer

- **Context**: Four places wrote their own copy of the manifest and the generated banner.
- **Decision and rationale**: `StdPackage` defines the files once:
  - `StdPackage::files_with_banner` yields every file of the package.
  - `StdPackage::files` adds `GENERATED_BANNER` to each module.
  - `StdPackage::materialize(root)` writes the package when absent.

  The content mapper and the language service call `materialize`. The contextual pass serves `files()` in memory, and the projection serves `files_with_banner("")`. The projection keeps the sources byte-identical to what it served before, so emitted standard-library declarations do not change. `--emit-std` prints `GENERATED_BANNER`.

### Decision 3: Upgrade only a manifest that earlier ttc releases wrote

- **Context**: Materializers never overwrite an existing package. A project materialized by an earlier release would keep the broken manifest.
- **Alternatives considered**: Always overwrite, which destroys a package the user manages. Detect ownership by the modules' `@generated` banner, but the manifest carries none.
- **Decision and rationale**: `materialize` replaces `package.json` and adds the `cjs/` copy only when the manifest's bytes equal what earlier releases wrote (`legacy_manifest`). Module files and any other manifest are untouched.

## Work log

- 2026-09-28: Reproduced with `tsc -p . --runExternalCode` (`"type": "module"`, `nodenext`, `verbatimModuleSyntax`, `contentMappers` pointing at this build): 54 TS1287 errors in `node_modules/@tt/{std,runtime}`.
- 2026-09-28: First version: `"type": "module"` and single-target `exports`. The coordinator rejected it because `node16` CommonJS files regressed (TS1479/TS1541).
- 2026-09-28: Reworked as a dual-format package (Decision 1) and replaced the four copies with `StdPackage` (Decision 2).
- 2026-09-28: Matrix run with a scratch script against the build before this task (the hunter's `ttc`) and after. Each project has a `.tt` file using `@tt/std`, `@tt/std/option`, `@tt/std/result`, and a pipeline, plus a hand-written `.ts` file using `@tt/std`. `tsc --runExternalCode` was run a second time because the mapper materializes during the first run, as before. The table gives error counts as `tsc --runExternalCode` / `ttc --check-types`.

  | `module`/`moduleResolution`, package `"type"`, `verbatimModuleSyntax` | Before | After |
  | --- | --- | --- |
  | `node16`, `commonjs` | 0 / 0 | 0 / 0 |
  | `node16`, `module` | 3 / 5 | 0 / 0 |
  | `node16`, `module`, verbatim | 54 / 5 | 0 / 0 |
  | `nodenext`, `commonjs` | 0 / 0 | 0 / 0 |
  | `nodenext`, `module` | 3 / 5 | 0 / 0 |
  | `nodenext`, `module`, verbatim | 54 / 5 | 0 / 0 |
  | `esnext`/`bundler`, `module` ± verbatim | 0 / 0 | 0 / 0 |
  | `preserve`/`bundler`, `commonjs` | 0 / 0 | 0 / 0 |
  | `commonjs`/`bundler`, `commonjs` | 0 / 0 | 0 / 0 |
  | `commonjs`/`node10` | 1 / 1 (TS5108) | 1 / 1 (TS5108) |

  - **node10.** TypeScript 7 removed `node10`: TS5108 in both builds, so it cannot be checked. The top-level `"types"` field is what such a resolver would read.
  - **Verbatim CommonJS.** `nodenext`/`node16` CommonJS with `verbatimModuleSyntax` reports the same errors before and after, all in the user's own ESM-syntax files (TS1287/TS1295), none in `node_modules`.
  - **Mixed formats.** A mixed `node16` project (`legacy.cts` with `import … = require("@tt/std/option")`, `modern.mts` importing it and `@tt/std/option`) is clean after this change under both `tsc --runExternalCode` and `ttc --check-types`. Before, it had TS2307 errors.
- 2026-09-28: Tests:
  - `tests/stdlib.rs`: `materialized_packages_are_dual_format_with_an_exports_map` (manifest text, condition order, `cjs/` manifest, byte-identical copies) and `a_package_ttc_wrote_before_exports_is_upgraded_and_any_other_is_kept`.
  - `tests/content_mapper.rs`: `std_imports_resolve_under_node_esm_with_verbatim_module_syntax` (starts from the legacy manifest) and `std_imports_resolve_from_commonjs_and_esm_files_under_node16`. With the `require` condition pointed at the ESM files, the second fails with TS1479/TS1541, so it guards the regression the first version had.
  - `tests/native/cases_03.rs`: `the_standard_library_resolves_from_either_module_format_in_every_resolution_mode` (`ttc --check-types` across seven configurations, each with exactly one deliberate error).

## Issues and resolutions

### Issue 1: The ES-module-only package regressed `node16` CommonJS files

- **Symptom**: In a `"type": "commonjs"` project under `module: node16`, a file importing `@tt/std/option` reported TS1479, and one with `import type { TOption } from "@tt/std"` reported TS1541.
- **Cause**: `"type": "module"` made the package ESM for every importer. `node16` does not allow `require` of an ES module. Before, the in-memory package had no manifest and was CommonJS in a CommonJS project.
- **Resolution**: The dual-format package in Decision 1. `std_imports_resolve_from_commonjs_and_esm_files_under_node16` fixes it as a regression test.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/stdlib.rs`, `src/lib.rs`, `src/content_mapper.rs`, `src/engine/language/service.rs`, `src/engine/projection.rs`, `src/typescript/contextual.rs`, `src/main/command.rs`, `tests/stdlib.rs`, `tests/content_mapper.rs`, `tests/native/cases_03.rs`, `docs/design/content-mapper.md`, this record, and `docs/tasks/INDEX.md`. Every standard-library package ttc materializes or serves is now one dual-format package. It resolves from ES-module and CommonJS files in every supported `moduleResolution` mode, with no configuration that worked before losing it. Packages earlier releases wrote are upgraded.
