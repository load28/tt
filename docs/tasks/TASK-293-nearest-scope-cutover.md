# TASK-293: Cut over syntax and nearest-scope propagation

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: See git history

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

### Decision 1: Record the Result target on each claimed `try`

- **Context**: A statement-bodied Result block may contain both its own
  propagation and a nested function's propagation.
- **Alternatives considered**: Infer the target again during Core lowering,
  or preserve the claimer's nearest-scope result on the AST node.
- **Decision and rationale**: Preserve the direct `try` spans on the Result
  block and assign `ResultRegionId` only to matching HIR nodes. Nested
  function propagation therefore retains its enclosing-function target.

## Work log

- 2026-08-30: Started tracing the legacy bind-based Result claimer and the expression `try` parser.
- 2026-08-30: Added speculative statement-body claiming, nearest-scope span
  recording, Result-targeted Core propagation, and source-preserving return
  rewriting for direct Result `try` expressions.
- 2026-08-30: Repaired the nested-function boundary: Result-targeted
  propagations are emitted only by the Result body printer, while overlapping
  opaque source ranges retain the nested function's bytes.

## Issues and resolutions

None.

## Verification

Test obligation from the plan: every §5 example, nested claim stops, ASI and TypeScript passthrough corpus, one-target validation, source maps, both host capabilities, and the full M0 compatibility corpus.

Green condition: claimed inner try exits only its nearest ResultRegion; nested function try exits its function; no valid TypeScript bytes change; no M0-affected program silently moves from a function target to a ResultRegion target.

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Result claiming, nearest-target lowering, nested function boundaries, source
maps, and both host printers use the same Result-region identity.
