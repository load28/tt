# TASK-310: Expand generated control flow into readable TypeScript

- **Status**: Complete
- **Started**: 2026-09-01
- **Completed**: 2026-09-01
- **Commit**: `TASK-310: expand generated control flow readability`

## Purpose

Generated TypeScript still compresses declarations, conditions, bindings, value
delivery, and control-flow exits onto single lines. Expand compiler-owned glue
into a stable readable layout without reformatting source-backed TypeScript.

## Scope

- Included: Readable statement and block layout for generated match, pattern,
  propagation, and Result control flow; full-output regression snapshots
- Excluded: Reformatting pass-through TypeScript, changing temporary-name
  identity, or changing runtime and type semantics

## Decisions

### Decision 1: Layout only compiler-owned glue

- **Context**: Whole-output formatting would also rewrite valid TypeScript that
  tt promises to preserve byte for byte.
- **Alternatives considered**: Run a TypeScript formatter over the output, add
  an optional pretty-printing pass, or express readable structure in codegen.
- **Decision and rationale**: Express line and indentation structure through
  the existing mapping-aware rope. This improves generated regions while
  preserving source-backed bytes and mappings.

### Decision 2: Make generated statements the unit of layout

- **Context**: Existing indentation was structurally correct, but declarations,
  conditions, bindings, assignments, and exits still shared physical lines.
- **Alternatives considered**: Add spaces around the compact form, format only
  selected snapshots, or give each generated statement and block boundary a
  rope break.
- **Decision and rationale**: Emit one generated statement per line and expand
  generated blocks. The rule applies uniformly to switch and conditional
  matches, `try`, `result`, let-else, if-let, and rewritten exits.

### Decision 3: Keep semantic tests independent from presentation snapshots

- **Context**: Many semantic tests asserted the former single-line spelling,
  duplicating the complete emit snapshots and resisting intentional layout
  changes.
- **Alternatives considered**: Rewrite every semantic assertion with exact
  indentation or compare its generated tokens independent of whitespace.
- **Decision and rationale**: Keep exact layout in emit snapshots and the new
  focused readability test. Existing semantic assertions compact whitespace
  when they only need to verify generated structure or ordering.

### Decision 4: Align rewritten exits with their authored block body

- **Context**: Conditional match lowering keeps authored block bytes at their
  source column while generated wrapper braces use the lowering depth.
- **Alternatives considered**: Reindent authored source, align rewritten exits
  with generated braces, or align rewritten exits with authored statements.
- **Decision and rationale**: Preserve authored bytes and align rewritten value
  delivery and breaks with the authored statements. Switch arms retain their
  generated offset because their complete case body is indented by codegen.

## Work log

- 2026-09-01: Ran `./scripts/doctor`, reviewed TASK-198 and all emit snapshots,
  and identified same-line generated control flow as the remaining readability
  defect.
- 2026-09-01: Expanded generated match cases and guards, propagation failure
  branches, Result exits, let-else guards, if-let chains, bindings, assignments,
  and control-flow exits through structured rope breaks.
- 2026-09-01: Preserved authored multi-line block layout while aligning rewritten
  returns, and kept inline authored returns compact when no reliable line base
  exists.
- 2026-09-01: Added a focused one-generated-statement-per-line regression test,
  updated full emit snapshots, and ran the complete local CI gate.
- 2026-09-01: Addressed PR review by aligning conditional block-arm exits with
  authored statements, covering multi-line arms, let-else, and Result layout,
  and consolidating repeated switch-miss and depth-aware break emission.

## Issues and resolutions

### Issue 1: Binding groups disappeared after gaining layout breaks

- **Symptom**: Tuple match snapshots temporarily omitted multiple destructuring
  groups.
- **Cause**: The emptiness check used `resolved_text`, which deliberately
  returns no text for ropes containing unresolved layout breaks.
- **Resolution**: Added a structural `Rope::is_empty` query and used it before
  appending binding groups.

### Issue 2: Block-arm indentation depended on its dispatch form

- **Symptom**: One fixed indentation offset aligned switch arms but moved
  conditional-arm assignments and breaks right of authored statements.
- **Cause**: Switch cases indent the complete spliced body, while conditional
  chains preserve the authored body's absolute source column.
- **Resolution**: Select the generated exit offset from the dispatch form and
  lock both forms with focused layout tests and full-output snapshots.

## Verification

- [x] Focused codegen and snapshot tests — 372 compile tests and 4 snapshot tests passed
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci` — agents, rust, npm, native, and extension passed
- [x] Review follow-up `./scripts/ci rust` — format, clippy, all Rust tests, snapshots, and doctests passed

## Result

Compiler-owned TypeScript control flow now uses expanded blocks and one
generated statement per line across match, propagation, Result, let-else, and
if-let lowering. Source preservation, emit mappings, runtime behavior, typed
integration, npm tooling, and the editor suite all remain green.

Changed files: `src/codegen/core.rs`, `src/codegen/rope.rs`,
`tests/compile.rs`, `tests/fixtures/emit/`, and the task records.
