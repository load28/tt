# TASK-552: Never annotate storage with a type that names generated storage

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

`const C = match (n) { 1 => class { q = 1 }, _ => class { q = 2 } };`
emitted `let $tt_v0: typeof $tt_v0;`, which `tsc` rejects with TS2502
(`'$tt_v0' is referenced directly or indirectly in its own type
annotation`). An annotation on generated storage must never reference that
storage, or any other storage the lowering declared: those names are the
compiler's glue, not names the program's types are written in.

## Scope

- Included: the backend host's annotation check
  (`src/typescript/host.mjs`), the contextual query that tells the host
  which declarations are generated storage (`src/typescript/backend.rs`,
  `contextual.rs`, `native.rs`), the design note, and a regression test.
- Excluded: writing another form for such a type; the storage is typed from
  its assignments, as for every rejected annotation.

## Decisions

### Decision 1: Reject a name that resolves to generated storage, by symbol

- **Context**: The join path asks for the type of each assignment's right
  side. TypeScript gives a class expression assigned to a binding that
  binding's name, so both arms print as `typeof $tt_v0`. TASK-546's check
  resolves `$tt_v0` at the declaration and at the assignment to the same
  symbol, so it accepted the annotation.
- **Alternatives considered**: (a) Recognize generated names by their
  `$tt_` spelling: a string rule, and a script's derived names
  (`$tt_v0$total`) show spelling is not the identity. (b) Reject only the
  storage being annotated: another slot's name is equally glue, and the
  task's rule covers any generated binding. (c) Use the verbatim source
  mappings to call everything else generated: a `variant`'s type alias is
  generated text too, and annotations must keep naming it.
- **Decision and rationale**: The host collects the symbols of every
  storage declaration the query lists for the module, and a type reference
  or type query whose head resolves to one of them rejects the annotation.
  Storage an earlier round annotated is no longer asked about, so the query
  now lists it as well, marked `annotated`, after every slot an answer can
  name (`ContextualSlotQuery::annotated`); `materialize` keeps those
  declaration ends, moving each by the annotations inserted before it.

## Work log

- 2026-09-29: Reproduced with `ttc` and `tsc --strict` (TS2502).
- 2026-09-29: Changed `src/typescript/host.mjs` (`storageOf`, the
  `annotation` check), `src/typescript/backend.rs`,
  `src/typescript/contextual.rs`, and `src/typescript/native.rs`; documented
  the rule in `docs/design/contextual-type-materialization.md`.
- 2026-09-29: Added
  `an_annotation_never_names_the_storage_the_lowering_declared`
  (`tests/integration/contextual.rs`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib typescript`, `--test integration`, `--test native`
- [x] The new test fails without the change (TS2502).

## Result

Changed `src/typescript/host.mjs`, `src/typescript/backend.rs`,
`src/typescript/contextual.rs`, `src/typescript/native.rs`,
`docs/design/contextual-type-materialization.md`,
`tests/integration/contextual.rs`, `docs/tasks/INDEX.md`, and this record.
