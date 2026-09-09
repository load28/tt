# TASK-305: Emit a type-clean Result discriminator

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: `TASK-302: repair Result completion defects`

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

### Decision 1: Discriminate Result by success-field presence

- **Context**: Literal `kind` comparison is impossible for statically single-case inputs and produces TS2367.
- **Alternatives considered**: Widen emitted values with assertions, special-case constructors, or use the structural Result ABI.
- **Decision and rationale**: Core IR records success-field presence as the discriminator and every propagation emits the same `in` operation. No source spelling or TypeScript assertion is required.

## Work log

- 2026-08-30: Began reproducing strict TypeScript discriminator failures across direct, widened, aliased, and generic Result values.
- 2026-08-30: Added strict TypeScript coverage and connected downstream checker consequences through TypeScript symbol-declaration identity.
- 2026-08-30: Updated the editor regression to assert the semantic try diagnostic instead of the obsolete `.kind` property-error code.

## Issues and resolutions

- Structural narrowing correctly yields `unknown` on an unreachable direct-Err success edge; typed reporting now uses checker symbol identity to suppress only consequences of the already-reported try mismatch.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

All direct, widened, aliased, and generic Result shapes use one type-clean
structural discriminator while typed diagnostics preserve one causal error.
