# Isolate bundler module ID recognition

Task: [TASK-753](../tasks/TASK-753-bundler-module-ids.md). Base: `f9900b1016a0c2e211ec4c24cec229e995b454d6`.
Program: [behavior-preserving refactoring](behavior-preserving-refactoring.md).

## Boundary and implementation

Move TS/TSX suffixes, TT_FILE, isFile, fileOf, SPECIAL_QUERY, MODULE_MARKERS, moduleMarker, SCANNED_FILE, TT_MODULE_ID, queryOf, moduleId and sourceFileOfId into internal module-id.js. Export only names used by index.js and list the new internal module in the npm package files allowlist. Keep standard-library virtual IDs, compiler transport, caching and watch policy in index.js.

## Invariants and exclusions

Preserve file metadata checks, literal question/hash characters in filenames, path resolution, Windows separators, query stripping, marker ordering, special-query exclusion and all regex bytes. Package layout gains the required internal file; public exports and entry points remain unchanged.

No algorithms, state ownership, request ordering, public API, dependencies, or
reference expectations change. Inspect the complete diff, compare moved bodies
and reconstruct the parent against the saved base. Unexpected observations block
completion; behavior fixes require separate tasks.

## Allowed files and validation

Production scope: `integrations/unplugin/index.js`, `integrations/unplugin/module-id.js`, `integrations/unplugin/package.json`.
Task/index/progress documentation may also change. Base source snapshots and
comparison evidence live under `/tmp/tt-refactor/753`.

Run unplugin tests before/after with TTC_BINARY; inspect npm pack --dry-run to verify the new module is included. Final npm integration gates remain required.

The user requested autonomous completion of the remaining roadmap on 2026-10-04.
Slices are prepared sequentially on the local branch, with focused checks before
advancing and the complete required gates before completion/publication. No
intermediate change is merged while its required gates remain pending. Rust
1.98.0 and TypeScript 7.1.0-dev.20260826.1 remain pinned. Do not regenerate baselines.
