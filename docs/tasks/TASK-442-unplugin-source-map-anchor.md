# TASK-442: Anchor unplugin source map sources to the tt file

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

With the esbuild adapter, a bundle written to `dist-es/out.mjs` listed the tt
source as `lib.tt`. Stack frames and debuggers resolved that to `dist-es/lib.tt`
instead of `src/lib.tt`. `ttc -p --source-map inline` names the source
relative to the compiled file, and `detachInlineSourceMap` passed the map to
the host without the location that relative name depends on.

## Scope

- Included: Anchoring `sources` (and folding `sourceRoot`) in the map that
  the shared `load` hook returns, tests, and the README.
- Excluded: ttc's own source map output, which is correct for the files it
  writes, and the standard modules, which return no map.

## Decisions

### Decision 1: Resolve sources to absolute paths in the plugin, not in ttc

- **Context**: ECMA-426 (Source Map Format, §9 "sources" and "sourceRoot",
  §9.3.1 DecodeSourceMapSources) defines each source as a possibly relative
  URL. `sourceRoot` is prepended to it, and the result is resolved against the
  source map's URL. For `-p`, ttc has no output location, so `lib.tt` means
  "beside the compiled file". A map detached from its inline data URL has no
  URL of its own, so each host picks a base: Rollup and Vite use the module
  id's directory, while esbuild, for a module in a plugin namespace, keeps the
  name and emits it relative to the output file.
- **Alternatives considered**:
  - Change ttc's `-p` map to absolute paths. That changes a compiler output
    contract that other consumers and snapshots rely on, to fix a problem
    that exists only after the plugin takes the map out of its context.
  - Emit `file://` URLs. These are valid ECMA-426 URLs, but Rollup composes
    sources with `path.resolve(dirname(id), sourceRoot, source)`, which would
    corrupt them. Vite's development server relativizes only sources for
    which `path.isAbsolute` is true.
  - Set `sourceRoot` to the file's directory and keep relative sources. That
    depends on every host honouring `sourceRoot` in load-hook maps.
- **Decision and rationale**: The plugin knows the file that ttc compiled, so
  `detachInlineSourceMap` resolves every non-null source against
  `path.resolve(dirname(file), sourceRoot ?? "")` and drops `sourceRoot`.
  That applies ECMA-426's resolution with the compiled file as the base and
  gives every adapter an absolute file path. Rollup and Vite then emit
  output-relative paths as before (`../../src/lib.tt`). Vite development
  relativizes the path to the module (`lib.tt`, served beside
  `/src/lib.tt`). esbuild emits the absolute path, which resolves to the
  real file. `null` sources stay `null`, as ECMA-426 permits. `sourcesContent`
  is unchanged.

## Work log

- 2026-09-27: Reproduced with `esbuild@0.28.2`: `dist-maps/es/out.mjs.map`
  listed `lib.tt`, while Rollup and Vite listed `../../src/lib.tt`.
- 2026-09-27: Anchored the sources in `detachInlineSourceMap`. esbuild now
  lists `<app>/src/lib.tt`, and
  `node --enable-source-maps dist-maps/es/out.mjs` reports
  `at boom (<app>/src/lib.tt:6:1)`, the same frame that the Vite bundle
  reports. Rollup and Vite build maps are unchanged (`../../src/lib.tt`). The
  Vite 6 and Vite 8 development maps for `/src/lib.tt?lang.ts` list
  `lib.tt`, relative to the served module.
- 2026-09-27: Updated the source assertion in the shared-hooks test. Added a
  test with a stub compiler whose map has `sourceRoot: "../shared"` and a
  `null` source. Against the TASK-441 state, both tests fail. With the change,
  they pass.
- 2026-09-27: Ran `./scripts/ci npm`, the stage that runs the unplugin suite
  (9 tests) together with the npm launcher, create-tt, and
  deliberation-bot suites. It passed. The only warning was the unrelated
  missing global `rolldown`.

## Issues and resolutions

None.

## Verification

- [x] `./scripts/ci npm` (includes
  `TTC_BINARY=target/debug/ttc npm --prefix integrations/unplugin test`,
  9 tests pass)
- [x] `node scripts/check-task-index`
- [x] `./scripts/ci agents`
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
  `cargo test`: not run for TASK-439 to TASK-442. No Rust source changed.

## Result

Changed `integrations/unplugin/index.js`, `integrations/unplugin/README.md`,
and `integrations/unplugin/test/plugin.test.mjs`. Every adapter now receives
a map whose sources resolve to the `.tt` file, independent of the output
directory. On Windows, the absolute sources are native paths, which Rollup
and Vite handle with `path` functions. esbuild's handling of them on Windows
was not exercised.
