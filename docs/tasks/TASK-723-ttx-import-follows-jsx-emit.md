# TASK-723: Name a `.ttx` import's output as `tsc` names it under the project's `jsx` option

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-723`

## Purpose

With the default `--rewrite-imports js`, ttc rewrote `./view.ttx` to
`./view.jsx` whatever the project's `jsx` option. ttc emits `view.tsx`, and
under `"jsx": "react-jsx"` (and `react`, `react-jsxdev`, `react-native`)
`tsc` writes it as `view.js`, so the compiled program failed at runtime
with `ERR_MODULE_NOT_FOUND`. `docs/ai/tt.md` §Modules documented the
`.jsx` spelling; the round-8 probe found the gap between it and
TypeScript's emit.

## Scope

- Included: the rewrite's extension for a `.ttx` specifier
  (`src/lib/api.rs`, codegen's import emission), a library option
  (`Options::jsx_preserve`), reading the option from the project's
  configuration (`src/engine/config.rs`, `ttc::engine::jsx_preserve`), the
  CLI build and `-p` (`--project` now accepted there), the server's
  `print`, the sidecar's restoration of `.ttx` specifiers
  (`ImportRewrite::source_candidates`), `docs/ai/tt.md`, the CLI help,
  the website copy, `CHANGELOG.md`, and tests.
- Excluded: `--rewrite-imports ts` (TypeScript's own
  `rewriteRelativeImportExtensions` names the output) and `off`; `--watch`
  reads the option once, when it starts.

## Decisions

### Decision 1: `.ttx` becomes `.jsx` only under `"jsx": "preserve"`

- **Context**: TypeScript names an output by
  `GetOutputExtension(fileName, jsx)` (typescript-go
  `internal/outputpaths/outputpaths.go`; `getOutputExtension` in
  TypeScript's `emitter.ts`): `.jsx` when `jsx == JsxEmitPreserve` and the
  input is `.tsx`/`.jsx`, `.mjs`/`.cjs` for `.mts`/`.cts`, otherwise `.js`.
  The TSConfig reference ("jsx") lists `preserve` as emitting `.jsx` files
  and `react`, `react-jsx`, `react-jsxdev`, and `react-native` as emitting
  `.js` files. Checked with the pinned `tsc` (7.1.0-dev.20260826.1) on one
  `.tsx` file per value: `preserve` and `Preserve` wrote `view.jsx`; the
  four others and an unset option wrote `view.js` (the option's value is
  case-insensitive, as for every enumerated option).
- **Alternatives considered**: (a) Keep `.jsx` and document it (the state
  before): a program built under any value but `preserve` does not run.
  (b) Always write `.js`: wrong under `preserve`, whose `.jsx` output a
  later tool transforms. (c) Write the extension `tsc` will write, from
  the project's option.
- **Decision and rationale**: (c). `ImportRewrite::Js` keeps its meaning
  ("the JavaScript file the output becomes") and reads `Options::jsx_preserve`
  for `.ttx`; the library default is TypeScript's (unset, `.js`). Codegen
  receives the resolved pair of extensions (`RewrittenExtensions`), so the
  emitter has one path for `js` and `ts`.

### Decision 2: The CLI reads `jsx` from the project's configuration as TypeScript does

- **Context**: A plain build needs no TypeScript, so the option cannot be
  asked of the compiler; the build must read it from the same
  `tsconfig.json` the user's `tsc` will use.
- **Alternatives considered**: (a) A `--jsx` flag: a second place to state
  what the configuration already states, which drifts. (b) Ask the
  TypeScript backend: spawns the compiler for every build and fails where
  no TypeScript is installed. (c) Read `compilerOptions.jsx` from the
  configuration `--project` names, or else the nearest `tsconfig.json` at
  or above the inputs (the discovery `--check-types` already uses), in
  JSON with comments and trailing commas, through `extends` as
  TypeScript's `getExtendsConfigPath` resolves it (a rooted or `./`/`../`
  path with `.json` added when needed, or a package in `node_modules`
  read as a file, a file plus `.json`, its `package.json` `tsconfig`
  field, or its `tsconfig.json`; a list whose later entry wins, the
  file's own options winning over all).
- **Decision and rationale**: (c), in the engine
  (`src/engine/config.rs`), beside the configuration discovery it already
  owns. Package `exports` maps are not followed: such an `extends` cannot
  be found. A configuration that cannot be read fails a build that has a
  `.ttx` source, with a message that names the remedies (`--project`,
  `--rewrite-imports ts|off`); any other build does not need the answer
  and keeps TypeScript's default, so no existing `.tt`-only build breaks.

### Decision 3: A `.js` specifier in declarations may come from a `.ttx`

- **Context**: `--sidecar` and `--types` restore tsc's declaration
  specifiers to the source spelling with `ImportRewrite::source_specifier`,
  which mapped `.js` only to `.tt`.
- **Decision and rationale**: `ImportRewrite::source_candidates` lists
  every source a rewritten specifier can come from (`./m.js` → `./m.tt`,
  `./m.ttx`), and the sidecar takes the first that names a source;
  `source_specifier` keeps its answer (the first candidate).

## Work log

- 2026-10-01: Read typescript-go's `GetOutputExtension`; confirmed each
  `jsx` value with the pinned `tsc` (above).
- 2026-10-01: Implemented the rewrite, `Options::jsx_preserve`,
  `RewrittenExtensions`, `source_candidates`, `ttc::engine::jsx_preserve`,
  `project_jsx_preserve` (`src/main/build.rs`) and its use in
  `src/main/command.rs` and `src/server.rs`; accepted `--project` in build
  and `-p` modes; updated the help, `docs/ai/tt.md`, the website copy, and
  `CHANGELOG.md`.
- 2026-10-01: Added `tests/cases/compiler/ttxImportNamesTheOutputTscWrites.tt`
  (`@run`, `react-jsx`, a static and a dynamic `.ttx` import), the CLI tests
  `a_ttx_import_names_the_output_tsc_writes_under_the_projects_jsx_option`
  (each `jsx` value, `extends` by path with comments and by packages, and
  `--project`; the tree compiled by the pinned `tsc`, the specifier naming
  a file it wrote, and the program run by Node.js) and
  `an_unreadable_jsx_option_fails_only_a_build_that_has_a_ttx_source`
  (`tests/cli.rs`); a `.ttx` sidecar specifier in
  `sidecars_name_tt_modules_as_the_source_does` (`tests/cli_outputs.rs`);
  `jsx_preserve` in the two `tests/compile` rewrite tests; and a unit test
  of the configuration text reader. Updated
  `modes_reject_options_they_would_otherwise_ignore`, whose build-mode
  `--project` rejection is now `--check`'s.
- 2026-10-01: `UPDATE_EXPECT=1 cargo test --test public_api`: the API
  baseline adds `ttc::engine::jsx_preserve`,
  `ImportRewrite::source_candidates`, and `Options::jsx_preserve`.

## Issues and resolutions

### Issue 1: `tsc` refuses a `.tsx` module when `jsx` is unset

- **Symptom**: The CLI test's "unset" project failed `tsc` with TS6142
  ("Module './view.js' was resolved to '…/view.tsx', but '--jsx' is not
  set").
- **Cause**: TypeScript does not compile a `.tsx` module without a `jsx`
  option, whatever the specifier.
- **Resolution**: That configuration checks the specifier only.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/ttxImportNamesTheOutputTscWrites.tt`;
  `tests/cli.rs`
  `a_ttx_import_names_the_output_tsc_writes_under_the_projects_jsx_option`
- **Observed failure**: with the non-test changes reverted, the case's
  `.ts` and `.stdout` baselines were out of date: the output imported
  `./view.jsx`, and the program failed (`.stderr`: `node main.js (exit 1)`,
  `ERR_MODULE_NOT_FOUND`); the CLI test failed at its `react` project,
  whose `out/app.ts` imported `./view.jsx`.

## Verification

- [x] `cargo test --test cli --test cli_outputs --test compile --test
  public_api --test snapshot`: pass
- [x] `cargo test --doc ImportRewrite`, `cargo test --lib config`: pass
- [x] `TT_CASES=ttxImportNamesTheOutputTscWrites,noSubstitutionTemplateDynamicImportIsRewritten,yieldCrossingResultInTemplateOrJsx,variant-default-export
  cargo test --test case_baselines`: pass (the default case configuration
  preserves JSX, so existing baselines keep `.jsx`)
- [x] The full gate, run once for TASK-719 to TASK-725 (see TASK-725)
- [x] Baseline changes reviewed and committed with the change

## Result

A `.ttx` import is emitted as the file `tsc` writes for it under the
project's `jsx` option, so a tree built for `react-jsx` runs.
