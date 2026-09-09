# TASK-283: Close host projection and source-preservation crashes

- **Status**: Complete
- **Started**: 2026-08-29
- **Completed**: 2026-08-30
- **Commit**: —

## Purpose

Item **P1** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`P1` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: make C-style declaration-init Legal by hoisting only its initializer value while retaining `let i` in the header; make repeated for-test a located rejection; make discarded Result source preservation diagnose rather than ICE.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: `src/program_syntax.rs::{owner_reach,HostContinuation}`, `src/program_syntax.rs::emit_result_region` (the ProgramSyntax projection at `:1078`, not the codegen printer of the same name at `src/codegen/core.rs:1832`), `src/evaluation_ir.rs`, `src/codegen/core.rs::lowering_plan`, and relevant diagnostics.

## Decisions

Record every decision this task makes. Naming a new public diagnostic code, a
wire-compatibility choice, or a seam that the design leaves open is a decision
and belongs here with its alternatives.

### Decision 1: Model C-style declaration propagation as a separate host rewrite

- **Context**: A normal propagation statement cannot be projected into a
  C-style `for` initializer, but moving a declaration's successful binding
  out of the header would change its scope.
- **Alternatives considered**: Keep a statement placeholder in the header;
  reconstruct the loop from source text; or add a typed propagation rewrite.
- **Decision and rationale**: Project propagation as a typed expression-plus-
  terminator overlay, then plan a dedicated prelude before the loop and a
  payload declaration in the original header. This preserves evaluation order
  and declaration scope without source-shape heuristics.

## Work log

- 2026-08-29: Isolated the C-style declaration-init failure to a statement
  placeholder projected inside the loop header. Added the typed
  `HostContinuation::ForInitialize` boundary; the remaining implementation
  must plan and print its prelude before the enclosing `for` statement.
- 2026-08-30: Added a typed for-initializer propagation plan and target
  rewrite. The Result evaluation and error exit now precede the loop while
  the C-style header retains its successful payload declaration.
- 2026-08-30: Rejected repeated loop-header propagation, assignment
  initializers, and discarded Result expressions in Evaluation IR before they
  can reach invalid target emission or source-preservation validation.
- 2026-08-30: Ran the complete local gate.

## Issues and resolutions

### Repeated loop header and assignment initializer

- **Symptom**: These placements reached output verification or inline emission
  without a statement-safe target.
- **Cause**: Statement propagation was not checked against owner reach or the
  declaration-only C-style initializer contract.
- **Resolution**: Evaluation IR returns a located lowering failure before
  codegen for repeated and assignment placements.

### Discarded Result expression

- **Symptom**: Source-preservation validation raised an internal compiler
  error after a discarded Result expression omitted pass-through bytes.
- **Cause**: A discarded Result has no target that can preserve its failure
  completion.
- **Resolution**: Evaluation IR returns a located lowering failure before
  target emission.

## Verification

Test obligation from the plan: statement and expression-boundary host cases, `for (let i = try n();;)`, `for (; try ready();)`, assignment-init guard, and `result { ... };`, all wrapped around `analyze` and verified as parseable output or located diagnostics.

Green condition: Legal init executes once and retains declaration semantics; repeated test and discard never panic.

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Ships to `main` alone: yes.

`src/program_syntax.rs` adds the propagation projection and C-style
continuation. `src/evaluation_ir.rs` records legal for-initializer rewrites
and rejects unsafe placements. `src/codegen/core.rs` emits the prelude and
header payload. `tests/compile.rs` fixes the legal output and every rejected
placement as a non-panicking, located diagnostic.
