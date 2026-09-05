# TASK-327: Generalize scoped match host continuations

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Close the host-continuation coverage gaps retained by TASK-324. Its scoped invocation repair covers discarded, single-argument identifier calls, not arbitrary contextual expression consumers.

## Scope

- Included: Consumed call results, method and optional calls, explicit generic arguments, and scoped matches inside surrounding argument, object, or array expressions.
- Excluded: Control-flow-bearing arm bodies (TASK-328) and multiple scoped siblings (TASK-329).

## Decisions

### Decision 1: Model host evaluation and source ownership structurally

- **Context**: The existing call-completion proof intentionally rejects these host forms.
- **Alternatives considered**: Broaden textual matching, cast emitted values, suppress diagnostics, or introduce match callbacks; these would weaken the language and evaluation contracts.
- **Decision and rationale**: Extend the responsible AST/evaluation/continuation model only after reproducing each form. Preserve receivers, optional short-circuiting, overload selection, generic context, and authored-source ownership.

## Work log

- 2026-09-05: Recorded the unvalidated host forms from TASK-324 without changing compiler code. The earlier bare-call payload reproducer is repaired and must not be reported as an outstanding failure.

## Issues and resolutions

### Issue 1: Scoped values have a bounded host-completion implementation

- **Symptom**: Method/optional/generic calls and larger expressions are outside the validated scoped contextual forms. Individual current failures have not yet been established for every form.
- **Cause**: The implemented structural proof accepts only a discarded, single-argument identifier call; it does not model the broader continuations.
- **Resolution**: Pending. Start with a payload-binding match returning `{ kind: "item", run: x => x + value }` into an `Item` context whose callback parameter is `number`. Compare a consumed call result, `api.consume(...)`, `consume?.(...)`, `consume<Item>(...)`, and `consume({ item: ... })` against valid native TypeScript equivalents. Record exact diagnostics before selecting repairs.

## Verification

- [ ] Strict TypeScript/TSX reproductions and native TypeScript oracles for each host form
- [ ] Receiver/getter, optional short-circuit, exception, shadowing, and evaluation-order runtime checks
- [ ] Mixed `.tt`/`.ttx`/`.ts`/`.tsx` fixtures and reviewed complete-output snapshots
- [ ] Live `.tt`/`.ttx` editor diagnostics, callback hover/completion, and invalid-member source ranges
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Pending; records a known coverage boundary, not a claim that every listed form currently fails. Follow-up to [TASK-324](./TASK-324-scoped-contextual-continuations.md).
