# TASK-513: Plan the package.json update with every other init write

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: see `git log --grep TASK-513`

## Purpose

A PR #130 review found that `create-tt init` checked the project boundary only
for generated files. `package.json` was written afterward by a separate
`writeJson` call without the check. When `package.json` was a symlink to a
manifest in another directory, init changed that outside manifest, adding
`devDependencies.typescript`, and also wrote `tsconfig.tt.json` inside the
project.

## Scope

- Included: the write plan in `initializeExisting`
  (`packages/create-tt/src/installer.js`) and a regression test.
- Excluded: `createProject`, which requires an empty target directory, so none
  of its outputs can already be a link.

## Decisions

### Decision 1: One write plan for every output, validated before any write

- **Context**: TASK-508 validated the generated files before writing them. The
  manifest took a second write path, so the boundary contract depended on
  which call site wrote a file.
- **Alternatives considered**: adding `assertInsideProject(realRoot,
  manifestPath)` before the manifest write. That special-cases one more file
  name and leaves the contract tied to call sites.
- **Decision and rationale**: each output becomes a planned write
  `{ path, content, replace }`. `replace` is `false` for generated files,
  which must be absent or identical, and `true` for the manifest, which init
  updates. One loop checks the canonical destination of every planned write
  against the real project root before any file is written. A second loop then
  writes them in the previous order: generated files, then the manifest. The
  manifest text is built with the same `JSON.stringify` form and indentation
  as before (`jsonText`), so the output bytes do not change.

## Work log

- 2026-09-29: Reproduced the reviewer's case as a test. It fails against the
  previous installer: the outside manifest changes and `tsconfig.tt.json`
  appears in the project.
- 2026-09-29: Moved the manifest into the write plan. The test passes, and
  the existing installer tests pass unchanged.

## Issues and resolutions

### Issue 1: The manifest bypassed the pre-write boundary check

- **Symptom**: a `package.json` symlink to an outside file let init modify
  that file.
- **Cause**: the boundary check covered only the `generated` list, and the
  manifest was written outside it.
- **Resolution**: all outputs go through one validated write plan (Decision 1).

## Verification

- [x] `node --test packages/create-tt/test/installer.test.mjs`: 21 pass. With
  the previous `installer.js`, the new test fails and the other 20 pass.
- [x] `./scripts/ci npm`
- [x] `node scripts/check-task-index`

## Result

Changed `packages/create-tt/src/installer.js` and
`packages/create-tt/test/installer.test.mjs`. `create-tt init` now validates
the destination of every file it writes, the manifest included, before it
writes any of them.
