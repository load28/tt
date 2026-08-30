# TASK-304: Repair expression-boundary and template-hosted Result

- **Status**: Pending
- **Started**: —
- **Completed**: —
- **Commit**: —

## Purpose

Repairs defect **D3, D4** in the shipped Result completion model. All of these
defects first ship in `a308c64` (#88) and are independent of any future language
proposal — they are wrong behaviour in released code, not design questions.

Severity: High / Medium.

## Symptom

Two related defects. D3: an ordinary Result success at an expression boundary reaches `ValueContinuation::assignment_prefix` and aborts the compiler; only the sole trailing `return try X;` family works today, so class fields, parameter defaults, and `yield` operands ICE on accepted input. D4: a template-hosted Result reports `result-no-success-value` even though the block owns a success, rejecting valid input with the wrong diagnostic.

## Scope

- Included: Give `ValueContinuation` an explicit expression-return destination and stop an already-returning boundary continuation from reaching `assignment_prefix`. Cover the sole trailing `return try X;` family and ordinary Result success in class fields, parameter defaults, `yield`, templates, async, source maps, and the constructor and generator runtime protocols.
- Excluded: any change to the Result language model. This task does not revisit
  what `return` means, the success channel, or placement rules. A change to those
  is a change to `docs/design/try-result-scopes.md` first.

Files and symbols: `ValueContinuation` in `src/codegen/core.rs`, plus template success ownership traced through the parser, HIR, `src/program_syntax.rs`, `src/evaluation_ir.rs`, and `src/sema.rs`.

## Green condition

No accepted host aborts the compiler; an unsupported shape receives a located structural diagnostic instead. The template false positive is fixed by one structural change rather than a special case, and the working `return try X;` family is preserved.

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
