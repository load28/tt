# TASK-740: Design a behavior-preserving refactoring program

Workflow update: [TASK-742](TASK-742-service-response-projections.md) records
the user's 2026-10-04 authorization to publish and merge intermediate PRs
sequentially after review and verification. It supersedes this design's
original preparation-only publication policy. The final merge stays with
the user; behavior preservation and verification requirements are unchanged.

- **Status**: Complete
- **Started**: 2026-10-03
- **Completed**: 2026-10-03
- **Commit**: —

## Purpose

Make the whole project easier to follow through independently reviewable,
behavior-preserving PRs. Establish shared design principles, observable
contracts, and detailed implementation handoffs before changing production code.

## Scope

- Included: Current architecture audit, current official TypeScript 7 source
  research, a repository-wide PR roadmap, and the first implementation brief.
- Excluded: Language changes, dependency upgrades, baseline acceptance, and
  changes to the unfinished TASK-732.

## Decisions

### Decision 1: Separate the design from each implementation PR

- **Context**: The user requires isolated subagent contexts and main-agent
  review of every implementation under one shared design.
- **Alternatives considered**: A single repository-wide rewrite obscures
  equivalence; unrelated local cleanup gives no shared direction.
- **Decision and rationale**: Research and audit independently, then record
  small PR boundaries, exact invariants, and verification gates. Each
  implementation receives a separate task and a fresh subagent context.

## Work log

- 2026-10-03: Read AGENTS.md, ran `./scripts/doctor`, and inspected the clean
  checkout at `b62f2b6e727724748747d2a76351034f4daa010e`. GitHub's main branch
  endpoint confirmed the same SHA. Created a dedicated design branch.
- 2026-10-03: Delegated read-only TypeScript 7 research and repository
  architecture audit to separate subagents.
- 2026-10-03: Reviewed both reports against the source. Confirmed the upstream
  migration and pinned current TypeScript commit
  `50d70a3f5f453a79a4323b263165da51f656a4e3`; independently read its host,
  shared-data structural tests, and path type contract.
- 2026-10-03: Wrote `docs/design/behavior-preserving-refactoring.md` with
  compatibility rules, six source-backed lessons, shared design principles,
  a 16-slice implementation roadmap, untouched-surface inventory, and subagent
  handoff/main-review requirements. Wrote the exact six-helper extraction
  brief in `docs/design/refactoring-server-responses.md`.

## Issues and resolutions

### Issue 1: Development tools are missing from this execution environment

- **Symptom**: `./scripts/doctor` reports missing cargo, rustc, and bun;
  existing target binaries predate the checked-out commit.
- **Cause**: No tool executables were found in the available runtime paths.
- **Resolution**: Provision the pinned toolchain outside the repository;
  never treat pre-existing binaries as the current-source baseline.
  Installed Rust 1.98.0, rustfmt, clippy, and Bun under
  `/workspace/tt-refactor-tools`; `./scripts/doctor` now passes. Runtime
  build/gate results belong to the implementation task.

## Regression test (fails before the fix)

Not applicable: This task specifies a refactoring program and fixes no bug.

## Verification

- [x] Source citations checked against pinned official commits.
- [x] PR scopes and invariants reviewed against the current project.
- [x] `node scripts/check-task-index`
- [x] `git diff --check`

## Result

Added the shared design and first implementation brief, plus this task record
and index entry. No production code or test expectations changed. TASK-741
will implement the first bounded extraction and record its independent review
and base/head verification. The remaining roadmap is planned work, not complete.
