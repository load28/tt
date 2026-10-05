# TASK-759: Preserve editor structure during incomplete syntax

- **Status**: In progress
- **Started**: 2026-10-05
- **Completed**: —
- **Commit**: —

## Purpose

Prevent incomplete syntax during typing from invalidating independent tt/ttx
constructs. Adopt TypeScript's structural recovery principles within tt's
existing syntax substrate and preserve the repository's testing contracts.

## Scope

- Included: Design, parser-owned recovery, editor projection continuity,
  causal diagnostics, and regression coverage through existing test runners.
- Excluded: New language syntax, relaxed build acceptance, debounce tuning,
  toolchain upgrades, and unrelated refactoring.

## Decisions

### Decision 1: Recover within the existing syntax ownership boundary

- **Context**: `program_syntax` uses the in-process SWC AST to establish host
  ownership and evaluation structure. A fatal parse error ends this model.
- **Alternatives considered**: Delay diagnostics; retain stale successful
  output; replace the syntax substrate with a TypeScript service call; add
  parser-owned missing/error nodes and grammatical synchronization.
- **Decision and rationale**: Propose the last option. It addresses the
  structural failure without moving syntax ownership into the type backend.
  See [the design](../design/structural-editor-recovery.md).

### Decision 2: Reuse existing regression and parity infrastructure

- **Context**: The user explicitly requires repository-aligned tests.
- **Alternatives considered**: A new standalone typing harness or the existing
  editor cases, incremental tests, content mapper tests, and compiler cases.
- **Decision and rationale**: Extend existing suites. Editor cases compare
  engine and server answers, exercise LSP publication, and support TypeScript
  twins; deterministic edit sequences complement static baselines.

## Work log

- 2026-10-05: Ran `./scripts/doctor`; all required checks passed. Fetched
  `origin/main`; `git rev-list --left-right --count HEAD...origin/main` returned
  `0 0`. Created `codex-structural-editor-recovery` from that revision.
- 2026-10-05: Read the host parser, projection recovery, content mapper,
  service diagnostics, `CONTRIBUTING.md`, and existing editor, compiler,
  incremental, and mapper test runners. Prepared the design for review.
- 2026-10-05: Preserved the pre-existing untracked `.task-agent-disabled` file.
- 2026-10-05: Reviewed the design against the actual test runners. Static
  editor cases and deterministic incremental sequences have distinct roles;
  mapper integration uses the existing real-TypeScript process fixture.
  `./scripts/check-task-index` passed (757 records), and `git diff --check`
  passed. Rust and integration gates are pending implementation.

## Issues and resolutions

### Issue 1: The default branch prefix conflicts with an existing ref

- **Symptom**: Git rejected `codex/structural-editor-recovery`.
- **Cause**: `refs/heads/codex` already exists and prevents nested branch names.
- **Resolution**: Used `codex-structural-editor-recovery` without altering the
  existing branch.

## Regression test (fails before the fix)

- **Path**: Planned editor cases and deterministic incremental assertions are
  specified in `docs/design/structural-editor-recovery.md`; not yet added.
- **Observed failure**: Not yet recorded. Implementation must run the new
  assertions against unchanged production code and replace this entry with
  actual failing output before accepting a fix.

## Verification

- [x] `./scripts/doctor`
- [x] `./scripts/check-task-index`
- [x] `git diff --check`
- [ ] Design review
- [ ] Failing regression assertions observed before production changes
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`
- [ ] Baseline changes reviewed and committed with the change

## Result

Design prepared in `docs/design/structural-editor-recovery.md`. No production
code or test expectations have changed. Implementation is pending design
review and a concrete implementation plan.
