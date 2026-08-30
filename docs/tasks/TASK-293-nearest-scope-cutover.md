# TASK-293: Cut over syntax and nearest-scope propagation

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Item **L2** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`L2` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: parse statement-bodied Result blocks claimed by one nearest lexical tt `try`, lower inner propagation to ResultRegion, preserve #87 prefix-primary syntax, and retract the `<-` help at cutover.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: `src/parser/tries.rs` and result parser, AST/HIR, resolve/analysis nearest-scope walk, Core lowering, `src/sema.rs:451-466`, `docs/ai/tt.md:65-77`.

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

Test obligation from the plan: every §5 example, nested claim stops, ASI and TypeScript passthrough corpus, one-target validation, source maps, both host capabilities, and the full M0 compatibility corpus.

Green condition: claimed inner try exits only its nearest ResultRegion; nested function try exits its function; no valid TypeScript bytes change; no M0-affected program silently moves from a function target to a ResultRegion target.

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Ships to `main` alone: not stated in the plan.

Summarize the changed files and the outcome, then set this record and the index
row to `Complete`.
