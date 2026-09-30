# TASK-666: Decide three reported behaviours: let-else placement, the `if let` head, and payload reads

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-666`

## Purpose

A probe reported three behaviours without deciding whether each is a
defect: (a) a let-else whose `else` only `throw`s is rejected inside a
`match` arm, with a message about `return`/`break`/`continue`; (b)
`if let P = x as { ... } {` does not parse; (c) a nested pattern reads a
payload getter once per arm test (four reads in the probe's example). Each
is decided here, fixed where it is a defect, and recorded in
`docs/ai/tt.md` either way. This record also holds the full gate over
TASK-661 to TASK-666.

## Scope

- Included: the `if let` head scan (`src/parser/iflets.rs`), a case under
  `tests/cases/compiler/`, and `docs/ai/tt.md` for all three.
- Excluded: changing let-else placement or the nested-pattern emission.

## Sources

- `docs/ai/tt.md`, "let-else": "Position limits same as try"; `src/sema.rs`
  and `src/sema/checker.rs` (`check_let_else`): placement is the
  `in_function` flow fact of the statement's region, as for `try`.
- `docs/ai/tt.md`, "match": "A block arm's direct `return` delivers the
  match value" and "A `break`, `continue`, or `yield` may target only
  control flow written inside that arm".
- `src/lexer/facts.rs` and `src/lexer/facts/expressions.rs`: the token
  facts are the lexer's one model of where an expression ends
  (`ends_expression`: "a value expression, or a type in a type position,
  is whole after it"), and the `if let` scrutinee frame already ends at "a
  `{` after an operand" (`ExprCfg::brace_ends`).
- TypeScript grammar (`src/compiler/parser.ts`, `parseBinaryExpressionRest`
  for `as`/`satisfies`): the operator is followed by a type, so a `{`
  directly after it opens an object type literal, never a block.
- TypeScript `src/compiler/checker.ts`, `isMatchingReference` and
  `getFlowTypeOfReference`: narrowing by `x.value.kind === "Some"` treats
  the property access path `x.value` as one reference read wherever it
  appears; getters are not modelled.

## Decisions

### Decision 1: (a) is by design, and the guide now says so

- **Context**: A throw-only `else` would behave the same in an arm as in a
  function, so allowing it is possible.
- **Alternatives considered**: (a) Allow a let-else in a value region when
  its `else` has no `return`, and no `break`/`continue` leaving it. Its
  validity would then depend on the `else` block's contents: adding a
  `return` to the `else` would move the error to the statement's head, and
  the rule would differ from `try`'s, which the guide says it shares. (b)
  Keep the positional rule and document that it applies to a throw-only
  `else` too.
- **Decision and rationale**: (b). Placement is a fact about where the
  statement stands, shared with `try`; the message names the exits that
  make the position unsound in general. Recorded in `docs/ai/tt.md`.

### Decision 2: (b) is a defect, fixed in the parser with the lexer's expression-end fact

- **Context**: `expr_until_block` took the first top-level `{` as the body,
  so the object type after `as` became the body and the rest failed to
  parse (`source-not-typescript`, or `stray-if-let` for an object literal
  head), although TypeScript's grammar leaves no other reading.
- **Alternatives considered**: (a) Document "parenthesize the head", as
  Rust does for struct literals. The head is TypeScript, and TypeScript
  decides this `{` without parentheses. (b) Special-case `as {` and
  `satisfies {`. Misses `A | { ... }`, `A & { ... }`, and object literals.
  (c) Take the body at the first `{` that follows a token completing an
  expression, the token fact the lexer's facts machine already uses for
  the same frame.
- **Decision and rationale**: (c). A `{` after a token that does not end
  an expression (`as`, `|`, `&`, `=`, the head's start) is a bracket of
  the expression and is skipped whole. The block-bodied arrow rule is
  unchanged (`=> {` still asks for parentheses).

### Decision 3: (c) is by design, and the guide now says so

- **Context**: The match reads `$tt_m.value` in each arm's test and again
  to bind.
- **Alternatives considered**: (a) Read each nested payload once into a
  temporary per match. Every nested-pattern output changes, and the typed
  exhaustiveness pass asks the checker at the receiver it writes
  (`PayloadTemp`), which would move. (b) Keep the per-test reads: a
  pattern is not a source expression the match evaluates, the scrutinee
  is evaluated once as documented, and the reads are the ones a
  hand-written `if` chain over a `kind` union makes, whose narrowing
  TypeScript computes by treating the path as one reference.
- **Decision and rationale**: (b), with the guide stating it and how to
  read a getter-backed payload once (bind it, then `match` the binding).

## Work log

- 2026-09-30: Reproduced (a) (`let-else-placement` at the arm's let-else),
  (b) (`source-not-typescript` at the `{` after `as`), and (c) (three reads
  of a getter payload in a three-test match).
- 2026-09-30: Changed `expr_until_block` in `src/parser/iflets.rs`; added
  `tests/cases/compiler/ifLetHeadEndingInObjectType.tt` with `@run`;
  documented (a), (b), and (c) in `docs/ai/tt.md`.
- 2026-09-30: Ran the full gate over TASK-661 to TASK-666 (Verification).

## Issues and resolutions

### Issue 1: A `match` or `result` head lost its body brace

- **Symptom**: The full gate failed
  `tests/compile.rs::a_value_inside_a_let_else_or_if_let_subject_is_lowered_once`:
  `if let Some(value: v) = match (k) { ... } { ... }` became
  `stray-if-let`.
- **Cause**: The head scan skips a whole `match (...) { ... }` or
  `result { ... }` as one operand, but the token facts do not mark that
  construct's closing `}` as ending an expression, so the next `{` was
  taken for part of the head.
- **Resolution**: A skipped tt construct counts as a complete operand for
  the brace that follows it.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/ifLetHeadEndingInObjectType.tt`
  (`cargo test --test case_baselines`)
- **Observed failure**: `error[stray-if-let]: \`if let\` could not be
  parsed here` at the `satisfies { ... } {` head in both sections of an
  unexpected `.errors.txt`; nothing was emitted or run.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`: `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1
  TTC_REQUIRE_TYPESCRIPT_CASES=1 TT_REQUIRE_EXTENSION=1
  TT_BASELINE_TRACKING_DIR=<dir> cargo test --no-fail-fast` over TASK-661
  to TASK-666 (Issue 1 was its one failure; `tests/compile.rs` then passed
  579/579), and a second full run after that fix
- [x] `node scripts/check-baselines --tracking <dir>`: 292 compared, none
  unused
- [x] Extension suite (`node --test server/out/test/*.test.js
  client/out/test/*.test.js` with `target/debug` on `PATH`): 232 passed,
  no `SKIP no ttc`/`SKIP no tsgo`
- [x] `./scripts/ci agents`: passed
- [x] Baseline changes reviewed and committed with the change

## Result

`if let` heads that end in an object type or are object literals parse;
let-else placement and nested payload reads are documented as designed.
Changed: `src/parser/iflets.rs`, `docs/ai/tt.md`, the case and its
baselines.
