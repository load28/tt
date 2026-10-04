# TASK-753: Isolate bundler module ID recognition

- **Status**: In progress
- **Started**: 2026-10-04
- **Completed**: —
- **Commit**: —

## Purpose

Move TS/TSX suffixes, TT_FILE, isFile, fileOf, SPECIAL_QUERY, MODULE_MARKERS, moduleMarker, SCANNED_FILE, TT_MODULE_ID, queryOf, moduleId and sourceFileOfId into internal module-id.js. Export only names used by index.js and list the new internal module in the npm package files allowlist. Keep standard-library virtual IDs, compiler transport, caching and watch policy in index.js.

## Scope

- Included: `integrations/unplugin/index.js`, `integrations/unplugin/module-id.js`, `integrations/unplugin/package.json`.
- Excluded: Behavior changes, public API changes, baseline updates, and TASK-732.

## Decisions

### Decision 1: Extract the existing responsibility without redesign

- **Context**: The roadmap identifies a coherent operation mixed with its caller.
- **Alternatives considered**: Keeping the boundary implicit; combining the move
  with algorithm or state redesign, which would make equivalence harder to review.
- **Decision and rationale**: Preserve definitions and callers where possible,
  using the smallest visibility required by the existing consumers. See the
  [detailed brief](../design/refactoring-bundler-module-ids.md).

## Work log

- 2026-10-04: Started from local base `f9900b1016a0c2e211ec4c24cec229e995b454d6` on
  `refactor/remaining-roadmap`. Preserved base source under
  `/tmp/tt-refactor/753` before production edits.

- 2026-10-04: The base and corrected head each passed 21 unplugin tests with zero failures/skips. Exact helper bytes and inverse reconstruction of index.js passed. npm pack --dry-run confirms module-id.js is included, with public package exports unchanged. Evidence: /tmp/tt-refactor/753/{base,focused}.log and pack.json.

## Issues and resolutions

### Issue 1: Loader suffix dependency missed during extraction

- **Symptom**: Three existing unplugin tests reported `TSX_SUFFIX is not defined`.
- **Cause**: The adapter loader also consumes the suffix constant moved with ID recognition.
- **Resolution**: Added TSX_SUFFIX to the internal helper exports and parent imports;
  reran the complete unplugin suite. No test expectations or algorithms changed.

### Issue 2: Read-only npm cache

- **Symptom**: The initial dry-run pack reported EROFS under the default npm cache.
- **Cause**: The managed environment's home directory is read-only.
- **Resolution**: Use `--cache /tmp/tt-npm-cache` for this read-only package inspection.

## Regression test (fails before the fix)

Not applicable: Behavior-preserving extraction; no bug is fixed.

## Verification

- [x] Moved definitions and parent changes reviewed.
- [x] Focused checks: Run unplugin tests before/after with TTC_BINARY; inspect npm pack --dry-run to verify the new module is included. Final npm integration gates remain required.
- [ ] Applicable complete repository gates.
- [ ] Reference expectations unchanged; task-index and whitespace checks.

## Result

Implementation and focused validation complete; final repository gates remain pending.
