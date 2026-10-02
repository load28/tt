# TASK-598: Declare pipeline helpers in a module written with CommonJS syntax

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-598`

## Purpose

In a `"type": "commonjs"` package compiled with `module: nodenext` and
`verbatimModuleSyntax: true`, a `.tt` module that uses `flow` or `|>` and
ends in `export = f;` was emitted with
`import { $tt_fl } from "./tt/runtime.js"` (TS1295: ECMAScript imports
cannot be written in a CommonJS file under `verbatimModuleSyntax`), and the
materialized `tt/runtime.ts`, a CommonJS file in that package, failed with
TS1287 on its `export function`s. The source is valid TypeScript for that
project, so the output must be too; `ttc --check-types` reported TS1295 on
the source as well.

## Scope

- Included: recording CommonJS module syntax in `ProgramSyntax`
  (`src/program_syntax.rs`, `src/program_syntax/projection.rs`), carrying
  it through the lowering plan (`src/evaluation_ir.rs`,
  `src/evaluation_ir/evaluation.rs`, `src/codegen/core/planning.rs`), the
  runtime prelude (`src/codegen/core/mod.rs`), `docs/ai/tt.md`,
  `docs/design/program-lowering.md` §4.4, and regression tests.
- Excluded: a CommonJS-format module that writes only type-only imports
  and exports has no syntactic evidence of its format and still imports
  the runtime; so does an ECMAScript-format output whose `tt/` support
  directory lies in a CommonJS package scope. Both are recorded under
  Issues.

## Decisions

### Decision 1: Decide from the module's own syntax, as script detection does

- **Context**: Under `verbatimModuleSyntax`, TypeScript writes imports and
  exports as they are, so a file must use the syntax of its format:
  `import x = require()`/`export =` in a CommonJS file, `import`/`export`
  in an ECMAScript one (TypeScript handbook, "Modules - Reference" and the
  `verbatimModuleSyntax` option reference). The format of a `.ts` file
  under `node16`/`nodenext` follows Node's module-format detection (the
  nearest `package.json` `"type"`; Node.js documentation, "Modules:
  Packages — Determining module system"); under `module: commonjs` every
  file is CommonJS. ttc has no module-format model; the only format fact it
  already derives is TypeScript's own syntactic one, script versus module
  (`is_script`, TypeScript's `isFileProbablyExternalModule`), and for a
  script it already declares the helpers as typed `var`s.
- **Alternatives considered**: (a) Read `module`, `verbatimModuleSyntax`,
  and `package.json` `"type"` and re-derive TypeScript's format decision.
  ttc does not read `tsconfig.json` (with `extends`); the checker does,
  and the build does not consult it for a file with no typed slots, so the
  decision would be missing exactly where the probe failed (a project
  whose `include` names only the output). (b) Emit
  `import $tt_runtime = require("./tt/runtime.js")` and materialize the
  runtime with `export =`. The support module's own format follows its
  own location, so it would still be wrong for an ECMAScript importer in
  the same tree, and the import form would still need the format. (c)
  `export =` and a non-type `import x = require(...)` are TypeScript's
  CommonJS module forms, and a file that writes them is a CommonJS module
  (TS1202/TS1203 reject them when a file is emitted as an ECMAScript module
  outside `module: preserve`). Record that fact beside `is_script` and
  have such a module declare its helpers locally, as a script does.
- **Decision and rationale**: (c). The evidence is the file's own syntax,
  read the way ttc already reads script versus module, with no guess about
  configuration. Local `var` helpers are valid TypeScript in either format
  and under every `module` setting, so the choice can never produce
  invalid output; the module imports no support module, so none is
  materialized for it, which removes the TS1287 as well.

## Work log

- 2026-09-30: Reproduced `target/probe5-compiler/cjs` (TS1295 in `m.ts`,
  TS1287 in `tt/runtime.ts`) and TS1295 from `--check-types` with a
  tsconfig that includes the sources.
- 2026-09-30: `uses_commonjs_syntax` in `src/program_syntax.rs`, carried as
  `commonjs` through `ProgramSyntax`, `EvaluationFile`, `LoweringPlan`, and
  `TargetRewritePlan`; `src/codegen/core/mod.rs` declares the helpers
  locally and leaves `@tt/runtime` out of `support_imports`.
- 2026-09-30: Tests `a_commonjs_module_declares_its_pipeline_helpers` and
  `a_commonjs_module_type_checks_under_verbatim_module_syntax`
  (`tests/cli.rs`: build output, `--check-types`, and `tsc -p` over a
  `nodenext` + `verbatimModuleSyntax` CommonJS package).

## Issues and resolutions

### Issue 1: Modules without CommonJS syntax in a CommonJS package

- **Symptom**: A CommonJS-format module that writes only `import type`/
  `export type` still imports `@tt/runtime` with ECMAScript syntax.
- **Cause**: Its syntax is valid in either format, so it carries no
  evidence of its format.
- **Resolution**: Left open; deciding it needs TypeScript's per-file format
  (`impliedNodeFormat`) before code generation, which only the checker
  holds.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/program_syntax.rs`, `src/program_syntax/projection.rs`,
`src/evaluation_ir.rs`, `src/evaluation_ir/evaluation.rs`,
`src/codegen/core/planning.rs`, `src/codegen/core/mod.rs`, `docs/ai/tt.md`,
`docs/design/program-lowering.md`, `tests/cli.rs`, `docs/tasks/INDEX.md`,
and this record.
