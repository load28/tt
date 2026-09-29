# TASK-528: Answer completion and signature help inside an unfinished tt value

- **Status**: In progress
- **Started**: 2026-09-29
- **Completed**: —
- **Commit**: —

## Purpose

Signature help and completion answered nothing while the user typed the
operand of a value `try`, a `try` inside a `result` block, or the body of a
match's last arm (`const n = try parse("1", |` → no signature, no items),
although `const n = parse("1", |` answers both.

## Scope

- Included: How the projection of a buffer whose TypeScript does not parse
  emits an unfinished value `try`, how the service reaches a cursor the
  projection did not copy, and the completion resolve edits of such a probe.
- Excluded: The typed batch path and the lowering of files whose TypeScript
  parses.

## Decisions

## Work log

## Issues and resolutions

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## Result
