# TASK-303: Preserve statement-position match inside Result blocks

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

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
