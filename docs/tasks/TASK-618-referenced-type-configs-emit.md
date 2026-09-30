# TASK-618: Let a referenced type-check config emit declarations into a cache

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-618`

## Purpose

`create-tt init` in a repository whose composite project `a` is referenced
by another project `b` wrote every `*.tt.json` with `noEmit: true`, and
`tsc -b tsconfig.tt.json --runExternalCode` failed with TS6310
"Referenced project '.../a/tsconfig.tt.json' may not disable emit". The
generated graph must be valid under TypeScript's project-reference rules.

## Scope

- Included: the generated config graph (`typeConfigGraph`, `tsconfig()` in
  `packages/create-tt/src/installer.js`), its tests
  (`packages/create-tt/test/installer.test.mjs`), and the user docs
  (`packages/create-tt/README.md`, `docs/getting-started.md`).
- Excluded: the user's own configs, which the initializer never edits;
  `composite`, which a referenced project already needs for the user's own
  `tsc -b` (TS6306).

## Decisions

### Decision 1: A config referenced by a compiled config emits declarations only, into a cache

- **Context**: TypeScript's project references require a referenced
  project to produce its declaration outputs, which the referencing project
  reads in place of the referenced sources (TypeScript handbook, "Project
  References": `composite`, "What is a Project Reference?", and build mode).
  A referenced project that sets `noEmit` is TS6310. The error is reported
  for a reference from a project that builds a program; a solution-style
  config (`"files": []`, no `include`, handbook "Overall Structure") builds
  none, which is why Vite's solution template, whose referenced configs set
  `noEmit`, passed (`the generated solution check reaches the referenced
  sources`).
- **Alternatives considered**: (a) Drop `noEmit` and keep the user's
  `outDir`: the check would write `.d.ts` files and a build info file into
  the user's `dist`, next to and over their real build's outputs (the
  probe's `a/dist/tsconfig.tt.tsbuildinfo` was already such a file). (b)
  Make every generated config emit: the solution-style case works today,
  and changing those configs would make a re-run `init` refuse its own
  earlier output as "customized". (c) `emitDeclarationOnly` with `outDir`
  and `declarationDir` in a cache directory, only for a config some
  non-solution config references.
- **Decision and rationale**: (c). The graph is read first
  (`readConfigGraph`), then every config that a non-solution config
  references gets `noEmit: false`, `emitDeclarationOnly: true`, and
  `outDir`/`declarationDir` under `node_modules/.cache/tt/<project path>`
  (tsconfig reference: `emitDeclarationOnly`, `outDir`, `declarationDir`;
  `node_modules` is already left out of `include` by default, and so is the
  `outDir`). Its build info follows `outDir` unless the project names a
  `tsBuildInfoFile`, which is then redirected into the same cache
  (tsconfig reference: `tsBuildInfoFile`). A `tsBuildInfoFile` is only
  written for a config that names one, since without `incremental` or
  `composite` TypeScript rejects it (TS5069); a referenced project is
  composite. `node_modules/.cache` is the conventional, ignored location for
  tool caches. Every other config keeps `noEmit: true`, byte for byte.

## Work log

- 2026-09-30: Reproduced `target/probe6-cli/c1`: TS6310 at
  `b/tsconfig.tt.json(16,5)`.
- 2026-09-30: Rewrote `a/tsconfig.tt.json` by hand to the chosen form;
  `tsc -b` passed, wrote only into `node_modules/.cache/tt/{a,b}`, and
  reported a TS2322 added to `b`.
- 2026-09-30: Implemented `readConfigGraph`, `solutionStyle`, and
  `declarationOutput`; the per-config content is generated after the
  graph is known, in the same order as before.
- 2026-09-30: Tests
  `init lets a project another project references emit declarations into a cache`
  and `the generated graph builds when a referenced project is referenced by another`
  (runs the repository's `tsc -b`: no TS6310, the TS2322 in `b`, then a
  clean pass, and nothing in `a/dist`). Both fail on the previous installer.

## Issues and resolutions

### Issue 1: A solution-style config inherited `include`

- **Symptom**: None observed; recorded as a limit.
- **Cause**: `solutionStyle` reads the config's own `files`/`include`; an
  `include` inherited through `extends` is not seen.
- **Resolution**: Left as is: such a config is then treated as solution
  style and its references keep `noEmit`, which is the previous output.

## Verification

- [x] `npm --prefix packages/create-tt test` (23 passed)
- [x] Full gate run once at the end of the TASK-614–620 series; see
  TASK-620.

## Result

Changed `packages/create-tt/src/installer.js`,
`packages/create-tt/test/installer.test.mjs`,
`packages/create-tt/README.md`, `docs/getting-started.md`,
`docs/tasks/INDEX.md`, and this record.
