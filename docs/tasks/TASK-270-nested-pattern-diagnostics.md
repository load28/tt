# TASK-270: Render nested pattern errors in source vocabulary

- **Status**: Complete
- **Started**: 2026-08-28
- **Completed**: 2026-08-28
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

### Decision 2: Generic payload recovery requires unique exact ownership

- **Context**: A nested pattern under built-in `Result.Ok(value: T)` cannot
  recover the concrete `T` from declaration text, while its exact case tag
  may identify the declaration visible at the source site.
- **Alternatives considered**: inspect generated structural types; choose the
  first declaration with the tag; ask the checker for property lists; or use
  an exact tag only when one visible variant owns it.
- **Decision and rationale**: Use the unique exact owner as conservative
  declaration evidence. It generalizes source name resolution without
  guessing a generic substitution. Two variants with the same tag remain
  unresolved and therefore stay TypeScript's responsibility.

## Work log

- 2026-08-28: Created from nightly audit finding 5.
- 2026-08-28: Started after TASK-269 restored typed project boundaries.
- 2026-08-28: Added unique exact-tag recovery for nested constructors whose
  declared payload type is generic, then reused the ordinary field resolver
  and diagnostic author.
- 2026-08-28: Added an ambiguity regression proving that two visible `Card`
  declarations do not license a source-level field diagnostic.
- 2026-08-28: Added untyped CLI, typed CLI/server, and content-mapper
  regressions for `Ok(value: Card(brnd))`, including token spans, applicable
  edits, and suppression of the generated TS2339 structural consequence.
- 2026-08-28: Updated the name-resolution contract and ran formatting,
  Clippy, and the complete Cargo suite; all passed.

## Issues and resolutions

- The first cross-path assertions expected column 21, while the exact `brnd`
  token begins at column 20 in the fixture. The assertions now use the
  measured source coordinate.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Nested field misspellings under generic payloads now resolve through a unique
source declaration and render as `unknown-field` at the misspelled token with
an applicable edit. Generated structural types and outer-match anchors no
longer reach any tested compiler path. Changed files: `src/resolve/mod.rs`,
`docs/ai/tt.md`, `tests/resolve.rs`, `tests/integration.rs`, `tests/native.rs`,
and `tests/content_mapper.rs`.
