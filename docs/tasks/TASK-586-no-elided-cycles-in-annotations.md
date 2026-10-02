# TASK-586: Never annotate storage with a type whose cycle the node builder elided

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-586: Never annotate storage with a type whose cycle the node builder elided`

## Purpose

TASK-570 Issue 2: `match (n) { 1 => ({ k: 1, m() { return this; } }), _ => ({ k: 2, m() { return this; } }) }`
joined as `let $tt_v0: { k: number; m(): { k: number; m(): any; }; };`, so
`b.m().m().zzz` was not reported; `function mk() { return { m() { return this; } }; }`
with `match (n) { 1 => mk(), _ => mk() }` joined as `{ m(): any; }`. An
annotation must never lose type information (TASK-553).

## Scope

- Included: the annotation check of the backend host
  (`src/typescript/host.mjs`), the design note, `docs/ai/tt.md`, notes on
  TASK-553 and TASK-570, and regression tests.
- Excluded: writing such a type in another form; TypeScript has no
  printable form for it, and the storage typed from its values already has
  it whole.

## Decisions

### Decision 1: Pair every `any` keyword of the node with the `any` type

- **Context**: With `NoTruncation`, TypeScript's node builder writes a
  cycle through an anonymous type as `any` (its declaration emitter adds a
  `/*elided*/` comment the API client does not carry, and `typeToString`
  prints none either, checked with a standalone script under
  `target/repro/api/`); without the flag it writes `...`, which truncation
  also writes, so the two answers cannot tell elision from length.
- **Alternatives considered**: (a) Re-check the printed annotation with
  the checker's type comparison: the API exposes assignability, not
  identity, and `{ k; m(): { k; m(): any } }` is assignable both ways to
  the recursive type. (b) Detect cycles on the type side by replaying the
  node builder's rules for which types it expands: a copy of its internals
  that would drift. (c) Reject every annotation containing `any`: an `any`
  of the source (`[JSON.parse(s)]`) would lose its annotation.
- **Decision and rationale**: `denotes` (TASK-575) already walks the node
  and the type together; an `any` keyword is now a part it pairs, and it
  must stand for a type with `TypeFlags.Any`. The elided cycle stands for
  the object type itself, so the node is not taken and the storage stays
  unannotated, typed from its values, which TASK-570 types as at their
  source position (the carrier `const`). A genuine `any` keeps its
  annotation.

## Work log

- 2026-09-30: Reproduced on the branch after TASK-585 (`target/repro/r4`):
  both joins annotated with `any` one level down, `--check-types` silent.
- 2026-09-30: Added the `any` pairing to `denotes` in
  `src/typescript/host.mjs`; the repro reports both TS2339s, and
  `[JSON.parse(…)]` still joins as `any[]`.
- 2026-09-30: Added
  `a_recursive_anonymous_type_is_never_annotated_with_its_elided_cycle`
  (`tests/integration/contextual.rs`) and
  `types_type_a_recursive_anonymous_join_whole` (`tests/cli.rs`); both fail
  with the previous host.
- 2026-09-30: Updated `docs/design/contextual-type-materialization.md`,
  `docs/ai/tt.md`, and the TASK-553 and TASK-570 records.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` (full, `RUST_TEST_THREADS=4`)
- [x] `cd editors/vscode && npm run compile && node --test "server/out/test/*.test.js" "client/out/test/*.test.js"`
- [x] The new tests fail without the change.

## Result

Changed `src/typescript/host.mjs`,
`docs/design/contextual-type-materialization.md`, `docs/ai/tt.md`,
`docs/tasks/TASK-553-untruncated-annotations.md`,
`docs/tasks/TASK-570-context-free-storage-writes.md`,
`tests/integration/contextual.rs`, `tests/cli.rs`, `docs/tasks/INDEX.md`,
and this record.
