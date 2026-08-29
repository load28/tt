# TASK-269: Respect tsconfig source boundaries and surface backend failures

- **Status**: Complete
- **Started**: 2026-08-28
- **Completed**: 2026-08-28
- **Commit**: —

## Purpose

The typed engine currently projects `.tt` files outside the configured
TypeScript program and can crash the backend with a synthetic-file lookup.
Align compiler inputs with project membership and preserve backend failure
information through the editor boundary.

## Scope

- Included: tsconfig membership, projection/query selection, CLI ICE contract,
  server `backendError`, and VS Code presentation
- Excluded: ordinary TypeScript module-resolution diagnostics

## Decisions

### Decision 1: Project membership comes from the TypeScript project model

- **Context**: Recursively scanning every `.tt` under the root disagrees with
  tsconfig `files`, `include`, and `exclude`.
- **Alternatives considered**: skip the failing query; ignore particular
  directories; or derive membership from the configured program.
- **Decision and rationale**: Use the configured project graph. Directory and
  error-string filters are heuristics and cannot implement tsconfig semantics.

### Decision 2: Backend failures carry a typed classification

- **Context**: A missing toolchain and a running backend that violates its
  protocol both collapsed into one string and the editor guessed that every
  failure meant TypeScript was not installed.
- **Alternatives considered**: classify by message text in each consumer;
  keep exit 2 for every failure; or carry an availability/internal sum type
  across the backend, engine, server, CLI, and editor seams.
- **Decision and rationale**: Carry the classification structurally. The CLI
  preserves exit 2 for unavailable infrastructure and routes internal
  failures through the compiler ICE contract (exit 101). The server returns a
  structured `backendError`, and the editor shows an internal-failure notice
  once while retaining text-level diagnostics.

## Work log

- 2026-08-28: Created from nightly audit finding 4.
- 2026-08-28: Started from the completed TASK-268 compiler stack.
- 2026-08-28: Kept the root scan as a layered-filesystem candidate set and
  made TypeScript return the exact candidate modules admitted to its program.
  Literal, tag, symbol, declaration, and tt diagnostic reporting now use that
  membership; imported modules outside `include` remain members.
- 2026-08-28: Added the backend failure classification and threaded it through
  native transport, engine output, CLI exit policy, JSON-lines protocol, and
  VS Code presentation without exposing Node stack frames.
- 2026-08-28: Added end-to-end regressions for an unrelated `examples/*.tt`,
  an imported module outside `include`, surviving `val-mutation` diagnostics,
  CLI ICE rendering, server `backendError`, and editor failure classification.
- 2026-08-28: Ran Rust formatting, Clippy, and the complete Cargo suite; all
  passed. TypeScript compilation and all six typed-check adapter tests passed.

## Issues and resolutions

- The first regression wrote `examples/demo.tt` before creating `examples/`.
  The fixture now creates every non-`src` directory explicitly.
- The complete local CI reached the extension stage after the agents, Rust,
  npm, and native stages passed. Three pre-existing macOS install-path tests
  compare `/private/var/...` with its `/var/...` alias; the other 127 extension
  tests, including every changed typed-check and server path, passed. This task
  does not change package-install path canonicalization.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

The TypeScript program now authoritatively selects project members from the
candidate virtual modules. Unrelated `.tt` files outside tsconfig no longer
receive type queries or poison the typed pass, while imported files outside
`include` remain in the graph. Backend failures are structurally classified
and presented according to their cause. Changed files: `src/typescript/`,
`src/engine/`, `src/main.rs`, `src/server.rs`, `tests/native.rs`, and
`editors/vscode/server/src/{server.ts,ttc.ts,test/typedcheck.test.ts}`.
