# TASK-584: Infer a join only from values typed by settled storage

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-584: Infer a join only from values typed by settled storage`

## Purpose

`declare const flag: boolean; export const f = () => match(flag) { true => [1], false => [] }; export const g = match(flag) { true => [f()], false => [] }; export const bad = g[0]![0]!.toUpperCase();`
was reported to emit `let $tt_v0: number[];` in `f` but
`let $tt_v1: any[][];` for `g`, so `--check-types` missed the TS2339 on
`toUpperCase`. `docs/design/contextual-type-materialization.md` says
unresolved, error, `any` and `unknown` inputs do not provide a definite
annotation; this `any` is not the program's but the unsettled storage's.

## Scope

- Included: the join inference of the backend host
  (`src/typescript/host.mjs`), the design note, and a regression test.
- Excluded: joins whose inputs read no unsettled storage, which keep their
  round and annotation; contextual (non-join) annotations.

## Decisions

### Decision 1: A join that writes `any` waits while an incoming value reads unsettled storage

- **Context**: The host infers every join slot in one round, from the
  program in which none of them is annotated. Storage with no annotation
  has no type of its own: without `noImplicitAny` it is `any`, and an
  evolving variable read in a closure is `any` as well. `f()` is then `any`
  when `g`'s join is computed, and `g`'s annotation keeps it after `f`'s
  storage is annotated in the same round.
- **Alternatives considered**: (a) Reject every join whose type involves
  `any` anywhere: an `any` of the source (`[JSON.parse(s)]`) would never
  be annotated, and without `noImplicitAny` the storage would then read as
  `any` instead of `any[]`, hiding errors the source reports.
  (b) Provisional joins recomputed every round until they stop changing:
  an annotation feeds the types it is recomputed from, so termination needs
  a cap. (c) Infer one join per round: the order that is right is the
  dependency order, which a slot index does not give. (d) Defer an
  `any`-writing join only while another `any`-writing join makes progress,
  and annotate the rest when none does: without `noImplicitAny` every
  unsettled slot reads as `any`, so `f = [JSON.parse(…)]` and `g = [f()]`
  would stall and `g` would still be written from `f`'s unsettled `any`.
- **Decision and rationale**: The host decides the dependency itself. A
  join whose printed annotation writes `any` is left for a later round when
  one of its incoming values reads storage no round has settled yet:
  directly, or through a declaration in a lowered module whose inferred
  type is computed from it (an unannotated variable's initializer, an
  unannotated function's body, and for an unannotated parameter the
  statement whose context types it). Only lowered modules declare storage,
  so only their declarations are followed; the slot's own storage does not
  count (a value cannot be typed by the storage it is written to except
  through a cycle, which TypeScript types `any` at the source too). The
  rounds already continue while a round annotates anything, so the waiting
  join is asked again after its inputs' storage is annotated: joins settle
  in dependency order, an `any` of the source is annotated as soon as its
  inputs are settled, and a join that stays dependent is left unannotated
  and typed from its assignments. A join that writes no `any` keeps its
  round: the storage's own type is then not `any`, and waiting would only
  add rounds.

## Work log

- 2026-09-29: The reported repro (`strict`) no longer reproduces on
  `claude/ecstatic-dijkstra-qw5pf9`: TASK-570 carries `[]` through an
  arm-local `const`, so `f`'s unsettled storage reads as
  `number[] | never[]` and `g` joins as `number[][]`. The same program
  without `noImplicitAny` (`"strict": false`) still emits
  `let $tt_v1: any[];` for `g`, and `--check-types` exits 0; so does the
  reported output `target/probe4-cli/pj/out8/m1.ts`, written before
  TASK-570, where `f`'s evolving `[]` made `f(): any[]`.
- 2026-09-29: Changed `src/typescript/host.mjs` (`pendingStorage`,
  `readsPending`, `writesAny`; `annotation` now returns the node so the
  join can look at it before printing). Documented the rule in
  `docs/design/contextual-type-materialization.md`.
- 2026-09-29: A first version followed an unannotated parameter to its
  statement without excluding the slot's own storage; the join of
  `xs.map((x) => match (flag) { true => [f(), x], false => [] })` then
  waited on itself and stayed unannotated (Issue 1).
- 2026-09-29: Added `types_join_storage_after_the_storage_its_values_read`
  (`tests/cli.rs`): a project without `noImplicitAny`, the join in the
  same module and across modules, and a source `any` (`JSON.parse`) whose
  `any[][]` join still reports `.foo` on `any[]`. It fails with the
  previous host.

## Issues and resolutions

### Issue 1: A join waited on its own storage

- **Symptom**: `let $tt_v2;` for a match inside `xs.map((x) => …)` whose
  value reads `x`.
- **Cause**: The statement that types `x` holds the slot's own storage.
- **Resolution**: The walk starts with the slot's own symbol marked as
  followed.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test integration --test snapshot --test compile --test cli --test native --test content_mapper`
- [x] The new test fails without the change.

## Result

Changed `src/typescript/host.mjs`,
`docs/design/contextual-type-materialization.md`, `tests/cli.rs`,
`docs/tasks/INDEX.md`, and this record.
