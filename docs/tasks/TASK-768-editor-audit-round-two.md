# TASK-768: Fix defects found by the second editor audit

- **Status**: In progress
- **Started**: 2026-10-06
- **Completed**: —
- **Commit**: —

## Purpose

A second audit of `ttc --server` found quick fixes that break code, hint
ranges that miss text, completions that are empty, and overlay state that
diverges between layers. Fix each in the layer that owns it.

## Scope

- Included: the field-typo fix on a shorthand binding (first).
- Excluded: to be recorded as the task proceeds.

## Decisions

### Decision 1: A field-typo fix keeps a shorthand binding's name

- **Context**: `Circle(raduis) => raduis` was fixed to `Circle(radius) =>
  raduis`, which no longer binds `raduis`.
- **Decision and rationale**: A shorthand binding is both the field name
  and the binding name, so the replacement writes the field with the
  written name as its alias (`radius: raduis`). The suggested name stays
  separate from the edit text.

## Work log

- 2026-10-06: Started from the second audit's report.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: pending
- **Observed failure**: pending

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Baseline changes reviewed and committed with the change

## Result

In progress.
