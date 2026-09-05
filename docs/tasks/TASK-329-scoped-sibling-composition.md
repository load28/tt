# TASK-329: Audit mixed scoped siblings and nested match composition

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Extend TASK-324's binding-free sibling coverage to owners containing payload bindings, declaration-bearing arms, and nested scoped matches.

## Scope

- Included: Scoped/plain sibling mixtures, multiple scoped siblings, and nested scoped values in arguments, arrays, objects, and TSX expressions across all four source extensions.
- Excluded: Reopening the repaired binding-free sibling defect; isolated host and arm modeling belong to TASK-327 and TASK-328.

## Decisions

### Decision 1: Preserve evaluation order across the entire expression owner

- **Context**: Current inline sibling planning requires every participating value to be expression-compatible. A scoped sibling falls outside that proof.
- **Alternatives considered**: Delay all arm values until all subjects have run, independently rewrite siblings, or special-case argument positions; these do not establish owner-wide semantics.
- **Decision and rationale**: Audit and model the whole owner, preserving each subject and selected arm value at its original evaluation point. Coordinate with TASK-327/328 rather than add separate textual exceptions.

## Work log

- 2026-09-05: Recorded the remaining scoped-sibling and nesting matrix gap from TASK-324. The original plain sibling reproducer now passes strict checking.

## Issues and resolutions

### Issue 1: Mixed scoped siblings lack validated contextual composition

- **Symptom**: The 112 repaired sibling matrix cells do not establish support for payload/declaration-bearing sibling values or arbitrary nesting. Current concrete failures must be reproduced before claiming a product defect.
- **Cause**: The owner-level inline proof requires all roots to be expression-compatible; scoped completion currently supports a single eligible call argument.
- **Resolution**: Pending. Start from `pair(first: Item, second: Item)` with one payload-binding match and one plain match, reverse their positions, then make both scoped. Repeat with local declarations and nested matches. Use callback parameters without annotations so strict checking verifies genuine contextual typing.

## Verification

- [ ] Matrix covers scoped/plain permutations, multiple scoped values, nesting, and `.tt`/`.ttx`/`.ts`/`.tsx` boundaries
- [ ] Earlier arm effects precede later subjects, including later-subject mutations and abrupt completion
- [ ] Payload/local name shadowing, generated-name hygiene, strict unused checks, and source ownership
- [ ] Strict type checks, runtime oracles, reviewed snapshots, and live editor diagnostics/hover/completion/source ranges
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Pending. Follow-up to [TASK-324](./TASK-324-scoped-contextual-continuations.md), coordinated with [TASK-327](./TASK-327-scoped-host-continuations.md) and [TASK-328](./TASK-328-control-flow-contextual-arms.md).
