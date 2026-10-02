# TASK-619: Derive `tt:build` from the configured source roots

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-619`

## Purpose

`create-tt init` without a bundler wrote `"tt:build": "ttc -o .tt-build
src"` for every project, so the script failed in a repository whose
sources are not in a root `src` (a solution with `a/src` and `b/src`, or a
library whose configuration includes `lib`). The script must build what
the project's configuration says its sources are.

## Scope

- Included: the source-root derivation and the script
  (`packages/create-tt/src/installer.js`), tests
  (`packages/create-tt/test/installer.test.mjs`), and the user docs
  (`packages/create-tt/README.md`, `docs/getting-started.md`).
- Excluded: `exclude` patterns (ttc has no exclude option, so a root builds
  everything under it, as `ttc -o .tt-build src` did); an `extends` that
  names a package rather than a path; the bundler scripts, which are
  unchanged.

## Decisions

### Decision 1: The roots are the configured inputs' non-wildcard bases

- **Context**: TypeScript takes a project's inputs from `files` and
  `include`, relative to the configuration that declares them and inherited
  through `extends` (a later `extends` entry and the config itself taking
  precedence); with neither, `include` defaults to `**/*` in the config's
  directory (tsconfig reference: `files`, `include`, `extends`). TypeScript
  watches the directory part of each pattern before its first wildcard
  (its "wildcard directories"). A solution-style config (`"files": []`)
  has no inputs of its own (handbook, "Project References").
- **Alternatives considered**: (a) `ttc -o .tt-build .`: builds every `.ts`
  in the repository, including build outputs and scripts TypeScript leaves
  out. (b) Ask the user: `init` is non-interactive. (c) Derive the roots
  from the configuration graph the initializer already reads.
- **Decision and rationale**: (c). For every non-solution config in the
  graph, the declared `files` and each `include` pattern's base before its
  first `*`/`?` segment, resolved from the declaring config (following
  relative `extends`), or the config's directory when it declares neither.
  Roots inside another root are dropped, since `ttc` rejects overlapping
  input roots. Entries that do not exist are skipped as TypeScript skips a
  pattern that matches nothing; when none exists, they are kept so `ttc`
  reports them, as `tsc` reports TS18003. Without a `tsconfig.json`, the
  roots come from the config the initializer writes (`src`), so that case
  is unchanged.

### Decision 2: One root builds at `.tt-build`; several keep their paths under it

- **Context**: `ttc -o <dir> <input>` mirrors each input's own tree
  directly under `<dir>`, so `ttc -o .tt-build a/src b/src` wrote
  `.tt-build/a.ts` and `.tt-build/b.ts`, and `b`'s `"../../a/src/a.tt"`
  import no longer pointed at `a`'s output (checked with the probe).
- **Alternatives considered**: One invocation over the roots' common
  ancestor: it builds everything else under that directory too.
- **Decision and rationale**: A single root keeps `ttc -o .tt-build
  <root>`, the layout the fixed script produced for `src`. Several roots
  run one `ttc` per root with `-o .tt-build/<root>` (a file root mirrors at
  its directory), joined with `&&`, so each tree sits at its own relative
  path and imports between roots still resolve.

## Work log

- 2026-09-30: Checked the multi-input layout with `ttc -o .tt-build a/src
  b/src` (flattened, Decision 2).
- 2026-09-30: Implemented `configInputs`/`declaredInputs`, `inputBase`,
  `sourceRoots`, and `buildScript`; `typeConfigGraph` and the default
  config path both return their roots.
- 2026-09-30: Tests
  `init derives tt:build from the source roots the configuration includes`
  (solution with two projects, inherited `include`, and the unconfigured
  `src` default) and
  `a derived multi-root tt:build keeps imports between the roots` (runs the
  script's commands with the repository's `target/debug/ttc` and checks the
  rewritten import).

## Issues and resolutions

None.

## Verification

- [x] `npm --prefix packages/create-tt test` (25 passed)
- [x] Full gate run once at the end of the TASK-614–620 series; see
  TASK-620.

## Result

Changed `packages/create-tt/src/installer.js`,
`packages/create-tt/test/installer.test.mjs`,
`packages/create-tt/README.md`, `docs/getting-started.md`,
`docs/tasks/INDEX.md`, and this record.
