# TASK-553: Annotate storage with the whole type, never a truncated one

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

An annotation on generated storage must not lose type information. The
reported case was
`const b = match (n) { 1 => ({ k: 1, m() { return this; } }), _ => ({ k: 2, m() { return this; } }) }; return b.m().zzz;`,
emitted as `let $tt_v0: { k: number; m(): any; }`, so `--check-types`
missed the TS2339 plain TypeScript reports. The investigation found two
separate causes; this task removes the one in the annotation and records
the other (Issue 1).

## Scope

- Included: the node-builder flags of the backend host's annotation
  (`src/typescript/host.mjs`), the design note, and a regression test.
- Excluded: the reported object-literal case, whose `any` is not written by
  the annotation (Issue 1).

## Decisions

### Decision 1: Ask the node builder for the type without truncation

- **Context**: `checker.typeToTypeNode` truncates by default: for a join of
  a 40-member object type the host received
  `{ p0: …; p1: …; p2: …; ... 36 more ...; p39: … }` and printed it as the
  annotation, which is neither the type nor TypeScript (`generated
  TypeScript failed to parse: Expected ident`). With a shorter truncation
  the output could still parse and hold fewer members than the value.
- **Alternatives considered**: Checking the printed node against the type
  (following every `any` in the node to the part of the type it was
  written for). A standalone script against the pinned TypeScript showed
  the node builder writes no node at all for a type it would elide as
  cyclic (the object type `function mk() { return { m() { return this; } }; }`
  returns), which TASK-551 already leaves unannotated, and every `any` in a
  node it did write was the type's own; the check had nothing to reject.
- **Decision and rationale**: The host passes
  `NodeBuilderFlags.NoTruncation`, TypeScript's own flag for writing a type
  whole, as its declaration emitter does.

## Work log

- 2026-09-29: Reproduced with `ttc` and a standalone API script
  (`target/probe4-compiler/api/`), with and without `NoTruncation`.
- 2026-09-29: Changed `annotation`/`typeNode` in `src/typescript/host.mjs`;
  documented the rule in `docs/design/contextual-type-materialization.md`.
- 2026-09-29: Added `a_long_type_is_annotated_whole`
  (`tests/integration/contextual.rs`).

## Issues and resolutions

### Issue 1: The reported `m(): any` is the object literal's own type in the lowered program

- **Symptom**: `b.m().zzz` reports no TS2339 after lowering; the join
  annotation reads `{ k: number; m(): any; }`.
- **Cause**: The arm value is assigned to storage declared as `let $tt_v0;`.
  TypeScript types such a `let` with its evolving `auto` type, which is
  `any`, and an assignment's right operand is contextually typed by its
  target, so `this` in the object literal's method is `any`
  (`getContextualThisParameterType`). A plain `tsc --strict` check of
  `let x; x = { k: 1, m() { return this; } }; x.m().zzz;` reports nothing
  either, and the checker's return type of `m` there has `TypeFlags.Any`.
  The annotation writes that type faithfully; removing it does not bring
  the error back. Parameters are not affected: the `auto` type has no
  signature, so `x = (y) => y` still reports TS7006.
- **Resolution**: Not resolved here; TASK-570 resolves it. Only the emission can give the value
  no contextual type (the storage has one in every declaration form), and
  whether the source position had a contextual type is known only after
  the backend's rounds. Left as a follow-up.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --test integration`
- [x] The new test fails without the change (`generated TypeScript failed
  to parse`).

## Result

Changed `src/typescript/host.mjs`,
`docs/design/contextual-type-materialization.md`,
`tests/integration/contextual.rs`, `docs/tasks/INDEX.md`, and this record.
The reported object-literal case remains (Issue 1).
