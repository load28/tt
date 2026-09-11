# TASK-364: Audit and repair a structural compiler defect

- **Status**: Complete
- **Started**: 2026-09-11
- **Completed**: 2026-09-11
- **Commit**: `TASK-364: fix(parser): preserve private match ownership`

## Purpose

Audit compiler boundaries on the latest `main`, reproduce a valid-program or
diagnostic contract failure, and repair it at the compiler layer that owns the
underlying invariant.

## Scope

- Included: Parser, HIR, resolution, semantic analysis, code generation, and
  typed-engine behavior needed to reproduce and repair one structural defect
- Excluded: New language syntax, editor-only behavior, release automation, and
  input-specific fallbacks or diagnostic suppression

## Decisions

### Decision 1: Preserve declaration-name spans as ownership ranges

- **Context**: SWC represents a private method key with the span of `#name`,
  while the tt lexer represents the ambiguous candidate with the identifier
  span of `name`.
- **Alternatives considered**: Special-case private methods in tt candidate
  parsing, discard private methods from host proof, adjust the private span by a
  fixed byte, or preserve complete AST name ranges.
- **Decision and rationale**: Preserve complete host declaration-name ranges and
  ask whether the tt identifier token is contained by one. Public names remain
  exact ranges and SWC's `#match` range naturally contains the lexer-owned
  `match` token. Sorted ranges retain logarithmic lookup. The parser learns
  neither private syntax nor a byte adjustment.

## Work log

- 2026-09-11: Fast-forwarded `main` from `fcaf2a0` to `09243c6`, preserved the
  existing untracked `.task-agent-disabled` file, and confirmed `./scripts/doctor`
  reports a ready development environment.
- 2026-09-11: Created `task-364-compiler-structural-repair` from the updated
  `main` and began a read-only audit of compiler contracts.
- 2026-09-11: Confirmed the baseline `cargo test` suite and the complete local
  TypeScript pass-through corpus pass before compiler changes.
- 2026-09-11: Reproduced a valid private method named `#match` being rejected as
  malformed tt syntax, then traced the failure to SWC and tt span coordinates.
- 2026-09-11: Normalized private method name ownership in `src/parser/host.rs`
  and added pass-through and mixed-source regressions in `tests/passthrough.rs`.
- 2026-09-11: Ran formatting, lint, the complete Rust test suite, snapshots, and
  doctests successfully.

## Issues and resolutions

### Issue 1: Private methods named `#match` are claimed as tt matches

- **Symptom**: Valid TypeScript such as `class C { #match(x) { y => y } }`
  fails with `malformed-match` instead of passing through byte for byte.
- **Cause**: The host ownership collector recorded the start of SWC's `#match`
  private-name span, but the tt parser looked up the start of its `match`
  identifier token. The one-byte mismatch discarded valid AST ownership proof.
- **Resolution**: Carry AST declaration-name ranges through host ownership and
  compare them with complete tt identifier spans. Regressions cover ordinary,
  static, async, generator, getter, and setter private methods, plus a real tt
  match nested in the method.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

`src/parser/host.rs` and `src/parser/parse.rs` now represent host ownership as
AST name ranges and compare full parser token spans against them.
`tests/passthrough.rs` fixes the valid TypeScript and mixed-source behavior as
regression contracts.
