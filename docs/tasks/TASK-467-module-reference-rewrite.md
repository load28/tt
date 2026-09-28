# TASK-467: Rewrite every literal relative `.tt`/`.ttx` module reference

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`--rewrite-imports` rewrote static imports, re-exports, literal dynamic imports and import types, but left three module references with a relative `.tt` path in the emitted TypeScript: `import def = require("./sub/a.tt")`, ``import(`./sub/a.tt`)`` and `declare module "./sub/a.tt" { ... }`. In the emitted tree no `.tt` file exists, so TypeScript reports TS2307/TS2664 and a CommonJS `require` or a runtime `import()` loads a missing file. AGENTS.md contract 1 makes the relative `.tt`/`.ttx` specifier rewrite the one passthrough exception, and `docs/ai/tt.md` §Modules says `./token.tt` becomes `./token.js` by default; a reference that names the module and is not rewritten breaks that contract.

## Scope

- Included: Recognition of the import-equals external module reference, the no-substitution template argument of an import call, and the module augmentation name in the parser (`src/parser/imports.rs`, `src/parser/parse.rs`); the existing AST/HIR/codegen import path carries them unchanged. Regression tests, `docs/ai/tt.md` §Modules and the website's module limits.
- Excluded: Paths inside comments (`/// <reference path>`, JSDoc `import()` types). Computed specifiers. A template-literal `require` argument or module name, which TypeScript rejects syntactically.

## Decisions

### Decision 1: Classify each reported form by the grammar that owns it

- **Context**: The rewrite is an exception to byte passthrough, so it must apply exactly to the strings the language defines as module references, and to nothing else.
- **Alternatives considered**:
  - Rewrite every string or template whose text looks like a relative `.tt` path. A textual heuristic that would also rewrite ordinary strings, `require("./a.tt")` calls TypeScript does not treat as imports in `.ts` files, and comment text; ruled out by contract 3.
  - Keep TASK-020's exclusion of import-equals declarations. It leaves emitted CommonJS code requiring a nonexistent file.
- **Decision and rationale**: Each form was checked against its grammar and against pinned TypeScript `7.1.0-dev.20260826.1`:
  - `import [type] Identifier = require ( StringLiteral )` is TypeScript's `ImportEqualsDeclaration` whose `ModuleReference` is an `ExternalModuleReference` (TypeScript spec §11.3.3; `parseModuleReference`/`parseExternalModuleReference` in the TypeScript parser). The string is a module specifier that module resolution resolves and that CommonJS emit turns into `require(...)`. It is rewritten, including `export import` and `import type`. The binding names the whole module, so the declaration-collection API records it as a namespace import.
  - `ImportCall : import ( AssignmentExpression ,opt )` (ECMA-262 §13.3.10) evaluates its argument to a string. A `NoSubstitutionTemplate` (ECMA-262 §12.9.6) evaluates to its cooked text exactly as a string literal does, and TypeScript resolves it as the module name (`isStringLiteralLike`). The pinned `tsc` type-checks ``import(`./sub/a.js`)`` against `sub/a.ts`. A template with a substitution is computed and stays untouched. In a type position an import type requires a string literal (`import(`...`).A` is TS1141 "String literal expected"), so a template there is already invalid TypeScript and the shared import-call recognition does not need to distinguish it.
  - `declare module StringLiteral { ... }` whose name is relative is not an ambient external module declaration but a module augmentation: TypeScript resolves the name through module resolution relative to the declaring file. The pinned `tsc` applied `declare module "./sub/a.js" { export const B: string }` to `sub/a.ts` (a use of `B` as `number` failed with TS2322), and reported TS2664 "Invalid module name in augmentation, module './sub/missing.js' cannot be found" for a missing target, in both module and script files. The name refers to the emitted module, so it is rewritten. Recognition follows the TypeScript parser: `module` followed by a string literal on the same line (`nextTokenIsIdentifierOrStringLiteralOnSameLine`); a line break in between leaves two expression statements.
  - `export ... from`, `export * as ns from`, `import type ... from`, `typeof import("./a.tt")` and `import("./a.tt").T` were already rewritten; verified unchanged.
  - `/// <reference path>` is a comment directive that names a file, not a module specifier, and JSDoc `import()` types are comment text that TypeScript does not type-check in `.ts` output. Neither is part of the program's syntax, so both pass through byte for byte.
- **Measured evidence**: A two-file project (`a.tt` plus a `main.ts` with the three forms) built with `ttc -o out` and checked with `module: commonjs` passes the pinned `tsc` and prints `[{"value":42,"label":"ok"},42]` under Node. The same tree emitted with `--rewrite-imports off` (the pre-fix output for these forms) fails with TS2307 on the `require` and the `import()`, and TS2664 on the augmentation.

### Decision 2: Reuse the import AST/HIR/codegen path

- **Context**: The new forms need the same specifier rewrite, standard-library mapping, module-graph edge (`tt_imports`/`scan_module`) and mapping as the forms already recognized.
- **Alternatives considered**: A separate segment kind for non-import references. It would duplicate the rewrite and the graph collection.
- **Decision and rationale**: The parser lifts the specifier into the existing `Segment::TtImport`. `tt_spec_span` now takes the token, accepting a string literal or a template whose only part is its raw text, so the codegen's quote-preserving rewrite works for backticks without change.

### Decision 3: Reverse TASK-020's exclusion of import-equals and TASK-348's template lookalike

- **Context**: TASK-020 scoped `import x = require(...)` out as passthrough, and TASK-348's test listed ``import(`./feature.tt`)`` among unchanged lookalikes.
- **Decision and rationale**: Both are module references by the grammar above. The earlier records carry a note pointing here, and the lookalike case moved to the rewrite tests.

## Work log

- 2026-09-28: Reproduced with `ttc -p --no-banner` on `.tt` and `.ts` inputs: import-equals (`import`, `export import`, `import type`), the no-substitution template import call and the relative module augmentation stayed `.tt`; `export ... from`, `export * as ns from`, `import type`, `typeof import()`, `import().T` and import attributes were rewritten; triple-slash and JSDoc paths stayed.
- 2026-09-28: Checked each form with the pinned `tsc` (see Decision 1).
- 2026-09-28: Added `import_equals` and the `module` branch to `parse_tt_import`, taught `tt_spec_span` no-substitution templates (`src/parser/imports.rs`), and dispatched `module` from the segment parser (`src/parser/parse.rs`).
- 2026-09-28: Tests: `import_equals_require_reference_is_rewritten`, `no_substitution_template_dynamic_import_is_rewritten`, `module_augmentation_name_is_rewritten` (`tests/compile/cases_04.rs`), the extended `tt_imports_reports_specifiers_and_names` (`tests/compile/cases_05.rs`), and `tt_paths_outside_module_references_are_untouched` (`tests/passthrough.rs`). Replaced `import_assignment_is_untouched` and removed the template case from `computed_and_non_module_import_lookalikes_remain_unchanged`.
- 2026-09-28: Updated `docs/ai/tt.md` §Modules and the module limits in `website/src/content.json`; added reversal notes to TASK-020 and TASK-348.

## Issues and resolutions

### Issue 1: Lookalike cases that are not valid TypeScript failed verification

- **Symptom**: `import fs = require("./legacy.tt" + suffix)` and ``declare module `./token.tt` {}`` as passthrough cases failed with "generated TypeScript failed to parse".
- **Cause**: Both are TypeScript syntax errors (an external module reference and an ambient module name must be string literals), so they are not valid TypeScript to pass through.
- **Resolution**: Removed them from the passthrough test; the parser still rejects both shapes (only a string literal token is accepted there).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/parser/imports.rs`, `src/parser/parse.rs`, `tests/compile/cases_04.rs`, `tests/compile/cases_05.rs`, `tests/passthrough.rs`, `docs/ai/tt.md`, `website/src/content.json`, `docs/tasks/TASK-020-import-specifier-rewrite.md`, `docs/tasks/TASK-348-product-composition-audit.md`, this record, and `docs/tasks/INDEX.md`. Every literal relative `.tt`/`.ttx` module reference in the program's syntax is now rewritten; comment paths and computed specifiers pass through.
