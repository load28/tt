# TASK-636: Require a test that fails before the fix, and reviewed baselines

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-636`

## Purpose

Records here name the tests a fix added, but nothing asks whether those
tests failed before the fix, and nothing asks that baseline changes be read.
TypeScript's contribution rules require both. This task writes them into
`AGENTS.md`, `CONTRIBUTING.md`, the task template, the task-index check,
and a pull request template.

## Scope

- Included: `AGENTS.md` ("구현과 검증 규칙", in English per the
  documentation-language rule), `CONTRIBUTING.md` ("Housekeeping"),
  `docs/tasks/TEMPLATE.md`, `scripts/check-task-index`, and
  `.github/pull_request_template.md`.
- Excluded: records before TASK-636, which are not required to gain the new
  section.

## Sources modelled

- microsoft/TypeScript `release-6.0` `CONTRIBUTING.md`, "Housekeeping": a
  pull request should "Include adequate tests: At least one test should fail
  in the absence of your non-test code changes. If your PR does not match
  this criteria, please specify why; Tests should include reasonable
  permutations of the target fix/change; Include baseline changes with your
  change".
- The same file, "Managing the baselines": "Be sure to validate the changes
  carefully -- apparently unrelated changes to baselines can be clues about
  something you didn't think of."
- `.github/pull_request_template.md` in microsoft/TypeScript: a checklist
  that includes "There are new or updated unit tests validating the change"
  and "You've successfully run `hereby runtests` locally".

## Decisions

### Decision 1: Enforce the record section, not the test's behaviour

- **Context**: Whether a test fails without the fix can only be shown by
  running it against the unfixed code, which a static gate cannot do.
- **Alternatives considered**: (a) A CI job that reverts the non-test
  changes of a pull request and expects some test to fail: needs a reliable
  split of test and non-test files, and a full build of the base per pull
  request. (b) Leave it to the pull request checklist alone: unchecked.
  (c) Require the evidence in the task record, where this repository
  already records verification, and check that it is present.
- **Decision and rationale**: (c). `scripts/check-task-index`, which the
  `agents` gate and CI already run, requires a
  `## Regression test (fails before the fix)` section in every record from
  TASK-636 on, with both `- **Path**:` and `- **Observed failure**:` filled
  (a `<placeholder>` does not count) or a line starting with
  `Not applicable:` and a reason, which is TypeScript's "please specify
  why". Records before TASK-636 are not checked, so the history is not
  rewritten.

### Decision 2: Baselines travel with their change

- **Context**: TASK-635 makes CI fail on a baseline that differs from the
  tests' output; that proves the baselines are current, not that someone
  read them.
- **Decision and rationale**: The rule and the checklist ask for the
  baseline diff to be read and committed with the change that causes it,
  quoting TypeScript's reason. The template's verification list gains
  "Baseline changes reviewed and committed with the change".

## Work log

- 2026-09-30: Added the rules to `AGENTS.md` and `CONTRIBUTING.md`, the
  section and checkbox to `docs/tasks/TEMPLATE.md`, the check to
  `scripts/check-task-index`, and `.github/pull_request_template.md`.
- 2026-09-30: Ran the full gate over the final tree of TASK-634 to
  TASK-636.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: this task changes contribution rules and a documentation
check and fixes no compiler bug. The check's own behaviour is shown under
Verification: it rejects this record without the section and with the
template's placeholders.

## Verification

- [x] `node scripts/check-task-index` fails for this record while it has no
  regression section: "TASK-636: TASK-636-contribution-regression-rules.md
  has no "## Regression test (fails before the fix)" section".
- [x] With the section holding the template's placeholders it fails with
  "must name the test that fails without the fix".
- [x] With `Not applicable:` and a reason it passes, as it does with
  filled `- **Path**:` and `- **Observed failure**:` fields; the earlier
  records pass unchanged.
- [x] Full gate over TASK-634 to TASK-636 (before merging the advanced base
  branch): `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`;
  `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 TT_BASELINE_TRACKING_DIR=<dir>
  cargo test` (43 test binaries, 1739 passed, 0 failed, no `SKIP`);
  `node scripts/check-baselines --tracking <dir>` ("95 compared, none
  unused"); `./scripts/ci agents` passed (warnings: rolldown not on PATH,
  doctor reports the checkout not ready, both environmental).
- [x] The same gate after merging `claude/ecstatic-dijkstra-qw5pf9` at
  `4e5b766` (TASK-629 to TASK-633): fmt and clippy clean; `cargo test`
  43 binaries, 1745 passed, 0 failed, no `SKIP`; "95 compared, none
  unused"; `./scripts/ci agents` passed.

## Result

Changed files: `AGENTS.md`, `CONTRIBUTING.md`, `docs/tasks/TEMPLATE.md`,
`scripts/check-task-index`, `.github/pull_request_template.md`,
`docs/tasks/INDEX.md`, and this record.
