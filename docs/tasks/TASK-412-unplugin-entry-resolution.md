# TASK-412: Resolve entry and root-relative tt specifiers in the bundler plugin

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`@openload28/unplugin-tt` returned `null` for a relative entry such as `./src/main.tt` (no importer), so the host parsed raw tt as JavaScript; and it treated Vite's root-relative `/src/main.tt` (from `index.html`) as a file-system path that does not exist.

## Scope

- Included: `resolveId` in `integrations/unplugin/index.js`.
- Excluded: Standard-library virtual modules and package specifiers, which are unchanged.

## Decisions

### Decision 1: Follow the host's resolution rules

- **Context**: Rollup calls `resolveId` for entry points with an undefined importer and resolves relative `input` paths against the working directory (Rollup plugin API, `resolveId`; `input` option). Vite serves `/src/main.ts` in `index.html` as a path relative to the project root, which its own resolver maps.
- **Alternatives considered**: Guessing a root from the importer would reimplement host configuration (`root`, aliases).
- **Decision and rationale**: A relative specifier without an importer resolves against `process.cwd()`. An absolute specifier that does not exist on disk is handed to the host resolver (`this.resolve(..., { skipSelf: true })`) when the host provides one, exactly like a bare specifier; the result gets the `.ts`/`.tsx` suffix. Existing absolute paths and relative paths with an importer resolve as before.

## Work log

- 2026-09-27: Added a plugin test for both cases (fails before the change) and implemented the resolution.

## Issues and resolutions

None.

## Verification

- [x] `TTC_BINARY=target/debug/ttc npm --prefix integrations/unplugin test`: 7 passed.

## Result

Changed `integrations/unplugin/index.js` and `integrations/unplugin/test/plugin.test.mjs`.
