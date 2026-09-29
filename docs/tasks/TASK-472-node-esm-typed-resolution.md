# TASK-472: The typed engine resolves `.tt` specifiers through a content mapper, as `tsc` does

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`ttc --check-types` failed on ES-module projects under `module`/`moduleResolution` `node16`/`nodenext`. With `package.json` `"type": "module"`, a `src/c.ts` importing `"./a.tt"` reported `ts2307 Cannot find module './a.tt'`. `tsc -p . --runExternalCode`, which uses the TypeScript 7.1 content mapper, accepted the same project. The typed engine must resolve `.tt`/`.ttx` specifiers as `tsc` does, in every `moduleResolution` mode, without changing the user's compiler options or TypeScript's verdict on the user's code. The `@tt/std` half of the report is TASK-473.

## Scope

- Included: How the TypeScript backend's host (`src/typescript/host.mjs`) presents lowered modules to tsgo, and the regression tests in `tests/native/cases_03.rs`.
- Excluded:
  - The engine's own module identity. It still names a lowered module `x.tt.ts` (`engine::projection::module_path_of`), and the host translates at its boundary, so no tsgo concept leaves `src/typescript/`.
  - The editor language-service session (`src/engine/language/`), which drives a tsgo language server over the real disk.

## Decisions

### Decision 1: Open configured projects through an identity content mapper

- **Context**: The engine served `src/a.tt` to tsgo as the file `src/a.tt.ts` and relied on TypeScript appending `.ts` to `./a.tt`.
  - TypeScript's module reference says ES-module files under `node16`/`nodenext` must write relative imports with their extension, and resolution does not add one ([Modules reference: `node16`, `nodenext`](https://www.typescriptlang.org/docs/handbook/modules/reference.html#node16-nodenext); [Relative file path resolution](https://www.typescriptlang.org/docs/handbook/modules/reference.html#relative-file-path-resolution)). Extension probing happens only for CommonJS files and `bundler`.
  - `tsc --runExternalCode` resolves `./a.tt` in every mode because `contentMappers[].extensions` makes `.tt` a supported extension (`docs/design/content-mapper.md`). `./a.tt` is then an exact file name.
  - The tsgo API client runs content mappers when created with `runExternalCode: true` (`dist/api/options.d.ts`).
- **Alternatives considered**:
  - A declaration file `a.d.tt.ts` per module plus `allowArbitraryExtensions` forced on the project: the first version of this task. It was rejected because it changes the user's compiler options: TS6263 disappears for the user's own `x.d.css.ts` files.
  - The same declaration file without the option: TypeScript reports TS6263 on every import of a `.tt` file.
  - Rewriting specifiers to `./a.tt.js`: this breaks the source-preservation contract.
  - Running `ttc --content-mapper` as the mapper: every file would be lowered again in another process, with the CLI's options instead of the engine's (`defer_to_checker`, materialized contextual annotations). Every probe position would then have to be translated through a second span map.
- **Decision and rationale**: With a configuration open, the host serves each lowered module's text as the content of the `.tt` file itself.
  - **The mapper.** The root configuration's `contentMappers` becomes one entry for `.tt`/`.ttx` naming `@tt/typed-engine-mapper`. That mapper is an identity mapper: it returns the content unchanged, with a `.ts`/`.tsx` virtual extension and one verbatim span in UTF-16 units. The virtual text and every position in it are therefore exactly the lowered module's, as before.
  - **What changes.** Resolution becomes TypeScript's own for a supported extension. `compilerOptions` are untouched, and so is the user's source text.
  - **Where the package lives.** The mapper package lives in the host's own temporary session directory, deleted with the session. It is reached from `<config dir>/node_modules/@tt/typed-engine-mapper` through the layered file system's `realpath`, the same way a symlinked package is. Nothing is written to the user's tree.
  - **Why `realpath`.** Measured: tsgo spawns the mapper with the package directory as its working directory, so the directory must exist on disk. The CLI does not resolve an absolute path as a `package`, only a name found through `node_modules` lookup.
  - **Engine names.** Diagnostics, project membership, questions, and declaration paths are translated between the engine's `x.tt.ts` and TypeScript's `x.tt` at the host boundary (`served`, `moduleName`, `addressed`). Declaration emit names `x.tt`'s output `x.d.tt.ts`, which becomes the engine's `x.tt.d.ts`.
  - **Unserved sources.** A `.tt` file the engine did not serve does not exist for TypeScript (`fileExists` → `false`, hidden from listings). Before, TypeScript could never see one either.

### Decision 2: Keep the previous arrangement where running external code would change the result

- **Context**: `runExternalCode` is a per-process switch, not a per-mapper one.
  - A configuration that already names a content mapper for another extension would have that mapper executed too. Before this task, ttc never executed it, and tsgo reported TS100024 ("Content mappers require the '--runExternalCode' command line flag") exactly as `tsc -p .` does.
  - An inferred project (no `tsconfig.json`) has no configuration to name a mapper in.
- **Alternatives considered**:
  - Run the user's other mappers as well: this executes external code the user never opted into through ttc, and changes those projects' results.
  - Strip the other mappers: this silently drops TS100024, which changes TypeScript's verdict on the configuration.
- **Decision and rationale**: The host reads the effective configuration with `parseConfigFile`, whose `raw.contentMappers` includes inherited entries (measured with an `extends` chain).
  - **Any other extension.** If any mapper entry covers an extension other than `.tt`/`.ttx`, the project keeps the previous arrangement. Modules are served as `x.tt.ts`, `.tt` entries are stripped as before, and `runExternalCode` stays off.
  - **Inferred projects** keep it too.
  - **Otherwise**, the host reconnects with `runExternalCode: true` and the identity mapper.
  - **Measured.** Both kept cases produce byte-identical output to the build before this task.
- **Consequence**: An ES-module project that also configures a non-tt content mapper still cannot resolve `./x.tt` under `node16`/`nodenext` in `ttc --check-types`. Its behavior is unchanged, not regressed. It remains a known limitation.

### Decision 3: A root outside the configuration keeps its inferred project

- **Context**: A requested `.tt` file outside the configuration is opened in its inferred project (`a_requested_file_outside_the_configuration_is_checked_in_its_inferred_project`). That project names no content mapper, so `other/x.tt` would not be a TypeScript file there.
- **Decision and rationale**: When mapped, each lowered module is also served under its engine name `x.tt.ts`, unlisted, so no `include` glob selects it. Outside roots are opened under that name, and their imports reach `y.tt.ts` by extension probing, exactly as before. Measured in `bundler`, `nodenext` (ESM and CommonJS), and `commonjs`: the configured program never contains an alias, because `./a.tt` resolves to the exact `.tt` file first.

## Work log

- 2026-09-28: First version (a `x.d.tt.ts` declaration per module, plus `allowArbitraryExtensions` forced on) was rejected in review, because it changes the user's compiler options. It was reworked here.
- 2026-09-28: Spike through the tsgo API (`runExternalCode`, layered file system, identity mapper):
  - `./a.tt` resolves under `nodenext` ESM.
  - `getTypeAtPosition` and diagnostics on `a.tt` use the lowered text's positions.
  - `getDeclarationEmit` writes `a.d.tt.ts`.
  - Initialization requires `diagnosticSource`.
  - The mapper package directory must exist on disk.
  - `parseConfigFile(...).raw` carries inherited `contentMappers`.
- 2026-09-28: Implemented in `src/typescript/host.mjs`:
  - the identity mapper and its package in the session directory, reached through `realpath`;
  - arrangement selection and reconnection;
  - boundary name translation;
  - hidden unserved `.tt` files;
  - engine-served configurations naming modules by the served name;
  - aliases for roots outside the configuration.
- 2026-09-28: Compared against the build before this task (the hunter's `ttc`) on a project with `a.tt` (variant plus default export), `c.ts` and `d.tt` importing `./a.tt`, a `.ttx`, and `@tt/std`.
  - **Unchanged.** Output is byte-identical for `node16`/`nodenext` CommonJS, `esnext`/`bundler` ± `verbatimModuleSyntax`, `preserve`/`bundler`, and `commonjs`/`bundler`. The same holds for a project without `tsconfig.json` and for a project configuring a `.foo` mapper (TS2307 plus TS100024, as before).
  - **Changed.** `node16`/`nodenext` ESM (± `verbatimModuleSyntax`) went from TS2307 on `./a.tt` to the same single intended type error that `tsc --runExternalCode` reports.
  - **Hunter projects.** `p6`, `p7`, and `p10` check clean.
- 2026-09-28: Tests in `tests/native/cases_03.rs`:
  - `tt_specifiers_resolve_under_node_esm_as_tsc_resolves_them`
  - `tt_specifiers_keep_resolving_in_commonjs_and_bundler_projects`, which includes a CommonJS `export =` module imported with `import … = require("./legacy.tt")`
  - `a_project_with_another_content_mapper_runs_no_external_code`

## Issues and resolutions

### Issue 1: The identity mapper failed to initialize

- **Symptom**: `ts100057 The content mapper '@tt/typed-engine-mapper' could not be initialized`.
- **Cause**: The `initialize` answer carried only `positionEncoding`. tsgo also requires `diagnosticSource`, as `ttc --content-mapper` answers (`src/content_mapper.rs`).
- **Resolution**: The mapper answers `{ positionEncoding: "utf-16", diagnosticSource: "tt" }`.

### Issue 2: Contextual materialization lost its inferred slot types

- **Symptom**: `a_std_program_is_typed_without_the_package_on_disk` and `contextual_support_respects_ancestor_standard_packages` (`tests/cli.rs`) failed with "missing inferred slot type".
- **Cause**: The contextual pass serves its own configuration, whose `files` names the requested module by the engine's `x.tt.ts`, which no longer exists when the project is mapped.
- **Resolution**: In a configuration the engine serves itself, `files`/`include`/`exclude` entries are translated with `served`. User configurations are never renamed when mapped, because their patterns already name `.tt`.

### Issue 3: A requested file outside the configuration was not checked

- **Symptom**: `a_requested_file_outside_the_configuration_is_checked_in_its_inferred_project` exited 0 instead of 1.
- **Cause**: The file's inferred project has no content mapper, so `x.tt` is not a TypeScript file there.
- **Resolution**: Decision 3.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `./scripts/ci extension` (207 server tests) and `./scripts/ci npm`

## Result

Changed `src/typescript/host.mjs`, the `module_path_of` documentation in `src/engine/projection.rs`, `tests/native/cases_03.rs`, `docs/design/tsgo-native-backend.md`, supersession notes at the top of TASK-074 and TASK-079, this record, and `docs/tasks/INDEX.md`.

`ttc --check-types` now resolves relative `.tt`/`.ttx` specifiers exactly as `tsc --runExternalCode` does, in every `moduleResolution` mode. It does so by the same content-mapper mechanism, with no compiler option changed and no diagnostic filtered.

Projects whose configuration names another content mapper, and inferred projects, behave exactly as before. For the first, that includes the remaining ESM limitation.
