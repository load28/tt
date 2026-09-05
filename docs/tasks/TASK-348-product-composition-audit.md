# TASK-348: Audit product and mixed-source composition failures

- **Status**: Complete
- **Started**: 2026-09-06
- **Completed**: 2026-09-06
- **Commit**: —

## Purpose

Find and repair real product failures across documentation, CLI, create-tt, and mixed-source compiler composition.

## Scope

- Included: Product gates, source-kind combinations, compiler regressions, and documentation contracts.
- Excluded: Publication and unrelated working branches.

## Decisions

### Decision 1: Extend executable structural coverage

- **Context**: Arbitrary programs form an unbounded space; existing extension edges alone cannot prove semantic composition.
- **Alternatives considered**: Repeat existing fixtures or add failure-driven structural cases.
- **Decision and rationale**: Run all product gates and reduce failures to compiler ownership contracts, with typed and runtime regression oracles.

## Work log

- 2026-09-06: Doctor passed. Fetched origin and fast-forwarded main (already current). Created product-composition-audit-331 because the existing codex branch prevents codex/ branch names.

- 2026-09-06: Baseline Rust, native, and editor gates passed. Reproduced mixed-source dynamic import failures before modifying the parser.
- 2026-09-06: Added literal import recognition and 168 emission cases. Both rewrite modes pass strict type checking, Bun bundling, and Node execution across sixteen directed edges, including JSX and JSX-hosted matches.
- 2026-09-06: Fixed Beta channel selection and protected customized init configs; all ten create-tt unit tests pass.
- 2026-09-06: Generated and reviewed the complete literal-import TSX snapshot; all four snapshot tests pass.

## Issues and resolutions

### Issue 1: Emitted dynamic imports reference removed source extensions

- **Symptom**: The mixed-source dynamic-import reproducer emitted successfully,
  but TypeScript reported TS2307 for `.tt` and `.ttx` targets from all four source
  kinds. The pre-fix run is recorded in `/tmp/tt-331-repro.log`.
- **Cause**: The import parser explicitly excluded dynamic import and import-type
  strings from the relative module rewrite used by static imports.
- **Resolution**: Recognize literal first arguments with an argument-boundary
  proof and use the existing import AST/HIR/codegen path. Preserve computed
  expressions and surrounding source. Add 168 emission cells, 32 executable
  edge/mode combinations, and a reviewed TSX snapshot.

### Issue 2: Beta initializers select stable dependencies

- **Symptom**: `dependencyChannel('0.3.0-beta')` returned `latest`, despite the
  release contract publishing Beta artifacts under `beta`.
- **Cause**: The selector handled nightly and RC prereleases but omitted Beta.
- **Resolution**: Map Beta prereleases to `beta` and add channel regression cases.

### Issue 3: Reinitialization overwrites customized configuration

- **Symptom**: Existing `tsconfig.tt.json` and bundler wrappers were overwritten.
- **Cause**: Generated files were written immediately without checking existing
  contents or validating the complete generated output set.
- **Resolution**: Preflight every generated config before writes. Accept identical
  files for idempotent reruns and reject customized files while preserving the
  original manifest and all other files. Test both conflict positions and reruns.

### Issue 4: Sandbox restrictions prevent two baseline gate stages

- **Symptom**: create-tt dependency installation could not open registry sockets;
  website prerendering failed with `listen EPERM`.
- **Cause**: Restricted network and loopback access, rather than product behavior.
- **Resolution**: Re-run the full gate with approved network and local-listen
  access; do not skip the affected stages.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

- [x] 168 literal import emission cells across parse modes, targets, quotes, hosts, and rewrite modes
- [x] 32 dynamic import edge/mode combinations: strict checking, real JSX, Bun bundling, and Node execution
- [x] `UPDATE_EXPECT=1 cargo test --test snapshot` — four tests passed; reviewed complete output
- [x] `node --test packages/create-tt/test/*.test.mjs` — ten passed
- [x] `./scripts/ci website` — passed again after the final website copy update
- [x] `git diff --check`

The full gate passed all six stages: agents, Rust (including fmt, clippy, tests,
and fuzz harness build), npm (including freshly installed scaffold builds),
website, native, and extension (127 passed, zero skipped). Logs are available
locally at `/tmp/tt-331-ci-final.log` and `/tmp/tt-331-website-final.log`.
The final JSX matrix enhancement also passed its focused CLI test and clippy.

## Result

Repaired three confirmed failures: emitted lazy module resolution, Beta
initializer dependency selection, and customized initializer config loss.
Updated the language reference, public module API comments, website copy,
initializer README, and composition coverage document.

Changed files:

- Compiler: `src/parser/imports.rs`, `src/parser/parse.rs`, `src/lib/api.rs`.
- Initializer: `packages/create-tt/src/installer.js`, its unit tests and README.
- Regressions: `tests/cli.rs`, `tests/cli/dynamic_imports.rs`,
  `tests/compile/cases_04.rs`, `tests/passthrough.rs`, and the complete
  `tests/fixtures/emit/literal-imports/` fixture.
- Documentation: `docs/ai/tt.md`,
  `docs/design/mixed-source-composition-matrix.md`, `website/src/content.json`,
  this record, and `docs/tasks/INDEX.md`.

This completed repair batch does not establish arbitrary program correctness.
Existing scope-sensitive contextual continuation work remains in
[TASK-327](./TASK-327-scoped-host-continuations.md),
[TASK-328](./TASK-328-control-flow-contextual-arms.md), and
[TASK-329](./TASK-329-scoped-sibling-composition.md); those tasks were not closed
by the import/initializer fixes. Changes remain on `product-composition-audit-331`
for review and are not merged into main.
