# TASK-332: Preserve contextual typing for matches inside larger argument expressions

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Close the host form TASK-327 reproduced but did not repair: a scoped match
nested inside a larger argument, object, or array expression, such as
`consume({item: match …})`.

## Scope

- Included: Whole-value positions inside object literals, array literals, and
  multi-argument calls whose surrounding expression must stay at its authored
  evaluation point.
- Excluded: Whole-argument calls (repaired in TASK-327), control-flow-bearing
  arms (TASK-328), and sibling composition (TASK-329).

## Decisions

### Decision 1: Do not duplicate authored source across dispatch arms

- **Context**: Completing the consumer from an arm would have to re-emit the
  containing literal per arm, duplicating authored text (only one copy can own
  the source mapping) or reordering sibling property evaluation.
- **Alternatives considered**: Textual duplication, callback boundaries, and
  slot type synthesis; each violates an existing contract.
- **Decision and rationale**: Require an owner-level structural model (or a
  typed-backend contextual-type query) before changing emission. Record exact
  diagnostics first.

## Work log

- 2026-09-05: Reproduced during TASK-327: `consume({item: match (state) {
  Ready(value) => ({kind: "item", run: x => x + value}), Empty => … }});`
  emits the match through an unannotated join slot inside the literal and
  strict checking reports TS7006 on the callback parameter and TS2345 on the
  argument. Semantics are correct; only contextual typing is lost. No
  compiler change was made.

## Issues and resolutions

### Issue 1: A join slot inside a literal severs the consumer's context

- **Symptom**: TS7006/TS2345 for object/array-wrapped scoped matches under
  strict checking.
- **Cause**: The value's slot is an unannotated `let`; the literal around it
  keeps its authored position, so TypeScript cannot propagate the parameter
  context into the arm values.
- **Resolution**: Pending.

## Verification

- [ ] Strict reproductions and native oracles per wrapped form across
  `.tt`/`.ttx`
- [ ] Property/element evaluation order and effect-bearing sibling checks
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Pending; records a known coverage boundary split from
[TASK-327](./TASK-327-scoped-host-continuations.md).
