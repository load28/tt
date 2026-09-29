# TASK-551: Leave storage unannotated when TypeScript cannot write its type

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

A match whose arms have an anonymous class type stopped the whole
compilation. With `function Tagged<B extends new (...a: any[]) => {}>(Base: B)
{ return class extends Base { tag = "t"; }; }`,
`const M = match (n) { 1 => Tagged(Base), _ => Tagged(Base) };` (and arms
`new (class { x = 1 })()` or `[class {}]`) reported
`error[other]: the TypeScript backend failed: Offset is outside the bounds
of the DataView`, and nothing was emitted. Emitted TypeScript must not
depend on whether one annotation can be written: `docs/design/
contextual-type-materialization.md` already leaves storage unannotated when
no definite annotation exists.

## Scope

- Included: how the backend host asks for an annotation's type node
  (`src/typescript/host.mjs`), the design note, and a regression test.
- Excluded: making `--check` skip the TypeScript backend (another task
  owns the CLI).

## Decisions

### Decision 1: The failure is TypeScript's "no node" answer, which the client cannot decode

- **Context**: A standalone script against the pinned
  `typescript@7.1.0-dev.20260826.1` (`checker.typeToTypeNode` on the join
  types, with and without an enclosing declaration) showed the exception
  comes from `new RemoteSourceFile` inside `decodeNode`, reading the header
  of a 4-byte payload. The raw `typeToTypeNode` response for these types is
  the bytes `null`: the node builder produced no node, as it does for a type
  it cannot name (TypeScript's declaration emitter reports such types as
  not nameable). `api/sync/client.js` returns `undefined` only for an empty
  payload, so `typeToTypeNode` passes the encoded `null` to the decoder
  instead of returning its documented `undefined`. ttc's call is correct;
  the throw escaped `annotation()`, then the whole `ask`.
- **Alternatives considered**: (a) Pass `IgnoreErrors` so the node builder
  always writes a node: it would write a node for a type it could not
  name, which is not the type. (b) Pass `WriteClassExpressionAsTypeLiteral`:
  a structural literal is not the class type (nominal private members,
  `instanceof` narrowing), so the storage would hold a different type.
  (c) Make the request below the client: that reads the checker's private
  fields.
- **Decision and rationale**: `typeNode` in the host calls
  `typeToTypeNode` and, when the call throws, asks the same session a
  second question about the same type (`typeToString`). An answer means the
  session is healthy and the failure was the node answer itself, so the
  type has no node and the storage stays unannotated, as for any type
  without a definite annotation; a session failure still propagates.

## Work log

- 2026-09-29: Reproduced with `ttc` and with a standalone API script under
  `target/probe4-compiler/api/` (raw payload `6e756c6c`, `null`).
- 2026-09-29: Added `typeNode` to `src/typescript/host.mjs` and used it in
  `annotation`; documented the rule in
  `docs/design/contextual-type-materialization.md`.
- 2026-09-29: Added
  `a_type_typescript_cannot_write_leaves_its_storage_unannotated`
  (`tests/integration/contextual.rs`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] The new test fails without the change (the DataView backend error).

## Result

Changed `src/typescript/host.mjs`, `docs/ai/tt.md`,
`docs/design/contextual-type-materialization.md`,
`tests/integration/contextual.rs`, `docs/tasks/INDEX.md`, and this record.
