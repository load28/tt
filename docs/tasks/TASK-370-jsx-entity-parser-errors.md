# TASK-370: Preserve unrecognized JSX numeric references

- **Status**: Complete
- **Started**: 2026-09-12
- **Completed**: 2026-09-12
- **Commit**: —

## Purpose

Supersede TASK-369's claim that removing a scanner resolved the parser defect.
Fix the numeric entity decoder itself and remove panic-to-syntax-error masking.

## Scope

- Included: SWC JSX decoding, verifier, direct parser regressions, fuzz dependency alignment.
- Excluded: Unrelated language changes.

## Decisions

### Decision 1: Patch the dependency at its failure site

- **Context**: SWC 45.0.0 and locally available 45.1.1 unwrap failed numeric JSX conversions.
- **Alternatives considered**: Preflight text scanners duplicate grammar; catching panics hides internal failures; upgrading to the available version does not fix the defect.
- **Decision and rationale**: Vendor the pinned parser with a minimal entity-decoding patch, shared by every parser consumer including fuzzers. Preserve its license and upstream provenance. Treat unrecognized references as literal text, matching the TypeScript source acceptance contract.

## Work log

- 2026-09-12: Located two unwraps of `parse_from_code`, which returns None for empty, overflowing, and out-of-range numbers.
- 2026-09-12: Reproduced the original dependency panic directly, without tt preflight or catch_unwind.
- 2026-09-12: Propagating the lexer error exposed a second unreachable branch in JSX child parsing. Added error-token handling and propagated child results through elements and fragments.
- 2026-09-12: A normal-input regression exposed entity scanning past a JSX delimiter. Restricted entity scanning to ASCII entity characters before advancing; semicolon-free literal text now preserves its enclosing JSX.
- 2026-09-12: Compared against pinned TypeScript using `tsc --noEmit --jsx preserve --ignoreConfig /tmp/tt-370-entity-check.tsx`: empty and out-of-range numeric references are accepted. Replaced the initial error-propagation approach with literal preservation and reverted the JSX parser changes.
- 2026-09-12: Removed verifier panic masking. Root and fuzz lockfiles now select the same vendored parser. Only lexer/mod.rs differs from upstream source.

## Issues and resolutions

### Issue 1: Numeric conversion errors unwind out of JSX lexing

- **Symptom**: Empty and out-of-range numeric entities panic.
- **Cause**: Fallible numeric conversion is unwrapped in decimal and hexadecimal branches.
- **Resolution**: Numeric decoding returns an optional code point. Unrecognized references retain their literal text, following the existing unknown-reference model. No conversion failure is unwrapped or caught as a panic.

## Verification

- [x] Direct SWC regression matrix: 12 formerly panicking numeric cases, 22 other valid cases, and original incomplete JSX input; valid cases also preserve tt output byte-for-byte.
- [x] Final `cargo fmt --check`
- [x] Final `cargo clippy --all-targets -- -D warnings`
- [x] Final `cargo test`: full suite passed, including 151 integration tests.
- [x] `./scripts/ci rust`: passed; log `/tmp/tt-task370-final-ci.log`.
- [x] Standalone fuzz target dependency check
- [x] Rebuilt `compile_any_bytes`: 62,339 runs in 31 seconds, exit 0; log `/tmp/tt-task370-final-fuzz.log`.

## Result

The shared SWC entity decoder now preserves TypeScript-compatible literal
references without unwinding. The verifier no longer converts panics into
syntax diagnostics. Changed files: root and fuzz dependency manifests/locks,
vendored parser and provenance, verifier, direct parser tests, task records.
