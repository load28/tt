# TASK-741: Isolate server response projection

Workflow update: The user subsequently authorized publication and then
sequential intermediate merges, as recorded in
[TASK-742](TASK-742-service-response-projections.md). PR #136 was merged into
the design branch, and #135 integrated its identical verified tree into main
at 21793c6b2aefe6342d0261df712fb68a40210369 after all applicable CI passed.
The historical publication restriction below is superseded; the final
roadmap merge remains reserved for the user.

- **Status**: Complete
- **Started**: 2026-10-03
- **Completed**: 2026-10-03
- **Commit**: —

## Purpose

Separate conversion of computed engine answers to JSON from server transport,
dispatch, and session state without changing observable results.

## Scope

- Included: The six existing helpers specified in
  [the implementation brief](../design/refactoring-server-responses.md),
  relevant characterization, and independent review/verification.
- Excluded: Request handlers, inline projections, public APIs, semantics,
  dependencies, baseline changes, and unfinished TASK-732 work.

## Decisions

### Decision 1: Move existing functions before redesigning any response

- **Context**: `src/server.rs` interleaves pure response projections with
  operations on live projects and document overlays.
- **Alternatives considered**: Extract all dispatch branches at once, or
  introduce serializable response types. Both require a much larger
  equivalence argument than moving the existing helper bodies.
- **Decision and rationale**: A private `server::responses` module with
  `pub(super)` functions gives the projection boundary one owner while
  preserving call sites and implementation exactly.

## Work log

- 2026-10-03: Created the task before implementation on a branch stacked on
  design commit `9fffbc2f`. Original production base is
  `b62f2b6e727724748747d2a76351034f4daa010e`.
- 2026-10-03: Built original source with Rust 1.98.0 (`cargo build --locked`),
  copied that executable to `/tmp/tt-refactor-base-ttc`, and started
  `./scripts/ci agents rust` before production changes. Logs:
  `/tmp/tt-refactor-base-build.log`, `/tmp/tt-refactor-base-ci.log`.
- 2026-10-03: Delegated implementation to a fresh-context subagent with the
  shared design and exact brief. The agent prepared fixed JSON-lines requests
  and raw-output capture scripts in `/tmp/tt-server-responses-review`.
- 2026-10-03: After all baseline test executables and the CLI had compiled,
  allowed the source-only extraction while those original binaries continued
  testing. No rebuild or extension clean/compile was allowed during that run.
  Source-reading `public_api` checks inspect dispatch/parameter reads, which
  the extraction leaves unchanged.
- 2026-10-03: Main reviewed every moved function and all remaining parent
  changes, independently comparing against `b62f2b6e`. Every body is
  byte-identical, signatures differ only in `pub(super)` and rustfmt wrapping,
  and reversing moves/imports restores the whole original parent exactly.
  `ProtocolPositions` stays in both modules where used; only `Location` and
  `Range` leave the parent's engine imports. The new module introduces no
  state, filesystem access, backend dependency, or public API.
- 2026-10-03: Reviewed the agent's separate reproducible comparator,
  `/tmp/tt-server-responses-review/compare-source.py`, and its raw wire
  comparison strategy. No redundant helper-mirroring tests were added.
- 2026-10-03: Base `./scripts/ci agents rust` passed: 30 test suites,
  1,286 libtest tests passed, none failed/ignored/filtered; 5,476 baselines
  compared, 5,793 baseline files belonging to unsampled matrix cases left
  unjudged, none unused. TypeScript and extension prerequisites were required,
  the upstream TypeScript corpus was available, and the fuzz crate check passed.
  These are the repository's normal sampled gates, not full nightly matrices.
- 2026-10-03: Started the same head gate in
  `/tmp/tt-refactor-head-ci.log`. Main independently inspected base differential
  replies: recursive symbols, nonempty definitions, same-file and cross-file
  labels, literal completion ordering/replacement ranges, and working typed
  checks (`backendError: null`).
- 2026-10-03: Main ran the head differential capture and compared raw stdout,
  stderr, and exit bytes. A follow-up temporary case explicitly added Hangul
  alongside an emoji on CRLF input. Final corpus: 15 text replies (4,834
  stdout bytes) and 24 typed replies (9,771 bytes); both suites matched the
  base exactly, exited zero, and wrote no stderr. All six helpers are exercised.
  The Unicode case's diagnostic range is `2:30..2:40` and its edits target
  `2:57..2:58`, measured in UTF-16 rather than Unicode scalar values.
- 2026-10-03: Head `./scripts/ci agents rust` passed with the same 30 suites,
  1,286 tests, and 5,476 compared baselines as the base. No failed, ignored,
  or filtered libtest tests; the same 5,793 unsampled matrix baseline files
  remained unjudged. Baseline tracking reported none unused, and the fuzz
  crate check passed. Neither reference baselines nor fixtures changed.
- 2026-10-03: Main accepted the bounded implementation after source review,
  independent differential checks, and completed base/head gates. Marked the
  task and index Complete and prepared a separate local implementation commit.

### Differential reproduction and limits

Temporary evidence is under `/tmp/tt-server-responses-review`, outside tracked
test expectations. The fixed project and request streams use the same absolute
paths for both binaries. Reproduce with `prepare.py`, then:

```sh
python3 /tmp/tt-server-responses-review/run.py /tmp/tt-refactor-base-ttc base text
python3 /tmp/tt-server-responses-review/run.py /tmp/tt-refactor-base-ttc base typed
python3 /tmp/tt-server-responses-review/run.py /workspace/tt/target/debug/ttc head text
python3 /tmp/tt-server-responses-review/run.py /workspace/tt/target/debug/ttc head typed
python3 /tmp/tt-server-responses-review/compare-wire.py
python3 /tmp/tt-server-responses-review/check-hangul.py
python3 /tmp/tt-server-responses-review/compare-source.py
```

These temporary artifacts are session evidence; the durable regression gate is
the repository's unchanged CLI, public API, editor, and native suites. The
corpus also observes null advice, optional/absent fields, empty collections,
all pattern-completion kinds, replacement ranges, and successful requests after
malformed JSON/UTF-8. Unavailable suggestion/label sources, an explicit label
path equal to the default path, and non-UTF8 paths were not exercised by this
corpus. Their unchanged bodies and unchanged input/type bindings provide the
structural preservation argument; do not claim new dynamic coverage for them.

## Issues and resolutions

### Issue 1: Remote publication requires explicit approval

- **Symptom**: Automatic approval review rejected pushing the completed
  design branch to `https://github.com/load28/tt.git`.
- **Cause**: The reviewer requires explicit authorization for this payload
  and external destination, beyond the request to work in PR-sized units.
- **Resolution**: Continue local implementation and verification. Do not
  retry via another tool. Request approval with concrete reviewed branch
  contents before remote push/PR creation; no remote PR has been created.

## Regression test (fails before the fix)

Not applicable: This is a behavior-preserving extraction, not a bug fix.
Any new characterization must pass against unchanged production code first.

## Verification

- [x] Base `./scripts/ci agents rust`
- [x] Main review: moved bodies identical apart from visibility
- [x] Main review: parent unchanged apart from imports, declarations, and moves
- [x] Relevant external server behavior unchanged
- [x] Head `cargo fmt --check`
- [x] Head `cargo clippy --all-targets -- -D warnings`
- [x] Head `cargo test`
- [x] Head Rust gate including baseline tracking
- [x] Reference baselines and fixtures unchanged
- [x] `node scripts/check-task-index` and `git diff --check`

## Result

Moved the six existing JSON projection helpers from `src/server.rs` to
`src/server/responses.rs`, preserving their bodies and all callers. The parent
now owns transport/dispatch/session orchestration, with one private leaf module
owning these response projections. Added this record and its index entry;
no production files beyond the two server files changed.

The design lives on `refactor/task-740-behavior-preservation-design`; this
implementation lives on `refactor/task-741-server-responses`, stacked on the
design commit. Both are local review units; no remote PR was created because
publication requires the approval described above. Any eventual merge follows
the repository's normal PR-to-main workflow, rebasing dependent work after the
design PR lands.

This completes the first implementation slice, not the whole-project roadmap.
The next candidate is PR 2's remaining inline server response projections;
it needs its own detailed brief and fresh implementation-agent context before
work starts. TASK-732 remains independent and unfinished.
