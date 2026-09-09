# TASK-347: Rebase product audit onto updated main

- **Status**: Complete
- **Started**: 2026-09-09
- **Completed**: 2026-09-09
- **Commit**: —

## Purpose

Integrate the preserved product audit and contextual type materialization with main at a96426b.

## Decisions

Preserve upstream call completion and editor repairs, and retain the local checker-backed contextual storage and regression coverage. A backup branch preserves the original commits. Resolve task number collisions by assigning new numbers to the local records.

## Work log

- 2026-09-09: Doctor passed; fetched origin and fast-forwarded local main. Backed up the work branch and began rebasing both local commits.

## Issues and resolutions

Both branches changed call completion, regression tests, and task records. Preserve upstream structural improvements and combine independent coverage.

## Verification

Initial full CI exposed an upstream getter/receiver regression: contextual annotation of a receiver capture used the narrower `this` parameter type and removed its callable member. Receiver captures now retain inference; value and argument captures still participate in contextual typing. The existing runtime/strict-type regression is the verification oracle.

The added generic editor diagnostic assertion now uses upstream's `answered` helper, preserving the new nullable engine-response contract. The receiver regression passes after repair.

- `./scripts/ci`: agents, npm/create-tt, website and native passed (`/tmp/tt-347-rebase-ci.log`).
- After receiver and nullable-response repairs, extension passed all 160 tests with zero skips (`/tmp/tt-347-rebase-verified.log`).
- Reviewed and updated three whole-output snapshots: upstream generic instantiation now uses separate callee/instantiation captures; literal and sibling slots now carry ordinary contextual annotations.
- Final `./scripts/ci rust` passed fmt, clippy, all Rust suites (including 147 integration tests and 65 native tests), snapshots and fuzz compilation (`/tmp/tt-347-rust-final.log`).
- `git diff --check` passed. `origin/main` is an ancestor of the rebased branch.
- No remote push or remote CI was performed.

## Result

Rebased both preserved commits onto `a96426b`. Upstream call completion, recovery and CLI changes coexist with local contextual materialization, dynamic imports and installer repairs. Task records 331/332 from the local branch were renumbered to 348/349; original commit identities remain documented.

The original branch is retained at `recovery/product-composition-audit-331-before-rebase-20260909` (`eeb8eb0`). All local CI stages passed across the initial run and focused reruns.

## Changed files


- `docs/ai/tt.md`
- `docs/design/contextual-type-materialization.md`
- `docs/design/mixed-source-composition-matrix.md`
- `docs/tasks/INDEX.md`
- `docs/tasks/TASK-328-control-flow-contextual-arms.md`
- `docs/tasks/TASK-329-scoped-sibling-composition.md`
- `docs/tasks/TASK-332-wrapped-argument-contextual-values.md`
- `docs/tasks/TASK-333-captured-argument-contextual-types.md`
- `docs/tasks/TASK-347-rebase-product-audit.md`
- `docs/tasks/TASK-348-product-composition-audit.md`
- `docs/tasks/TASK-349-contextual-type-materialization.md`
- `editors/vscode/server/src/test/engine.test.ts`
- `editors/vscode/server/src/test/server.test.ts`
- `packages/create-tt/README.md`
- `packages/create-tt/src/installer.js`
- `packages/create-tt/test/installer.test.mjs`
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
- `src/lib/api.rs`
- `src/lib/compile.rs`
- `src/lib/mapped.rs`
- `src/parser/imports.rs`
- `src/parser/parse.rs`
- `src/program_syntax.rs`
- `src/program_syntax/collector.rs`
- `src/program_syntax/visit.rs`
- `src/typescript/backend.rs`
- `src/typescript/contextual.rs`
- `src/typescript/host.mjs`
- `src/typescript/mod.rs`
- `src/typescript/native.rs`
- `tests/cli.rs`
- `tests/cli/dynamic_imports.rs`
- `tests/compile/cases_04.rs`
- `tests/content_mapper.rs`
- `tests/fixtures/emit/contextual-literal-argument/expected.ts`
- `tests/fixtures/emit/contextual-scoped-call/expected.ts`
- `tests/fixtures/emit/contextual-scoped-call/input.tt`
- `tests/fixtures/emit/contextual-sibling-completion/expected.ts`
- `tests/fixtures/emit/jsx-children-and-attributes/expected.tsx`
- `tests/fixtures/emit/literal-imports/expected.tsx`
- `tests/fixtures/emit/literal-imports/input.ttx`
- `tests/integration/cases_02.rs`
- `tests/integration/contextual.rs`
- `tests/native.rs`
- `tests/passthrough.rs`
- `website/src/content.json`
