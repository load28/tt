# TASK-344: A sidecar directory that cannot be resolved is reported, not ignored

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-344: say when tt.sidecarDir has no root to resolve against`

## Purpose

`tt.sidecarDir` is a directory relative to the workspace root. For a file
that belongs to no workspace folder there is no root, and the server
answered that with the same value it uses for "the setting is empty" — so
it wrote generated declarations next to the source, in the tree the setting
exists to keep clean, and said nothing.

## Scope

- Included: How a configured sidecar directory is resolved for one file,
  what the caller does when it cannot be, and what the setting's
  documentation says about it.
- Excluded: What `ttc --types` writes on a CLI build, and the sidecar
  refresh itself.

## Decisions

### Decision 1: "Adjacent" and "unresolved" are different answers

- **Context**: `resolveSidecarDir` returned `string | undefined`, and
  `refreshSidecar` treats `undefined` as "write beside the source". An
  empty setting and a relative setting with no containing root both
  produced `undefined`. The first is what the user asked for; the second is
  a location the server could not compute.
- **Alternatives considered**: Resolve against the file's own directory
  (invents a base the setting does not define, and scatters
  `.tt-types` directories through unrelated trees); resolve against the
  first workspace folder (writes one folder's declarations into another).
- **Decision and rationale**: `sidecarLocation` returns a discriminated
  `adjacent | directory | unresolved`. The conflation was the defect, so
  the type stops expressing it, and each caller decides.

### Decision 2: An unresolved location writes nothing and says so

- **Context**: The remaining choice is between writing somewhere the user
  did not name and writing nothing.
- **Alternatives considered**: Keep writing beside the source and only log
  it (the file still lands in the tree the setting was set to protect, and
  a log nobody has open is the silence this task is about).
- **Decision and rationale**: Skip the refresh for that file and say once,
  through the notice ledger (TASK-343), what is configured, why it cannot
  be resolved, and the two ways out — open the folder in the workspace, or
  set an absolute path. The notice re-arms when the folders change, which
  is exactly when it may have stopped being true.

### Decision 3: `sidecarLocation` takes its roots as an argument

- **Context**: The old resolution read the `workspaceRoots` module global,
  so it could not be tested without an extension host.
- **Alternatives considered**: Test it through the LSP harness (a saved
  buffer, a settings response the harness's client cannot give).
- **Decision and rationale**: Put it in `roots.ts` beside the other
  root-relative questions and pass the roots in, matching `folderRoots` and
  `applyFolderChange`. `containingRoot` is shared by it and named for what
  it answers.

## Work log

- 2026-09-05: Moved the resolution into `server/src/roots.ts` as
  `sidecarLocation`/`containingRoot` with the three-case result, and made
  `rebuildSidecar` skip and raise `sidecar-dir-unresolved`.
- 2026-09-05: Extended `server/src/test/roots.test.ts` with the five cases
  (empty, relative in a folder, nested folders, absolute, no folder).
- 2026-09-05: Documented the rule in the setting's own description and in
  the extension README.

## Issues and resolutions

### Issue 1: A configured sidecar directory was silently ignored

- **Symptom**: With `tt.sidecarDir` set to `.tt-types`, saving a `.tt` file
  that is not inside any workspace folder wrote `x.tt.d.ts` and its map
  next to the source instead, with nothing said anywhere.
- **Cause**: `resolveSidecarDir` returned `undefined` both for an empty
  setting and for a relative one with no containing root, and `undefined`
  means "beside the source".
- **Resolution**: Decisions 1 and 2.

## Verification

- [x] An empty setting still writes beside the source; a relative one
  resolves against the deepest containing folder; an absolute one needs no
  folder; a relative one with no containing folder is `unresolved`
- [x] An unresolved location writes no sidecar and reports once
- [x] `node --test server/out/test/*.test.js client/out/test/*.test.js`
- [x] `./scripts/ci`

## Result

Changed files: `editors/vscode/server/src/roots.ts`,
`editors/vscode/server/src/server.ts`,
`editors/vscode/server/src/notices.ts`,
`editors/vscode/server/src/test/roots.test.ts`,
`editors/vscode/package.json`, `editors/vscode/README.md`, and this record.
