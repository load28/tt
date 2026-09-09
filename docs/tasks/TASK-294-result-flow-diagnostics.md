# TASK-294: Add control-flow and use diagnostics

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: See git history

## Purpose

Item **L3** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`L3` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: implement `result-no-success-value`, `result-value-discarded`, let-else completion, outward break/continue/label/yield crossing checks, and permanent isolated-region crossing behavior while retaining M0's one-release migration help.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: resolve/analysis/sema CFG, `HostExit` capture, diagnostic registry/explain, server/content mapper.

## Decisions

Record every decision this task makes. Naming a new public diagnostic code, a
wire-compatibility choice, or a seam that the design leaves open is a decision
and belongs here with its alternatives.

### Decision 1: Claim-time syntax and completion checking remain separate

- **Context**: A statement-bodied Result block is claimed by an inner lexical
  `try`; a later control-flow check must diagnose a reachable fallthrough.
- **Alternatives considered**: Keep the old direct-try placement rejection, or
  add the Result completion diagnostic at the semantic layer.
- **Decision and rationale**: Keep claim and completion independent. The
  semantic layer reports `result-no-success-value` before emission, so a
  claimed block never reaches TypeScript verification without a completion.

### Decision 2: Reuse the statement CFG for Result completion

- **Context**: A direct `return` token is insufficient when only one branch
  returns.
- **Alternatives considered**: Scan tokens for a return, or build a separate
  Result-only control-flow walker.
- **Decision and rationale**: Reuse the existing flow CFG over the Result
  body token span. This preserves its established handling of branch joins,
  unreachable statements, and TypeScript control-flow syntax.

### Decision 3: Reuse lexical control scopes for Result crossing checks

- **Context**: An unlabeled break inside a loop written in the Result body is
  local, while a labeled break or a break to an enclosing loop crosses the
  ResultRegion.
- **Alternatives considered**: Reject every control-transfer keyword, or
  resolve the target with the existing loop, switch, and label model.
- **Decision and rationale**: Reuse the flow scanner's lexical scopes and
  report only transfers without a local target. Yield is also checked inside
  opaque expression statements, while nested user-written functions remain a
  boundary.

## Work log

- 2026-08-30: Started after L2 made statement-bodied Result blocks claimable.
- 2026-08-30: Added `result-no-success-value` and
  `result-value-discarded`, including a Result-body CFG query and suppression
  of the redundant fallthrough diagnostic for a discarded Result.
- 2026-08-30: Rewrote Result-owned returns in unbraced conditional branches
  as blocks so the generated Result break remains inside its source branch.
- 2026-08-30: Added named Result crossing diagnostics for break, continue,
  labeled transfers, and yield. Local loop and switch transfers remain valid.
- 2026-08-30: Projected inline let-else and if-let bodies beneath the Result
  host arrow and emitted their returns through the same Result completion
  printer as direct returns.

## Issues and resolutions

None.

## Verification

Test obligation from the plan: all abrupt completions, unreachable versus reachable fallthrough, discard variants, inline if-let/let-else, nested Decisions, and generator/yield crossings.

Green condition: every path either produces one Result completion or one named diagnostic; no redundant primary diagnostics.

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Result completion and crossing diagnostics are structural, and public clients
share the same located diagnostic set.
