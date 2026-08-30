# TASK-304: Repair expression-boundary and template-hosted Result

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: `TASK-302: repair Result completion defects`

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

### Decision 1: Model lexical return as an explicit continuation

- **Context**: Expression-boundary Result success was routed through assignment-only code, while template lexing hid nested Result returns from flow analysis.
- **Alternatives considered**: Add host-specific branches, force assignment slots, or represent return as a continuation destination.
- **Decision and rationale**: Add an explicit return destination and lex the owned Result body span independently. The same continuation now covers class fields, defaults, generators, templates, and async boundaries.

## Work log

- 2026-08-30: Began pinning expression-boundary ICEs and template success ownership with minimal accepted inputs.
- 2026-08-30: Added compile, runtime, and mapping coverage for every affected host protocol.

## Issues and resolutions

- Template literals arrived as opaque outer lexer tokens; flow analysis now lexes the Result-owned body span directly.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Ordinary Result success now completes every accepted expression boundary, and
template-hosted Result bodies retain their success ownership.
