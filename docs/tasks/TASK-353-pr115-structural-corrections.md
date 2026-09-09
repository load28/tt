# TASK-353: Correct PR 115 at the owning compiler and tooling layers

- **Status**: Complete
- **Started**: 2026-09-10
- **Completed**: 2026-09-10
- **Commit**: `TASK-353: fix(compiler): preserve control-flow and tooling ownership`

## Purpose

Resolve the seven reported compiler and tooling defects and the two reproduced
PR review regressions without input-specific patches or diagnostic suppression.

## Scope

- Included: control-flow ownership, expression projection, val assignment analysis,
  inferred join types, watch lifecycle, temporary files, and output path planning
- Excluded: unrelated language changes and release publication

## Decisions

### Decision 1: Repair the existing PR with explicit regression contracts

- **Context**: PR 115 is based on the current main and needs corrections.
- **Alternatives considered**: Replace the PR or append isolated fixes to its head.
- **Decision and rationale**: Continue from the PR head, preserve its changes, and
  repair each defect in the compiler/tooling layer that owns its invariant.
  Runtime results, strict type checking, filesystem behavior, and lifecycle tests
  establish behavior rather than matching particular source spellings.

## Work log

- 2026-09-10: Ran `./scripts/doctor` successfully; checked out PR 115 at aed5979
  on `fix/pr115-review`. The PR base is the current main, e3b89e2.
- 2026-09-10: Began reproducing the seven reported defects and the output-path
  escape and duplicate-input staging race from the prior review.

- 2026-09-10: Added runtime, strict typing, assignment-target, filesystem, watch,
  concurrent publication, and editor request-lifetime regressions.
- 2026-09-10: Reviewed full emitted snapshots. Existing changes are inferred
  annotations; the new fixture also pins lexical exits and template capture.
- 2026-09-10: The broader contextual matrix exposed premature inner inference;
  split contextual propagation and join inference into ordered fixed-point rounds.

## Issues and resolutions

### Inline decision returns

- Symptom: a return inside `if let` escaped its enclosing match arm.
- Cause: projection hid statement bodies behind synthetic function boundaries;
  the ordinary decision printer also discarded the arm continuation.
- Resolution: project inline bodies in their lexical region and pass a body
  continuation through the shared decision emitter. Remove the duplicate
  Result-specific if-let/let-else printers.

### Template capture boundaries

- Symptom: scheduling a template before a match left malformed `${}` syntax.
- Cause: delimiters were synthesized outside source relocation ranges.
- Resolution: retain alternating raw/interpolation chunks, including empty raw
  chunks, and give raw spans ownership of their adjacent delimiters.

### Val assignment targets

- Symptom: parenthesized references and destructuring escaped write checks.
- Cause: mutation detection inspected only tokens following a flat access path.
- Resolution: parse assignment targets recursively and resolve collected root
  occurrences through the existing lexical scope walk and typed symbol probes.
  Defaults and computed keys remain reads; rest targets remain writes.

#- Full Rust suites passed, including 413 compile cases, 151 integration cases,
  69 CLI cases, 66 native cases, and all emitted/diagnostic snapshots.
- The original `pick` reproduction prints `105` after strict TypeScript compilation.
- npm scaffold dependency downloads and website prerender listening were blocked
  by the sandbox on the initial run; both stages passed when rerun with the
  required network/listening access.

## Changed files

- Compiler: `src/program_syntax/projection.rs`, `src/lexer.rs`,
  `src/hir/lower.rs`, `src/evaluation_ir/{evaluation,planning}.rs`,
  `src/codegen/core/mod.rs`, `src/codegen/core/emitter/{mod,source,expression,host,result}.rs`.
- TypeScript: `src/typescript/{backend,contextual,native}.rs` and `host.mjs`.
- Val and CLI: `src/val.rs`, `src/val/{checker,targets}.rs`,
  `src/main/{build,command,output}.rs`.
- Editor: `editors/vscode/server/src/ttc.ts` and `test/compiler.test.ts`.
- Tests: `tests/cli.rs`, compile cases 01/05/06/09, `tests/emit_map.rs`,
  `tests/integration.rs`, integration `cases_02.rs` and `pr115.rs`,
  eleven updated emit snapshots and the new `owned-inline-control-flow` fixture.
- Documentation: contextual type materialization design, task index, TASK-352
  supersession notice, and this record.

## Result-owned expression propagation

- Symptom: assignment RHS `try` emitted declarations in expression position.
- Cause: the planner excluded every Result-owned propagation from scheduling.
- Resolution: schedule expressions normally and emit failure exits through the
  active lexical ResultRegionId continuation. Conditional operand ownership
  ignores enclosing tt regions while retaining checks on nested tt syntax.

### Inferred join storage

- Symptom: an empty-array branch caused strict evolving-array diagnostics.
- Cause: generated join declarations had no contextual storage marker/type.
- Resolution: mark all join sites; propagate contextual types to a fixed point,
  then query TypeScript for incoming RHS types and remove subsumed union members.
  Re-enter contextual propagation after inferred annotations expose new facts.
  This ordering preserves nested callbacks' contextual parameters and literals.

### Watch failures

- Symptom: a missing input directory was reported every polling interval.
- Cause: enumeration both printed an error and returned only an exit status.
- Resolution: return structured failure text to callers; watch owns the error
  state and reports transitions, resets on recovery, and keeps watching.

### Editor check lifetime

- Symptom: temporary directories survived one-shot compiler checks.
- Cause: the server allocated one process-lifetime directory without disposal.
- Resolution: each request owns a unique directory until its child completes,
  then removes it in `finally`, including failed checks and concurrent versions.

### Output planning and publication

- Symptom: parent path components escaped `-o`; duplicate inputs raced over one
  PID-named staging file.
- Cause: raw relative paths entered common-root computation, and neither jobs
  nor staging files had exclusive ownership.
- Resolution: normalize absolute source roots before relative output planning,
  reject root mismatches instead of falling back to basenames, deduplicate
  source/output identities, and exclusively create per-write staging files.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci` (all six stages; npm/website passed in the permission-enabled rerun)

- Full Rust suites passed, including 413 compile cases, 151 integration cases,
  69 CLI cases, 66 native cases, and all emitted/diagnostic snapshots.
- The original `pick` reproduction prints `105` after strict TypeScript compilation.
- npm scaffold dependency downloads and website prerender listening were blocked
  by the sandbox on the initial run; both stages passed when rerun with the
  required network/listening access.

## Changed files

- Compiler: `src/program_syntax/projection.rs`, `src/lexer.rs`,
  `src/hir/lower.rs`, `src/evaluation_ir/{evaluation,planning}.rs`,
  `src/codegen/core/mod.rs`, `src/codegen/core/emitter/{mod,source,expression,host,result}.rs`.
- TypeScript: `src/typescript/{backend,contextual,native}.rs` and `host.mjs`.
- Val and CLI: `src/val.rs`, `src/val/{checker,targets}.rs`,
  `src/main/{build,command,output}.rs`.
- Editor: `editors/vscode/server/src/ttc.ts` and `test/compiler.test.ts`.
- Tests: `tests/cli.rs`, compile cases 01/05/06/09, `tests/emit_map.rs`,
  `tests/integration.rs`, integration `cases_02.rs` and `pr115.rs`,
  eleven updated emit snapshots and the new `owned-inline-control-flow` fixture.
- Documentation: contextual type materialization design, task index, TASK-352
  supersession notice, and this record.

## Result

All seven reported defects and both prior PR review regressions are corrected.
The changes preserve source ownership, lexical exits, type inference, and
resource lifetimes through their owning compiler/tooling layers.
