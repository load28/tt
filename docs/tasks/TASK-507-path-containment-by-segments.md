# TASK-507: Judge path containment by path segments and canonical identity

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: see `git log --grep TASK-507`

## Purpose

Two review comments on PR #130 reported that containment checks compared
path strings: `create-tt init` wrote a `tsconfig.tt.json` outside the project
through a symlinked reference, and both `create-tt` and the VS Code sidecar
treated a child directory such as `..cache` as a parent.

## Scope

- Included: the project reference graph in
  `packages/create-tt/src/installer.js` (canonical node identity, boundary,
  relative reference paths) and the ancestor check used by the VS Code
  sidecar (`editors/vscode/server/src/paths.ts`).
- Excluded: path handling that does not decide containment.

## Decisions

### Decision 1: Canonical identity for reference-graph nodes

- **Context**: A reference such as `./packages/app` may be a symlink to a
  directory outside the project. Checking the lexical path let the writer
  follow the link out of the project; the visited set also keyed nodes by
  lexical path, so one config reached through two paths was visited twice.
- **Alternatives considered**: rejecting every symlink (breaks workspaces
  that link packages inside the project); resolving only the boundary check
  but keeping lexical writes (the write would still follow the link).
- **Decision and rationale**: every graph node is the `realpath` of its
  config (Node.js `fs.promises.realpath`), and the project root is
  canonicalized the same way. The boundary check, the visited set, the
  generated file location and the relative reference paths all use that one
  identity. A reference whose target leaves the project stays as written.

### Decision 2: Containment by path segments

- **Context**: `relative.startsWith('..')` is true for a child named
  `..cache`; `relativePath` also treated `..cache/...` as already relative.
- **Decision and rationale**: a path leaves its ancestor only when the
  relative path is `..` or begins with the `..` segment followed by the
  platform separator (`path.relative` yields native separators; Node.js
  `path` documentation). The VS Code server gets one shared `isWithin` in
  `src/paths.ts`; `create-tt` applies the same rule in `insideRoot` and in
  `relativePath` (which works on `/`-joined output).

## Work log

- 2026-09-28: Reproduced both comments with new tests in
  `packages/create-tt/test/installer.test.mjs` (symlink out of the project,
  a `..cache` reference, one config reached through two paths); all three
  fail on the previous `installer.js`. Implemented the canonical graph and
  segment rule; moved the sidecar check into `editors/vscode/server/src/paths.ts`
  with `src/test/paths.test.ts`.

## Issues and resolutions

### Issue 1: `relativePath` had the same prefix test

- **Symptom**: the `..cache` reference was written as `..cache/tsconfig.tt.json`.
- **Cause**: `path.startsWith('.')` decided whether to add `./`.
- **Resolution**: the same segment rule decides it.

## Verification

- [x] `node --test packages/create-tt/test/*.test.mjs`
- [x] `./scripts/ci extension`
- [x] `node scripts/check-task-index`

## Result

Changed `packages/create-tt/src/installer.js`,
`packages/create-tt/test/installer.test.mjs`,
`editors/vscode/server/src/paths.ts`, `editors/vscode/server/src/sidecar.ts`
and `editors/vscode/server/src/test/paths.test.ts`.
