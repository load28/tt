# TASK-317: Build-test generated create-tt projects

- **Status**: Complete
- **Started**: 2026-09-02
- **Completed**: 2026-09-02
- **Commit**: `TASK-317: build-test generated projects`

## Purpose

Make the merge gate exercise a freshly generated `create-tt` project through
dependency installation, content-mapper type checking, and a production Vite
build. Catch package-manager and transitive native-package failures that file
shape assertions cannot observe.

## Scope

- Included: A generated-project end-to-end test, its CI runtime setup, and
  create-tt test documentation.
- Excluded: Pinning architecture-specific Rolldown packages or changing the
  supported bundler and package-manager matrix.

## Decisions

### Decision 1: Test the scaffold through its declared package manager

- **Context**: New projects use Bun and Vite, but the current tests only read
  generated files without installing or executing them.
- **Alternatives considered**: Keep structural tests only; install with npm;
  exercise the generated Bun workflow in a clean temporary directory.
- **Decision and rationale**: Use Bun to install the generated manifest and run
  both `check` and `build`, matching the command path presented to users.

### Decision 2: Keep platform packages out of the template

- **Context**: Vite resolves a platform-specific Rolldown binding transitively.
- **Alternatives considered**: Add every native binding to the generated
  manifest; pin the current machine's binding; leave ownership with Vite and
  detect broken resolutions in the end-to-end gate.
- **Decision and rationale**: Keep the generated manifest portable and test the
  real transitive resolution instead of encoding one platform in create-tt.

## Work log

- 2026-09-02: Compared create-vite, create-next-app, React Router, and Astro
  setup contracts. Confirmed that templates declare top-level tools while
  installation and integration tests validate the resulting dependency graph.
- 2026-09-02: Started a clean generated-project gate for the Bun/Vite default.
- 2026-09-02: Added a separate end-to-end suite that substitutes only the two
  unpublished local tt packages, freshly resolves Vite and its transitive
  dependencies, creates `bun.lock`, and runs the generated `check` and `build`
  scripts without creating `.tt-types`.
- 2026-09-02: Added Bun to the hosted check job and mirrored the end-to-end
  command in the local npm gate.

## Issues and resolutions

### Issue 1: Bun could not write its default sandbox temporary directory

- **Symptom**: The first local end-to-end run stopped before resolution with
  `bun is unable to write files to tempdir: PermissionDenied`.
- **Cause**: Bun selected a temporary location unavailable to the sandboxed
  child process.
- **Resolution**: Scoped `TMPDIR` and `BUN_INSTALL_CACHE_DIR` to the disposable
  generated project. The unrestricted registry run then passed.

### Issue 2: A local directory dependency resolved outside the generated project

- **Symptom**: Hosted Vite build failed because `integrations/unplugin/index.js`
  could not resolve its `unplugin` dependency.
- **Cause**: Bun linked the local `file:` directory into the generated project.
  Node followed the real repository path, whose dependencies are intentionally
  not installed by the root manifest. Published packages are extracted rather
  than linked, so the harness did not match the user installation topology.
- **Resolution**: Pack both unpublished tt packages with `npm pack` and install
  those tarballs. Their dependencies now resolve from the generated project in
  the same layout as a registry installation.

## Verification

- [x] Generated project installs with Bun and creates `bun.lock`
- [x] Generated project passes `bun run check`
- [x] Generated project passes `bun run build` without `.tt-types`
- [x] `(cd packages/create-tt && npm test)` — 8 passed
- [x] `(cd packages/create-tt && npm run test:e2e)` — 1 passed
- [x] `./scripts/ci` — `agents`, `rust`, `npm`, `native`, and `extension` passed

## Result

The local and hosted merge gates now install and build a freshly generated
Bun/Vite project. The scaffold remains platform-neutral: no Rolldown binding is
declared directly, while a broken registry or native transitive resolution now
fails before merge.

Changed files: `.github/workflows/ci.yml`, `scripts/ci`,
`packages/create-tt/package.json`, `packages/create-tt/e2e/scaffold.test.mjs`,
and this task record and index.
