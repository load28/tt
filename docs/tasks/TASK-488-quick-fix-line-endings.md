# TASK-488: Write authored match arms with the file's line ending

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

In a CRLF `.tt` file, the quick fix for a non-exhaustive match whose `}`
stands on its own line wrote `"    Point => undefined,\n"`. Applying it left
the file with mixed line endings. The compiler's own output already follows
the file's line ending (TASK-377, TASK-395), so its edits should too.

## Scope

- Included: The arm-insertion edit in `src/diagnostics/suggestions.rs`
  (both the missing-arms and the `_` suggestions, for tag, tuple, and
  literal matches on both the default and the typed pipelines, which share
  it) and a regression test in `tests/compile/cases_08.rs`.
- Excluded: Other suggestions. Every other edit either replaces text
  inside one line (`try `, a corrected case or field name, a parenthesized
  scrutinee, the removed `default`) or copies source bytes, so none of
  them writes a line break of its own.

## Decisions

### Decision 1: Use the line ending that the emitted TypeScript uses

- **Context**: The edit writes one line per arm. It needed a line
  terminator, and `\n` was hard-coded.
- **Alternatives considered**: (a) Use the terminator of the line that
  holds the closing `}`. It matches the insertion point, but a quick fix
  would then follow a rule that differs from the rest of the compiler's
  output in a mixed file. (b) Use `crate::line_ending(source)`, the rule
  the emitter and the source-map comment already use: the first line's
  terminator.
- **Decision and rationale**: (b). One rule decides every line break the
  compiler writes into a file, so a fix and a rebuild never disagree.

## Work log

- 2026-09-28: Reproduced with a CRLF file whose match has its `}` on its
  own line: the edit ended in `\n`. Checked every `suggest(...)` and
  `Edit` construction for other line breaks; only `insert_arms` writes one.
- 2026-09-28: `insert_arms` now ends each arm line with
  `crate::line_ending(source)`. Added
  `authored_arm_lines_end_with_the_line_ending_of_a_crlf_file`, which
  covers a tag match, a tuple match, and a last arm without a separator.
  It applies each suggestion, asserts every `\n` is preceded by `\r`, and
  asserts the result has no diagnostics.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/diagnostics/suggestions.rs`, `tests/compile/cases_08.rs`, and
the task index. Arm-insertion quick fixes in a CRLF file now write CRLF
lines.
