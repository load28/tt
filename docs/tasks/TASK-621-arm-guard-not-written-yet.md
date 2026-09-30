# TASK-621: Read an arm whose guard is not written yet as a malformed arm

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-621`

## Purpose

A match arm whose `if` has nothing after it (`B if`, `B if `, `B(x) if`,
`(B, _) if`, `"b" if`) stopped the compiler with an internal compiler
error, `byte range starts at 95 but ends at 94`, raised by
`parse_arm_tail` in `src/parser/matches.rs`. The editor emits the file on
every keystroke (`--emit-map`), so typing a guard crashed the feed at the
moment `if` was typed.

## Scope

- Included: the guard span the open arm grammar (TASK-605) computes, a
  diagnostic regression test, and a typing-simulation test over every
  prefix of tag, tuple, and literal arms with and without guards.
- Excluded: what an arm with an empty guard means. It stays what it was
  before TASK-605 for `B if => 1`: an arm that fails the arm grammar,
  reported as `malformed-match`.

## Decisions

### Decision 1: A guard that runs to the end spans its own tokens

- **Context**: In the open grammar, a guard with no `=>` after it runs to
  the end of the arm's tokens (`guard_runs_to_end`). Its start was the
  first token after `if` (or the region end when there is none,
  `Cursor::stop_byte_at`), and its end was the end of the *last token of
  the region*, which is the `if` itself when the guard is empty. The start
  was then past the end, and slicing the source panicked before the
  existing empty-guard check could reject the arm.
- **Alternatives considered**: (a) Clamp the end to the start. That hides
  the fact that the two bounds were taken from different token ranges.
  (b) Check for an empty token range before computing the span. A second
  rule for a case the span already expresses.
  (c) Take the end from the guard's own tokens, `tokens[idx..]`, as the
  closed guard takes its end from the `=>` that follows its tokens.
- **Decision and rationale**: (c). Both bounds are now read from the same
  token range, so an empty guard has an empty span at its start, and the
  existing empty-guard rule (`src[g_start..g_end].trim().is_empty()`)
  rejects it as it rejects `B if => 1`.

### Decision 2: An arm with an empty guard is `malformed-match`, not `missing-arm-body`

- **Context**: TASK-605 claims an arm whose guard or `=>` is written but
  whose body is not, with `missing-arm-body`. An arm with `if` and nothing
  after it has neither a guard nor a body.
- **Alternatives considered**: (a) Claim it with a missing guard and a
  missing body. That needs a missing-guard form through AST, HIR, Core IR,
  and codegen for a state that lasts one keystroke, and the emitted test
  would have to invent a condition. (b) Keep it malformed.
- **Decision and rationale**: (b). TypeScript's twin, `if ()`, is a syntax
  error too, and `B if => 1` was already malformed. The projection's arm
  recovery erases the arm as it does any other malformed arm, and the arm
  is claimed again with `missing-arm-body` as soon as the guard has a
  token.

## Work log

- 2026-09-30: Reproduced the internal compiler error for `B if`, `B if `,
  `B(x) if`, `(B, _) if`, and `"b" if` with `ttc a.tt` (exit 101,
  `src/parser/matches.rs:877`).
- 2026-09-30: `src/parser/matches.rs` (`parse_arm_tail`): the open guard's
  end is the end of `cur.tokens[cur.idx..]`. The five forms now report
  `malformed-match` (exit 1).
- 2026-09-30: Tests `every_prefix_of_an_arm_being_typed_is_served`
  (`tests/emit_map.rs`: every prefix of seven arms, before `};`, before
  `, _ => 0 };`, and at the end of the file, through `emit_mapped` with the
  mapping invariants and through `compile`; fails with the old end) and
  `an_arm_whose_guard_is_not_written_yet_is_a_malformed_arm`
  (`tests/compile/cases_05.rs`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/parser/matches.rs`, `tests/emit_map.rs`,
`tests/compile/cases_05.rs`, `docs/tasks/INDEX.md`, and this record. An
arm whose guard is not written yet is reported as `malformed-match`
instead of stopping the compiler.
