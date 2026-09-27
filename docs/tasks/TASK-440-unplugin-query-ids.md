# TASK-440: Preserve import queries on unplugin tt module ids

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`import W from "./w.tt?worker"` and `new Worker(new URL("./w.tt", import.meta.url))`
sent raw tt to Vite's JavaScript parser. `resolveId` accepted only specifiers
ending in `.tt`, and `sourceFileOfId` required the exact `.tt.ts` ending, so a
query such as Vite's `?worker_file&type=module` made the plugin ignore the
module.

## Scope

- Included: The `.tt`/`.ttx` module id scheme, query-aware resolution and
  loading, host resolution for dev-server urls, esbuild filters, regression
  tests, and the README.
- Excluded: The dependency scanner (TASK-441), source map anchoring
  (TASK-442), and standard-module ids (TASK-439).

## Decisions

### Decision 1: Keep the real file before the query and mark the module in the query

- **Context**: Vite handles query-suffixed imports by stripping the query with
  `cleanUrl` (`/[?#].*$/`) and working with the file path in front of it. The
  worker plugin bundles `cleanUrl(id)`, and in development it serves the
  worker script at `fileToUrl(cleanUrl(id)) + "?worker_file&type=module"`.
  The asset plugin reads `cleanUrl(id)` for `?raw` and `?url`. The old id
  `<file>.tt.ts` put a nonexistent file in front of any preserved query, so
  every one of these paths would read a file that does not exist.
- **Alternatives considered**:
  - Append the preserved query to `<file>.tt.ts`. The query survives, but
    `cleanUrl` still points at a file that does not exist on disk.
  - Use a `\0` id. The Rolldown plugin API (Virtual Modules Convention),
    which the Vite plugin guide cites, says that "modules directly derived
    from a real file, as in the case of a script module in a Single File
    Component … don't need to follow this convention" and that "using `\0`
    for these submodules would prevent sourcemaps from working correctly".
    The host TypeScript transforms also skip `\0` ids (TASK-439).
  - Follow the SFC sub-module shape: the real file plus a query whose last
    parameter carries the language (`?vue&type=script&lang.ts` in
    `@vitejs/plugin-vue`).
- **Decision and rationale**: A tt module id is
  `<file>?<original query params>&lang.ts` (`lang.tsx` for `.ttx`). Vite's
  TypeScript transform filter (`/\.(m?ts|[jt]sx)$/`, which the Vite `esbuild`
  and `oxc` options document as the default include) matches the id, and
  `cleanUrl` yields the real `.tt` file. `load` compiles only ids that end in
  the marker, and the marker is removed before it is added again, so
  resolving an id twice returns the same id.

### Decision 2: Leave non-module query forms to the host

- **Context**: The plugin is `enforce: "pre"`, so its `load` runs before
  Vite's asset and worker plugins. If it compiled `?raw` or `?url` ids, those
  documented forms would break.
- **Alternatives considered**: Compiling every query form would shadow Vite
  core features. Handling only an allowlist of queries would reject arbitrary
  user queries.
- **Decision and rationale**: `resolveId` returns `null` for the query forms
  that the Vite guide documents as asking for something other than the module:
  "Static Asset Handling" lists `?url` and `?raw`, and "Web Workers → Import
  with Query Suffixes" lists `?worker`, `?sharedworker`, `?worker&inline`,
  and `?worker&url`. The pattern matches Vite's own `SPECIAL_QUERY_RE`
  (`/[?&](?:worker|sharedworker|raw|url)\b/`). Vite then resolves the real
  file, and its plugins return the source text, the url, or the worker
  wrapper. The wrapper requests `?worker_file&type=module`, which the plugin
  compiles.

### Decision 3: Resolve through the host resolver when one exists

- **Context**: In development, Vite calls `resolveId` with urls such as
  `/src/w.tt?worker_file&type=module` after a restart or for worker scripts.
  A root-relative url is not a file system path.
- **Alternatives considered**: Converting root-relative urls inside the plugin
  would duplicate Vite's `root`, `fs.allow`, and alias rules.
- **Decision and rationale**: When the context provides `this.resolve` (Rollup
  plugin context, "this.resolve": "Resolve imports to module ids … using the
  same plugins that Rollup uses"), resolve the query-free file with
  `skipSelf` and mark the result. Otherwise (esbuild, the unit harness) keep
  the previous path arithmetic against `cleanUrl(importer)`. A side effect is
  that Vite's resolver now resolves relative imports inside a `.tt` file
  against that file's directory. Vite uses the importer's directory only when
  `cleanUrl(importer)` exists, and the old `.tt.ts` id did not exist.

## Work log

- 2026-09-27: Reproduced with `vite@6.4.3` in a scratch app. In development,
  the request `/src/w.tt?worker_file&type=module` returned raw tt. In build,
  both worker forms failed with
  `Expected ';', '}' or <eof>` until the plugin was also listed in
  `worker.plugins`, which the Vite docs require ("`config.plugins` only
  applies to workers in dev, it should be configured here instead for
  build"). With `worker.plugins`, both forms built.
- 2026-09-27: Implemented the id scheme, query-aware `resolveId`/`load`, and
  the esbuild filters. Updated the id expectations in `test/plugin.test.mjs`
  and added a query regression test. Against the previous `index.js`, it fails
  (`actual: ~` for the worker-file id). With the change, it passes.
- 2026-09-27: Checked end to end with `vite@6.4.3` and `vite@8.3.1`. In
  development, `/src/main.ts`, the worker script request, `?raw` (source text),
  `?url` (`/src/w.tt`), a cold `/src/lib.tt?lang.ts`, and a `.tt` file that
  imports `./util.ts` from its own directory all worked. Worker builds for both
  forms, with `worker.plugins`, produced compiled worker chunks. With
  `rollup@4.63.5` and `esbuild@0.28.2`, bundles built, and the esbuild output
  ran.

## Issues and resolutions

### Issue 1: Rollup names chunks after the query-bearing id

- **Symptom**: The worker chunk is named `w.tt_lang-<hash>.js` (previously
  `w.tt-<hash>.js`).
- **Cause**: Rollup derives a default chunk name from the id's basename
  without its extension and sanitizes `?`.
- **Resolution**: Accepted as cosmetic. The README explains how to name entries
  with an input object.

## Verification

- [x] `TTC_BINARY=target/debug/ttc npm --prefix integrations/unplugin test`:
  7 tests pass.
- [x] `node scripts/check-task-index`
- [x] `./scripts/ci agents`
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
  `cargo test`: not run for this change. No Rust source changed.

## Result

Changed `integrations/unplugin/index.js`, `integrations/unplugin/README.md`,
and `integrations/unplugin/test/plugin.test.mjs`. `.tt` modules imported with
a query now resolve and compile according to Vite's query conventions. Worker
imports work in development, and in build when the plugin is also listed in
`worker.plugins`. Raw and url imports return the file itself.
