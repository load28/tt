# TASK-752: Separate engine wire types from session transport

- **Status**: In progress
- **Started**: 2026-10-04
- **Completed**: —
- **Commit**: —

## Purpose

Move all exported Engine interfaces and EngineAnswer from engine.ts to engine-protocol.ts. Keep RENAME_PLACEHOLDER and every executable definition in engine.ts. Import the types for local use and re-export them through engine.ts so existing consumers remain unchanged.

## Scope

- Included: `editors/vscode/server/src/engine.ts`, `editors/vscode/server/src/engine-protocol.ts`.
- Excluded: Behavior changes, public API changes, baseline updates, and TASK-732.

## Decisions

### Decision 1: Extract the existing responsibility without redesign

- **Context**: The roadmap identifies a coherent operation mixed with its caller.
- **Alternatives considered**: Keeping the boundary implicit; combining the move
  with algorithm or state redesign, which would make equivalence harder to review.
- **Decision and rationale**: Preserve definitions and callers where possible,
  using the smallest visibility required by the existing consumers. See the
  [detailed brief](../design/refactoring-engine-protocol-types.md).

## Work log

- 2026-10-04: Started from local base `09f07efb8a03f565a8540f2fe15271adbd90c3cc` on
  `refactor/remaining-roadmap`. Preserved base source under
  `/tmp/tt-refactor/752` before production edits.

- 2026-10-04: All 24 moved wire types match their original declaration bytes; inverse reconstruction restores engine.ts exactly. Extension TypeScript compilation and 40 engine/session/compiler tests passed with zero failures/skips; /tmp/tt-refactor/752/focused.log. Runtime request, queue and lifecycle bodies are unchanged.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: Behavior-preserving extraction; no bug is fixed.

## Verification

- [x] Moved definitions and parent changes reviewed.
- [x] Focused checks: Run extension compilation and engine/session/compiler tests; compare all moved declarations and reconstruct the transport source. The TASK-751 base extension gate covers the unchanged transport; final extension gates rerun the complete set.
- [ ] Applicable complete repository gates.
- [ ] Reference expectations unchanged; task-index and whitespace checks.

## Result

Implementation and focused validation complete; final repository gates remain pending.
