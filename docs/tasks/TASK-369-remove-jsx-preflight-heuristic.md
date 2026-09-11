# TASK-369: Remove the duplicated JSX preflight heuristic

> Superseded by TASK-370: scanner removal did not fix the dependency's panic;
> TASK-370 repairs numeric decoding and removes panic-to-syntax-error masking.

- **Status**: Complete
- **Started**: 2026-09-12
- **Completed**: 2026-09-12
- **Commit**: —

## Purpose

Replace the recovery-only raw JSX entity scanner with the host verification
boundary's structural panic-to-diagnostic contract.

## Scope

- Included: Remove the duplicated incomplete-JSX byte scanner and its test; retain the token-based lexical checks and verifier boundary.
- Excluded: Changes to SWC, JSX grammar, or user-facing diagnostic wording beyond the existing verifier fallback.

## Decisions

### Decision 1: Keep one owner for malformed host syntax

- **Context**: `invalid_jsx_entity` reparsed incomplete JSX with a byte-level heuristic even though `verify_output` already owns host-parser failures.
- **Alternatives considered**: Keep both scanners (duplicate logic), or remove all malformed JSX protection (unsafe panic propagation).
- **Decision and rationale**: Remove only the recovery scanner and keep the verifier's panic boundary, so malformed host syntax has one structural owner and cannot unwind through compiler APIs.

## Work log

- 2026-09-12: Audited branch changes and identified the recovery-only raw scanner as temporary duplication.
- 2026-09-12: Removed the scanner and retained token delimiter validation plus verifier panic isolation; malformed JSX now follows the shared host-syntax path.

## Issues and resolutions

### Issue 1: Incomplete JSX required a second byte-level parser

- **Symptom**: `src/lexer/validation.rs` contained a small raw scanner that guessed JSX state after the token lexer intentionally gave up.
- **Cause**: The scanner duplicated host-parser recovery in the lexical layer.
- **Resolution**: Removed the duplicate scanner and updated the regression to assert the shared structural diagnostic path.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib malformed_jsx_is_a_validation_error`
- [x] `cargo test --release --test compile` (416 passed)

## Result

The recovery-only JSX byte scanner is removed; malformed host syntax is owned by the verifier boundary.
