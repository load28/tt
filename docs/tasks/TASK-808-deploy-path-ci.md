# TASK-808: Run the test gates on pull requests only, not on the deploy path

- **Status**: Complete
- **Started**: 2026-10-14
- **Completed**: 2026-10-14
- **Commit**: `TASK-808: Run the test gates on pull requests only, not on the deploy path`

## Purpose

Every change reaches `main` and `release-X.Y` through a pull request whose
CI already runs the full gate. The deploy path (a push to those branches,
the schedule, and a manual run there) ran the same gate again, plus the
exhaustive sweeps, before an artifact could be published; a sweep failure
held back the Nightly for ten days (TASK-799).

## Scope

- Included: which events run the `check`, `bench`, `coverage`, and `native`
  jobs of `ci.yml`; what `release-metadata` waits for; where the
  `exhaustive` and `typescript-parity` jobs run; the workflow contract test
  and the documents that describe CI.
- Excluded: the content of every gate and sweep, the release build, the
  publish workflow and its smoke test, and the release branch model.

## Decisions

### Decision 1: The gates run where a change is proposed

- **Context**: The gate jobs had no `if:`, so they ran on every CI event,
  and `release-metadata` listed them in `needs`, so the release artifact
  waited for them.
- **Alternatives considered**: Keeping a reduced `cargo test` on the
  deploy path would still pay for a build and a test run the pull request
  already did. Splitting the release build into a separate workflow would
  change the run ID contract `release-publish.yml` relies on.
- **Decision and rationale**: The four gate jobs carry the exact complement
  of the release jobs' condition: pull requests, the merge queue, and a
  manual run on any other branch. A manual run on a feature branch keeps
  rerunning the gates, as `CONTRIBUTING.md` describes. `release-metadata`
  needs only the release jobs. GitHub reports a skipped job as successful
  ("Using conditions to control job execution", GitHub Actions
  documentation), so the run still concludes `success` and
  `release-publish.yml` promotes it unchanged. The publish job's smoke test
  of the promoted compiler stays as the check on the built artifact.

### Decision 2: The exhaustive sweeps move to `Soak`

- **Context**: `exhaustive` and `typescript-parity` ran only on the schedule
  and a manual run, which are the Nightly's deploy path; a pull request runs
  fixed samples of the same suites.
- **Alternatives considered**: Deleting them would lose the whole-set
  search. Leaving them in `CI` would keep them as a Nightly gate.
- **Decision and rationale**: Move both jobs, unchanged, into `soak.yml`,
  which already runs nightly and exists so that a finding never decides
  whether a Nightly publishes (TASK-637). They keep running every night.

## Work log

- 2026-10-14: Read `ci.yml`, `release-publish.yml`, `soak.yml`, and
  `npm/scripts/workflow-publish-paths.test.mjs`.
- 2026-10-14: Added the gate condition to `check`, `bench`, `coverage`,
  `native`; reduced `release-metadata`'s `needs`; moved `exhaustive` and
  `typescript-parity` to `soak.yml`.
- 2026-10-14: Added the contract test "the gates run on proposed changes and
  the deploy path only builds artifacts"; it fails against the previous
  `ci.yml` with `job "check" runs on the deploy path`.
- 2026-10-14: Updated `AGENTS.md`, `CONTRIBUTING.md`, and
  `docs/releasing.ko.md`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: this changes CI policy and fixes no bug. The new contract
test in `npm/scripts/workflow-publish-paths.test.mjs` pins the policy.

## Verification

- [x] `node --test npm/scripts/workflow-publish-paths.test.mjs`
- [x] `node scripts/check-task-index`
- [x] `./scripts/ci agents` and `node --test npm/scripts/*.test.mjs packages/create-tt/test/*.test.mjs` (84 passed)
- [ ] `cargo fmt --check`, `cargo clippy`, `cargo test`: not run; no Rust
  file changed.

## Result

Changed `.github/workflows/ci.yml`, `.github/workflows/soak.yml`,
`npm/scripts/workflow-publish-paths.test.mjs`, `AGENTS.md`,
`CONTRIBUTING.md`, `docs/releasing.ko.md`, and the task index. The deploy
path builds and publishes artifacts without rerunning the gates; the
sweeps keep running nightly in `Soak`.
