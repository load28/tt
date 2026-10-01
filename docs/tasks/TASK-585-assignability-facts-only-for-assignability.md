# TASK-585: Render only TypeScript's assignability diagnostics as type mismatches, about their own subject

> **Superseded in part by TASK-695**: an assignability diagnostic whose
> span TypeScript places on user-written text is no longer rendered as a
> type mismatch; ttc reports TypeScript's code, message, and range. The
> rendering this record describes applies only to a diagnostic whose span
> lands on code ttc generated.

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-30
- **Commit**: `TASK-585: Render only TypeScript's assignability diagnostics as type mismatches`

## Purpose

A `.tt` file with no tt syntax,
`function g(a: number): number { return a; } function f(s: string): string { return s; } export const r = f(g());`,
reported one error, `error[ts2554]: type mismatch: expected string, found number`;
TypeScript's TS2554 "Expected 1 arguments, but got 0." and TS2345 were
gone. A `.ttx` `<Row />` missing required props rendered TS2741 as
``found `(props…) => Element` ``, and `x |> scale()` lost its TS2554. The
same happened in the editor, which shows the compiler's typed check.
`docs/ai/tt.md` renders assignability errors from checker facts; every
other diagnostic keeps TypeScript's words, and every contract says a
TypeScript diagnostic is reported, not replaced.

## Scope

- Included: the backend host's mismatch fact (`contextualMismatch` in
  `src/typescript/host.mjs`), the deduplication of finished diagnostics
  (`finish_diagnostics` in `src/engine/semantics/translate.rs`), the
  receiver of a missing-property diagnostic that replaces the ownership the
  old fact gave by accident (`host.mjs`, `src/typescript/backend.rs`,
  `native.rs`, `src/engine/projection.rs`), `docs/ai/tt.md`, tests.
- Excluded: the message renderer (`diagnostic_message`) and the pipeline
  step wording, which already render the fact they are given; the
  editor's language-service diagnostics, which never carried the fact.

## Decisions

### Decision 1: Compute the fact only for TypeScript's assignability diagnostics

- **Context**: `contextualMismatch` ran for every semantic diagnostic and
  attached the first expression around its span whose type does not fit
  its contextual type; `diagnostic_message` renders any diagnostic with a
  fact as a mismatch.
- **Alternatives considered**: (a) Check the code in the renderer: the
  fact would still move the span and the ownership. (b) Compare the fact's
  types with the types quoted in the message: reading the checker's prose
  back, which the renderer exists to avoid.
- **Decision and rationale**: The host computes the fact only for the codes
  whose message is a head message of TypeScript's assignability relation:
  1360, 2322, 2345, 2375, 2379, 2412, 2418, 2559, 2560, 2719, 2739, 2740,
  2741, 2820. Excess-property (2353, 2561), comparability (2352, 2678),
  overload (2769) and type-argument constraint (2344) diagnostics do not
  relate an expression's type to its context.

### Decision 2: The subject is the expression the error node stands for

- **Context**: Even for an assignability code the search took any
  expression near the span: for `<Row />` TypeScript reports TS2741 at the
  tag name, and `getContextualType` of a tag name answers the attributes'
  contextual type, so the tag's function type was reported as found.
- **Alternatives considered**: Keep the search and exclude tag names: a
  rule for one shape.
- **Decision and rationale**: TypeScript reports the relation at an error
  node that is the expression, or stands for it. Among the nodes starting
  at the diagnostic, innermost first: the name of a declaration, property
  or binding stands for its initializer, a `return` statement for its
  expression, an assignment target for the assigned value, a JSX tag name
  for the element's attributes, and a JSX attribute's name for the
  attribute (TypeScript types the name as the attribute); otherwise an
  expression spanning the diagnostic exactly is itself. No fact is
  attached when the checker gives no type for the subject: the TypeScript
  7 API answers `any` for JSX attributes, so `<Row />` keeps TypeScript's
  own TS2741 sentence.

### Decision 3: Merge only diagnostics identical in code as well

- **Context**: `finish_diagnostics` merged diagnostics with the same span
  and message whatever their code, so a TS2554 rewritten to TS2345's
  words disappeared into it.
- **Decision and rationale**: The code is part of the key; the sort
  includes it so equal diagnostics stay adjacent. `docs/ai/tt.md` says so.

### Decision 4: Own a missing property by the value it is missing from

- **Context**: `proven_statement_and_tuple_errors_own_only_their_checker_cascades`
  failed: the TS2339s a non-diverging let-else causes at `brand` and
  `last4` in the lowered `const { brand, last4 } = $tt_t0;` had been owned
  by the let-else only because the old search attached a fact about the
  glue `$tt_t0` to them.
- **Alternatives considered**: Own every exactly mapped diagnostic inside
  a tt error's owner extent: an independent error in a match arm or in the
  let-else scrutinee would be hidden.
- **Decision and rationale**: For TS2339/TS2551 the host reports the
  receiver of the lookup (the object of a property access, or the value an
  object binding pattern destructures), and the tt-cause ownership check
  also maps that range: the consequence is owned where the value it is
  about is the lowering's glue.

## Work log

- 2026-09-29: Reproduced on `claude/ecstatic-dijkstra-qw5pf9` with the
  three sources above (`target/repro/r3`).
- 2026-09-29: Rewrote `contextualMismatch` (codes and subject roles).
  `cli_reports_every_practical_diagnostic_at_its_source` then failed: the
  API gives `any` for a JSX attribute's string initializer, while the old
  search had used the attribute name; the name is now the attribute's
  subject (Decision 2).
- 2026-09-30: Added the receiver (Decision 4) after the native ownership
  test failed; the container restarted in between, and the work resumed
  from the committed TASK-584.
- 2026-09-30: Changed `finish_diagnostics` and its unit test
  (`diagnostics_are_source_sorted_and_only_identical_duplicates_are_merged`).
- 2026-09-30: Added `types_renders_only_assignability_reports_as_type_mismatches`
  (`tests/cli.rs`) and
  `an_editor_check_keeps_each_typescript_diagnostic_in_its_own_words`
  (`tests/native/cases_09.rs`, the server's `typedCheck`). Both fail with
  the previous host.

## Issues and resolutions

### Issue 1: A JSX attribute's initializer has no type through the API

- **Symptom**: The `dashboard` practical-diagnostics baseline lost its
  `type mismatch: expected number, found string` for `count="many"`.
- **Cause**: `getTypeAtLocation` on the attribute's string literal answers
  `any` in the TypeScript 7 API.
- **Resolution**: The attribute's name is the subject (Decision 2).

### Issue 2: Let-else cascades reappeared

- **Symptom**: TS2339 for the bindings of a non-diverging let-else.
- **Cause and resolution**: Decision 4.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native --test cli --test practical_diagnostics --test snapshot --test content_mapper --lib`
- [x] `cd editors/vscode && npm run compile && node --test "server/out/test/*.test.js" "client/out/test/*.test.js"`
- [x] The new tests fail without the host change.

## Result

Changed `src/typescript/host.mjs`, `src/typescript/backend.rs`,
`src/typescript/native.rs`, `src/engine/projection.rs`,
`src/engine/semantics/translate.rs`, `src/engine/semantics/tests.rs`,
`docs/ai/tt.md`, `tests/cli.rs`, `tests/native/cases_09.rs`,
`docs/tasks/INDEX.md`, and this record.
