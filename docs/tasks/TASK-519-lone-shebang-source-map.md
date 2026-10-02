# TASK-519: Keep a lone shebang on the first line of its source map

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: see `git log --grep TASK-519`

## Purpose

`printf '#!/usr/bin/env node' > only.tt; ttc -o o --source-map file only.tt`
wrote a map whose only segment sat on generated line 3 (`;;AAAA`). The
shebang is on generated line 1 of the output, so the map pointed two lines
below it.

## Scope

- Included: the banner placement `write_banner` reports
  (`src/main/build.rs`) for a shebang with no line break after it, and a CLI
  regression test.
- Excluded: where the banner is written, which was already right
  (`#!...\n// @generated ...`), and the map builder
  (`SourceMapRequest::generated_line_offset_at`).

## Decisions

### Decision 1: The banner line follows the shebang in both shebang branches

- **Context**: `BannerPlacement::at_line` is the generated line the banner
  is written at; lines before it did not move. The contract test
  `a_source_map_follows_the_banner_past_a_shebang` holds a shebang on
  generated line 1 of the map. `write_banner` set `at_line = 1` only when the
  file had a second line. For a shebang that runs to the end of the file it
  left `at_line = 0`, so the map shifted the shebang's own line down by the
  banner's line count, although the banner goes after the shebang in that
  branch too.
- **Alternatives considered**: counting the prefix line break as part of
  the shebang line in the map builder. That moves a CLI placement fact into
  the library's map builder, which only knows the offset it is given.
- **Decision and rationale**: a shebang always keeps line 0, so any file
  that starts with one reports `at_line = 1`. The lone-shebang branch still
  adds the line break the banner needs. Its `lines` count applies only to
  lines at or after `at_line`, and such a file has none.

## Work log

- 2026-09-29: Reproduced: mappings `;;AAAA`.
- 2026-09-29: Set `at_line = 1` for every shebang in `write_banner`. Added
  `a_source_map_keeps_a_lone_shebang_on_the_first_line` to `tests/cli.rs`
  (mappings are exactly `AAAA`; the output starts with the shebang and then
  the banner). The existing banner and shebang tests pass unchanged.

## Issues and resolutions

### Issue 1: The map shifted a lone shebang below the banner

- **Symptom**: the map's only segment was on generated line 3.
- **Cause**: `write_banner` left `at_line = 0` in its `prefix_newline`
  branch.
- **Resolution**: Decision 1.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --test cli`
- [x] `node scripts/check-task-index`

## Result

Changed `src/main/build.rs` and `tests/cli.rs`.
