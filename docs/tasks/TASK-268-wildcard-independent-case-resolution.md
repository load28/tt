# TASK-268: Diagnose unknown cases independently of wildcard coverage

- **Status**: Complete
- **Started**: 2026-08-28
- **Completed**: 2026-08-28
- **Commit**: —

## Purpose

Unknown case resolution currently depends on whether other arms identify the
variant and therefore changes when a wildcard is added. Resolve case names
from the scrutinee's authoritative domain independently of coverage.

## Scope

- Included: case-name resolution with and without `_`, batch and typed parity,
  server and content-mapper regressions
- Excluded: field transport and exhaustiveness suppression ownership

## Decisions

### Decision 1: Wildcards provide coverage but no name evidence

- **Context**: A wildcard changes coverage but must not change whether a named
  constructor exists.
- **Alternatives considered**: treat `_` as evidence that disables source
  resolution; rely on TS2678; or apply the existing conservative near-miss
  licence to the position's named constructors only.
- **Decision and rationale**: A position with one named constructor reports
  only a unique one-edit candidate across visible variants. `_` contributes no
  name evidence. Several named constructors continue through full subject
  identification, and ambiguous candidates remain silent for hand-written
  TypeScript unions.

## Work log

- 2026-08-28: Created from nightly audit finding 3.
- 2026-08-28: Started after TASK-267 made resolver-error ownership the sole
  suppression source.
- 2026-08-28: Generalized the resolver's single-constructor near-miss licence
  from statement patterns to every pattern site, without adding a
  wildcard-specific semantic branch.
- 2026-08-28: Added resolver, batch CLI, typed CLI/server, and content-mapper
  regressions for an imported `Crad` pattern followed by `_`.
- 2026-08-28: Updated the language contract to state that `_` changes coverage
  but contributes no name-resolution evidence.
- 2026-08-28: Ran `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, and `cargo test`; all passed.

## Issues and resolutions

- The initial content-mapper assertion expected column 32, but the generated
  fixture places `Crad` at column 27. The assertion was corrected to the source
  token's measured location.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Adding a wildcard no longer hides a uniquely identifiable imported case typo.
The source-level `unknown-case` diagnostic and applicable edit now agree across
batch, typed server, and content-mapper paths, while ambiguous single-name
patterns remain unclaimed. Changed files: `src/resolve/mod.rs`,
`docs/ai/tt.md`, `tests/resolve.rs`, `tests/integration.rs`, `tests/native.rs`,
and `tests/content_mapper.rs`.
