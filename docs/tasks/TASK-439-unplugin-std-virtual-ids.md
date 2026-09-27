# TASK-439: Serve unplugin standard modules under path-free ids

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

On Windows, `@openload28/unplugin-tt` resolved `@tt/std` to
`C:\proj\__tt_std__\types.ts`. Vite normalizes resolved ids to POSIX
separators (`C:/proj/__tt_std__/types.ts`), and then `load` returned `null`
and the standard module's own `./option.js` import failed to resolve.
Dependency invalidation compared Vite-normalized paths from the watcher with
native paths from `ttc --dependencies` in the same way.

## Scope

- Included: Standard-module id scheme, recognition of the plugin's own
  standard ids, native-path comparison for compiler dependencies, a Windows
  path-semantics regression test, and the README.
- Excluded: The id scheme for `.tt`/`.ttx` source modules (TASK-440 and
  TASK-441) and source maps (TASK-442).

## Decisions

### Decision 1: Give standard modules an id with no file system path

- **Context**: `stdId` was `path.resolve(process.cwd(), "__tt_std__", …)`, and
  `stdModuleOfId` compared ids to it by exact string. The Vite plugin API guide
  (Plugin API, "Path Normalization") states that "Vite normalizes paths while
  resolving ids to use POSIX separators ( / ) while preserving the volume in
  Windows", so a plugin that compares ids with native paths fails on Windows.
  The fake path was also a problem beyond Windows: Vite's dependency scanner
  reads any absolute, non-`\0`, script-extension id from disk (see TASK-441).
- **Alternatives considered**:
  - Normalize separators on both sides of the comparison. This fixes the
    string comparison, but the id is still a path to a file that does not
    exist, so any host that reads it from disk still fails.
  - Use Rollup's `\0` virtual-module prefix. The Rollup plugin guide
    ("Conventions") says the prefix exists so that other plugins do not try to
    process the module. Standard modules are TypeScript, and the host must
    transpile them: Vite's `vite:esbuild` (Vite 6/7) and `vite:oxc` (Vite 8)
    transforms run only when `createFilter` matches, and that filter rejects
    every id that contains `\0` (checked in `vite@6.4.3`
    `dist/node/chunks/dep-*.js` and `vite@8.3.1` `dist/node/chunks/node.js`).
    With `\0`, the type annotations from `ttc --emit-std` would reach the
    parser.
  - Use a `virtual:`-namespaced id without `\0`. The Vite plugin API guide
    ("Virtual Modules") and the Rolldown plugin API (Virtual Modules
    Convention) name `virtual:` as the virtual-module namespace and advise
    using the plugin name to avoid collisions. Vite's module graph also
    recognizes the prefix (`_resolveUrl` skips extension appending for
    `virtual:` urls).
- **Decision and rationale**: Standard modules resolve to
  `virtual:unplugin-tt/std/<module>.ts`. The id contains no separator that
  normalization can change, and it does not depend on `process.cwd()`. It
  keeps the `.ts` extension so the host's TypeScript transform and the esbuild
  `loader` hook treat it as TypeScript. `resolveId` returns its own standard
  ids unchanged, because Vite's development server resolves
  `/@id/virtual:unplugin-tt/std/…` again after a cold start.

### Decision 2: Compare compiler dependencies as native paths

- **Context**: `watchChange` and `handleHotUpdate` receive Vite-normalized
  file paths, while `ttc --dependencies` prints native paths.
- **Alternatives considered**: Rewriting backslashes by string would copy
  Vite's rule into the plugin. `path.resolve` produces the platform's
  canonical form for both inputs.
- **Decision and rationale**: Store dependencies and look up changed files
  through `path.resolve`, which is an identity on POSIX and converts
  `C:/proj/x` to `C:\proj\x` on Windows.

## Work log

- 2026-09-27: Ran `./scripts/doctor`. It reported missing TypeScript and
  release artifacts, so I installed the pinned TypeScript with `npm ci`,
  installed the package dependencies with `npm ci` in
  `integrations/unplugin`, and built `target/debug/ttc` with `cargo build`.
- 2026-09-27: Reproduced the defect with the hunter's scratch setup (the
  `node:path` → `path.win32` loader shim in `hunt-tools/win/run.mjs`).
  `load` of the normalized id returned `null`.
- 2026-09-27: Replaced the standard-module ids, added own-id pass-through, and
  compared dependencies by native path. Added `test/windows.test.mjs`. It runs
  the plugin in a child process whose `node:path` is `path.win32` and whose
  `cwd` is `C:\proj`, with a stub compiler that reports a native Windows
  dependency. Against the previous `index.js`, the test fails with
  `Cannot read properties of null (reading 'replace')` (the std `./option.js`
  import is unresolved). With the change, it passes.
- 2026-09-27: Using scratch installs of `vite@6.4.3`, `rollup@4.63.5`,
  `esbuild@0.28.2`, and `vite@8.3.1` outside the repository, I checked that the
  Vite build, the Rollup build, the esbuild bundle (which also runs), and a
  cold Vite development request for `/@id/virtual:unplugin-tt/std/option.ts`
  all compile and transpile the standard modules.

## Issues and resolutions

None.

## Verification

- [x] `TTC_BINARY=target/debug/ttc npm --prefix integrations/unplugin test`:
  6 tests pass.
- [x] `node scripts/check-task-index`
- [x] `./scripts/ci agents`
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
  `cargo test`: not run for this change, which touches only
  `integrations/unplugin` and documentation. No Rust source changed.

## Result

Changed `integrations/unplugin/index.js`, `integrations/unplugin/README.md`,
and `integrations/unplugin/test/windows.test.mjs`. Standard modules now have
path-free virtual ids that work under Vite's Windows normalization, and
dependency invalidation matches paths regardless of separator form.
