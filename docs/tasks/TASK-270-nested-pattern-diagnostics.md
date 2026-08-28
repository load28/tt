# TASK-270: Render nested pattern errors in source vocabulary

- **Status**: Pending
- **Started**: 2026-08-28
- **Completed**: —
- **Commit**: —

## Purpose

Nested field misspellings currently surface as generated structural types at
the outer `match`. Report the invalid source field at its token using variant
and case vocabulary.

## Scope

- Included: nested resolver facts, diagnostic ownership, source spans, and
  typed/content-mapper parity
- Excluded: general mismatch wording outside pattern lowering

## Decisions

### Decision 1: Resolver facts own pattern spelling diagnostics

- **Context**: Generated structural types are a lowering detail.
- **Alternatives considered**: rewrite TS2339 text; add source-map exceptions;
  or diagnose the unresolved nested field before lowering consequences.
- **Decision and rationale**: The resolver owns field existence and exact
  tokens. Rendering checker glue would preserve the wrong abstraction.

## Work log

- 2026-08-28: Created from nightly audit finding 5.

## Issues and resolutions

None.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## Result

Pending.
