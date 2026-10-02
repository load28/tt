# TASK-628: Claim a match whose arms reach a guard before any `=>`

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-628`

## Purpose

A match whose only arm is a guarded arm with no `=>` yet,
`match (o) { A(n) if n > 0 }`, was not claimed. The file was read as
TypeScript and reported `source-not-typescript` (`Expected ';', got '{'`)
at the body, and nothing else in the file; with a second arm and a `=>`
after it, the same arm was claimed with `missing-arm-body` (TASK-605).

## Scope

- Included: when a match body reads as arms (`src/parser/matches.rs`),
  a case file, a pass-through test, `docs/ai/tt.md`, and
  `docs/design/lsp-architecture.md`.
- Excluded: how a claimed arm with no body is lowered and reported
  (TASK-605), and an arm whose guard is empty (TASK-621), which the claim
  now reaches and reports as `malformed-match` as it does in a longer arm
  list.

## Decisions

### Decision 1: A guard `if` no statement list can hold makes the body arms

- **Context**: TASK-605 claims an arm with a missing body only where the
  body already reads as arms (`body_reads_as_arms`, TASK-229): a `=>` at
  the body's level before any `;`. The rule exists because `match (x) { … }`
  is also a method named `match`, whose body is a statement list, and
  AGENTS.md contract 1 forbids claiming valid TypeScript. With no `=>` in
  the body, the rule refused every arm list.
- **Alternatives considered**: (a) Treat any `if` after a pattern as a
  guard. A method body `A(n)`, a line break, `if (n > 0) g()` is valid
  TypeScript and would be claimed. (b) Claim only a single arm. The same
  question arises for every arm before the first `=>`, and a second
  guarded arm (`A(n) if n > 0, B if c`) would stay unclaimed. (c) Read the
  body as arms when it opens with comma-separated patterns and one of them
  reaches an `if` that TypeScript cannot read as the start of a statement:
  one on the pattern's line (ECMA-262 §12.10.1: no semicolon is inserted
  before a token that follows the previous one on the same line, unless
  that token is `}` or the `)` of a `do`-`while`), or one not followed by
  `(` (§14.6: `if ( Expression ) Statement`). A pattern never ends with a
  statement keyword (tag names exclude reserved words) or with the `)` of
  a statement head, so the `if` after it on its line can only be a guard.
  A pattern that ends with `}` (`is E { code }`) could close a block, so
  it takes the second test only.
- **Decision and rationale**: (c), `guard_outside_statements`, asked
  where the body ends with no `=>`. It is a fact of TypeScript's grammar,
  not of the arm's text, and it covers tag, tuple, literal, and `is`
  patterns and any arm of the list. A body it admits then takes the open
  arm grammar of TASK-605, so the arm is claimed with `missing-arm-body`,
  its pattern binds, and its guard is served.

## Work log

- 2026-09-30: No repro existed under `target/probe6-compiler`; wrote
  `tests/cases/compiler/onlyArmGuardWithoutBody.tt` (a tag, a tuple, a
  literal alternative, two guarded arms, and a guard on the next line)
  and generated its baselines on the unfixed code: `source-not-typescript`
  at the first body, nothing emitted.
- 2026-09-30: `src/parser/matches.rs`: `body_reads_as_arms` takes the
  cursor and asks `guard_outside_statements` where the body ends with no
  `=>`.
- 2026-09-30: `tests/passthrough.rs`:
  `method_named_match_whose_statements_read_as_a_guarded_arm` (methods of
  a class and an object literal whose statements are a call, a line
  break, and a parenthesized `if`) stays byte-identical.
- 2026-09-30: `cargo test --test compile --test emit_map --test snapshot
  --test passthrough --test case_baselines`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/onlyArmGuardWithoutBody.tt`
  (`cargo test --test case_baselines`)
- **Observed failure**: without the change to `src/parser/matches.rs`, the
  case reported `error[source-not-typescript]: the TypeScript here does not
  parse: Expected ';', got '{'` at `onlyArmGuardWithoutBody.tt:4:20` for
  both `ttc --out-dir` and `ttc --check-types`, where the baseline has
  `missing-arm-body` for each arm and `match-not-exhaustive` for each
  variant match.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Baseline changes reviewed and committed with the change

## Result

Changed `src/parser/matches.rs`, `tests/passthrough.rs`, `docs/ai/tt.md`,
and `docs/design/lsp-architecture.md`; added
`tests/cases/compiler/onlyArmGuardWithoutBody.tt` and its baselines. A
match whose arms reach a guard before any `=>` is claimed and reports
`missing-arm-body`.
