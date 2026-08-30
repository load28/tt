# TASK-303: Preserve statement-position match inside Result blocks

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: `TASK-302: repair Result completion defects`

## Purpose

Repairs defect **D2** in the shipped Result completion model. All of these
defects first ship in `a308c64` (#88) and are independent of any future language
proposal — they are wrong behaviour in released code, not design questions.

Severity: Critical.

## Symptom

A statement-position `match` inside a claimed `result` block drops its dispatch and its source side effects, and can emit a slot that is never declared. Output is both semantically wrong and, in some shapes, invalid TypeScript.

## Scope

- Included: Pin complete dispatch, strict TypeScript output, runtime side-effect order, typed projection, and source maps.
- Excluded: any change to the Result language model. This task does not revisit
  what `return` means, the success channel, or placement rules. A change to those
  is a change to `docs/design/try-result-scopes.md` first.

Files and symbols: `src/program_syntax.rs::{emit_result_body, emit_result_decision}`, `src/evaluation_ir.rs`, and `src/codegen/core.rs::{emit_result_statement_decision, emit_result_statements_with_exits}`.

## Green condition

Every arm's side effects run in source order, every emitted slot is declared, and the output type-checks under strict TypeScript.

## Decisions

Record every decision this task makes, including any new public diagnostic code
and any wire-compatibility choice, with its alternatives.

### Decision 1: Consume the structured match plan in the Result statement stream

- **Context**: Result-owned statement emission bypassed the ordinary host rewrite and printed only the match join slot.
- **Alternatives considered**: Reparse emitted source, special-case the fixture shape, or reuse the Core/Evaluation IR plan.
- **Decision and rationale**: Reuse the existing owner rewrite or value slot and emit the decision structurally. This preserves all arms and keeps source mapping attached to the original match.

## Work log

- 2026-08-30: Began reproducing statement-position match loss through the syntax, evaluation, and target layers.
- 2026-08-30: Added structural statement emission, runtime order checks, and emit-map invariants for match and template hosts.

## Issues and resolutions

- A completed match inside an incomplete TypeScript owner had no safe host plan; the editor projection now uses an isolated recovery IIFE only for that plan-less statement value.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Statement-position matches inside Result blocks retain dispatch, arms, effects,
and source mappings. Incomplete editor owners also remain completion-capable.
