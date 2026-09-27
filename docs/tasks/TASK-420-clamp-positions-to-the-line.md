# TASK-420: Clamp a position past the end of its line to the line's length

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`ttSymbol` at `{ line: 0, character: 30 }` on a five-character first line
answered with a symbol on line 1. `u16_offset`
(`src/engine/language/service.rs`) added the character to the line's start and
clamped only to the end of the text, so a position past the end of a line
landed on a later line.

LSP 3.17, `Position`: "If the character value is greater than the line length
it defaults back to the line length." The line length does not include the
line terminator.

## Scope

- Included: `u16_offset`. Every protocol position the server receives becomes
  an offset through it: `source_byte` (`ttSymbol`, `ttCompletions`, hover and
  definition fallbacks), `to_service` (every semantic request), completion
  probes, and ranges returned by the TypeScript service.
- Excluded: which characters end a line. The conversions in both directions
  (`u16_offset`, `u16_position`) count `\n` lines, with `\r\n` handled as a
  terminator. Treating a lone `\r` as a line break would change the line
  model in both directions, not only this clamp.

## Decisions

### Decision 1: Clamp inside the one conversion every request uses

- **Context**: The server has one Position → offset conversion.
- **Alternatives considered**: Clamp at each request handler in `src/server.rs`.
  That duplicates the rule and misses engine consumers such as the extension's
  in-process calls and service ranges.
- **Decision and rationale**: `u16_offset` measures the addressed line's
  content in UTF-16 units: the text up to the next `\n`, without a trailing
  `\r`. It returns `line_start + min(character, length)`. A line past the end
  of the text still clamps to the end of the text, which is what the existing
  test pins. The doc comment now states the LSP rule instead of the old
  "spills forward".

## Work log

- 2026-09-27: Reproduced through `ttc --server` `ttSymbol`. Implemented the
  clamp.
- 2026-09-27: Added the unit test
  `a_character_past_the_line_end_defaults_back_to_the_line_length` (CRLF and
  LF lines, a multibyte line, and the last line without a terminator). Added
  `tests/cli/cases_01.rs::a_server_position_past_the_line_end_stays_on_its_line`,
  which drives `ttSymbol` and `ttCompletions` past a line's end. The server
  test fails before the change and passes after it. Corrected the comment in
  `u16_positions_round_trip_over_multibyte_text`, which described a
  line-past-end case as a character spilling forward.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib engine::language::tests` and `cargo test --test cli`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` and `./scripts/ci extension` (the final run at the end of TASK-421)

## Result

A position past the end of a line addresses the end of that line. Changed
`src/engine/language/service.rs`, `src/engine/language/tests.rs`, and
`tests/cli/cases_01.rs`.
