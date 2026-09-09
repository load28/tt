# TASK-278: Align resolver documentation with structural ownership

- **Status**: Complete
- **Started**: 2026-08-29
- **Completed**: 2026-08-29
- **Commit**: `TASK-278: align resolver contract documentation`

## Purpose

Make the documented name-resolution and exported-variant contracts match the
structural ownership rules shipped by TASK-275.

## Scope

- Included: `docs/ai/tt.md`, public API documentation in `src/lib.rs`, doctest verification
- Excluded: resolver behavior changes

## Decisions

### Decision 1: Document evidence-based ownership only

- **Context**: Documentation still promises spelling-based and generic-payload ownership rules that were removed.
- **Alternatives considered**: Restore heuristic behavior; document the structural resolver contract.
- **Decision and rationale**: Describe full and unique-best partial tag coverage, typed nested-field ownership, and checker deferral for generic payloads. This follows the compiler-layer contract and AGENTS.md requirement to keep language documentation current.

### Decision 2: Describe `ExternVariant` as tag-only

- **Context**: The public API documentation claims payload retention that its data type does not provide.
- **Alternatives considered**: Expand the API; document its actual boundary and point to the engine's rich symbol path.
- **Decision and rationale**: Correct the documentation only. Exhaustiveness and case checking use exported tags, while imported field checking uses `VariantSymbol` in the engine.

## Work log

- 2026-08-29: Compared the proposal with TASK-275 resolver tests and selected a documentation-only contract correction.
- 2026-08-29: Rewrote the name-resolution contract around full coverage, unique best partial coverage, and declared nested-field types.
- 2026-08-29: Removed the stale one-edit and generic unique-tag ownership promises and documented checker deferral.
- 2026-08-29: Corrected `exported_variants*` API documentation to distinguish tag-only `ExternVariant` data from the engine's rich `VariantSymbol` path.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

The language guide and public API documentation now describe the structural resolver and declaration boundaries that the compiler actually ships. No runtime or compiler behavior changed.
