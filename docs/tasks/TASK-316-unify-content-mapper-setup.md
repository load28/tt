# TASK-316: Unify setup on the TypeScript content mapper

- **Status**: Complete
- **Started**: 2026-09-02
- **Completed**: 2026-09-02
- **Commit**: `TASK-316: unify setup on the content mapper`

## Purpose

Make the TypeScript 7.1 content mapper the single recommended setup across the
initializer, repository guides, package documentation, AI context, and public
website. Prevent new projects from falling back to declaration sidecars merely
because an older example appears earlier or is more complete.

## Scope

- Included: `create-tt` output and tests, user-facing setup documentation,
  package READMEs, AI context, and public website installation content.
- Excluded: Removing the supported `ttc --types` implementation or rewriting
  historical task and design records.

## Decisions

### Decision 1: Make content mapping the default consumer contract

- **Context**: Current guides describe both sidecars and content mapping, while
  the initializer emits neither the mapper declaration nor a mapper-enabled
  TypeScript command.
- **Alternatives considered**: Keep both paths equally prominent; retain
  sidecars as the default; make the content mapper the default and keep
  sidecars only as a compatibility fallback.
- **Decision and rationale**: Use the content mapper for all new setup examples
  and generated projects because it resolves `.tt` and `.ttx` imports without
  generated declaration trees. Keep sidecars documented only as a legacy
  fallback for TypeScript hosts that cannot load content mappers.

## Work log

- 2026-09-02: Confirmed that `create-tt` omitted `contentMappers`, while the AI
  guide presented the legacy sidecar recipe before the content-mapper recipe.
- 2026-09-02: Changed new projects to generate a mapper-enabled
  `tsconfig.json` and mapper-enabled TypeScript check/build scripts. Changed
  `init` to generate `tsconfig.tt.json` as a non-destructive extension of an
  existing TypeScript configuration.
- 2026-09-02: Made the content mapper the only recommended setup in the
  canonical installation guide, AI context, package READMEs, VS Code guide,
  repository README, and public website. Retained sidecars only as an explicit
  legacy-host compatibility path.
- 2026-09-02: Regenerated website syntax-highlighted content and verified all
  37 prerendered routes.

## Issues and resolutions

### Issue 1: Repository root is not an npm workspace

- **Symptom**: `npm test --workspace @openload28/create-tt` failed with
  `No workspaces found`.
- **Cause**: The root manifest does not declare npm workspaces.
- **Resolution**: Ran `npm test` from `packages/create-tt`; all eight tests
  passed. The repository-wide CI later ran the same tests successfully.

### Issue 2: Website prerender could not bind inside the sandbox

- **Symptom**: The first website build failed with `listen EPERM` on `::1`.
- **Cause**: TanStack prerender starts a local preview server, which the
  filesystem/process sandbox did not permit to bind.
- **Resolution**: Re-ran the same build with local-server permission; all 37
  routes prerendered successfully.

## Verification

- [x] `(cd packages/create-tt && npm test)` — 8 passed
- [x] `(cd website && npm run typecheck)`
- [x] `(cd website && npm run build)` — 37 routes prerendered
- [x] `cargo test --test content_mapper` — 12 passed
- [x] Documentation consistency search
- [x] `./scripts/ci` — `agents`, `rust`, `npm`, `native`, and `extension` passed

## Result

`create-tt` now generates content-mapper TypeScript configuration and runs
TypeScript with `--runExternalCode`. All current setup surfaces use that same
default, including the public website and AI guide; declaration sidecars remain
documented only for legacy TypeScript hosts.

Changed files: `packages/create-tt/src/installer.js`, its tests and README,
`docs/getting-started.md`, `docs/getting-started.ko.md`, `docs/ai/tt.md`,
`docs/ai/README.md`, `README.md`, `npm/tt-lang/README.md`,
`integrations/unplugin/README.md`, `editors/vscode/README.md`, website content
and generated highlighting, plus this task record and index.
