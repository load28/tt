# TASK-617: Import the standard library in CommonJS syntax from a CommonJS module

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-617`

## Purpose

In a `"type": "commonjs"` package compiled with `module: nodenext` and
`verbatimModuleSyntax`, a `.tt` module that ends in `export =` and imports
`@tt/std` (type-only or through `import x = require("@tt/std/option")`)
was built with `tt/option.ts` and `tt/result.ts` in ECMAScript syntax. Both
are CommonJS files in that package, so `tsc` rejected every `export const`
with TS1287, while `ttc --check-types` on the source passed (it resolves
the installable `@tt/std` package, whose `require` condition serves
declarations). The build's output must type-check as the source does.

## Scope

- Included: the CommonJS form of the support modules
  (`StdModule::commonjs_source`, `src/stdlib.rs`), the per-module choice of
  support imports (`StdImports::commonjs`; `src/codegen/core/mod.rs`), the
  module-syntax fact for a file whose only tt syntax is a standard-library
  import (`CoreFile::imports_std`, `src/core_ir/mod.rs`), the emit's
  `commonjs` flag (`src/lib/mapped.rs`, `src/lib/compile.rs`,
  `src/codegen/rope*.rs`), materialization (`src/main/build.rs`),
  `docs/ai/tt.md`, and tests.
- Excluded: the installable package (`StdPackage`), which already serves a
  CommonJS importer through its `require` condition; bundler adapters,
  which serve the modules virtually.

## Decisions

### Decision 1: The importer's own syntax chooses the support modules' syntax, per module

- **Context**: Under `verbatimModuleSyntax` a CommonJS file must use
  `export =`/`import x = require()` and an ECMAScript file
  `import`/`export` (TypeScript `verbatimModuleSyntax` reference;
  "Modules - Reference"). A `.ts` file's format under `node16`/`nodenext`
  follows the nearest `package.json` `"type"` (Node.js "Modules: Packages —
  Determining module system"; TypeScript "Modules - Reference", module
  format detection), and under `module: commonjs` every file is CommonJS.
  No syntax is valid in both formats. TASK-598 took a module's
  `export =`/non-type `import x = require()` as syntactic evidence that it
  is CommonJS. ttc does not read the configuration (TASK-598, Decision 1).
- **Alternatives considered**: (a) Choose one form for the whole `tt/`
  directory from the build's importers: the choice would depend on which
  files a run compiles (a watch rebuild of one ECMAScript-syntax file, or
  `ttc -o out src/b.tt`, would rewrite the directory in the other form).
  (b) Write `tt/package.json` with `"type": "module"` so the ECMAScript
  form is always ECMAScript: `tsc` does not copy `package.json` into
  `outDir`, so the emitted ECMAScript files would run as CommonJS under the
  package's own `"type"`, and under `module: commonjs` TypeScript would emit
  CommonJS into a directory Node.js then reads as ECMAScript. (c) Name the
  modules `.mts`: the format is then fixed, but a CommonJS importer depends
  on `require(esm)` at run time, and every ECMAScript importer's output
  would change. (d) Keep `tt/` in ECMAScript syntax and add a CommonJS copy
  in `tt/cjs/`, the directory name the installable package already uses for
  its CommonJS entries (`STD_PACKAGE_COMMONJS_DIR`); a module written with
  CommonJS syntax imports from `tt/cjs/`, every other module from `tt/`.
- **Decision and rationale**: (d). The decision is per importer and depends
  only on that importer's own syntax, so a partial or watch build writes the
  same files a full build does, and an ECMAScript importer's output is
  unchanged. `StdImports` carries the CommonJS replacements in its new
  `commonjs` field; code generation uses them for a module with CommonJS
  syntax (not a script), and the build writes a form's modules only when an
  output that uses that form imports them.

### Decision 2: The CommonJS form wraps the one source in a namespace published with `export =`

- **Context**: A CommonJS module has one export (`export =`). Importers
  need both value members (`option.Some`) and type members
  (`import type { TOption } from "./option.js"`, `index.ts`'s
  `export type { ... } from`).
- **Alternatives considered**: Hand-written CommonJS copies: a second copy
  of every combinator that would drift from the ECMAScript source, whose
  values are guarded to be byte-identical to what the variants compile to
  (`tests/stdlib.rs`). `export = { Some, ... }` of an object: carries no
  types, and a named value import from it is TS2305.
- **Decision and rationale**: `StdModule::commonjs_source` keeps the
  source's leading comments and `import type` lines and wraps the rest,
  unchanged, in `namespace option { ... }` followed by `export = option;`.
  A namespace member keeps its `export const`/`export type` declaration, so
  every binding keeps its name and value. The type-only entry (`index.ts`)
  is already valid in a CommonJS file and is written as is. Checked with
  `tsc` under `nodenext` + `verbatimModuleSyntax` (+ `declaration`),
  `node16`, `commonjs` (+ `isolatedModules`), and `preserve`, and run with
  Node.js. A CommonJS-syntax file is already outside `erasableSyntaxOnly`
  (TS1294 on its own `export =`), so the namespace adds no new restriction.

### Decision 3: A file whose only tt syntax is a standard-library import still gets its module syntax facts

- **Context**: `lowering_plan_with` returned an empty plan when nothing
  needed host lowering, so a module like `import type { TOption } from
  "@tt/std"; ... export = v;` never had its syntax read and was treated as
  ECMAScript.
- **Decision and rationale**: `CoreFile::imports_std` makes such a file
  build its `ProgramSyntax`, the one reader of `export =`/`import x =
  require()` (TASK-598). Its lowering has no regions, so its output is
  unchanged apart from the import specifiers.

## Work log

- 2026-09-30: Reproduced `target/probe6-cli/p8` (TS1287 in
  `out/tt/option.ts`), and the same with a value import through
  `import option = require("@tt/std/option")`.
- 2026-09-30: Prototyped the wrapped form with a script and `tsc` in five
  configurations; all passed, and Node.js ran the emitted CommonJS.
- 2026-09-30: Implemented Decisions 1–3. The first attempt still wrote
  `tt/` (Issue 1).
- 2026-09-30: Tests
  `a_commonjs_module_imports_the_standard_library_in_commonjs_syntax`
  (`--check-types`, build, `tsc -p`) and
  `an_ecmascript_module_keeps_the_standard_library_in_tt`
  (`tests/cli.rs`); `tests/compile/cases_04.rs` and `tests/integration.rs`
  name the new field.

## Issues and resolutions

### Issue 1: A module with no lowering was never recognized as CommonJS

- **Symptom**: After the change the probe still imported `./tt/option.js`.
- **Cause**: The file needed no host lowering, so `lowering_plan_with`
  returned `LoweringPlan::default()` and `commonjs` stayed false.
- **Resolution**: Decision 3.

### Issue 2: `StdImports` gained a public field

- **Symptom**: `tests/compile/cases_04.rs` and `tests/integration.rs`
  built `StdImports` with every field and no longer compiled.
- **Cause**: A struct literal must name every field.
- **Resolution**: They name `commonjs: None`; a library user building the
  struct literally does the same (or uses `..Default::default()`).

### Issue 3: A support directory in another package scope

- **Symptom**: None observed; recorded as a limit.
- **Cause**: The evidence says the importer is CommonJS; `tt/cjs/` is
  assumed to share its package scope, as `tt/` is today.
- **Resolution**: Left open, like TASK-598 Issue 1; deciding it needs the
  checker's per-file format.

## Verification

- [x] `cargo test --test cli`, `--test compile`, `--test integration`,
  `--test snapshot`, `--test stdlib`, `--test cli_outputs`,
  `--test passthrough`, `--test corpus`, `--test emit_map`
  (with `TTC_REQUIRE_TSGO=1`)
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] Full gate run once at the end of the TASK-614–620 series; see
  TASK-620.

## Result

Changed `src/stdlib.rs`, `src/codegen/core/mod.rs`, `src/codegen/rope.rs`,
`src/codegen/rope/builder.rs`, `src/core_ir/mod.rs`, `src/lib/compile.rs`,
`src/lib/mapped.rs`, `src/main/build.rs`, `docs/ai/tt.md`, `tests/cli.rs`,
`tests/compile/cases_04.rs`, `tests/integration.rs`, `docs/tasks/INDEX.md`,
and this record.
