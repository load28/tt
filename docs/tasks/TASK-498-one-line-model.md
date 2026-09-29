# TASK-498: Count lines once, under the line breaks each consumer speaks

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

Line and column counting only counted LF, so files that end lines with CR,
U+2028 or U+2029 reported wrong positions in the CLI and the editor.
`export {};\rval const b = { c: 1 };\rb.c = 2;\r` rendered `cr.tt:1:36`
under `ttc -p`, and `ttc --server` `check` answered line 1, column 36; the
mutation is on line 3, column 1. About three dozen sites each counted lines
their own way.

## Scope

- Included: one line model (`src/lines.rs`) with an explicit line-break
  policy, and every consumer that turns a byte into a line and column, or
  back, or asks where a line starts or ends: compile errors and
  `ttc::line_col`, the CLI renderer, the typed engine's diagnostics, the
  `--server` protocol (`check`, `typedCheck`, labels, suggestion edits,
  blocked files), the engine's LSP adapter to the TypeScript language
  server, semantic tokens, tt symbols, completions and hints, source maps,
  sidecar declaration maps, the build banner after a `#!` line, quick-fix
  and lowering indentation lookups, variant-body comment layout, the
  printer's line-start and line-end questions, `line_ending`, and the VS
  Code server's one-shot fallback conversion (`protocolPosition`). Merge
  conflict marker detection now accepts every ECMA-262 line terminator
  before a marker, as TypeScript's scanner does.
- Excluded: the content mapper, which speaks byte offsets (`utf-8`
  position encoding) and has no line model; its behavior is pinned by a new
  test. Lexical scanning of unterminated string literals
  (`scanner::scan_string` stops at LF only) is a lexer question, not a
  position one.

## Decisions

### Decision 1: Two line-break policies, named by the consumer

- **Context**: ECMA-262 (§12.3 `LineTerminatorSequence`) ends a line at
  LF, CR, CR LF, U+2028 and U+2029. LSP 3.17 ("Text Documents") says: "the
  protocol specifies the following end-of-line sequences: '\n', '\r\n' and
  '\r'", and "If the character value is greater than the line length it
  defaults back to the line length." The pinned TypeScript
  (`7.1.0-dev.20260826.1`) was checked directly: `tsc --noEmit` reports a
  type error on the third line of a CR-only file and of a file whose lines
  end in U+2028/U+2029 as `(3,7)`, and its binary carries both
  `core.ComputeECMALineStarts` (diagnostics, `sourcemap.ECMALineInfo`) and
  `lsconv.LSPLineMap` (the language server). ECMA-426 §11.1.2.1 splits a
  generated file's lines at "\u000D\u000A", "\u000A", "\u000D", "\u2028",
  "\u2029" — "matches the LineTerminatorSequence production" — and counts
  columns in UTF-16 code units.
- **Alternatives considered**: (a) Adopt ECMA-262 everywhere — editors would
  see a line break that their buffer does not have, and every position after
  U+2028 would be off by lines. (b) Adopt LSP everywhere — CLI output would
  disagree with `tsc` and source maps with every consumer. (c) Keep per-site
  counters and patch each — the defect exists because there were many.
- **Decision and rationale**: `LineBreaks::{Ecma, Lsp}`, chosen by the
  consumer. ECMA for compile errors, `ttc::line_col`, the engine's
  `Diagnostic` positions and the CLI renderer (matching `tsc`), source maps
  and sidecar maps (ECMA-426), `#!` line ends, and indentation lookups. LSP
  for everything an editor or the TypeScript language server reads. CR LF is
  one break under both, so CRLF files keep their positions exactly.

### Decision 2: One module owns lines; conversions go through the byte

- **Context**: The engine's `Diagnostic` is public and carries the
  compiler's coordinates (1-based ECMA line, code-point column), and the
  server needs protocol coordinates for the same place.
- **Alternatives considered**: change `engine::Diagnostic` to carry byte
  offsets — a larger public API change, and its labels point into other
  files whose text the renderer does not always hold; or keep converting
  only the column (`utf16_column`), which cannot move a position to another
  line.
- **Decision and rationale**: `src/lines.rs` measures a text once
  (`LineMap`: line starts and line ends under a policy) and answers
  byte ↔ (line, UTF-16 column) and byte ↔ (line, code-point column).
  `ProtocolPositions` converts a compiler position to a protocol one by
  resolving it to its byte under ECMA and re-measuring under LSP. Terminators
  are recognized through `scanner::line_break_end`/`line_terminator_len`,
  so the ECMA-262 set keeps one owner and multibyte text stays opaque.
  `error::line_col`, `error::utf16_column` (and its public `ttc::utf16_column`
  wrapper), `source_map::LineTable`, `engine::tokens::LineIndex`, the
  service's hand-rolled `u16_offset`/`u16_position` counters, the sidecar's
  `utf16_column`, and `server::protocol_position` are gone.

### Decision 3: A byte-order mark is a signature, not a column

- **Context**: Compiler positions already skipped a leading U+FEFF; source
  maps counted it as a column on the first line.
- **Decision and rationale**: every `LineMap` starts line 0 after the mark.
  Node and browsers strip the mark when they decode a script, so a source
  map column that counts it points one unit to the right.

### Decision 4: A file's line ending is its first protocol line break

- **Context**: With CR recognized, a quick fix that authors arms on their
  own lines in a CR-only file started writing LF lines into it, because
  `line_ending` answered `"\n"` for anything but CR LF; the printer's
  line-start (`line_indent`) and line-end (`ends_line`) questions also
  looked for LF only.
- **Decision and rationale**: `line_ending` answers the first LF, CR LF or
  CR of the file (LSP's set: U+2028/U+2029 end ECMAScript lines but are not
  a line-ending style). A CR-only file now emits and authors CR lines with
  the same layout as its LF form; CR LF files are unchanged.

## Work log

- 2026-09-28: Checked the LSP 3.17 and ECMA-426 texts and the pinned `tsc`
  (CR, U+2028/U+2029 and mixed files all report `(3,7)`); listed the
  `'\n'`-based sites with `grep`.
- 2026-09-28: Added `src/lines.rs` (`LineBreaks`, `LineMap`,
  `ProtocolPositions`, `line_col`, `line_start_before`,
  `ends_with_line_break`) and `src/lines/tests.rs`; exported it as
  `ttc::lines`.
- 2026-09-28: Routed every consumer through it: `src/lib/mapped.rs`,
  `src/diagnostics.rs`, `src/render.rs`, `src/server.rs`,
  `src/source_map.rs`, `src/sidecar.rs`, `src/engine/tokens.rs`,
  `src/engine/language/service.rs`, `src/engine/names.rs` and
  `src/engine/completions.rs` (test helpers),
  `src/diagnostics/suggestions.rs`,
  `src/codegen/core/emitter/{source,result}.rs`, `src/codegen/rope.rs`,
  `src/parser/variants.rs`, `src/lexer/validation.rs`, `src/main/build.rs`,
  `src/lib/api.rs`; removed the counters in `src/error.rs`.
- 2026-09-28: Replaced the VS Code server's `utf16Column` with
  `protocolPosition` (`editors/vscode/server/src/ttc.ts`), which converts
  line and column; noted the reversal at the top of TASK-400.
- 2026-09-28: Added regression tests at each public boundary (see
  Verification) and documented the model in
  `docs/design/compiler-architecture.md`.

## Issues and resolutions

### Issue 1: A CR shebang pushed the banner to the end of the file

- **Symptom**: `write_banner` looked for the shebang's end with
  `find('\n')`; in a CR-only file it found none and treated the whole file
  as the `#!` line.
- **Cause**: another LF-only line question.
- **Resolution**: the banner follows line 0 of the ECMA `LineMap`; tested
  for CR, U+2028 and CR LF shebangs together with the map's line shift.

### Issue 2: Typed diagnostics are reported for every project file

- **Symptom**: the new server test compared positions with the first
  `ts2322` in a `typedCheck` answer, which belonged to another file.
- **Cause**: `typedCheck` returns the whole project's diagnostics.
- **Resolution**: the test filters by `path`.

## Verification

- [x] `cargo fmt --check` (exit 0)
- [x] `cargo clippy --all-targets -- -D warnings` (exit 0)
- [x] `TTC_REQUIRE_TSGO=1 cargo test` (exit 0; no snapshot changed)
- [x] `./scripts/ci extension` (exit 0, 209 extension tests)

New tests: `src/lines/tests.rs` (LF, CR LF, CR, U+2028, U+2029, mixed, BOM,
astral, clamping, protocol conversion, backward walk agreement);
`tests/cli/line_breaks.rs` (CLI rendering for every style; banner and map
after CR, U+2028 and CR LF shebangs); `tests/native/cases_05.rs`
(`--check-types` rendering, and server `typedCheck`, `check`, suggestion
edits, `definition` and `hover` for every style);
`tests/content_mapper.rs` (checker lines through the mapper);
`src/source_map.rs` and `tests/sidecar.rs` (map lines and columns on both
sides); `src/engine/tokens.rs` and `src/engine/names.rs` (protocol
positions); `tests/compile/cases_08.rs` (CR quick fix and CR emission
layout); `tests/compile/cases_09.rs` (conflict markers after U+2028/U+2029);
`editors/vscode/server/src/test/parse.test.ts` (`protocolPosition`).

## Result

Positions are counted by one module under the policy each consumer speaks:
the CLI and compile errors match `tsc` (the CR example renders `cr.tt:3:1`),
the server and editor get LSP lines (the CR example answers line 3, column
1; a U+2028/U+2029 file stays on line 1 with its UTF-16 column), and source
maps follow ECMA-426. CRLF behavior is unchanged. `ttc::utf16_column` was
removed from the public API; `ttc::lines` replaces it.
