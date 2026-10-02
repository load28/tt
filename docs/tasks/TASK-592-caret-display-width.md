# TASK-592: Draw diagnostic carets at each character's display width

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-592: Draw diagnostic carets at each character's display width`

## Purpose

The terminal renderer (`src/render.rs`, `display_col`) counted every
character as one column, so a line holding Hangul, CJK or an emoji before
the reported construct drew its carets too far left, and a span over wide
characters drew too few carets.

## Scope

- Included: the renderer's column arithmetic, a render unit test, a
  diagnostic snapshot fixture, and `Cargo.toml`.
- Excluded: the `file:line:col` location and the server's JSON positions,
  which count characters (and UTF-16 units for the editor) and stay as
  they were.

## Decisions

### Decision 1: Display width per UAX #11, through `unicode-width`

- **Context**: A terminal gives an East Asian Wide or Fullwidth character
  two columns and a combining mark none (Unicode Standard Annex #11, "East
  Asian Width"; emoji presentation characters are Wide there). The caret
  row has to use the same widths as the line above it; the location must
  not change.
- **Alternatives considered**: (a) A table of wide ranges in the
  renderer: a copy of Unicode data that would age with each Unicode
  release. (b) Keep counting characters and pad with the character itself
  (write the source's own characters, blanked, under themselves): the
  spaces a terminal draws for a blanked wide character are not defined.
- **Decision and rationale**: `unicode_width::UnicodeWidthChar::width`
  (the crate implements UAX #11 plus the zero width of combining marks,
  as rustc's own diagnostic emitter uses it). The dependency policy in
  `Cargo.toml` keeps the tree small; `unicode-width 0.2.2` is already in
  it through `swc_common` (`cargo tree -i unicode-width`), so the direct
  dependency adds an edge, not a crate (`Cargo.lock` gains one line). A
  character without a defined width (a control character) keeps the one
  column it had; a tab keeps `TAB_WIDTH`.

## Work log

- 2026-09-30: Reproduced with a line `const s = "한글🎉"; Circel(r);`: the
  caret started three columns left of `Circel`.
- 2026-09-30: Changed `display_col`; added
  `wide_characters_take_their_display_width_before_and_under_the_carets`
  (`src/render/tests.rs`, fails with the previous one-column rule) and the
  snapshot fixture `tests/fixtures/diagnostic/wide-characters` (generated
  with `UPDATE_EXPECT=1 cargo test --test snapshot`; the diff was read:
  the carets start under `match`, 48 display columns in, and the location
  stays `7:43`, the JSON `col` 44 in UTF-16 units). No other fixture
  changed.

## Issues and resolutions

None.

## Verification

Run for the TASK-587–592 series, on this commit:

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=3 TTC_REQUIRE_TSGO=1 cargo test` (1692 passed, 0 failed)
- [x] `cd editors/vscode && npm run compile && node --test "server/out/test/*.test.js" "client/out/test/*.test.js"` (222 passed)
- [x] `TTC_BINARY=target/debug/ttc npm --prefix integrations/unplugin test` (18 passed)
- [x] `node --test npm/scripts/*.test.mjs` (53 passed)
- [x] `node scripts/check-task-index`

The first full run found two regressions of the series, fixed in
follow-up commits of their tasks: the TASK-540 watch test against
TASK-588 (TASK-588 Issue 2) and the extension's sidecar refresh around a
recovered expression against TASK-590 (TASK-590 Issue 2).

## Result

Changed `Cargo.toml`, `Cargo.lock`, `src/render.rs`, and
`src/render/tests.rs`; added `tests/fixtures/diagnostic/wide-characters`.
