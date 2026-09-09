# TASK-279: Bound misplaced try recovery at ternary branches

- **Status**: Complete
- **Started**: 2026-08-29
- **Completed**: 2026-08-29
- **Commit**: `TASK-279: bound try recovery at ternary branches`

## Purpose

Keep a misplaced `try` diagnostic inside its ternary branch while allowing a
ternary expression inside the recovered operand.

## Scope

- Included: ternary-aware recovery scanning and parser regressions
- Excluded: changing valid `try` placement or broadly refactoring scanners with different stopping contracts

## Decisions

### Decision 1: Track ternary balance structurally

- **Context**: Delimiter depth alone lets a first-branch recovery consume the enclosing `: fallback`.
- **Alternatives considered**: Stop at every colon; inspect source strings; track pending depth-zero question marks.
- **Decision and rationale**: Count lexer `Punct('?')` tokens and consume a colon only when it closes a pending ternary. An unmatched depth-zero colon ends recovery, while fused optional-chain and coalesce tokens remain unaffected.

### Decision 2: Keep the change local unless contracts align exactly

- **Context**: `stmt_expr_end` has related ternary logic but a different statement-expression stopping contract.
- **Alternatives considered**: Force a shared helper; implement the small state transition locally.
- **Decision and rationale**: Prefer a localized parser fix unless extraction removes exact duplication without weakening either scanner's semantics.

## Work log

- 2026-08-29: Reviewed the recovery proposal against lexer token kinds and rejected string-shape or unconditional-colon heuristics.
- 2026-08-29: Added a depth-zero ternary counter to `parse_misplaced_try`; an unmatched colon now terminates recovery and a matched colon remains in the operand.
- 2026-08-29: Added first-branch and inner-ternary regressions beside the existing last-branch case. Confirmed `?.` and `??` remain fused lexer tokens and do not affect the counter.
- 2026-08-29: Regenerated snapshot fixtures and reviewed the empty diff.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Misplaced `try` recovery now ends at the enclosing ternary branch separator while preserving ternaries inside the recovered operand. The change is token-structural, local to the responsible parser, and leaves optional chaining and nullish coalescing untouched.
