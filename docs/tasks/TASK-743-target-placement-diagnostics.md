# TASK-743: Isolate target placement diagnostics

- **Status**: Complete
- **Started**: 2026-10-04
- **Completed**: 2026-10-04
- **Commit**: —

## Purpose

Separate the rules that turn an Evaluation IR lowering plan into placement
diagnostics from compile/analyze/report orchestration. Make the phase boundary
readable while preserving diagnostic text, spans, ordering and suppression.

## Scope

- Included: Eight existing helper/constant definitions moved to a private
  compile::target_diagnostics module, with two parent-visible entry points.
- Excluded: Public API changes, plan construction/recovery, diagnostic policy
  changes, expected-output updates, and the independent TASK-732 work.

## Decisions

### Decision 1: Project existing plan facts, leaving retries with orchestration

- **Context**: compile.rs mixes public pipelines with target placement
  message selection, accumulation, stable ordering and duplicate suppression.
- **Alternatives considered**: Moving recovered_target_errors too would mix
  retrying plan construction into answer projection; rewriting the message
  matches or deduplication obscures exact equivalence.
- **Decision and rationale**: Move the existing eight definitions unchanged,
  expose only target_errors/nonredundant_target_errors to the parent, and
  leave recovered_target_errors and every call site in compile.rs.

### Decision 2: Use existing public cases plus byte-exact structural evidence

- **Context**: Public compile tests already cover recovered placement order,
  lexical spans, crossing suppression and a 2,142-cell host/value matrix;
  case baselines pin full owner-specific messages and help.
- **Alternatives considered**: New helper-only tests would duplicate the
  implementation; changing references is outside this program's contract.
- **Decision and rationale**: Retain those tests and prove exact definition
  bytes plus inverse reconstruction of the parent against the accepted base.
  Synthetic absent-offset/tied-offset cases are structural-only evidence;
  no exhaustive test claim is made.

## Work log

- 2026-10-04: Main wrote the detailed extraction brief; a fresh-context
  implementation agent inspected dependencies and public coverage read-only
  while the preceding PR's checks ran. No tracked source edits or concurrent
  Cargo tests/builds occurred during preparation. Evidence is under
  /tmp/tt-task-743-review; the source-built prior-slice binary is preserved at
  /tmp/tt-task-743-base-ttc (SHA-256
  d937975634480d7ca867c167659457a7de32b8a35602249cc534bfb732b347df).
- 2026-10-04: After all five applicable checks of CI run 37165840916
  passed, main squash merged [PR #137](https://github.com/load28/tt/pull/137).
  Created branch refactor/task-743-target-diagnostics from latest main
  dea2ba2ab11e7813155ddfc449503e167b2753e8, whose tree exactly equals the
  locally verified TASK-742 head. Created this task and index entry before
  authorizing production edits. Detailed brief:
  [target diagnostics](../design/refactoring-target-diagnostics.md).
- 2026-10-04: The implementation agent extracted all eight definitions into
  src/lib/compile/target_diagnostics.rs. Only target_errors and
  nonredundant_target_errors gained pub(super); recovery and all callers stay
  in compile.rs. Focused cargo test --test compile --test snapshot
  --test practical_diagnostics passed (186 + 4 + 1 tests, no failed, ignored,
  or filtered tests). The agent's fmt and diff checks passed.
- 2026-10-04: Main read the entire production diff and new child, inspected
  the byte comparator, and independently reran it against main
  dea2ba2ab11e7813155ddfc449503e167b2753e8. All eight complete definitions
  match their original bytes after the two visibility prefixes, including
  every diagnostic literal. Reversing the extraction reconstructs the
  entire original compile.rs exactly. The child has no additional behavior;
  stable category ordering, absent-offset handling, owner-priority match
  arms, and span-sensitive duplicate suppression remain unchanged.
- 2026-10-04: Main started ./scripts/ci agents rust; log:
  /tmp/tt-task-743-ci.log. No focused backend process was still running.

- 2026-10-04: Main's ./scripts/ci agents rust exited 0: fmt, clippy with
  warnings denied, all 30 test suites (1,288 passed; zero failed, ignored,
  or filtered), baseline tracking, and the fuzz-crate check passed. The
  required TypeScript corpus and extension were present and enforced.
  Tracking compared 5,476 baseline files with none unused; 5,793 files from
  unsampled matrix cases remain unjudged under the existing standard policy.
  This is not a claim of an exhaustive nightly matrix run. Toolchain:
  Rust 1.98.0, Node 24.19.0, pinned TypeScript 7.1.0-dev.20260826.1.
- 2026-10-04: Main confirmed no baseline/fixture/test changes, reran task-index
  and whitespace checks, and accepted the complete diff for publication.
  The production files are compile.rs and its new target_diagnostics child;
  remaining changes are this record, the task index, the detailed brief,
  and the shared design's integration history.

## Issues and resolutions

### Issue 1: Child resolution from a path-attributed module

- **Symptom**: The initial fmt check sought src/lib/target_diagnostics.rs.
- **Cause**: lib.rs declares compile with a path attribute, so an unqualified
  child declaration resolves beside that file rather than beneath compile/.
- **Resolution**: Main reviewed and approved an explicit
  #[path = "compile/target_diagnostics.rs"] on the private declaration.
  Public module paths and re-exports remain unchanged; fmt then passed.

## Regression test (fails before the fix)

Not applicable: This task preserves behavior through mechanical extraction
and fixes no bug. Existing public-boundary tests remain unchanged.

## Verification

- [x] Exact moved-definition bytes and unchanged parent call sites.
- [x] Independent main review of the complete diff.
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` through `./scripts/ci agents rust`
- [x] Reference baselines and fixtures unchanged.
- [x] `node scripts/check-task-index` and `git diff --check`

## Result

Complete locally and accepted by main review for the intermediate PR.
Diagnostic policy now has a named private owner; compile/analyze/report
orchestration, recovery, public interfaces, and expected results are unchanged.
Remote checks must pass before the authorized intermediate squash merge.
The next candidate isolates recovery source projection; the final roadmap
merge remains reserved for the user.
