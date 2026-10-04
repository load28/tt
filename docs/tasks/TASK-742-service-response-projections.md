# TASK-742: Isolate service response projections

- **Status**: Complete
- **Started**: 2026-10-04
- **Completed**: 2026-10-04
- **Commit**: —

## Purpose

Keep request decoding and engine invocation visible in the JSON-lines
dispatcher while giving completion, signature, and diagnostic answer
projection one private owner. Preserve every existing observable result.

## Scope

- Included: Four response projections in server.rs; narrowly targeted
  public-protocol characterization; detailed design and review evidence.
- Excluded: Engine logic, request defaults, session/probe ownership, output
  expectations, dependencies, and the independent unfinished TASK-732.

## Decisions

### Decision 1: Project answers only after their existing engine calls

- **Context**: Four dispatch arms interleave request/engine orchestration
  with relatively long pure JSON projections.
- **Alternatives considered**: A generic serializer/DTO framework adds
  concepts and may erase differing omission/null conventions; extracting
  whole method handlers hides execution order.
- **Decision and rationale**: Move exact projection bodies into the
  existing responses module. Parameter reads and engine calls remain in
  server.rs, including converting diagnostics before asking for restates.

### Decision 2: Establish unchanged public-wire expectations before moving code

- **Context**: Existing engine-object tests do not pin every wire omission.
- **Alternatives considered**: Helper-only tests could mirror the new
  implementation; broad baseline regeneration would accept changes.
- **Decision and rationale**: Characterize complete small real server
  answers on unchanged production first, and compare a fixed larger
  before/after corpus as raw stdout/stderr/exit observations.

### Decision 3: Sequentially merge intermediate PRs after independent review

- **Context**: On 2026-10-04 the user explicitly authorized autonomous
  sequential implementation and intermediate merges, retaining the final
  merge for themselves. This supersedes TASK-740's original preparation-only
  publication policy and TASK-741's earlier publication restriction.
- **Alternatives considered**: Requesting approval for every intermediate
  PR contradicts the new instruction; merging before required checks or
  merging the final PR exceeds the authorized workflow.
- **Decision and rationale**: Implement each bounded slice in a fresh
  subagent context with a detailed main-authored brief. Main independently
  reviews, verifies local gates, publishes to main, waits for remote checks,
  and squash merges before the next implementation. Leave the final
  roadmap PR open for user inspection and merge.

## Work log

- 2026-10-04: Verified #136 had merged into its stacked design branch;
  #135 remained the integration PR to main. Its source tree exactly
  matched the locally verified TASK-741 tree at
  52966f9d7ebf395fc9d3a7af070d256a42e4314a. Marked #135 ready, updated its
  description to include both design and implementation, and waited for CI.
- 2026-10-04: Main wrote the detailed four-helper implementation brief;
  a fresh subagent prepared a fixed actual-response corpus read-only while
  the previous integration was pending. Preserved the source-built base
  executable outside the checkout at /tmp/tt-task-742-base-ttc, SHA-256
  15a40e7ade6924bf509e7a7de4840f8d2eb40c53d0bbd80a97e106c3b4a7d732.
- 2026-10-04: Created this task before tracked implementation changes on
  branch refactor/task-742-service-responses from main
  21793c6b2aefe6342d0261df712fb68a40210369 after #135 passed all five
  applicable remote CI jobs and was squash merged. The merged tree equals
  the source tree of the preserved baseline binary.
- 2026-10-04: The isolated implementation agent added two complete
  public-wire characterization tests. With production unchanged,
  `TTC_REQUIRE_TSGO=1 cargo test --test cli server_response_shapes` passed
  (2 passed, 0 failed/ignored, 129 unrelated tests filtered).
- 2026-10-04: The agent extracted completion_json, completion_detail_json,
  signature_help_json and service_diagnostic_json. `cargo fmt --check`
  passed; `TTC_REQUIRE_TSGO=1 cargo test --test cli --test public_api`
  passed all 131 CLI and 3 public API tests, with no failures/ignored/filtered.
- 2026-10-04: Main independently read the complete production/test diff,
  checked every moved projection and dispatcher call, and verified source
  snapshots against the exact merged base. The six existing helper bodies
  remain byte-identical; four moved bodies differ only in indentation and
  the necessary function/answer wrapper. Request decoding, defaults, engine
  arguments/error propagation, and diagnostic conversion before restates
  are preserved. No backend or reference-baseline file changed.
- 2026-10-04: Main verified raw equality for the fixed 18-request corpus:
  two baseline runs and head each return the same 4,256 stdout bytes,
  empty stderr and exit 0. Coverage includes completion null/empty fields,
  deprecated tags, direct auto-import additional edits, signature trigger
  variants and astral UTF-16 labels, diagnostics/related omissions and
  ordering, and an error followed by successful requests. Evidence/scripts
  are in /tmp/tt-task-742-review. Diagnostic warning/information, related
  Some(path), uncommon completion values, and nonempty restates are not all
  observed by this corpus; exact-body review and existing suites provide
  the remaining evidence. Both current related producers set path to None.
- 2026-10-04: Main started `./scripts/ci agents rust` with Rust 1.98.0,
  Node 24.19.0, pinned TypeScript 7.1.0-dev.20260826.1 and installed Bun;
  log: /tmp/tt-task-742-ci.log.
- 2026-10-04: The full local gate passed, exit 0: 30 suites, 1,288 tests
  passed, 0 failed/ignored/filtered. Formatting, clippy with warnings denied,
  pinned TypeScript/extension prerequisites and fuzz crate checks passed.
  Baseline tracking compared 5,476 files with none unused; 5,793 unsampled
  matrix files remain unjudged by the default gate (not a nightly sweep).
  Main reran structural/whole-dispatch comparisons and confirmed no
  reference baseline/fixture changes. Implementation accepted for PR review.

## Issues and resolutions

### Issue 1: Broad global completion order varies on the unchanged base

- **Symptom**: Two runs of the identical baseline executable returned a
  different order for broad global completion request IDs 13 and 19.
- **Cause**: The variability already occurs before production edits; it is
  not an extraction regression. Raw exploratory observations are preserved
  under /tmp/tt-task-742-review.
- **Resolution**: Do not fix or normalize this existing behavior. Preserve
  the exploratory observations separately, and use a repeatably identical
  fixed corpus for raw differential comparisons. Direct auto-import resolve
  still observes additionalEdits without the unstable global list. The
  unchanged completion projection body is independently reviewed.

### Issue 2: The initial characterization fixture cannot find TypeScript

- **Symptom**: Tests using an OS temporary project cannot discover the
  repository's pinned TypeScript package.
- **Cause**: Project toolchain discovery searches the project's ancestors.
- **Resolution**: Use the existing Workspace::in_repo test fixture. Both
  tests pass on unchanged production with the same expected answers.

## Regression test (fails before the fix)

Not applicable: This task preserves behavior and fixes no bug. New
characterization tests must pass against the unchanged production source.

## Verification

- [x] New characterization passes before production changes.
- [x] Independent structural equivalence and call-order review.
- [x] Fixed-corpus raw stdout/stderr/exit comparison.
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` through `./scripts/ci agents rust`
- [x] Reference baselines and fixtures unchanged.
- [x] `node scripts/check-task-index` and `git diff --check`

## Result

Moved four service answer projections to src/server/responses.rs and replaced
their inline dispatcher bodies with calls. Added two public-wire tests and
their CLI module, the detailed implementation brief, and this task record.
Updated shared workflow documentation and prior task notes to record the
user's superseding intermediate-merge authorization. No backend behavior,
reference baseline or public API changed. Main review and local gates passed;
publication/remote checks precede the intermediate merge. The next slice is
target-placement diagnostic ownership, with its own fresh implementation
context. The final roadmap merge remains reserved for the user.
