# TASK-358: Audit developer workflows across every product surface

- **Status**: Complete
- **Started**: 2026-09-10
- **Completed**: 2026-09-10
- **Commit**: `TASK-358: chore(dx): audit developer workflows`

## Purpose

Audit documentation, CLI, compiler, and editor developer workflows against
their executable contracts. Repair confirmed defects in the layer that owns
the behavior and add regression coverage at the public boundary.

## Scope

- Included: Repository setup and verification guidance, CLI workflows,
  compiler behavior exercised by corpus and fuzz inputs, and VS Code extension
  development workflows
- Excluded: Publishing releases, changing external services, and speculative
  changes without a reproducible defect

## Decisions

### Decision 1: Treat developer readiness as one explicit prerequisite contract

- **Context**: `./scripts/doctor` reports a checkout ready, while the default
  local gate can still stop immediately because a tool required by default
  stages was never diagnosed.
- **Alternatives considered**: Leave stage-specific discovery in
  `./scripts/ci`; document the discrepancy; make the read-only doctor own the
  complete prerequisite inventory used by the default developer workflow.
- **Decision and rationale**: The doctor will diagnose every tool required by
  the documented default workflow. This keeps the entry point and the gate
  aligned instead of duplicating an implicit exception in prose.

### Decision 2: Keep editor build and watch projections under one lifecycle

- **Context**: The clean build deletes outputs for removed TypeScript sources,
  but the development watch command bypasses that projection reset.
- **Alternatives considered**: Tell developers to clean manually; make watch
  perform the same initial reset; build a second watcher that deletes outputs
  after every source deletion.
- **Decision and rationale**: Watch will use the same clean entry point before
  starting TypeScript build mode. TypeScript then owns incremental updates,
  while every newly started watch session begins from the current source set.

## Work log

- 2026-09-10: Ran `./scripts/doctor`; it reported the configured checkout
  ready. Confirmed a clean `main` worktree and started the complete local gate.
- 2026-09-10: Inventoried current documentation, CLI help and option dispatch,
  compiler tests and soak targets, editor package scripts, and prior product
  audit records.
- 2026-09-10: Confirmed that doctor omits Bun although default npm and website
  stages require it, and that the editor watch script bypasses the exact-output
  reset used by its compile script.
- 2026-09-10: Added a contract regression that derives default-gate tool
  failures from `scripts/ci` and requires doctor to inventory the same tools.
- 2026-09-10: Ran the complete installed TypeScript corpus: 142 files passed
  through byte-identically, three invalid TypeScript inputs were excluded, and
  no valid file failed.
- 2026-09-10: Ran `compile_any_bytes` for 33,375 executions and
  `generated_tt_compiles` for 6,085 executions. Neither compiler fuzz target
  found a crash or invalid generated program.
- 2026-09-10: Exercised all 63 CLI contract tests and all 161 extension tests,
  including real compiler, typed project, LSP, mixed-source, filesystem, and
  diagnostic workflows. No additional CLI or compiler defect was reproduced.
- 2026-09-10: Ran `./scripts/ci` with registry and loopback access. All agent,
  Rust, npm, website, native backend, and extension stages passed.

## Issues and resolutions

### Issue 1: Doctor can report ready before the default gate's prerequisites are ready

- **Symptom**: A checkout without Bun passes `./scripts/doctor`, then fails the
  default `./scripts/ci` in its npm or website stage with an instruction to run
  the doctor that already passed.
- **Cause**: Required-tool ownership was split: doctor checks Rust, Node.js, and
  npm, while two default CI stages privately require Bun.
- **Resolution**: Added Bun to doctor's required-tool inventory and made the
  contributing guide name why it is required.

### Issue 2: Editor watch can retain output for removed source files

- **Symptom**: `npm run compile` starts from an exact projection, but `npm run
  watch` starts from whatever output tree a previous session left behind.
- **Cause**: Only the compile script invokes the extension's output cleaner.
- **Resolution**: Made both compile and watch start through the same cleaner,
  with a package-contract regression that holds both scripts to that lifecycle.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `cargo test --test cli` (63 passed)
- [x] `npm --prefix editors/vscode test` (161 passed)
- [x] `TTC_CORPUS_FULL=1 TTC_REQUIRE_CORPUS=1 cargo test --test corpus --release`
- [x] `cargo +nightly fuzz run compile_any_bytes -- -max_total_time=30 -rss_limit_mb=4096`
- [x] `cargo +nightly fuzz run generated_tt_compiles -- -max_total_time=30 -rss_limit_mb=4096`
- [x] `./scripts/ci`

## Result

Developer readiness now names the complete default-gate toolchain, and a
cross-script regression prevents doctor and the gate from drifting again. The
editor's compile and watch commands now share the same exact-output lifecycle,
with a regression over the package contract. CLI and compiler stress coverage
found no additional reproducible defect.

Changed files: `CONTRIBUTING.md`, `scripts/doctor`,
`npm/scripts/developer-workflow.test.mjs`, `editors/vscode/package.json`,
`editors/vscode/server/src/test/build.test.ts`, `docs/tasks/INDEX.md`, and this
record.
