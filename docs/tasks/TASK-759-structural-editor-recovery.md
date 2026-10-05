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
- 2026-10-05: The user approved the written design. Prepared
  `docs/superpowers/plans/2026-10-05-structural-editor-recovery.md` with four
  dependency-ordered stages, parser/host/projection interfaces, regression-first
  assertions, existing case and incremental runners, and full gate criteria.
  Reviewed the plan against the approved spec and the five review-focus input
  classes. Implementation remains pending plan review and execution selection.

## Issues and resolutions

### Issue 1: The default branch prefix conflicts with an existing ref

- **Symptom**: Git rejected `codex/structural-editor-recovery`.
- **Cause**: `refs/heads/codex` already exists and prevents nested branch names.
- **Resolution**: Used `codex-structural-editor-recovery` without altering the
  existing branch.

## Regression test (fails before the fix)

- **Path**: `tests/compile.rs::editor_projection_preserves_matches_after_a_missing_initializer`.
- **Observed failure**: Against the unchanged production code at `e12014d7`,
  `cargo test --test compile editor_projection_preserves_matches_after_a_missing_initializer`
  failed at `report.emit.is_some()`: SourceNotTypeScript at byte 76, Expression
  expected. The parser regression also failed on `TS1109` at span 16..17.
  Logs: `/tmp/tt-recovery-red.log`, `/tmp/tt-recovery-parser-red.log`.

## Verification

- [x] `./scripts/doctor`
- [x] `./scripts/check-task-index`
- [x] `git diff --check`
- [x] Design review
- [x] Implementation plan review and execution-method selection
- [x] Failing regression assertions observed before production changes
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`
- [ ] Baseline changes reviewed and committed with the change

## Result

Design approved in `docs/design/structural-editor-recovery.md`. The concrete
[implementation plan](../superpowers/plans/2026-10-05-structural-editor-recovery.md)
is ready for review. No production code or test expectations have changed.
Implementation is pending plan review and execution-method selection.

## Implementation ledger

- 2026-10-05: Native execution approved. Required execution instructions were
  absent from the installed partial skill bundle; read the exact pinned
  upstream commit `8ca22dba9a94f28898bbce59f2537ff4d87c747d` in temporary storage.
- 2026-10-05: Baseline `cargo test` reported 418 passing library tests and one
  failure: `typescript::native::idle_tests::an_idle_host_waits_for_the_next_request_without_spending_cpu`.
  Isolated reproduction identified sandbox denial of `ps`, not a compiler
  defect. The full verification gate must run with that read-only inspection
  permitted.
- Ruling: Preserve malformed host text in the editor output when the partial
  tree can locate every tt owner. Missing AST nodes require no synthetic
  output bytes; original mappings and TypeScript syntax diagnostics remain
  authoritative. Explicit repair mappings are needed only for actual generated
  recovery text. This avoids inventing edits at zero-width AST holes; if an
  unknown owner cannot be located, it must use parser-owned recovery instead.

- 2026-10-05: Added explicit strict/editor parser modes, transactional recovery
  records, host-owned skipped-input recovery, and shared projection metadata.
  Missing initializer, trailing member, missing annotation, nested list, lexical
  region, JSX, and valid speculative-syntax parser regressions pass.
- Ruling (supersedes the earlier raw-text-only projection assumption): An open
  call followed by a generated match prelude produced TS2304 on `let` and
  `$tt_v0$a`. Materialize parser-recorded missing closing delimiters before
  lowering; restore all source coordinates and classify inserted bytes as
  glue. Keep unmodified malformed text for holes needing no insertion.
- Ruling: Reuse existing `recovered` spans and add `syntax_repairs` for repaired
  host productions instead of a second diagnostic-origin enum. These have
  separate policies: replaced text owns recovery effects; delimiter repairs
  retain primary syntax causes but do not suppress independent type errors.
  Parser-local IDs remain local to each parse; report source provenance and
  the existing content-version snapshot are the consumer contract.
- 2026-10-05: Real mapper regression initially reported duplicate tt31/TS1109.
  The mapper now defers to TypeScript only when its input still contains the
  original malformed host syntax, using the same recovery provenance as the
  engine. The real-TypeScript regression passes.
- 2026-10-05: Unicode/emoji/CRLF regression verifies every copied mapping after
  multiple zero-width insertions and confirms strict compilation still fails.
  `cargo clippy --all-targets -- -D warnings` passed. Full gates are pending.

- 2026-10-05: The initial raw open-array projection reproduced generated-name
  errors in the deterministic sequence. Added argument/array/object grammatical
  boundaries and missing-delimiter records; both engine and real mapper
  sequences now pass for tt and ttx, including JSX deletion and repair.
- 2026-10-05: Full editor cases exposed an operand diagnostic shifted to the
  enclosing match anchor in JSX. Retaining incomplete operands preserves arm
  bindings/completion; copied-source boundary mapping now crosses only generated
  whitespace and no new owner, preserving the original diagnostic token.
  Rejected whole-match masking because it removed valid arm-local completions.
  Existing arm-diagnostic and deprecated-member completion baselines are unchanged.
- 2026-10-05: `cargo test --lib` passed all 419 tests with `ps` access, including
  strict pass-through/scaling/stack invariants. Final full gate remains pending.
