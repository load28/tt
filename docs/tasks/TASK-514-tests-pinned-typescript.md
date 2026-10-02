# TASK-514: Run every test against the TypeScript that package.json pins

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

The Rust suites type-check emitted TypeScript with whatever `tsc` is first on
`PATH` (CI installed `typescript@6` globally for it), while the typed suites
drive the `typescript` that `package.json` pins. A test run therefore answered
for two different TypeScript versions, and neither was checked against the pin.

## Scope

- Included: The shared test toolchain helper in `tests/common/mod.rs`, every
  suite that spawns `tsc` or probes for an install, the CI workflow,
  `scripts/ci`, and `CONTRIBUTING.md`.
- Excluded: The compiler's own TypeScript resolution (`src/typescript/`), and
  test assertions.

## Decisions

### Decision 1: One pinned TypeScript for every suite, resolved in `tests/common`

- **Context**: `tests/integration.rs`, its `integration/` cases, `tests/cli.rs`
  and `tests/cli/dynamic_imports.rs` spawned `Command::new("tsc")`, a `PATH`
  lookup. CI satisfied it with `npm install -g typescript@6`, locally it was
  whatever the machine had (6.0.2 in this container). The typed suites drove
  `node_modules/typescript` (7.1.0-dev.20260826.1). `tests/content_mapper.rs`,
  `tests/native.rs` and `tests/practical_diagnostics.rs` each searched for the
  install on their own, and none compared it with `package.json`.
- **Alternatives considered**: Prepending `node_modules/.bin` to `PATH` in
  the scripts keeps a `PATH` lookup that a bare `cargo test` does not get.
  Keeping `typescript@6` and pinning its exact version in the workflow would
  leave two TypeScript versions and a second pin outside `package.json`.
- **Decision and rationale**: `common::typescript()` is the repository's
  `node_modules/typescript` and asserts that its version equals
  `devDependencies.typescript`; a mismatch fails with "run `npm ci`" rather
  than answering for another version. `common::tsc()` runs that package's own
  `lib/tsc.js` (its `bin` entry) through `node`, so no `PATH` entry can
  substitute. `common::tsc_available()` is the guard and honours
  `TTC_REQUIRE_TSGO`. `toolchain_installed()`, the content mapper entry, and
  the declaration-emit probe read the same install. The CI comment that
  "major 7 takes a different command line" did not hold for the flags the
  suites pass: all 283 integration and CLI cases passed under 7.1 before any
  code changed.

## Work log

- 2026-09-29: Found `tsc` on `PATH` was 6.0.2 while `package.json` pins
  7.1.0-dev.20260826.1. Ran `npm ci`, then
  `PATH=node_modules/.bin:$PATH TTC_REQUIRE_TSGO=1 cargo test --test integration --test cli`:
  188 + 95 passed.
- 2026-09-29: Added `typescript()`, `tsc_available()` and `tsc()` to
  `tests/common/mod.rs`; replaced every `Command::new("tsc")` and
  `have("tsc")`; moved `tests/content_mapper.rs`, `tests/native.rs` and
  `tests/practical_diagnostics.rs` onto the shared install; removed the
  global `typescript@6` install and `tsc --version` from
  `.github/workflows/ci.yml`, and the `tsc` on `PATH` warning from
  `scripts/ci`; rewrote the skip paragraph in `CONTRIBUTING.md`.

## Issues and resolutions

### Issue 1: Two TypeScript versions answered one test run

- **Symptom**: `tsc --version` printed 6.0.2; `node_modules/typescript` was
  7.1.0-dev.20260826.1.
- **Cause**: The integration and CLI helpers resolved `tsc` through `PATH`.
- **Resolution**: They run the pinned package's `tsc` (Decision 1).

## Verification

- [x] With `/opt/node22/bin` (the global `tsc`) removed from `PATH` and only
  `node` linked back: `TTC_REQUIRE_TSGO=1 cargo test --test integration --test cli --test content_mapper --test native --test practical_diagnostics -- --nocapture`
  — 188, 95, 19, 104 and 1 passed; no skip message.
- [x] With `node_modules/typescript/package.json` temporarily set to 6.0.2:
  the case fails with "node_modules/typescript is 6.0.2 but package.json pins
  7.1.0-dev.20260826.1 — run `npm ci`". The file was restored.
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `./scripts/ci`: passed agents, rust, npm, website, native and
  extension (rolldown absent, so the std tree-shaking regression skipped).

## Result

Changed `tests/common/mod.rs`, `tests/integration.rs`, `tests/integration/`
(`cases_02.rs`, `cases_03.rs`, `cases_05.rs`, `contextual.rs`),
`tests/cli.rs`, `tests/cli/dynamic_imports.rs`, `tests/content_mapper.rs`,
`tests/native.rs`, `tests/practical_diagnostics.rs`,
`.github/workflows/ci.yml`, `scripts/ci` and `CONTRIBUTING.md`. Every suite
now runs the TypeScript `package.json` pins and fails on any other installed
version.
