# TASK-751: Isolate editor diagnostic symbol and fix projections

- **Status**: In progress
- **Started**: 2026-10-04
- **Completed**: —
- **Commit**: —

## Purpose

Move toDiagnostic, toDocumentSymbol, insertSymbol and suggestedFixes into lsp-projections.ts. Pass the existing editorUri callback to toDiagnostic and the current versioned-edit capability boolean to suggestedFixes at their call sites. Keep all bodies unchanged. Imports of engine/ttc are type-only; the projection module does not initialize a connection or read server state.

## Scope

- Included: `editors/vscode/server/src/server.ts`, `editors/vscode/server/src/lsp-projections.ts`.
- Excluded: Behavior changes, public API changes, baseline updates, and TASK-732.

## Decisions

### Decision 1: Extract the existing responsibility without redesign

- **Context**: The roadmap identifies a coherent operation mixed with its caller.
- **Alternatives considered**: Keeping the boundary implicit; combining the move
  with algorithm or state redesign, which would make equivalence harder to review.
- **Decision and rationale**: Preserve definitions and callers where possible,
  using the smallest visibility required by the existing consumers. See the
  [detailed brief](../design/refactoring-lsp-projections.md).

## Work log

- 2026-10-04: Started from local base `98dfc42b4369fc1c82e6d7ba049f43587ed5a3ee` on
  `refactor/remaining-roadmap`. Preserved base source under
  `/tmp/tt-refactor/751` before production edits.

- 2026-10-04: The unchanged extension stage passed 238 tests with zero failures/skips; /tmp/tt-refactor/751/base.log. Extraction preserves all four bodies exactly; only two signatures and their three call sites receive explicit dependencies. TypeScript compilation and 90 server/diagnostic/completion tests passed with zero failures/skips; /tmp/tt-refactor/751/focused.log. Removed newly unused imports and excess blank lines. Full final gates remain pending.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: Behavior-preserving extraction; no bug is fixed.

## Verification

- [x] Moved definitions and parent changes reviewed.
- [x] Focused checks: Run the extension stage before and after; existing server tests cover secondary diagnostic labels, nested symbols, stale suggestions and both WorkspaceEdit capability values. The final editor/Rust gates must also pass.
- [ ] Applicable complete repository gates.
- [ ] Reference expectations unchanged; task-index and whitespace checks.

## Result

Implementation and focused validation complete; final repository gates remain pending.
