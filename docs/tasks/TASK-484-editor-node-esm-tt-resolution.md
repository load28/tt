# TASK-484: The editor service resolves `.tt` imports under node16/nodenext ES modules through the installed mapper

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

The editor language service (`src/engine/language/`, a `tsgo --lsp` server over the real disk) could not resolve `./x.tt` under `module`/`moduleResolution` `node16`/`nodenext` with `package.json` `"type": "module"`.

- `ttc --server` `tsDiagnostics` on `src/main.tt` gave `2307 Cannot find module './x.tt'`.
- Hover in `src/use.ts` gave the same error.
- `ttc --check-types` and `tsc --runExternalCode` gave the intended `ts2322` in the same project.

The cause: `Project::serve` served every import as the document `x.tt.ts`, and ES-module resolution does no extension probing. TASK-472 fixed the same gap for the typed engine and left the editor service out of scope.

## Scope

- Included:
  - How the editor service reaches tsgo: `src/typescript/service.rs` (arrangement, initialization, document names).
  - The host job that reports the effective configured mappers (`src/typescript/host.mjs`, `src/typescript/native.rs`).
  - Document naming and answer mapping in `src/engine/language/`.
  - Regression tests: `tests/native/cases_04.rs`, `src/typescript/service.rs` unit tests, and an extension server test in `editors/vscode/server/src/test/engine.test.ts`.
- Excluded:
  - Projects whose configuration does not name `@openload28/tt-lang` (Decision 2).
  - The typed engine's own arrangement (TASK-472), which is unchanged.

## Decisions

### Decision 1: Reuse the user's installed `@openload28/tt-lang` mapper where the configuration names it

- **Context**: The typed engine serves lowered text through an identity mapper that it places in its own session directory, reached through its layered file system. A previous investigation measured four things about the language server:
  - It accepts `initializationOptions: { "runExternalCode": true }`.
  - It reads a mapper package only from the disk: an opened `package.json` document is ignored.
  - `custom/setContentMapperContributions` applies only to inferred projects.
  - An opened `tsconfig.json` document overrides the file on disk.
- **Alternatives considered**:
  - **Write an identity mapper package into the user's `node_modules`.** Rejected: it writes into the user's tree.
  - **Serve every document under its own name in every project.** Rejected: it changes projects that work today, and without a configured mapper `.tt` is not a TypeScript file.
  - **Option B (chosen by the user).** Reuse the installed `@openload28/tt-lang` mapper. It lowers the already-lowered TypeScript again, and by the passthrough contract the output is byte-identical with verbatim mappings.
- **Decision and rationale**: A project's service is *mapped* only when all of these hold:
  - the effective `contentMappers` of its configuration (inherited entries included) are non-empty;
  - every entry names `@openload28/tt-lang` for `.tt`/`.ttx` only;
  - the package, with a string `typescript.contentMapper.exec`, is found in `node_modules` at or above the configuration's directory.

  **Where the configuration is read.** The effective list comes from the typed backend's host, through a new `{"configuredMappers": true}` job. The host answers with `api.parseConfigFile(tsconfig).raw.contentMappers` and sets its own configuration rewrites aside while it reads. That makes it the same parse TASK-472 bases its arrangement on. Measured: `parseConfigFile` re-reads the file system on every call, so the answer is the disk's.

  **What mapped means.**
  - The server is initialized with `runExternalCode: true`.
  - The package's manifest is registered as the inferred-project contribution, so a `.tt` file outside the configuration still answers.
  - Each `.tt` document is opened under its own name, with the lowered text as its content.
  - Answers are mapped back through the same projection. `map_target` and `map_shared_target` accept both a `.tt`/`.ttx` URI and an `x.tt.ts`/`x.ttx.tsx` URI.

  **Where tsgo knowledge stays.** The decision and the naming live in `src/typescript/service.rs` (`Arrangement`, `Service::document_uri`). The engine passes only the source path, the lowered path, and whether the text passes through.

### Decision 2: No `tsconfig.json` override for a configuration that does not name the mapper

- **Context**: In `p3`, the installed package is present but `tsconfig.json` names no mapper. An opened `tsconfig.json` document that adds the `contentMappers` entry would make `./x.tt` resolve there too.
- **Measured** (scratch project `esm-not-named`, `node16` ESM):
  - `tsc -p .` and `tsc -p . --runExternalCode` both report `TS2307` on the hand-written `src/use.ts` and `src/ns.ts`.
  - With the overlay, the language server reports `TS2322` on `src/use.ts` instead.
  - The overlay therefore changes a diagnostic that TypeScript itself reports on the user's own TypeScript, under the user's own configuration.
  - It would also stand in for the file on disk for the whole session.
- **Decision and rationale**: The overlay is rejected. The fix is limited to configurations that already name the mapper. In `p3`, the editor keeps reporting what the user's `tsc` reports (`TS2307`).

  `ttc --check-types` reports `ts2322` in the same project, because the typed engine's identity mapper applies to every configuration (TASK-472). The two surfaces disagree there, and this record leaves that disagreement standing.

### Decision 3: A document whose lowering does not reproduce it is served as `x.tt.ts`

- **Context**: The installed mapper recompiles whatever it is given. A mid-edit buffer or a completion probe can lower to text that does not parse. Measured: served under its own name, `const t = Shape.` came back as an empty module with the mapper's `tt30` diagnostic, and hover answered `null`.
- **Alternatives considered**: Serving every document under its own name. Rejected: it loses answers in exactly the states completion is asked in.
- **Decision and rationale**: Before a document is opened under its own name, `lowering_reproduces` compiles the served text as the mapper does:
  - the output must equal the input;
  - there must be no error diagnostic.

  Otherwise the document is opened as `x.tt.ts`, and the previous name is closed. Measured: an open `x.tt.ts` document matching `include` joins the configured project, so it is answered under the user's options. The check runs only in the mapped arrangement (the closure is never called otherwise).

## Work log

- 2026-09-28: Reproduced with `ttc --server` on a scratch ESM `node16` project with the mapper configured and installed:
  - `tsDiagnostics` gave 2307 on `src/main.tt` and `src/use.ts`;
  - hover in `use.ts` answered `null`;
  - `ttc --check-types` and `tsc --runExternalCode` gave ts2322.
- 2026-09-28: Probed `tsgo --lsp` directly:
  - `runExternalCode` plus lowered text opened under `.tt` names gives 2322 at the lowered positions;
  - `custom/projectInfo` reports "no project found" for an out-of-configuration `.tt`;
  - an inferred-project contribution naming the same manifest answers it;
  - an opened `tsconfig.json` overlay turns `use.ts`'s TS2307 into TS2322 (Decision 2).
- 2026-09-28: Implemented:
  - `configuredMappers` in `host.mjs`;
  - `NativeBackend::configured_mappers`;
  - `Arrangement` (`of_configuration`, `of_project`);
  - `Service::start` taking the arrangement, and `Service::document_uri`;
  - `.ttx` URIs opening as `typescriptreact`;
  - `Project::service_arrangement`;
  - `open_served`, `lowering_reproduces`, `served_uri` reading the session's recorded name, and `tt_document` in `src/engine/language/`.
- 2026-09-28: Compared answers of the previous build and this one with a scratch battery. The battery covered `tsDiagnostics` on three files, seven hovers (including the import specifier and a namespace import), two definitions, references, completion, signature help, and rename.
  - **Byte-identical.** Not configured, not installed, foreign mapper, no `tsconfig.json`, and `bundler` not configured.
  - **Same answers, one ordering change.** `bundler`, `preserve`, `node16` CommonJS, and `commonjs` with the mapper configured and installed differ only in the order of cross-file references (Issue 2).
  - **Fixed.** `node16`/`nodenext` ESM with the mapper configured went from 2307 and `null` answers to the answers `tsc --runExternalCode` gives.
- 2026-09-28: Added the regression tests listed in Scope. Before the source change, three of the new native tests failed: node ESM resolution, the server test, and the broken-buffer test. The other three pin unchanged behavior.

## Issues and resolutions

### Issue 1: Hand-written files were reopened under a lowered name

- **Symptom**: In the first run of the comparison battery, references in `bundler` and no-`tsconfig.json` projects listed `src/ns.ts.ts`.
- **Cause**: `open_served` computed the lowered name for every projected file, including host `.ts` overlays, which `served_uri` had always served under their own name.
- **Resolution**: `open_served` serves a host source under its own path, exactly as before.

### Issue 2: Cross-file references come back in a different order in mapped projects

- **Symptom**: In `bundler`/CommonJS projects that configure the mapper, `references` returns the same locations, but `main.tt` precedes `use.ts`.
- **Cause**: The order is TypeScript's program file order. Mapped, `.tt` files are root files listed from the disk by `include`. Before, they were open `x.tt.ts` overlays added after the disk files.
- **Resolution**: Accepted and recorded. This is the order `tsc --runExternalCode` builds the program in. `a_configured_installed_mapper_changes_no_answer_where_extension_probing_already_resolved` compares references as a sorted set and every other answer byte for byte.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `./scripts/ci extension` (209 server and client tests, 0 skipped)
- [x] `./scripts/ci npm`

## Result

Changed files:
- `src/typescript/service.rs`, `src/typescript/native.rs`, `src/typescript/host.mjs`
- `src/engine/project.rs`, `src/engine/language.rs`, `src/engine/language/project.rs`, `src/engine/language/service.rs`
- `tests/native.rs`, `tests/native/cases_04.rs`
- `editors/vscode/server/src/test/engine.test.ts`
- `docs/design/tsgo-native-backend.md`, `docs/design/content-mapper.md`
- this record and `docs/tasks/INDEX.md`

Outcome:
- In a `node16`/`nodenext` ES-module project whose configuration names the installed `@openload28/tt-lang` mapper, the editor service now gives the answers of `tsc --runExternalCode` and `ttc --check-types`, at identical positions.
- Every other configuration keeps the previous arrangement.

Remaining limitations:
- A project whose configuration does not name the mapper still reports TS2307 there, as its own `tsc` does (Decision 2).
- The decision is taken once per service session from the project's own configuration. Another configuration in the same workspace does not take part in it.
