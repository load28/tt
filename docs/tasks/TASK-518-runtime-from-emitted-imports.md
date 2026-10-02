# TASK-518: Write support modules from the imports codegen emitted

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: see `git log --grep TASK-518`

## Purpose

`printf 'export const a = 1 |> String;\n' > src/a.tt; ttc src` wrote
`src/tt/runtime.ts` although `src/a.ts` (`export const a = String(1);`) does
not import it. A project of script files gets the same unused runtime,
which TASK-483 recorded as a known residual.

## Scope

- Included: a record of the support modules an emission imports
  (`MappedEmit::support_imports`, filled by codegen), the CLI build's
  support-module decision (`compile_jobs` in `src/main/build.rs`), the typed
  projection's module graph (`src/engine/projection.rs`), regression tests,
  `docs/ai/tt.md`, and a note on TASK-483.
- Excluded: `ModuleScan::uses_pipeline`, a public fact about the source that
  no longer decides anything inside ttc and stays for API consumers; the
  editor's language service, which materializes the runtime for its own
  project.

## Decisions

### Decision 1: Codegen reports the support modules it imported

- **Context**: The build decided to write `tt/runtime.ts` from
  `scan.uses_pipeline`, a syntactic walk for pipeline segments. Whether an
  output imports the runtime is decided later, by codegen: a literal-headed
  pipeline lowers to a direct call, and a script declares its helpers as
  `var`s instead of importing them. The syntactic fact cannot answer a
  question about the emitted code.
- **Alternatives considered**: teaching the scan which pipelines lower to a
  direct call and which files are scripts. That copies codegen's decisions
  into a second place, where they drift apart as lowering changes.
  Searching the emitted text for the runtime specifier would work for these
  cases, but it is a string test on output that codegen already knows the
  answer to.
- **Decision and rationale**: the emitter records each `@tt/std` module an
  import names (`emit_import`) and whether the module prelude imports runtime
  helpers. `emit_with_map` returns them as `support_imports` in
  `StdModule::ALL` order, and `MappedEmit` exposes the list.

### Decision 2: The build writes support modules after compiling, before writing outputs

- **Context**: `compile_jobs` placed and wrote the support modules before
  compiling, because each output needs the specifier of the `tt/` directory.
  The specifier depends only on where the directory goes, not on whether it
  is written.
- **Decision and rationale**: `std_placement` always names the `tt/`
  directory under the support root. Jobs compile in parallel without
  writing. The build then takes the modules the emitted outputs import: all
  three public modules if any output imports one of them (unchanged), and
  the runtime if any output imports it. It runs the same ownership and
  collision checks as before, writes those modules, and writes the job
  outputs in parallel. Messages print in the same order as before: the
  `std →` line, then each job's diagnostics and `→` line in job order. A
  failed support check still stops the build before any output is written.
  A file that fails to compile writes no output, so it no longer causes a
  runtime file on its own.

### Decision 3: The typed project graph uses the same fact

- **Context**: `ProjectedDocument::uses_pipeline` ("whether lowering needs
  the runtime module") added `@tt/runtime` to the checked program from the
  same syntactic scan.
- **Decision and rationale**: the field is now `imports_runtime` and reads
  the projection's own `support_imports`, so the type-checked program and
  the build agree on when the runtime exists.

## Work log

- 2026-09-29: Reproduced: `ttc src` writes `src/tt/runtime.ts` for
  `export const a = 1 |> String;`; a script with `input() |> step` does the
  same.
- 2026-09-29: Added `support_imports` to `Flat`/`MappedEmit` and filled it
  in `src/codegen/core/mod.rs` and `emitter/expression.rs`. Restaged
  `compile_jobs` (compile, support modules, parallel writes via
  `write_emitted`). Switched the projection to `imports_runtime`.
- 2026-09-29: Added `a_pipeline_that_imports_no_runtime_writes_none`
  (`tests/cli.rs`, with and without `-o`) and
  `an_emission_reports_the_support_modules_it_imports`
  (`tests/compile/cases_05.rs`).

## Issues and resolutions

### Issue 1: A support-module collision test relied on the syntactic guess

- **Symptom**: `a_source_cannot_claim_a_compiler_support_module_output`
  failed: the build succeeded.
- **Cause**: its `main.tt` pipes a literal (`1 |> twice`), which lowers to
  `twice(1)`. The output never imported the runtime, so the `tt/runtime.tt`
  input did not collide with anything the build needed.
- **Resolution**: the test pipes `input() |> twice`, whose output imports the
  runtime, so the collision the test is about still happens.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --test cli`, `cargo test --test compile`, full `cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/codegen/core/mod.rs`, `src/codegen/core/emitter/mod.rs`,
`src/codegen/core/emitter/expression.rs`, `src/codegen/rope.rs`,
`src/codegen/rope/builder.rs`, `src/lib/mapped.rs`, `src/lib/compile.rs`,
`src/engine/projection.rs`, `src/main/build.rs`, `tests/cli.rs`,
`tests/compile/cases_05.rs`, `docs/ai/tt.md` and the TASK-483 record.
