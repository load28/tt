# TASK-424: Measure every reported position in the decoded text of a file with a byte order mark

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

In a `.tt` file that starts with a UTF-8 byte order mark, `ttc
--check-types` and the server's `typedCheck` placed assignability
diagnostics one column early with one caret too many
(`const n: number = "x";` on line 2: `2:18` and `^^^^` instead of `2:19` and
`^^^`). Columns on line 1 counted the byte order mark, and positions in a
hand-written `.ts` file with one disagreed with TypeScript's.

## Scope

- Included: The coordinate convention between tt byte offsets, TypeScript
  offsets, protocol positions and rendered line/column
  (`src/error.rs`, `src/typescript/mapper.rs`,
  `src/engine/language/service.rs`), the text served to TypeScript
  (`src/typescript/native.rs`, `src/typescript/service.rs`), and the
  rendered snippet (`src/render.rs`).
- Excluded: Emitted output keeps its byte order mark (the banner placement in
  `src/main/build.rs` and `tests/compile/cases_10.rs` rely on it). Source map columns
  (`src/source_map.rs`, `src/sidecar.rs`) and the content-mapper entry point
  were not part of the report and are unchanged.

## Decisions

### Decision 1: A leading U+FEFF is not text; positions are measured without it

- **Context**: Three conventions were mixed.
  - TypeScript strips a leading byte order mark when it reads a file
    (`sys.readFile` in TypeScript, and the TypeScript 7 file system), so its
    offsets for a file on disk exclude it. When the host served a `.tt`
    module's text with the mark through the API file-system callback, the
    compiler kept it and its node and diagnostic offsets counted it, while
    the API client's `SourceFile.text` had it stripped. Measured with the
    pinned TypeScript: for `﻿const a = 1;\nconst n: number = "x";\n`
    served through `readFile`, `sourceFile.text.length` is 36 while the
    served text is 37 characters and the diagnostic's `pos`/`end` are
    20/21. `contextualMismatch` in `host.mjs` then computed
    `expression.getStart(sourceFile)` by scanning the stripped text from a
    position that counted the mark, so the mismatch started one unit early
    while its end (`getEnd`) did not move — the extra caret.
  - `mapper::to_utf16`/`from_utf16`, `u16_offset`/`u16_position` and
    `line_col` counted the mark as a character on line 1.
  - An editor holds the document without the mark (the LSP 3.17 positions a
    client sends are in that text).
- **Alternatives considered**:
  - Correct `getStart` in `host.mjs` by one when the file has a mark. That
    special-cases one symptom, and the served offsets would still disagree
    with the client's own text for every other API query.
  - Strip the mark from the source when it is read. The emitted file would
    lose the mark the source had, which contract 1 (bytes pass through)
    forbids.
- **Decision and rationale**: One convention, the one TypeScript and
  editors already use: byte offsets stay offsets into the text as given, and
  everything measured for a person or a protocol — UTF-16 offsets,
  line/character, line/column, the rendered snippet — is measured in the
  decoded text (`error::decoded`), which is the text without a leading
  U+FEFF (Unicode §23.8 treats U+FEFF at the start of a stream as the
  encoding signature). The conversions that cross between the spaces apply
  it in one place each: `line_col`, `utf16_column`, `mapper::to_utf16`,
  `mapper::from_utf16`, `u16_offset` and `u16_position`. The text served to
  TypeScript, through the API host and through the language service, is
  the decoded text, which is what TypeScript's own file reading yields, so
  its offsets and the API client's text agree and `getStart` is correct
  without any change to `host.mjs`.

## Work log

- 2026-09-27: Reproduced with the repository TypeScript: with the mark,
  `--check-types` reported `2:18` with four carets, and a line-1 error
  rendered two carets. Measured the API behavior with a standalone script
  against the pinned client (file on disk versus text served through
  `readFile`).
- 2026-09-27: Added `error::decoded` and `error::signature_len`, applied
  them in the conversions listed in Decision 1, served decoded text to
  TypeScript, and rendered decoded lines.
- 2026-09-27: Added `positions_are_measured_in_the_decoded_text`
  (`src/error.rs`), `utf16_offsets_are_measured_in_the_decoded_text`
  (`src/typescript/mapper.rs`),
  `protocol_positions_are_measured_in_the_decoded_text`
  (`src/engine/language/tests.rs`), and
  `a_byte_order_mark_moves_no_reported_position`
  (`tests/native/cases_03.rs`), which requires the `--check-types` report
  of a project to be identical with and without marks on its `.tt` and
  `.ts` files.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.
- [x] With the source changes reverted, all four new tests fail (the native
  one shows `2:18` with `^^^^` and a two-caret line-1 snippet); with them
  they pass.
- [x] `ttc --server`: `typedCheck` reports `b.tt` 1:19 and 2:19 and
  `h.ts` 1:26 with and without marks; `definition` into a marked
  `decl.tt` lands on character 13 of line 0.

## Result

Changed `src/error.rs`, `src/render.rs`, `src/typescript/mapper.rs`,
`src/typescript/native.rs`, `src/typescript/service.rs`,
`src/engine/language/service.rs`, `src/engine/language/tests.rs` and
`tests/native/cases_03.rs`. A byte order mark no longer moves any position
the CLI, the server or the language service reports.
