# TASK-320: Audit mixed-source runtime and project semantics

- **Status**: Complete
- **Started**: 2026-09-04
- **Completed**: 2026-09-05
- **Commit**: `TASK-320: prove mixed-source runtime semantics`

## Purpose

Find failures that remain after single-file parsing and emission succeed by
exercising mixed `.tt`, `.ttx`, `.ts`, and `.tsx` project graphs through typed
checking and runtime execution.

## Scope

- Included: Cross-source imports, generated declaration boundaries, evaluation
  order, runtime values, JSX-hosted tt constructs, and project compilation
- Excluded: New syntax, package publication, and external service changes

## Decisions

### Decision 1: Test semantic outcomes beyond parseable output

- **Context**: A program can emit parseable TypeScript while changing evaluation
  order, binding identity, import resolution, or runtime values.
- **Alternatives considered**: Continue parser-only fuzzing; add isolated unit
  cases; generate complete mixed-source projects with typed and runtime oracles.
- **Decision and rationale**: Use complete project graphs and deterministic
  semantic oracles, then reduce each failure to the responsible compiler layer.

### Decision 2: Execute the emitted tree through a real bundler

- **Context**: Preserved JSX output is not directly executable by Node, while
  generated projects consume the same output through a bundler.
- **Alternatives considered**: Avoid JSX in the runtime fixture; rewrite the
  emitted files in the test; bundle the compiler output without modifying it.
- **Decision and rationale**: Type-check the complete emitted tree with `tsc`,
  bundle its entry with Bun's production bundler, and execute the resulting
  JavaScript with Node. This keeps `.ttx` JSX in the matrix and tests the output
  contract used by generated applications.

## Work log

- 2026-09-04: Confirmed the pinned development environment, restored the audit
  branch after the app returned to `main`, and opened the mixed-source audit.
- 2026-09-05: Added four source-kind producers and four source-kind consumers.
  Every consumer calls every producer, forming all sixteen directed runtime
  edges. The `.tt` consumer lowers pipelines and the `.ttx` consumer lowers a
  match inside JSX.
- 2026-09-05: Added a shared trace module and a deterministic runtime oracle for
  returned values, left-to-right call order, and shared module identity.
- 2026-09-05: Built the fixture through the public CLI, type-checked its emitted
  tree, bundled it with Bun, and executed it with Node.
- 2026-09-05: Ran the complete local merge gate with registry and loopback
  access. All agents, Rust, npm, website, native-backend, and extension stages
  passed.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `cargo test --test cli mixed_source_project_preserves_all_directed_runtime_values`
- [x] `./scripts/ci` — agents, rust, npm, website, native, and extension passed

## Result

The mixed-source contract now covers all sixteen directed source-kind edges at
runtime as well as at parse, type, declaration, and emitted-tree boundaries.
The executable oracle proves values, evaluation order, shared module identity,
pipeline lowering, JSX-hosted match lowering, bundling, and Node execution.

Changed files: `tests/cli.rs`, `tests/fixtures/mixed-source-runtime/`,
`docs/design/mixed-source-composition-matrix.md`, this task record, and the task
index.
