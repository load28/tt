# TASK-328: Preserve contextual typing across control-flow and cleanup arms

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Investigate and repair contextual typing beyond the expression and linear-return arm proofs in TASK-324 without changing cleanup or exception semantics.

## Scope

- Included: Conditional and multiple returns, loops, nested structured tt, try/catch/finally, and resource-disposal-bearing match arms.
- Excluded: Broader host-call forms (TASK-327) and sibling composition (TASK-329).

## Decisions

### Decision 1: Represent completion and cleanup boundaries explicitly

- **Context**: Moving a consumer into an arm can move it inside a handler or before a finalizer/disposal action.
- **Alternatives considered**: Reuse linear-return lowering indiscriminately, flatten lexical scopes, or wrap matches in callbacks; these can change observable behavior.
- **Decision and rationale**: Require a structural completion model with scope and cleanup ownership. Do not widen the current proof until equivalent runtime order and exception boundaries are demonstrated.

## Work log

- 2026-09-05: Split TASK-324's deliberately excluded non-linear and cleanup-bearing arms into an independently verifiable follow-up. No implementation was made.

## Issues and resolutions

### Issue 1: Non-linear arm completion is outside the validated contextual path

- **Symptom**: Contextually typed returned object/callback values are not covered for these arm structures. This is an audit gap; each concrete compilation failure still needs reproduction.
- **Cause**: Current proofs accept expression arms or a final return following a restricted linear statement prefix, and exclude handler/finalizer/disposal boundaries.
- **Resolution**: Pending. Begin with an `Item` consumer and an arm containing `if (other) return { kind: "item", run: x => x }; return { kind: "item", run: x => x };`. Separately test the return inside `try/finally`, inside `try/catch`, and after a resource declaration. Establish valid native TypeScript oracles and preserve arm cleanup before consumer invocation.

## Verification

- [ ] Strict contextual typing for each control-flow family, including nested tt constructs
- [ ] Consumer exceptions stay outside arm handlers; finalizers and disposal run in original order
- [ ] Return/throw/break/continue and suspension semantics remain unchanged where applicable
- [ ] Mixed-source compilation, whole-output snapshots, and live `.tt`/`.ttx` editor diagnostics/hover/completion/source ranges
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Pending. Follow-up to [TASK-324](./TASK-324-scoped-contextual-continuations.md); no diagnostic suppression or fallback is authorized by this record.
