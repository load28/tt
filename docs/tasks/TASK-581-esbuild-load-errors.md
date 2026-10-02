# TASK-581: Report esbuild load errors with their watch files

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: the `TASK-581` commit on this branch

## Purpose

With the esbuild adapter, a `.tt` module that failed to compile reported only
`Do not know how to load path: @openload28/unplugin-tt:/…/bad.tt?lang.ts`,
and in `ctx.watch()` fixing the source never rebuilt: the compiler's
diagnostic and the module's watch files were lost.

## Scope

- Included: the esbuild adapter loads tt modules through esbuild's own
  `onLoad`; one `compileModule` shared by every host, so each registers a
  failed module's dependencies before reporting its error; tests; README.
- Excluded: the other hosts' error paths, which already reach the build (see
  Decision 2).

## Decisions

### Decision 1: tt modules load through esbuild's own `onLoad` on esbuild

- **Context**: The shared `load` called `this.error(message)` and returned
  `null`. unplugin's esbuild bridge (unplugin 2.3.11,
  `getEsbuildPlugin`/`buildSetup`) collects `this.error` into an `errors`
  array but returns `null` when the hook produced no code, so esbuild saw no
  answer at all: no errors, no `watchFiles`, and the path's namespace has no
  default loader. Throwing from `load` instead would reach esbuild as a plugin
  error, but a callback that throws has no result, so its `watchFiles` are
  lost and a watch still never looks at the source again (the id lives in the
  plugin's namespace, which esbuild does not watch by itself).
- **Alternatives considered**: (a) Throw from `load` for every host: loses
  esbuild's watch files, and in webpack the unplugin loader awaits the hook
  after calling `this.async()`, so a rejection is never passed to the
  callback (an unhandled rejection). (b) Patch around the bridge by returning
  placeholder code with the error: a fallback that would make esbuild bundle
  something for a module that did not compile.
- **Decision and rationale**: esbuild's plugin API defines an `onLoad` result
  with `errors`, `watchFiles` and `watchDirs` and no `contents` for exactly
  this case. The adapter's `esbuild.setup` receives the raw build and
  registers `onLoad({ filter: TT_MODULE_ID, namespace: PLUGIN_NAME })`, which
  answers either `{ contents, loader, resolveDir, watchFiles, watchDirs }` or
  `{ errors, watchFiles, watchDirs }`. The unplugin `onLoadFilter` narrows to
  the standard library's virtual ids, which never fail. The map is detached
  and written back inline so its `sources` stay absolute, as the bridge did.
  This also gives esbuild the directories of TASK-580 as `watchDirs`.

### Decision 2: Other hosts keep their documented error paths

- **Context**: The task asked for every bundler to be checked.
- **Decision and rationale**: Rollup, Rolldown, Vite and Farm: `this.error`
  throws, and Rollup attaches the files registered so far to the error
  (`error.watchFiles`), so its watch keeps them. webpack and Rspack:
  unplugin's `this.error` is the loader's `emitError`, measured with webpack
  5.111 to report the diagnostic; the files registered through
  `addDependency`/`fileDependencies` keep the module watched. The shared
  `compileModule` now returns the dependencies it learned together with the
  error, so every host registers them before the error is raised.

## Work log

- 2026-09-29: Reproduced with a scratch copy of the finder's `probe4-cli/eb`
  scripts against the worktree plugin: `esbuild.build` failed with only
  `Do not know how to load path`, and `ctx.watch()` built once and never
  again after the source was fixed.
- 2026-09-29: Added `compileModule`, `inlineSourceMap`, `TT_MODULE_ID`,
  `PLUGIN_NAME` and the esbuild `onLoad` (`integrations/unplugin/index.js`).
  The probe then reported
  `error[match-not-exhaustive]: … missing "B"` from `@openload28/unplugin-tt`,
  rebuilt after the fix, and rebuilt again after a file was added to the
  `include` directory (`watchDirs`). webpack still reports the diagnostic.
- 2026-09-29: Tests: an `esbuildHost` helper drives the published esbuild
  adapter through unplugin's real esbuild bridge with esbuild's callback
  rules (registration order, namespace, first answer wins). README.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `TTC_REQUIRE_TSGO=1 cargo test`: no Rust change here; the full run is
  recorded in TASK-583
- [x] `TTC_BINARY=<worktree>/target/debug/ttc npm --prefix integrations/unplugin test`:
  18 passed. The new `esbuild reports the compiler diagnostic and watches the
  source until it compiles` and `esbuild watches the directories a module
  listed as directories` fail with the TASK-580 `index.js` (no callback
  answers the load) and pass with this change.

## Result

Changed files: `integrations/unplugin/index.js`,
`integrations/unplugin/test/plugin.test.mjs`,
`integrations/unplugin/README.md`, `docs/tasks/INDEX.md`, this record.

esbuild builds report ttc's diagnostic, watch the module's files and
directories whether or not it compiled, and rebuild once it is fixed.
