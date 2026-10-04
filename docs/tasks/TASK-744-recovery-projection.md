# TASK-744: Isolate recovery source projection

- **Status**: Complete
- **Started**: 2026-10-04
- **Completed**: 2026-10-04
- **Commit**: —

## Purpose

Separate recovery selection, byte-preserving source masking, and recovered
variant declarations from compilation and retry orchestration.

## Scope

- Included: Move five existing helpers into private `compile::recovery`.
- Excluded: Recovery policy, parser behavior, retries, public APIs, baseline
  updates, and the independent TASK-732 work.

## Decisions

### Decision 1: Preserve the existing phase and state boundaries

- **Context**: `compile.rs` owns both the retry loop and leaf transformations.
- **Alternatives considered**: Moving the retry loop would couple recovery
  helpers to compilation; rewriting masking or selection would obscure equality.
- **Decision and rationale**: Move complete definitions unchanged except for
  parent visibility and rustfmt's wrapping of one signature. Keep
  `overwrite_recovery` private; expose the other four
  functions only to the parent. Leave every call site and retry in place.
  See the [detailed brief](../design/refactoring-recovery-projection.md).

## Work log

- 2026-10-04: Resumed from main `85729d01a6cc169d26bce81b9ee25e7b7734f9ae`
  after TASK-743 merged as PR #138. Confirmed remote main with `git ls-remote`;
  created branch `refactor/task-744-recovery-projection` before tracked edits.
- 2026-10-04: Located the existing tools under `/workspace/.tools` and passed
  `./scripts/doctor` using its cargo/npm bin directories on PATH,
  `CARGO_HOME=/workspace/.tools/cargo` and
  `RUSTUP_HOME=/workspace/.tools/rustup`. No setup or reinstall was performed.
- 2026-10-04: Started unchanged-source `cargo test --offline --test compile
  --test editor_cases --test native`; log: `/tmp/tt-task-744-base.log`.
- 2026-10-04: The unchanged compile suite passed all 186 tests; the editor
  suite failed against stale extension output before the native suite ran.
  Rebuilt the extension with `npm run compile` in `editors/vscode`, confirmed
  the parent still exactly matched HEAD, and reran editor/native with
  `TTC_REQUIRE_TSGO=1 TT_REQUIRE_EXTENSION=1`. Exit 0: 2 editor and 173 native
  tests passed, none failed or ignored; `/tmp/tt-task-744-base-rebuilt.log`.
- 2026-10-04: Fetched the pinned upstream TypeScript corpus through the
  environment's supported network permission. The initial editor runs lacked
  that corpus, so their upstream parity test returned early; a separate base
  parity run now enforces `TTC_REQUIRE_TYPESCRIPT_CASES=1` before head validation.
- 2026-10-04: The enforced base upstream parity run exited 0 (1 passed,
  1 intentionally filtered; `/tmp/tt-task-744-base-corpus.log`). Restored the
  extracted head and checked all five moved definitions against the pinned
  base with `/tmp/tt-task-744-compare.py`. Only four visibility prefixes and
  one rustfmt signature wrap differ; reversing the extraction reconstructs
  the complete original parent byte for byte. Reviewed the new child and all
  parent call sites, including withheld output, retries, and anchor fields.
- 2026-10-04: Started `./scripts/ci agents rust` on the extracted head;
  log: `/tmp/tt-task-744-ci.log`. No concurrent editor rebuild or Cargo suite
  was running. Toolchain: Rust 1.98.0, Node 24.19.0, TypeScript
  7.1.0-dev.20260826.1.
- 2026-10-04: `./scripts/ci agents rust` exited 0. Formatting, clippy with
  warnings denied, all 30 Rust test suites (1,288 passed; zero failed, ignored,
  measured, or filtered), baseline tracking, and the fuzz-crate check passed.
  The pinned TypeScript corpus and extension were present and enforced.
  Baseline tracking compared 5,476 files, with none unused; 5,793 files from
  unsampled matrix cases remain unjudged under the standard gate's sampling
  policy. This is not an exhaustive matrix run.
- 2026-10-04: Repeated the structural comparator after the gate, confirmed
  reference baselines and fixtures unchanged, and checked the task index and
  whitespace. Changed production files: `src/lib/compile.rs` and
  `src/lib/compile/recovery.rs`. Documentation: this task, its detailed brief,
  `docs/tasks/INDEX.md`, and the shared refactoring program's progress notes.

## Issues and resolutions

### Issue 1: Stale generated extension output

- **Symptom**: The unchanged base reported editor baseline differences, including
  completion kinds and snippets in `completionEntryKinds.baseline`.
- **Cause**: Generated extension JavaScript did not match the checked-out source.
- **Resolution**: Rebuilt with `npm run compile`; the unchanged base's editor
  suite then passed. No reference baselines or production fixes were accepted.

## Regression test (fails before the fix)

Not applicable: Mechanical behavior-preserving extraction; no bug is fixed.

## Verification

- [x] Byte-exact moved definitions (apart from visibility and one signature
  wrap) and inverse reconstruction of the parent.
- [x] Before/after public recovery and editor tests (head through the full gate).
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Baselines and fixtures unchanged.
- [x] Task-index and whitespace checks.

## Result

Complete locally. Recovery selection, masking, and variant declarations now
have a private module; public behavior and compilation orchestration remain
unchanged. The verified extraction is recorded as a local commit; it has not been
published. The next roadmap candidate is hover/documentation formatting
in `engine/language/service.rs`; it needs a fresh bounded brief before edits.
