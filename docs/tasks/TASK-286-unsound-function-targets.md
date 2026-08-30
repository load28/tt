# TASK-286: Reject unsound function targets

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: `TASK-286: reject unsound function targets`

## Purpose

Item **P4** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`P4` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: reject statement and expression `try` in constructors, generators, and async generators, including constructor positions before/inside/after `super`, while retaining nested ResultRegion and `using` legality.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: `src/program_syntax.rs::EvaluationOwner`, `src/evaluation_ir.rs::TargetCapability`, `src/sema.rs` placement reporting, diagnostics.

## Decisions

Record every decision this task makes. Naming a new public diagnostic code, a
wire-compatibility choice, or a seam that the design leaves open is a decision
and belongs here with its alternatives.

### Decision 1: Classify unsafe return targets before target emission

- **Context**: constructors and generators accept JavaScript `return`, but
  neither can carry a propagated Result failure safely.
- **Alternatives considered**: rely on TypeScript diagnostics; reject only
  statement `try`; or retain function kind through both semantic paths.
- **Decision and rationale**: classify statement hosts from the parser's
  brace model and expression hosts from SWC's function visitor, then reject
  both with the existing `try-placement` diagnostic.

## Work log

- 2026-08-30: Started tracing constructor and generator owner classification.
- 2026-08-30: Added parser-side function-target classification for statement
  propagation.
- 2026-08-30: Carried constructor and generator ownership through the SWC
  overlay for expression propagation.
- 2026-08-30: Added statement and expression regression coverage for
  constructor, generator, and async-generator hosts.

## Issues and resolutions

### Unsafe return completion

- **Symptom**: generated propagation could return an Err from a constructor
  or complete a generator with an Err value.
- **Cause**: both function kinds were previously classified as ordinary
  function bodies.
- **Resolution**: the semantic and host-evaluation paths now preserve the
  unsafe function kind and refuse statement-region lowering.

## Verification

Test obligation from the plan: located `try-placement` for both forms; runtime guard that `new C() instanceof C`; generator guard proving no emitted program can produce `{value: Err, done: true}` or silently truncate `for...of`; nested Result and disposal acceptance tests.

Green condition: unsafe inputs emit nothing and never rely on `ts2409`, TypeScript types, or consumer behavior as the signal.

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Ships to `main` alone: yes.

`src/flow/mod.rs` classifies statement hosts. `src/program_syntax.rs` retains
constructor and generator ownership for expression hosts. `src/evaluation_ir.rs`
refuses their statement regions, while `src/sema.rs` reports statement placement.
`tests/compile.rs` covers both forms and all unsafe function kinds.
