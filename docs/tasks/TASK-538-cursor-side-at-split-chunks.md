# TASK-538: Keep a cursor's side where lowering splits touching source text

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

Completion right after `s.ki` in `{ a: s.ki, b: match (s) {...} }` offered
the global scope instead of `kind`, and hover and definition at the end of
`helper` in `[helper, match (s) {...}]` answered nothing. One character
earlier both worked. TypeScript answers at the end of the name being typed.

## Scope

- Included: Source-to-service cursor mapping (`src/typescript/mapper.rs`,
  `to_service` and the new `to_service_typed` in
  `src/engine/language/service.rs`) and which of the two each language
  feature uses (`src/engine/language/project.rs`).
- Excluded: The completion probe's placement (`build_probe`), which maps the
  spliced placeholder rather than a cursor between two chunks.

## Decisions

### Decision 1: A cursor names a side, and the chunk on that side wins

- **Context**: The operand before a hoisted `match` is emitted ahead of the
  construct (`const $tt_v2 = (s.ki);`), while the text after it (`, `)
  stays in place. `to_output_inclusive` let the chunk starting at the
  cursor win, so the cursor at the end of `s.ki` landed before `, ` in
  the output, next to glue. Its test covered only chunks that also touch in
  the output, where both sides give the same position.
- **Alternatives considered**: Always preferring the earlier chunk breaks a
  cursor at the start of a name that follows a split, the mirror case.
- **Decision and rationale**: `cursor_to_output` takes an `Affinity`.
  Completion and signature help ask about what is typed before the cursor
  (`to_service_typed`, `Preceding`). Hover, navigation, references and
  rename ask about the name the cursor touches; TypeScript resolves that as
  the token starting at the position when it is a name, else the one ending
  there (`getTouchingPropertyName`), so `to_service` takes `Following` when
  the source byte at the cursor starts an identifier (the lexer's identifier
  class, or `#`) and `Preceding` otherwise. When only one side is mapped,
  that side answers, as before.

## Work log

- 2026-09-29: Reproduced with the editor probe's `q3` cases (object
  literal, binary operand, array element). Added `Affinity`,
  `cursor_to_output`, `to_service_typed`, and moved completion, completion
  resolve and signature help to it. Probed: completion after both `s.ki`
  answers `kind`, and hover and definition at the end and the start of
  `helper` answer the function.
- 2026-09-29: Added `a_cursor_between_chunks_split_in_the_output_keeps_its_side`
  (`src/engine/language/tests.rs`) and
  `an_operand_hoisted_ahead_of_a_later_match_still_answers_at_its_end`
  (`tests/native/cases_06.rs`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `editors/vscode`: all server and client tests, 225 passed

## Result

Changed `src/typescript/mapper.rs`, `src/engine/language/service.rs`,
`src/engine/language/project.rs`, `src/engine/language/tests.rs` and
`tests/native/cases_06.rs`. Completion, signature help, hover and navigation
answer at the end of an operand that lowering hoisted apart from the text
after it.
