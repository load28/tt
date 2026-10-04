# TASK-745: Isolate language service text presentation

- **Status**: Complete
- **Started**: 2026-10-04
- **Completed**: 2026-10-04
- **Commit**: —

## Purpose

Separate pure hover, documentation, and signature-label formatting from the
language service's session management and source-coordinate projection.

## Scope

- Included: Move `split_hover`, `split_markdown_hover`, `docs_text`, and
  `parameter_span` into a private `service::presentation` child.
- Excluded: Stateful source links, backend requests, coordinate mapping,
  behavior fixes, public API changes, baseline updates, and TASK-732.

## Decisions

### Decision 1: Preserve the current caller boundary through restricted re-exports

- **Context**: Three helpers are visible within `engine::language` and consumed
  by project operations; their formatting implementation needs no session state.
- **Alternatives considered**: A language-level sibling would require changing
  the language module and consumers; moving source links would pull session and
  filesystem concerns into presentation.
- **Decision and rationale**: Keep all callers unchanged and re-export the three
  entry points from `service` with their current visibility. In the private child,
  `pub(in super::super)` retains access only within `engine::language`.
  `split_markdown_hover` remains private. Preserve every body literally.

### Decision 2: Continue on the verified local stack

- **Context**: TASK-744 is locally complete and uncommitted on
  `refactor/task-744-recovery-projection`; the user requested the next slice.
- **Alternatives considered**: Discarding, stashing, or rewriting those changes
  would disrupt the completed recovery work.
- **Decision and rationale**: Preserve TASK-744's production and individual
  documentation files byte for byte and continue locally on the same branch.
  HEAD is `85729d01a6cc169d26bce81b9ee25e7b7734f9ae`; the effective validation
  base also includes the verified TASK-744 extraction. This is local preparation,
  not a claim that TASK-744 has merged or that a standalone successor PR exists.

## Work log

- 2026-10-04: Passed `./scripts/doctor` with the existing `/workspace/.tools`
  cargo/npm bin paths, `CARGO_HOME=/workspace/.tools/cargo`, and
  `RUSTUP_HOME=/workspace/.tools/rustup`. No setup or reinstall was needed.
- 2026-10-04: Inspected all four definitions and their callers. Saved the
  unchanged service source and hashes of TASK-744 files outside tracked sources.
  Started `TTC_REQUIRE_TSGO=1 cargo test --offline --lib engine::language::tests`
  before extraction; log: `/tmp/tt-task-745-base-language.log`.
- 2026-10-04: Base language tests exited 0: 28 passed, zero failed/ignored,
  391 intentionally filtered by the language-module selection. Base
  `TTC_REQUIRE_TSGO=1 TT_REQUIRE_EXTENSION=1 TTC_REQUIRE_TYPESCRIPT_CASES=1
  cargo test --offline --test native --test editor_cases` also exited 0:
  2 editor and 173 native tests passed, with zero failed, ignored, or filtered.
  Log: `/tmp/tt-task-745-base-services.log`.
- 2026-10-04: Moved all four complete definitions to
  `src/engine/language/service/presentation.rs`; added only the private child
  declaration and restricted re-exports to `service.rs`. The child imports no
  session/project state. The comparator `/tmp/tt-task-745-compare.py` confirms
  exact definitions after reversing the three visibility qualifiers, full
  byte-exact inverse reconstruction of the parent, and unchanged hashes for
  TASK-744's production and individual documentation files.
- 2026-10-04: Reviewed the complete parent diff, child, and project call sites.
  No signature wrapping, new imports, or caller edits were needed. Head
  `cargo fmt --check` and the focused language suite passed (28 passed,
  zero failed/ignored, 391 intentionally filtered). Head log:
  `/tmp/tt-task-745-head-language.log`.
- 2026-10-04: Started `./scripts/ci agents rust` with Rust 1.98.0,
  Node 24.19.0, and TypeScript 7.1.0-dev.20260826.1; log:
  `/tmp/tt-task-745-ci.log`. No other backend-heavy suite or extension build
  was running.
- 2026-10-04: The full gate exited 0: formatting, clippy with warnings denied,
  all 30 Rust suites (1,288 passed, zero failed/ignored/measured/filtered),
  baseline tracking, and the fuzz-crate check passed. The extension and pinned
  upstream TypeScript corpus were present and enforced. Baseline tracking
  compared 5,476 files with none unused; 5,793 files from unsampled matrix cases
  remain unjudged under the normal sampling policy. No exhaustive matrix claim
  is made.
- 2026-10-04: Repeated the exact-definition and parent-reconstruction checks
  after validation. TASK-744's four preserved files still match their starting
  hashes. Baselines, fixtures, the language module, project callers, and tests
  remain unchanged. Task-index and whitespace checks passed.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: This task is a behavior-preserving extraction and fixes no bug.

## Verification

- [x] Exact moved definitions and inverse reconstruction of the service parent.
- [x] Language-unit, native, and editor results before and after extraction.
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Reference baselines, fixtures, and TASK-744 files unchanged.
- [x] Task-index and whitespace checks.

## Result

Complete locally. Pure text presentation has a private owner, while stateful
link projection, service requests, and consumers retain their existing code.
Changed production files are `src/engine/language/service.rs` and its new
`service/presentation.rs` child. Documentation changes are this task, its
detailed brief, the task index, and the shared roadmap progress note.

TASK-744 and TASK-745 are recorded as separate local commits; neither has
been published. The next
roadmap candidate is completion operations under the existing language-project
owner and requires a fresh detailed brief before implementation.
