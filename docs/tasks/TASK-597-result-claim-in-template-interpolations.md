# TASK-597: Find a `result` block's `try` inside a template interpolation

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-597`

## Purpose

`export const b = result { return \`x${try r}\`; };` failed with
`source-not-typescript: Expected a semicolon`: the block was not claimed,
so `result { … }` went on as TypeScript, which it is not. A `try` in a
match arm of the same block is claimed and reported as
`try-crosses-value-region`.

## Scope

- Included: the speculative claim scan of `result` blocks
  (`src/parser/results.rs`), `docs/ai/tt.md`, and a regression test.
- Excluded: placement itself. `docs/design/try-result-scopes.md` §4.6
  already lists a template interpolation as an isolated value region
  whose `try` cannot target an enclosing `result` block, and the placement
  check already reports it once the block is claimed.

## Decisions

### Decision 1: Measure function depth by byte offset, descending into template tokens

- **Context**: The claim scan already visits template interpolations in the
  parsed body, but decides whether a `try`'s nearest Result scope is the
  block by its function depth, computed from the index of its token in
  the block's token stream. A template literal is one token whose
  interpolations carry their own token streams (`TplPart::Interp`), so a
  `try` there has no index in that stream and was never counted. The
  tt-owned arrows and braces (a match arm's `=>`, its body's `{`) were
  recorded as indices the same way, so an arm inside an interpolation
  would also have been taken for a user function.
- **Alternatives considered**: (a) Treat every `try` inside a template as
  owned by the block. That ignores an arrow function written in the
  interpolation, which is its own Result scope. (b) Flatten the nested
  streams into one. The depth walk pairs braces within one stream, and an
  interpolation's `}` is not a token of the outer stream. (c) Record
  tt-owned tokens by byte offset and compute a depth at an offset: the
  depth before the template token in the enclosing stream plus the depth
  at the offset inside the interpolation's own stream, recursively.
- **Decision and rationale**: (c) (`function_depth_at`, `tokens_within`).
  The walk is the same `user_function_depth_at` applied at each level, so
  an arrow function in an interpolation still owns its `try`, and a match
  arm's arrow is still not a function. The block is then claimed and the
  existing placement rule reports `try-crosses-value-region`, the same
  outcome as a `try` in a match arm.

## Work log

- 2026-09-30: Reproduced `target/probe5-compiler/repro/r5_try_in_template.tt`
  and the match-arm form `r5b.tt`.
- 2026-09-30: `src/parser/results.rs`: offset-keyed tt-owned tokens and
  `function_depth_at`.
- 2026-09-30: Test
  `a_try_in_a_template_interpolation_claims_its_result_block`
  (`tests/compile/cases_14.rs`): a direct, nested-template, and match-arm
  `try` in an interpolation each give one `try-crosses-value-region` at the
  `try`, and an arrow in an earlier interpolation does not stop a later
  direct `try` from claiming the block.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/parser/results.rs`, `docs/ai/tt.md`,
`tests/compile/cases_14.rs`, `docs/tasks/INDEX.md`, and this record.
