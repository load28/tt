# TASK-366: Preflight unbalanced delimiters before host verification

- **Status**: Complete
- **Started**: 2026-09-12
- **Completed**: 2026-09-12
- **Commit**: pending

## Purpose

Prevent pathological host-parser work on short malformed files by reporting
unbalanced TypeScript delimiters at the lexical validation boundary.

## Scope

- Included: Shared validation for parentheses, brackets, and braces; regression tests and benchmark evidence.
- Excluded: Angle-bracket generic/JSX balancing and changes to SWC itself.

## Decisions

### Decision 1: Reuse lexer tokens for delimiter validation

- **Context**: A 142-byte malformed input spent about 1.4 seconds in SWC verification while `--no-verify` completed immediately.
- **Alternatives considered**: Add an output-size limit or bypass verification; both would weaken the compiler contract.
- **Decision and rationale**: Inspect significant lexer tokens recursively so strings, comments, templates, regexes, and JSX raw text remain opaque while malformed delimiters receive a normal diagnostic.

## Work log

- 2026-09-12: Reproduced the slow input across `.tt`, `.ttx`, `.ts`, and `.tsx`; `.tt` took 1.4 seconds and `.ttx` 0.5 seconds in host verification.
- 2026-09-12: Added recursive token-based delimiter preflight and a compiler regression test; the same `.tt` input now returns the lexical diagnostic in 0.46 seconds.

## Issues and resolutions

### Issue 1: Pathological SWC verification on unmatched delimiters

- **Symptom**: A short malformed source required about 1.4 seconds to return a syntax error.
- **Cause**: Generated output reached SWC despite the compiler lexer already having enough structure to identify unmatched delimiters.
- **Resolution**: Shared validation now rejects unmatched `()`, `[]`, and `{}` before SWC output verification while preserving opaque lexical regions.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (416 unit/integration tests passed)

## Result

`src/lexer/validation.rs` performs the preflight, and `tests/compile/cases_04.rs` fixes the regression contract. The malformed benchmark returns a normal diagnostic without entering the pathological SWC path.
