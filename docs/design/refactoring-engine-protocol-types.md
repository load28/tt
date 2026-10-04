# Separate engine wire types from session transport

Task: [TASK-752](../tasks/TASK-752-engine-protocol-types.md). Base: `09f07efb8a03f565a8540f2fe15271adbd90c3cc`.
Program: [behavior-preserving refactoring](behavior-preserving-refactoring.md).

## Boundary and implementation

Move all exported Engine interfaces and EngineAnswer from engine.ts to engine-protocol.ts. Keep RENAME_PLACEHOLDER and every executable definition in engine.ts. Import the types for local use and re-export them through engine.ts so existing consumers remain unchanged.

## Invariants and exclusions

Preserve every optional/null field, discriminant, numeric type, nested record, and public type name. Preserve process lifetime, queue order, timeout/retry/replay behavior, wellFormedStrings conversion, request bodies and defaults. The new module has only erased TypeScript declarations and type imports.

No algorithms, state ownership, request ordering, public API, dependencies, or
reference expectations change. Inspect the complete diff, compare moved bodies
and reconstruct the parent against the saved base. Unexpected observations block
completion; behavior fixes require separate tasks.

## Allowed files and validation

Production scope: `editors/vscode/server/src/engine.ts`, `editors/vscode/server/src/engine-protocol.ts`.
Task/index/progress documentation may also change. Base source snapshots and
comparison evidence live under `/tmp/tt-refactor/752`.

Run extension compilation and engine/session/compiler tests; compare all moved declarations and reconstruct the transport source. The TASK-751 base extension gate covers the unchanged transport; final extension gates rerun the complete set.

The user requested autonomous completion of the remaining roadmap on 2026-10-04.
Slices are prepared sequentially on the local branch, with focused checks before
advancing and the complete required gates before completion/publication. No
intermediate change is merged while its required gates remain pending. Rust
1.98.0 and TypeScript 7.1.0-dev.20260826.1 remain pinned. Do not regenerate baselines.
