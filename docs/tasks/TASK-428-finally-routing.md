# TASK-428: Route abrupt exits through `finally` in the divergence graph

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

The let-else divergence check (docs/ai/tt.md, let-else) mis-modelled `finally`:

- Unsound: `else { x: { try { return -1; } finally { break x; } } }` and `else { while (true) { try { continue; } finally { break; } } }` were accepted, although the `finally`'s `break` replaces the `return`/`continue` and control falls out of the block; with `None` the function returned `undefined` for a `number`.
- Too strict: `else { x: { try { break x; } finally { return 1; } } }` was rejected, although the `finally` always returns.

## Scope

- Included: The `try` lowering in the flow graph builder (`src/flow/mod.rs`).
- Excluded: How exceptions reach a `catch`.

## Decisions

### Decision 1: Follow the TryStatement completion rules

- **Context**: ECMA-262 §14.15.3: the `finally` block runs after the guarded block (and handler) completes normally or abruptly; if the `finally` completes abruptly, that completion is the statement's, otherwise the earlier completion stands. The builder routed only normal completion through the `finally` and sent `return`/`break`/`continue` straight to their targets.
- **Alternatives considered**: Treating every `try` with a `finally` conservatively as non-diverging would reject the valid third case.
- **Decision and rationale**: While the guarded block and handler are built, every enclosing jump target is replaced by a copy of the `finally` that continues to the original target, and `return`/`throw` and jumps that leave the analysed body go through a `finally` copy that ends in `Return`/`Jump`. Targets are resolved against the scopes outside the `try`, and nested `finally` blocks chain.

## Work log

- 2026-09-27: Reproduced all three cases with `ttc --check`; implemented the routing; added a flow unit test with eight shapes (fails before, passes after).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.

## Result

Changed `src/flow/mod.rs` and `src/flow/tests.rs`.
