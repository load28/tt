# TASK-570: Type a value with no contextual type as TypeScript does at its source position

- **Status**: In progress
- **Started**: 2026-09-29
- **Completed**: —
- **Commit**: —

## Purpose

TASK-553 Issue 1: `const b = match (n) { 1 => ({ k: 1, m() { return this; } }), _ => ({ k: 2, m() { return this; } }) }; b.m().zzz;`
lowers each arm to `$tt_v0 = {…}`, an assignment to storage declared
`let $tt_v0;`. The assignment contextually types the object literal by the
storage's implicit `any`, so `this` in its methods is `any` and
`--check-types` misses the TS2339 that the equivalent
`const b = n === 1 ? {…} : {…}` reports.

## Scope

- Included: the refinement of generated storage after the backend's
  contextual rounds (`src/codegen/contextual.rs`,
  `src/typescript/contextual.rs`), the contextual query's list of storage
  that is not asked about (`src/typescript/backend.rs`, `native.rs`,
  `host.mjs`), the design note, `docs/ai/tt.md`, tests and fixtures.
- Excluded: storage whose source position has a contextual type; it keeps
  the annotation and assignment form TASK-546..553 settled.

## Decisions

## Work log

- 2026-09-29: Reproduced on `claude/ecstatic-dijkstra-qw5pf9`
  (`let $tt_v0: { k: number; m(): any; }`, `--check-types` exits 0).

## Issues and resolutions

None.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

