# TASK-409: Emit a C-style loop test prefix once

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`for (; j < match (xs[0]) { ... };) {}` crashed the compiler with a stack overflow (exit 134) instead of compiling (docs/ai/tt.md: "`while` and C-style `for` conditions own a region that runs on every test") or reporting an internal compiler error.

## Scope

- Included: Emission of the loop-test prefix for `for` loops.
- Excluded: Loop-test planning.

## Decisions

### Decision 1: Claim the prefix per loop owner

- **Context**: `source_range_rope` wrote the `for` prefix whenever a source range reached the test's first byte. The prefix itself emits the captured left operand `j`, whose range starts at that byte, so it re-entered the prefix without end. A `while` prefix is written only at the loop owner's start, which the capture never reaches.
- **Alternatives considered**: Guarding on `loop_region_depth` would also suppress the prefix of a loop legitimately written inside another loop's test region.
- **Decision and rationale**: The prefix is written once per loop owner, through the same claim registry type the compose rewrites use (`ClosedComposeBlocks`).

## Work log

- 2026-09-27: Reproduced the crash; added `emitted_loop_tests` and the claim. Added an integration test (fails with a stack overflow before, passes after).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.

## Result

Changed `src/codegen/core/emitter/source.rs`, `src/codegen/core/emitter/mod.rs`, `src/codegen/core/mod.rs`, and `tests/integration.rs`.
