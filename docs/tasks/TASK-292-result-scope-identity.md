# TASK-292: Add scope and completion identity

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: See git history (`TASK-292`).

## Purpose

Item **L1** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`L1` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: introduce stable `ResultRegionId`, Result-targeted exits, structural `is_async`, ownership stacks, labels, finally semantics, and validation shared by both printers.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: `src/core_ir/mod.rs::{ExitTarget,ResultRegion}`, `src/core_ir/lower.rs`, ProgramSyntax projection, Core validation, codegen completion mapping.

## Decisions

Record every decision this task makes. Naming a new public diagnostic code, a
wire-compatibility choice, or a seam that the design leaves open is a decision
and belongs here with its alternatives.

### Decision 1: Use the Result block node as the stable scope identity.

- **Context**: Result propagation needs a target that remains stable across
  projection and both completion printers.
- **Alternatives considered**: An anonymous region marker; a lowering-order
  ordinal; the Result block node identity.
- **Decision and rationale**: Use `ResultRegionId(NodeId)`. It is stable for
  the snapshot and keeps the target tied to the lexical Result region.

## Work log

- 2026-08-30: Started tracing ResultRegion lowering and existing exit-target validation.
- 2026-08-30: Added `ResultRegionId`, made Result bindings target their owning
  region, and made validation reject a non-owning target.
- 2026-08-30: Routed expression-boundary Result binding failures through the
  same Result completion delivery as statement-host lowering.
- 2026-08-30: Made structural await scanning skip nested function and class
  bodies, then ran the full repository gate.

## Issues and resolutions

- **Symptom**: A binding item's node shadowed the enclosing Result block node
  during lowering. **Cause**: Both pattern fields were named `node`.
  **Resolution**: Bind the Result block node as `region_node` and construct the
  target from that explicit identity.

## Verification

Test obligation from the plan: nested functions/results, invalid/stale target validation, await across nested boundaries, label collision, and finally override for success and failure.

Green condition: every completion has exactly one live destination and both printers consume an identical semantic record.

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Ships to `main` alone: not stated in the plan.

Changed `src/core_ir/mod.rs`, `src/core_ir/lower.rs`, `src/codegen/core.rs`,
`src/scanner.rs`, and Core/scanner tests. Result completion targets now have
one stable identity across lowering and both existing printers.
