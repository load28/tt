# TASK-656: Read `#` and `?` in a directory or file name as part of the path in the bundler integration

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-656`

## Purpose

`@openload28/unplugin-tt` ignored every `.tt` module under a directory such
as `C#/` or `issue#12/`: `cleanUrl()` cut a module id at its first `?` or
`#`, so `/p/C#/m.tt?lang.ts` became `/p/C`, which is not a `.tt` file, and
the module was neither resolved nor loaded.

## Scope

- Included: `integrations/unplugin/index.js` (every place a module id
  becomes a file), its README, and a test in
  `integrations/unplugin/test/plugin.test.mjs`.
- Excluded: Vite's own resolver, which cuts user paths at `?`/`#` before
  the plugin sees them (see Decision 2).

## Sources

- Rollup, "Plugin Development", "Conventions" and the `resolveId`/`load`
  hooks: a module id is whatever `resolveId` returned, by default the
  absolute file path; Rollup's default `load` reads the id as a path.
  Query suffixes are a Vite convention layered on ids, not part of a path.
- Vite `packages/vite/src/node/plugins/resolve.ts`, `tryFsResolve`: for a
  path with `#` under `node_modules` it first tries the whole path, up to a
  `?` after the `#`, as a file, and only then splits off the postfix with
  `splitFileAndPostfix` (`cleanUrl`, the first `?` or `#`); its comment
  states that `#` is not supported in user source paths.
- webpack, "Module Resolution" / `enhanced-resolve` `parseIdentifier`: a
  request is split into path, `resourceQuery`, and `resourceFragment`, and
  a literal `#` in a path is escaped as `\0#`, so the path itself may hold
  `#`.
- esbuild, "Plugins", `onResolve`/`onLoad`: `args.path` is the path the
  resolver chose; a suffix is carried separately in `args.suffix`.

## Decisions

### Decision 1: The file is the longest prefix of the id that names a file

- **Context**: An id is a path, possibly followed by a query or fragment,
  and `?`/`#` may occur in either.
- **Alternatives considered**: (a) Cut at the first `?`/`#` after the last
  path separator. A `#` in a file name (`a#b.tt`) would still be cut, and
  a query holding `/` would move the cut. (b) Ask each host for its own
  split (`this.getModuleInfo`, webpack's `resourceQuery`); the hooks the
  plugin shares do not all expose one. (c) Try the whole id, then each
  prefix cut before a `?` or `#` from the right, and take the first that
  is a file on disk, the check Vite's `tryFsResolve` makes for `#`; when
  none is a file, cut at the first `?`/`#` after the last separator.
- **Decision and rationale**: (c), `fileOf(id, base)`, used by
  `sourceFileOfId`, the esbuild scanner, and `resolveId` (a relative
  specifier is checked against the importer's directory). An id whose file
  exists is read exactly; an id for a file that does not exist yet behaves
  as before for every path without `?`/`#` in a directory name.

### Decision 2: Document Vite's limit instead of working around it

- **Context**: Vite cuts a user path at its first `?`/`#` before plugins
  run, so on Vite the module id no longer names the directory.
- **Alternatives considered**: Re-deriving the path from the importer's
  source text; it would guess at Vite's resolution.
- **Decision and rationale**: The README states that Rollup, Rolldown,
  esbuild, webpack, and Rspack build such a project and that Vite does not,
  for its own documented reason.

## Work log

- 2026-09-30: Reproduced with `target/probe7-cli/cases/03`.
- 2026-09-30: Replaced `cleanUrl` with `fileOf`; added the test over
  `C#/`, `what?/`, and `a#b?c/`; documented the rule and Vite's limit.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `integrations/unplugin/test/plugin.test.mjs` ("a tt file under
  a directory whose name holds # or ? resolves and compiles").
- **Observed failure**: With `index.js` reversed, the test failed with
  "Cannot read properties of null (reading 'id')": `resolveId` returned
  `null` for `<root>/C#/m.tt`, because `cleanUrl` made it `<root>/C`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (the full gate, see TASK-654's record for the run)
- [x] `TTC_BINARY=target/debug/ttc npm --prefix integrations/unplugin test`
- [x] Baseline changes reviewed and committed with the change (none)

## Result

`.tt` modules under directories or files whose names hold `#` or `?`
resolve and compile on every host but Vite. Changed files:
`integrations/unplugin/index.js`, `integrations/unplugin/README.md`,
`integrations/unplugin/test/plugin.test.mjs`.
