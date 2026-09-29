# TASK-413: Report a malformed variant behind declaration modifiers once

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`export variant V { A { r: number } }` reported `malformed-variant` twice, and `export declare variant ...` three times. docs/ai/tt.md: "identical position/range/message duplicates are merged."

## Scope

- Included: The parser's variant claim loop in `src/parser/parse.rs`.
- Excluded: The variant grammar.

## Decisions

### Decision 1: One claim attempt per declaration keyword run

- **Context**: After a `Claim::Malformed` (or `Claim::Unclaimed`) answer for `export`/`declare` + `variant`, the loop advanced one token and met `declare` or `variant` again, which started the same claim.
- **Decision and rationale**: After those answers the scan continues past the `variant` keyword the claim was made for, so each declaration is judged once.

## Work log

- 2026-09-27: Reproduced with `ttc --check`; fixed the loop and added a compile test over the four modifier combinations (fails before, passes after).

## Issues and resolutions

None.

## Verification

- [x] `cargo test --test compile a_malformed_variant_behind_modifiers_is_reported_once`

## Result

Changed `src/parser/parse.rs` and `tests/compile/cases_11.rs`.
