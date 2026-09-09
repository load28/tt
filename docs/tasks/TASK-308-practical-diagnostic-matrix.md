# TASK-308: Add a practical CLI/editor diagnostic matrix

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: `TASK-308: add practical diagnostic UI matrix`

## Purpose

Exercise realistic, multi-error application code and bring its diagnostics to
the standard set by rustc: cause-specific messages, precise primary spans,
explanatory secondary labels, and actionable help. Prove that the CLI and the
editor preserve the same structured answer.

## Scope

- Included: Shared practical fixtures, mixed tt and TypeScript errors,
  multi-file and TSX contexts, comparison with analogous rustc diagnostics,
  complete CLI and final LSP assertions, and fixes for quality defects exposed
  by the matrix.
- Excluded: Random fuzzing, exhaustive combinations of every diagnostic, and
  redesigning a diagnostic whose existing contract is already correct.

## Decisions

### Decision 1: Test user workflows instead of isolated syntax fragments

- **Context**: Existing unit and snapshot tests cover individual rules well,
  but they do not systematically prove what a user sees when independent tt
  and TypeScript mistakes coexist in an application-shaped project.
- **Alternatives considered**: Add more one-rule unit tests, or build a small
  matrix of multi-file scenarios shared by the CLI and editor harnesses.
- **Decision and rationale**: Use shared project fixtures. Each scenario will
  declare the diagnostic codes and source text that both surfaces must report.

### Decision 2: Treat rustc's diagnostic structure as the quality bar

- **Context**: Cross-surface parity can preserve a weak diagnostic just as
  reliably as a good one; equality alone does not answer whether the output is
  useful.
- **Alternatives considered**: Assert only codes and ranges, or audit the same
  categories rustc exposes: primary cause, supporting location, and concrete
  recovery advice.
- **Decision and rationale**: Use the latter. Local rustc output for analogous
  bad field, type mismatch, immutable mutation, non-diverging let-else, and bad
  call cases provides the comparison baseline.

### Decision 3: Keep one manifest as the CLI and editor contract

- **Context**: CLI text appends edit replacements to `help:` while LSP carries
  the same suggestion as a title plus a structured edit. Separate expectations
  could drift while both tests stayed green.
- **Alternatives considered**: Independent stderr and LSP snapshots, or one
  manifest containing the shared message/range plus surface-specific help.
- **Decision and rationale**: Use one manifest per project. The Rust harness
  checks rendered CLI blocks; the LSP harness checks final published codes,
  messages, UTF-16 ranges, suggestions, related information, and quick fixes.

### Decision 4: Improve defects exposed by the matrix

- **Context**: The audit found weak output rather than only missing coverage:
  typed `val` mutations lacked declaration context and a fix, merged editor
  diagnostics were not source ordered, and Result block ranges excluded `}`.
- **Alternatives considered**: Record the current output as expectations, or
  repair the responsible compiler/editor layers before pinning it.
- **Decision and rationale**: Repair the layers. A quality fixture must not turn
  a known weak diagnostic into the permanent contract.

## Work log

- 2026-08-30: Ran `./scripts/doctor`, reviewed TASK-223 and the existing CLI,
  engine, snapshot, and LSP diagnostic suites, and created the task branch.
- 2026-08-30: Compared analogous failures with the pinned local rustc and
  expanded the task from surface parity to diagnostic quality.
- 2026-08-30: Added four application-shaped projects covering a cross-file
  service, immutable cache and pipeline, TSX dashboard, and Result boundaries.
- 2026-08-30: Added `tests/practical_diagnostics.rs` for real
  `ttc --check-types` output and an LSP matrix over final
  `textDocument/publishDiagnostics` notifications.
- 2026-08-30: Added a declaration label and remove-`val` edit to typed mutation
  diagnostics, restored final LSP source ordering, and carried complete Result
  block spans through AST and HIR.
- 2026-08-30: Moved CLI and editor runs to disposable copies under
  `target/tt-tests` so compiler-generated standard-library packages never
  modify the source fixtures.
- 2026-08-30: Documented the `val` diagnostic behavior and ran the focused
  matrix, full Rust and extension suites, and `./scripts/ci`.

## Issues and resolutions

### Issue 1: A field typo hid exhaustiveness in the same match

- **Symptom**: The first service fixture expected both `unknown-field` and
  `match-not-exhaustive` from one match, but only the field error appeared.
- **Cause**: This is the existing owned-cause contract from TASK-267: a broken
  pattern suppresses consequences from that same match.
- **Resolution**: Kept the contract and put the independent missing-arm mistake
  in a second match in the same function.

### Issue 2: Typed `val` mutation lacked declaration context

- **Symptom**: The mutation site was underlined, but neither the CLI nor editor
  identified the `val` declaration or offered the Rust-equivalent mutability
  change.
- **Cause**: Binding probes retained only the identifier offset, and the typed
  verdict emitted no labels or suggestions.
- **Resolution**: Preserved the modifier offset through the probe and projection,
  labeled its declaration, and attached an edit that removes `val` and its
  following horizontal whitespace.

### Issue 3: The editor reordered independent diagnostic layers

- **Symptom**: The CLI reported source order, while the editor placed typed
  diagnostics after or between earlier text diagnostics.
- **Cause**: The language server appended asynchronously authored layers and
  never sorted the final merged list.
- **Resolution**: Sort the complete generation by source range and stable code
  immediately before publication.

### Issue 4: Result diagnostic ranges stopped before the closing brace

- **Symptom**: LSP ranges for `result-no-success-value` and
  `result-value-discarded` excluded `}`, despite the CLI drawing a complete
  multi-line block frame.
- **Cause**: Result AST/HIR nodes ended at the brace-excluded body span.
- **Resolution**: Store the complete half-open Result span in the AST and use it
  for semantic and lowering diagnostics.

### Issue 5: Direct extension tests lacked the compiler PATH contract

- **Symptom**: `npm test` launched directly caused existing LSP cases to fail or
  time out although focused tests passed.
- **Cause**: The standalone command does not add `target/debug` to `PATH`; the
  repository's `extension` gate does.
- **Resolution**: Re-ran through `./scripts/ci extension` and then the complete
  `./scripts/ci`; both passed.

### Issue 6: Typed checks materialized packages inside source fixtures

- **Symptom**: Running either matrix created generated `@tt/runtime` and
  `@tt/std` modules below each fixture's `node_modules` directory.
- **Cause**: Typed compilation materializes its virtual packages at the checked
  project root, and the harnesses used the source fixtures as that root.
- **Resolution**: Copy each fixture to a unique disposable project below
  `target/tt-tests`, exclude any stale `node_modules`, and run both surfaces
  against the copy.

## Verification

- [x] Focused CLI diagnostic matrix — 4 projects, 11 diagnostics
- [x] Focused editor/LSP diagnostic matrix — 4 projects, final publications
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci` — agents, rust, npm, native, and extension passed

## Result

The repository now continuously checks rustc-style diagnostic quality across
realistic mixed-error projects on both user surfaces. The matrix pins stable
codes, complete messages and ranges, actionable help, declaration labels,
quick-fix edits, multiplicity, and source order.

Changed files: `tests/practical_diagnostics.rs`,
`tests/fixtures/practical-diagnostics/`,
`editors/vscode/server/src/{server.ts,test/server.test.ts}`, `src/{ast.rs,
parser/results.rs,sema.rs,val.rs,hir/lower.rs,engine/projection.rs,
engine/semantics.rs}`, `docs/ai/tt.md`, and the task records.
