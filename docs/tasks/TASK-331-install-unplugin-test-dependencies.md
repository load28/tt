# TASK-331: Install bundler adapter dependencies before their CI gate

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-331: install bundler adapter dependencies in CI`

## Purpose

The `main` push CI for TASK-318 fails in the new bundler-adapter step because
`integrations/unplugin` never has its dependencies installed, so the gate that
was added to protect the adapters cannot run at all.

## Scope

- Included: Dependency installation for `integrations/unplugin` in
  `scripts/ci` and `.github/workflows/ci.yml` before its test step runs.
- Excluded: The adapter implementation, its tests, and every other CI stage.

## Decisions

### Decision 1: Install from the package's own lockfile in both gates

- **Context**: TASK-318 added `npm --prefix integrations/unplugin test` to the
  hosted workflow and `scripts/ci` without any install step. The package
  imports `unplugin` at module load, so the suite dies before the first test.
- **Alternatives considered**: Vendor or stub `unplugin` in the tests (breaks
  the point of testing the real adapter surface); hoist the dependency into
  the repository root `package.json` (the root manifest is deliberately only
  the TypeScript pin ttc resolves); install ad hoc with `npm install` (ignores
  the committed `package-lock.json`).
- **Decision and rationale**: Run `npm ci` against
  `integrations/unplugin`'s own committed lockfile. The hosted workflow gets
  an explicit install step; `scripts/ci` reuses `npm_install_if_stale` so
  repeated local gates do not pay a full reinstall.

## Work log

- 2026-09-05: Reproduced the failure locally: `./scripts/ci npm` fails with
  `ERR_MODULE_NOT_FOUND: Cannot find package 'unplugin' imported from
  integrations/unplugin/index.js`. Confirmed the identical failure in the
  hosted `main` push run 33937237393 (job `fmt / clippy / test`), the first CI
  run containing TASK-318's new step. Verified nothing installs the package's
  dependencies in either gate and that `integrations/unplugin/package-lock.json`
  is committed.
- 2026-09-05: Added the install to `stage_npm` in `scripts/ci` (via
  `npm_install_if_stale`) and an `Install bundler adapter dependencies` step
  to `.github/workflows/ci.yml` before the adapter test step. Re-ran
  `./scripts/ci npm` to green.

## Issues and resolutions

### Issue 1: The adapter test suite cannot resolve `unplugin`

- **Symptom**: `npm --prefix integrations/unplugin test` fails at import time
  with `ERR_MODULE_NOT_FOUND` for `unplugin`, in local `./scripts/ci npm` and
  in the hosted `main` CI run for TASK-318.
- **Cause**: TASK-318 added the test step to both gates but no step installs
  `integrations/unplugin`'s dependencies; the repository root install only
  provides the pinned TypeScript.
- **Resolution**: Install the package's committed lockfile with `npm ci` in
  both gates before the tests run.

## Verification

- [x] `./scripts/ci npm` fails before the change with `ERR_MODULE_NOT_FOUND`
- [x] `./scripts/ci npm` passes after the change (45 + 3 tooling suites, 8
  adapter tests, create-tt E2E, deliberation bot)
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Changed files: `scripts/ci`, `.github/workflows/ci.yml`,
`docs/tasks/TASK-331-install-unplugin-test-dependencies.md`,
`docs/tasks/INDEX.md`. The bundler-adapter gate now installs the package's
locked dependencies before running, in the hosted workflow and the local
mirror alike.
