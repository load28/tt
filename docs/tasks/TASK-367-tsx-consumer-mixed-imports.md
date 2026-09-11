# TASK-367: Type-check mixed imports from TSX consumers

- **Status**: Complete
- **Started**: 2026-09-12
- **Completed**: 2026-09-12
- **Commit**: pending

## Purpose

Cover the content mapper path where a handwritten `.tsx` consumer imports both
`.tt` and `.ttx` modules.

## Scope

- Included: A tsgo-backed mixed-import regression test.
- Excluded: Runtime bundler behavior already covered by the mixed-source fixture.

## Decisions

### Decision 1: Keep the fixture minimal and type-directed

- **Context**: Existing mapper coverage exercises `.ts` consumers and a `.ttx` module, but not a `.tsx` consumer importing both transformed extensions.
- **Alternatives considered**: Expand the runtime fixture or add a second project fixture; both would duplicate unrelated runtime coverage.
- **Decision and rationale**: Add one mapper project with a `.tsx` consumer and one `.tt` plus one `.ttx` provider, asserting TypeScript type-checks the complete graph.

## Work log

- 2026-09-12: Added the task before editing tracked tests.
- 2026-09-12: Added a `.tsx` consumer importing `.tt` and `.ttx`; the first run exposed a missing JSX intrinsic declaration in the minimal fixture, so the fixture now declares its `section` contract explicitly.

## Issues and resolutions

### Issue 1: Minimal TSX fixture lacked JSX intrinsic types

- **Symptom**: TypeScript reported TS7026 for the test's `<section>` element.
- **Cause**: The isolated mapper project intentionally has no React or DOM typings.
- **Resolution**: The consumer declares the single intrinsic element it uses, leaving the import graph under test unchanged.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --test content_mapper` (13 tests passed)

## Result

The content mapper now has an explicit `.tsx` consumer matrix covering simultaneous `.tt` and `.ttx` imports.
