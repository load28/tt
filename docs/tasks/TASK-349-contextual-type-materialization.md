# TASK-349: Materialize TypeScript contextual types for scoped values

- **Status**: Complete
- **Started**: 2026-09-06
- **Completed**: 2026-09-06
- **Commit**: Originally `eeb8eb0` (TASK-332); rebased as `fe6920c` and renumbered to avoid the upstream task collision.

## Purpose

Preserve TypeScript contextual typing when scoped tt expressions require statement lowering. Resolve the eight recorded host and cleanup regressions without generated IIFEs.

## Scope

- Included: Structural value-slot metadata, TypeScript contextual type queries, ordinary type annotations, source mappings, and project integration.
- Excluded: Generated callbacks, type assertions, and diagnostic suppression.

## Decisions

### Decision 1: Use the TypeScript backend for contextual type facts

- **Context**: Statement lowering separates arm expressions from their contextual host.
- **Alternatives considered**: IIFEs violate the emission contract; syntax-only continuation duplication does not cover arbitrary scoped hosts.
- **Decision and rationale**: The user approved integrating TypeScript analysis into compilation while retaining the no-IIFE contract. Validate checker-derived annotations at generated value declarations before extending the pipeline.

## Work log

- 2026-09-06: Preserved recovered changes in checkpoint commit `081adfc`. Began backend feasibility verification against the pinned TypeScript API.

## Issues and resolutions

### Issue 1: Scoped arm values lose contextual typing

- **Symptom**: Eight recorded cases report TS7006 or TS2345 after successful tt emission.
- **Cause**: Unannotated intermediate variables separate callbacks and object literals from their expected types.
- **Resolution**: Codegen records mutable slots and captured values. The checker supplies contextual types; fixed-point annotation insertion preserves scoped values without a runtime function boundary.

## Verification

- [x] Checker feasibility probe
- [x] Scoped contextual regression suite
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Complete. Contextual compilation, project and editor integration, structural nested-return delivery, mappings, and regression coverage are implemented. See [the design contract](../design/contextual-type-materialization.md).

## Implementation checkpoint

- Added explicit value-declaration marks to mapping-aware emission, batched contextual slot queries to the TypeScript seam, and fixed-point insertion of ordinary type annotations.
- Preserved every verbatim source mapping after annotation insertion, including UTF-8 text; scoped/finally backend tests pass.
- Connected standalone compilation, diagnostic reports, project snapshots, and editor service projections to the new phase.
- The eight recorded scoped host regressions now pass; the integration suite reports 133 passed, zero failed or ignored.
- Avoided diagnostic computation during contextual-only rounds. The full host protocol matrix completes in 2.69 seconds after this change.
- Updated the content-mapper fixture to inherit the pinned project TypeScript installation, as required by contextual compilation. All 12 mapper tests pass.
- Reviewed the sole output snapshot change: a JSX string value slot now has an ordinary `string` annotation.
- Remaining verification: mixed-source contextual graph coverage and the complete local gate. Standalone file/project resolution and editor dependency invalidation remain under review.

### Additional findings and repairs

- 2026-09-06: Added 136 strict TypeScript/TSX cells covering eight arm families, eight hosts, and JSX contexts. This exposed an untyped captured earlier object argument; codegen now records contextual sites for captured constants as well as mutable slots.
- 2026-09-06: Nested returned matches omitted their statement regions. Generalized structured return delivery and retained authored parentheses through a separate value delivery. A runtime test verifies nested `using` / `await using` disposal before method invocation.
- 2026-09-06: AST return records now retain the value beneath transparent wrappers. Projection-owned identifiers map through explicit overlay identities; they are not mistaken for copied TypeScript source spans.
- 2026-09-06: Added all 16 mixed-source directed edges, including ordinary CLI printing from `.tt` and `.ttx`. Separate analysis paths prevent `.tt` / `.ts` and `.ttx` / `.tsx` filename collisions. Imported annotations are rewritten by the existing import model.
- 2026-09-06: Expanded live `.tt` / `.ttx` editor diagnostics, hover, completion and invalid-member ranges to consumed, method, optional, wrapped, captured-sibling, cleanup and nested cases. The extension suite passed 128 tests without skips or cancellations.
- 2026-09-06: Updated the cross-file CLI fixture to inherit its required TypeScript dependency. The first full gate caught this missing fixture dependency; the fixture now models the supported consumer installation.

### Final verification (2026-09-06)

`./scripts/ci` passed agents, rust, npm/create-tt, website, native and extension (`/tmp/tt-332-ci-verified.log`). A final `./scripts/ci rust` passed after placing annotation emission in codegen (`/tmp/tt-332-rust-verified.log`). Key suites: 403 compile tests, 135 integration tests (including 136 family cells), 65 native tests (including 16 mixed-source edges), 12 content-mapper tests and 128 editor tests with zero skips/cancellations. Remote CI has not been run for these local commits.

## Changed files

- `docs/ai/tt.md`
- `docs/design/contextual-type-materialization.md`
- `docs/tasks/INDEX.md`
- `docs/tasks/TASK-324-scoped-contextual-continuations.md`
- `docs/tasks/TASK-327-scoped-host-continuations.md`
- `docs/tasks/TASK-328-control-flow-contextual-arms.md`
- `docs/tasks/TASK-329-scoped-sibling-composition.md`
- `docs/tasks/TASK-330-editor-refresh-test-timeout.md`
- `docs/tasks/TASK-349-contextual-type-materialization.md`
- `editors/vscode/server/src/test/engine.test.ts`
- `src/codegen/contextual.rs`
- `src/codegen/core/emitter/expression.rs`
- `src/codegen/core/emitter/host.rs`
- `src/codegen/core/emitter/result.rs`
- `src/codegen/core/emitter/source.rs`
- `src/codegen/mod.rs`
- `src/codegen/rope.rs`
- `src/codegen/rope/builder.rs`
- `src/engine/language/project.rs`
- `src/engine/project.rs`
- `src/engine/projection.rs`
- `src/lib/compile.rs`
- `src/lib/mapped.rs`
- `src/program_syntax.rs`
- `src/program_syntax/collector.rs`
- `src/program_syntax/visit.rs`
- `src/typescript/backend.rs`
- `src/typescript/contextual.rs`
- `src/typescript/host.mjs`
- `src/typescript/mod.rs`
- `src/typescript/native.rs`
- `tests/content_mapper.rs`
- `tests/fixtures/emit/jsx-children-and-attributes/expected.tsx`
- `tests/integration/cases_02.rs`
- `tests/integration/contextual.rs`
- `tests/native.rs`
