# TASK-305: Emit a type-clean Result discriminator

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Repairs defect **D5** in the shipped Result completion model. All of these
defects first ship in `a308c64` (#88) and are independent of any future language
proposal — they are wrong behaviour in released code, not design questions.

Severity: High.

## Symptom

`try Result.Err(e)` emits a discriminator comparison that is statically impossible, so strict TypeScript reports TS2367 on a path the language guide documents. The emit-to-tsc contract is not type-clean.

## Scope

- Included: Represent one general Result discriminator operation. Do not special-case the text `Result.Err`. Pin statically `Err`, statically `Ok`, widened `TResult`, aliases, and generics through emitted output, pinned strict tsc, the typed engine, and source maps.
- Excluded: any change to the Result language model. This task does not revisit
  what `return` means, the success channel, or placement rules. A change to those
  is a change to `docs/design/try-result-scopes.md` first.

Files and symbols: `src/core_ir/mod.rs`, `src/core_ir/lower.rs`, and `src/codegen/core.rs`.

## Green condition

Every documented propagation shape emits TypeScript that is clean under strict tsc, and the fix is one general operation rather than a syntactic special case.

## Decisions

Record every decision this task makes, including any new public diagnostic code
and any wire-compatibility choice, with its alternatives.

### Decision 1: <one-line summary>

- **Context**:
- **Alternatives considered**:
- **Decision and rationale**:

## Work log

- YYYY-MM-DD: ...

## Issues and resolutions

None.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/ci`

## Result

Summarize the changed files and the outcome, then set this record and the index
row to `Complete`.
