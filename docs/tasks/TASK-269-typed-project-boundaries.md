# TASK-269: Respect tsconfig source boundaries and surface backend failures

- **Status**: Pending
- **Started**: 2026-08-28
- **Completed**: —
- **Commit**: —

## Purpose

The typed engine currently projects `.tt` files outside the configured
TypeScript program and can crash the backend with a synthetic-file lookup.
Align compiler inputs with project membership and preserve backend failure
information through the editor boundary.

## Scope

- Included: tsconfig membership, projection/query selection, CLI ICE contract,
  server `backendError`, and VS Code presentation
- Excluded: ordinary TypeScript module-resolution diagnostics

## Decisions

### Decision 1: Project membership comes from the TypeScript project model

- **Context**: Recursively scanning every `.tt` under the root disagrees with
  tsconfig `files`, `include`, and `exclude`.
- **Alternatives considered**: skip the failing query; ignore particular
  directories; or derive membership from the configured program.
- **Decision and rationale**: Use the configured project graph. Directory and
  error-string filters are heuristics and cannot implement tsconfig semantics.

## Work log

- 2026-08-28: Created from nightly audit finding 4.

## Issues and resolutions

None.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## Result

Pending.
