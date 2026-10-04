# Isolate editor diagnostic symbol and fix projections

Task: [TASK-751](../tasks/TASK-751-lsp-projections.md). Base: `98dfc42b4369fc1c82e6d7ba049f43587ed5a3ee`.
Program: [behavior-preserving refactoring](behavior-preserving-refactoring.md).

## Boundary and implementation

Move toDiagnostic, toDocumentSymbol, insertSymbol and suggestedFixes into lsp-projections.ts. Pass the existing editorUri callback to toDiagnostic and the current versioned-edit capability boolean to suggestedFixes at their call sites. Keep all bodies unchanged. Imports of engine/ttc are type-only; the projection module does not initialize a connection or read server state.

## Invariants and exclusions

Preserve diagnostic ranges/fallbacks, related URIs, absent properties, suggestion versions, preferred-fix order, versioned and unversioned WorkspaceEdits, recursive symbol ordering and containment. Connection handlers, validation generations, timers and request sequence remain unchanged.

No algorithms, state ownership, request ordering, public API, dependencies, or
reference expectations change. Inspect the complete diff, compare moved bodies
and reconstruct the parent against the saved base. Unexpected observations block
completion; behavior fixes require separate tasks.

## Allowed files and validation

Production scope: `editors/vscode/server/src/server.ts`, `editors/vscode/server/src/lsp-projections.ts`.
Task/index/progress documentation may also change. Base source snapshots and
comparison evidence live under `/tmp/tt-refactor/751`.

Run the extension stage before and after; existing server tests cover secondary diagnostic labels, nested symbols, stale suggestions and both WorkspaceEdit capability values. The final editor/Rust gates must also pass.

The user requested autonomous completion of the remaining roadmap on 2026-10-04.
Slices are prepared sequentially on the local branch, with focused checks before
advancing and the complete required gates before completion/publication. No
intermediate change is merged while its required gates remain pending. Rust
1.98.0 and TypeScript 7.1.0-dev.20260826.1 remain pinned. Do not regenerate baselines.
