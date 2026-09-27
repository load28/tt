# TASK-441: Let the Vite dependency scanner read tt modules

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

Starting the Vite dev server failed with `Failed to scan for dependencies …
ENOENT src/lib.tt.ts [vite:dep-scan]` (Vite 6). Vite 8 reported
`[UNLOADABLE_DEPENDENCY] Could not load src/lib.tt.ts` and skipped
pre-bundling. Bare dependencies that only `.tt` files imported could not be
pre-bundled at startup.

## Scope

- Included: The Vite `config` hook that registers a scanner plugin, a
  scanner regression test, and the README.
- Excluded: The id scheme itself (TASK-439, TASK-440) and non-Vite hosts,
  which have no dependency scanner.

## Decisions

### Decision 1: Rely on file-backed ids for scanner resolution

- **Context**: Vite's scanner (`esbuildScanPlugin` in Vite 6,
  `rolldownScanPlugin` in Vite 8) resolves each import through the plugin
  container. It externalizes results that are not absolute or that contain
  `\0` (`shouldExternalizeDep`). It then reads `path.resolve(cleanUrl(id))`
  from disk when the id looks like a script. The old id `<file>.tt.ts` and the
  old standard id `<cwd>/__tt_std__/*.ts` were absolute script paths to files
  that do not exist.
- **Alternatives considered**: A `\0` id would be externalized. The scan would
  stop failing, but `.tt` content would never be scanned, and the TypeScript
  transform would skip the module (TASK-439, TASK-440).
- **Decision and rationale**: After TASK-439 and TASK-440, `.tt` ids strip to
  the real file, and standard ids are non-absolute `virtual:` ids that the
  scanner externalizes. The Rolldown plugin API (Virtual Modules Convention)
  says file-derived modules "don't need to follow this convention" for this
  reason. What remained was teaching the scanner to read a `.tt` file.

### Decision 2: Register a compile-on-load scanner plugin through documented optimizeDeps options

- **Context**: Once the scanner reaches the real `/src/lib.tt`, Vite 6 falls
  back to `export default {}` for unknown files (esbuild then reports
  `No matching export in "src/lib.tt"`). Vite 8's Rolldown scanner parses the
  raw file (`PARSE_ERROR`).
- **Alternatives considered**:
  - Rely only on Vite's runtime discovery of missing dependencies. That
    removes pre-bundling of `.tt`-only dependencies at startup, and the
    remaining scanner errors still surface.
  - Use `optimizeDeps.entries` or the internal `scan: true` resolve flag.
    Neither makes the scanner able to read `.tt` source, and the flag is not
    part of the documented plugin API.
  - Use the documented extension points. `optimizeDeps.extensions`
    (`DepOptimizationConfig`: "List of file extensions that can be
    optimized. A corresponding esbuild plugin must exist to handle the
    specific extension."). `optimizeDeps.esbuildOptions`: "`plugins` are
    merged with Vite's dep plugin" (deprecated in Vite 8).
    `optimizeDeps.rolldownOptions` in Vite 8: "`plugins` are merged with
    Vite's dep plugin". Vite's `config` hook "can return a partial config
    object that will be deeply merged into existing config". Rolldown-powered
    Vite is detected, as the Vite plugin API guide documents, with
    `this.meta.rolldownVersion` ("only available for Rolldown powered Vite
    (i.e. Vite 8+)").
- **Decision and rationale**: The Vite adapter's `config` hook adds
  `optimizeDeps.extensions: [".tt", ".ttx"]` and one scanner plugin. On
  esbuild-based Vite, the plugin is an esbuild plugin whose `onLoad` compiles
  the file with `ttc -p --rewrite-imports off` and returns it with the `ts`
  or `tsx` loader. On Rolldown-based Vite, it is a Rolldown plugin whose
  `load` returns the same code with `moduleType: "ts" | "tsx"` (the Rolldown
  load result field for the module's language). Choosing by
  `rolldownVersion` avoids Vite 8's deprecation warning for `esbuildOptions`.
  Compiler errors propagate. The scan reports them, and the module request
  reports them again through `load`, so no diagnostic is dropped.

## Work log

- 2026-09-27: Reproduced with `vite@6.4.3` and `vite@8.3.1` (dev server with
  `optimizeDeps.force`). Vite 6 failed with ENOENT on `src/lib.tt.ts`. Vite 8
  failed with `UNLOADABLE_DEPENDENCY`. After TASK-439 and TASK-440, both
  still failed (the `No matching export` error on Vite 6 and a `PARSE_ERROR`
  on Vite 8) because the scanner read raw tt.
- 2026-09-27: Added the `config` hook and the two scanner plugins, and shared
  the `ttc -p` argument construction with `load`. With a fixture where only
  `src/scan.tt` imports a CommonJS package `tt-scan-dep`, both Vite versions
  finished the scan, and `depsOptimizer.metadata.optimized` listed
  `tt-scan-dep`. The served `scan.tt` imports the pre-bundled
  `.vite/deps/tt-scan-dep.js`.
- 2026-09-27: Rechecked Vite build (both versions, including worker bundles),
  Rollup, and esbuild builds. All pass, and the esbuild bundle runs.
- 2026-09-27: Added the scanner regression test. Against the TASK-440 state
  of `index.js`, it fails with a `TypeError` (no `config` hook). With the
  change, it passes.

## Issues and resolutions

### Issue 1: Scratch disk filled during verification

- **Symptom**: Tool output was lost with `ENOSPC` on the shared temp file
  system.
- **Cause**: The shared host disk was nearly full from other sessions'
  worktrees and temporary directories.
- **Resolution**: Removed this task's own scratch build outputs, Vite caches,
  and the Cargo incremental cache, then re-ran the checks.

## Verification

- [x] `TTC_BINARY=target/debug/ttc npm --prefix integrations/unplugin test`:
  8 tests pass.
- [x] `node scripts/check-task-index`
- [x] `./scripts/ci agents`
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
  `cargo test`: not run for this change. No Rust source changed.

## Result

Changed `integrations/unplugin/index.js`, `integrations/unplugin/README.md`,
and `integrations/unplugin/test/plugin.test.mjs`. The Vite dev server's
dependency scan now succeeds on Vite 6 and Vite 8, and it pre-bundles bare
dependencies that `.tt` modules import.
