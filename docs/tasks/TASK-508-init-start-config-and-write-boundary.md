# TASK-508: Hold the start config and every written file to the project boundary

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: see `git log --grep TASK-508`

## Purpose

A follow-up review of TASK-507 on PR #130 found that the start node of the
reference graph, `root/tsconfig.json`, was canonicalized but never checked:
a symlink to `../outside/tsconfig.json` made `create-tt init` write
`../outside/tsconfig.tt.json`. The reviewer also asked for a separate check
that no output path leaves the project before anything is written.

## Scope

- Included: `packages/create-tt/src/installer.js` start-config boundary and a
  pre-write boundary check for every generated file.
- Excluded: the reference-graph rules from TASK-507, which are unchanged.

## Decisions

### Decision 1: Refuse a start config outside the project

- **Context**: every generated counterpart is placed next to its config, so a
  start config outside the project would place output outside it.
- **Alternatives considered**: writing the root counterpart at the lexical
  location while following the outside config's references (mixes two
  identities and still needs outside counterparts for its references).
- **Decision and rationale**: `projectConfig` canonicalizes the start config
  with `realpath` and throws when it leaves the project, before any file is
  written. The user sees why and can decide.

### Decision 2: Check the resolved write target of every generated file

- **Context**: a generated file name can itself be a symlink, including a
  dangling one, which `writeFile` follows (Node.js `fs.promises.writeFile`
  follows symbolic links).
- **Decision and rationale**: `assertInsideProject` resolves each output path
  with `lstat`/`readlink` until it reaches a non-link (existing file through
  `realpath`, new file through its canonical parent) and refuses a target
  outside the canonical root or a link cycle. It runs in the existing
  validate-before-write loop, so a refusal writes nothing.

## Work log

- 2026-09-28: Added `projectConfig`, `assertInsideProject` and `writtenPath`;
  two tests in `packages/create-tt/test/installer.test.mjs` (symlinked root
  tsconfig; generated file name symlinked outside) fail on the previous code
  and pass now.

## Issues and resolutions

### Issue 1: A dangling symlink escaped the first check

- **Symptom**: the second test still wrote outside the project.
- **Cause**: `existsSync` is false for a dangling link, so the check used the
  link's own location instead of its target.
- **Resolution**: follow links with `lstat`/`readlink` before resolving.

## Verification

- [x] `node --test packages/create-tt/test/*.test.mjs` (20 passed)
- [x] `./scripts/ci npm`
- [x] `node scripts/check-task-index`

## Result

Changed `packages/create-tt/src/installer.js` and
`packages/create-tt/test/installer.test.mjs`.
