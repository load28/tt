# TASK-309: Adopt compiler UI-test conventions for diagnostics

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: `TASK-308: add practical diagnostic UI matrix`

## Purpose

Adapt the practical diagnostic matrix to the regression-testing conventions
used by the TypeScript and Rust compilers, so error intent, complete rendered
output, editor behavior, and fixes remain independently reviewable.

## Scope

- Included: Source-local exhaustive error annotations, normalized CLI stderr
  baselines, an explicit baseline update workflow, and applied quick-fix
  verification for the practical diagnostic projects
- Excluded: Importing upstream test runners, changing diagnostic semantics, or
  converting unrelated unit and snapshot suites

## Decisions

### Decision 1: Combine source annotations with full output baselines

- **Context**: TypeScript compiler tests review generated output baselines,
  while rustc UI tests additionally require source-local error annotations so
  a generated snapshot cannot silently bless a missing or extra error.
- **Alternatives considered**: Keep JSON expectations alone, copy only one
  upstream system, or combine their complementary checks.
- **Decision and rationale**: Keep the structured manifest for shared CLI/LSP
  details, add exhaustive `//~ ERROR[code]` annotations to the entry source,
  and compare the complete normalized CLI output to `expected.stderr`.

### Decision 2: Exercise fixes as transformations

- **Context**: A code-action title and edit range can be correct individually
  while their combined application produces the wrong document.
- **Alternatives considered**: Continue checking edit fields only, or apply the
  action and compare the whole resulting document.
- **Decision and rationale**: Store a `.fixed` source next to any fixture with
  a quick fix, apply the editor edit, compare the entire document, and publish
  the edited version to ensure the repaired diagnostic disappears.

## Work log

- 2026-08-30: Reviewed the official TypeScript compiler baseline/Fourslash
  guidance and rustc compiletest UI/rustfix guidance, then compared them with
  TASK-308's practical matrix.
- 2026-08-30: Added exhaustive source-local error annotations and stripped
  them before invoking the real CLI and language server.
- 2026-08-30: Added normalized `expected.stderr` baselines for all four
  application projects and documented the deliberate update-and-review flow.
- 2026-08-30: Applied the `val` editor quick fix to the complete document,
  compared it with `expected/cache.fixed.tt`, and required the final
  republished diagnostics to omit the repaired error.
- 2026-08-30: Ran both focused matrices and the complete local CI gate.

## Issues and resolutions

None.

## Verification

- [x] Focused practical CLI diagnostic matrix — 4 projects, 11 exhaustive
  source annotations, and 4 complete stderr baselines
- [x] Focused practical editor/LSP diagnostic matrix — 4 projects and applied
  `val` quick-fix verification
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci` — agents, rust, npm, native, and extension passed

## Result

The practical matrix now uses compiler-style UI test contracts: source-local
exhaustive error declarations, complete normalized CLI snapshots with an
explicit bless-and-review workflow, structured editor assertions, and a
whole-document quick-fix result followed by diagnostic republication.

Changed files: `tests/practical_diagnostics.rs`,
`tests/fixtures/practical-diagnostics/`,
`editors/vscode/server/src/test/server.test.ts`, and the task records.
