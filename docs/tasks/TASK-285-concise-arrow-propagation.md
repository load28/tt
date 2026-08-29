# TASK-285: Repair concise-arrow propagation

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Item **P3** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`P3` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: make ordinary, parenthesized, and pipeline-step concise arrows containing expression `try` lower to valid block-bodied arrows without moving the return target.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: arrow/pipeline ownership in `src/program_syntax.rs`, `src/evaluation_ir.rs`, and corresponding codegen printers.

## Decisions

Record every decision this task makes. Naming a new public diagnostic code, a
wire-compatibility choice, or a seam that the design leaves open is a decision
and belongs here with its alternatives.

### Decision 1: <one-line summary>

- **Context**:
- **Alternatives considered**:
- **Decision and rationale**:

## Work log

- YYYY-MM-DD: ...

## Issues and resolutions

None.

## Verification

Test obligation from the plan: default verification and `--no-verify` output parse, preserve evaluation order and lexical capture, and keep standalone concise `=> try x` behavior.

Green condition: every accepted output parses and the arrow, never its enclosing function, owns failure.

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Ships to `main` alone: yes.

Summarize the changed files and the outcome, then set this record and the index
row to `Complete`.
