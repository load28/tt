# TASK-320: Audit mixed-source runtime and project semantics

- **Status**: In progress
- **Started**: 2026-09-04
- **Completed**: —
- **Commit**: —

## Purpose

Find failures that remain after single-file parsing and emission succeed by
exercising mixed `.tt`, `.ttx`, `.ts`, and `.tsx` project graphs through typed
checking and runtime execution.

## Scope

- Included: Cross-source imports, generated declaration boundaries, evaluation
  order, runtime values, JSX-hosted tt constructs, and project compilation
- Excluded: New syntax, package publication, and external service changes

## Decisions

### Decision 1: Test semantic outcomes beyond parseable output

- **Context**: A program can emit parseable TypeScript while changing evaluation
  order, binding identity, import resolution, or runtime values.
- **Alternatives considered**: Continue parser-only fuzzing; add isolated unit
  cases; generate complete mixed-source projects with typed and runtime oracles.
- **Decision and rationale**: Use complete project graphs and deterministic
  semantic oracles, then reduce each failure to the responsible compiler layer.

## Work log

- 2026-09-04: Confirmed the pinned development environment, restored the audit
  branch after the app returned to `main`, and opened the mixed-source audit.

## Issues and resolutions

None.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## Result

In progress.
