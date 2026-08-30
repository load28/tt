# TASK-295: Remove `<-` and the old tail surface

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Item **L4** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`L4` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: delete ResultBind parsing/HIR/anchors, semicolon-free tail semantics, and `result-tail-semicolon`, while activating the M0 migration and applicable `= try` edits in the same release.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: result parser, AST/HIR, `AnchorKind::ResultBind`, diagnostics/explain, mapper, semantic tokens, snapshots, docs.

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

Test obligation from the plan: applicable edits for old binds, `a < -b` passthrough, identifier `result` corpus, anchor/source-map replacements, and one-release aliases.

Green condition: no old syntax is emitted or silently accepted, every supported migration edit is valid, and all external diagnostic surfaces agree.

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Ships to `main` alone: not stated in the plan.

Summarize the changed files and the outcome, then set this record and the index
row to `Complete`.
