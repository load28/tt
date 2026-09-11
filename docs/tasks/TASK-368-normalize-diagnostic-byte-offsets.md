# TASK-368: Normalize diagnostic byte offsets at UTF-8 boundaries

- **Status**: Complete
- **Started**: 2026-09-12
- **Completed**: 2026-09-12
- **Commit**: pending

## Purpose

Prevent malformed UTF-8 boundary offsets from panicking the diagnostic renderer.

## Scope

- Included: `line_col` boundary normalization and a fuzz regression test.
- Excluded: Changes to parser spans or external parser behavior.

## Decisions

### Decision 1: Clamp offsets to the preceding UTF-8 boundary

- **Context**: Fuzz input caused `src[..offset]` to panic at byte 58 inside a two-byte character.
- **Alternatives considered**: Reject non-ASCII input or change every producer's span; both would weaken the shared diagnostic contract.
- **Decision and rationale**: Normalize once in `line_col`, preserving the source line while guaranteeing valid slicing for every caller.

## Work log

- 2026-09-12: Reproduced `error.rs:167` with `compile_any_bytes` artifact `crash-a8ae06486a20ab93f4cef3b08d6b0adcead59715`.
- 2026-09-12: Normalized offsets, added a unit regression, and replayed the artifact successfully with the rebuilt fuzz target.

## Issues and resolutions

### Issue 1: Diagnostic rendering panicked at a UTF-8 continuation byte

- **Symptom**: `end byte index 58 is not a char boundary`.
- **Cause**: Byte offsets were trusted before slicing a UTF-8 string.
- **Resolution**: `line_col` now backs offsets down to a valid UTF-8 boundary before slicing; the original crash artifact completes normally.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (including the UTF-8 boundary regression)

## Result

Diagnostic rendering no longer panics when a producer supplies an offset inside a multibyte UTF-8 character.
