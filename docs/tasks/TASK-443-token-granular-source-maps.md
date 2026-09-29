# TASK-443: Map every copied token to its own column

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: recorded in the `TASK-443:` commit

## Purpose

A stack frame on an unchanged TypeScript line of a compiled `.tt` always
reported column 1, although `docs/ai/tt.md` §Workflow promises that "a stack
frame names the `.tt` line and column". The map has to give each copied
token its own mapping.

## Scope

- Included: segment granularity of `src/source_map.rs` for verbatim
  (byte-identical) runs, the surface kind the map lexes the source under,
  the module and API documentation that describe the granularity, and
  regression tests.
- Excluded: mappings for generated glue (still one segment per construct
  anchor at each cut point), the `names` field, sidecar `.d.ts` maps
  (`src/sidecar.rs`), and any change to emission itself.

## Decisions

### Decision 1: Emit a segment at every source token a verbatim run copies

- **Context**: Reproduction — a `.tt` with a variant plus the line
  `const  x = 1;   function boom() { return [1].map(() => { throw new Error("boom"); }); }`
  compiled with `ttc -o out --source-map file src` and run with
  `node --experimental-strip-types --enable-source-maps out/m.ts` printed
  frames `m.tt:3:1`; the map held a single segment for that line. `build`
  placed segments only at run edges and line starts, and its module comment
  said the column was only "the start of the copied chunk". ECMA-426 (the
  Source Map format standard: the `mappings` field and how a generated
  position is resolved against it) answers a lookup with the nearest mapping
  at or before the generated position; consumers do not interpolate inside
  a segment. A run therefore
  needs a mapping at each position a consumer is asked about.
- **Alternatives considered**:
  - A segment per byte (or per UTF-16 unit) of a run: exact for every
    position, but multiplies the map by roughly the source size for no
    position a debugger or V8 ever asks about — frames and breakpoints
    land on token starts.
  - A segment per whitespace/identifier boundary found by scanning the
    emitted text: reads the output for meaning, which TASK-200 Decision 1
    forbids, and would misclassify bytes inside strings, comments,
    templates and regexes.
  - Lexing each run in isolation: a run may start or end inside a string,
    comment or template, so its own lexing would cut inside literals.
  - A segment per source token (chosen).
- **Decision and rationale**: `token_starts` lexes the whole source once with
  the compiler's own lexer (`lexer::lex_with_kind`), flattening template
  chunks, template interpolations and JSX expression containers, and `build`
  adds, for every `EmitMapping`, the output offset of each token start the
  run covers. Because a run is byte-identical on both sides, each such
  segment maps a token to the identical original line and column. This is
  the granularity TypeScript's own emitter uses: the pinned
  `typescript@7.1.0-dev.20260826.1` compiling the reproduction line with
  `tsc --sourceMap` writes `SAAS,IAAI,KAAK,OAAO,CAAC,CAAC,CAAC,CAAC,GAAG,...`
  — one segment per emitted token. A position inside a token or in the
  whitespace after it still resolves to that token's start, exactly as it
  does in a TypeScript-emitted map. Generated glue keeps its existing
  construct-anchor mappings.

### Decision 2: Only ASCII bytes start a token; the source kind comes with the request

- **Context**: The lexer emits a one-byte `Punct` for every byte it does not
  otherwise classify, including each byte of a multibyte UTF-8 sequence, and
  a `.ttx` source has to be lexed with JSX admitted.
- **Alternatives considered**: Filtering cut points by
  `str::is_char_boundary` would still split an identifier such as `café` at
  `é`; storing the kind on `MappedEmit` would widen a struct constructed in
  several places for a fact only the map needs.
- **Decision and rationale**: `Punct` tokens for non-ASCII bytes are skipped,
  so multibyte UTF-8 stays opaque (AGENTS.md) and columns stay counted in
  UTF-16 code units by the existing `LineTable`. `SourceMapRequest` gained
  `source_kind` (default `SourceKind::TypeScript`); the CLI fills it from the
  job's path.

### Decision 3: Accept the larger `mappings` field

- **Context**: More segments mean a larger map.
- **Decision and rationale**: Measured on a generated 145 KB `.tt` (500
  functions with `match`, array pipelines and templates): the `mappings`
  field grew from 93,555 to 276,454 bytes. The pinned `tsc --sourceMap`
  over the same emitted `.ts` writes a 418,637-byte `mappings` field, so the
  map remains smaller than TypeScript's own for the same code. The whole
  source is lexed once per map, which is the parser's own cost for the file.

## Work log

- 2026-09-27: Ran `./scripts/doctor` (TypeScript missing) and `npm ci`.
  Reproduced the column-1 frames and the single-segment line.
- 2026-09-27: `src/source_map.rs` — added `token_starts`, the per-run token
  cut points in `build`, `SourceMapRequest::source_kind`, and rewrote the
  module comment's granularity paragraph. `src/main/output.rs` passes the
  job's `SourceKind`. `src/lib/mapped.rs` — `MappedEmit::source_map` doc
  names the source's tokens as an input.
- 2026-09-27: Added unit tests
  `every_token_of_a_copied_line_maps_to_its_own_column` and
  `a_byte_outside_ascii_never_starts_a_segment` (decode the VLQ and resolve
  positions the way a consumer does), and the Node integration test
  `a_node_stack_frame_on_a_copied_line_names_its_column`
  (`tests/integration/cases_05.rs`). Both unit tests fail with the token cut
  points disabled; the ASCII test fails with the non-ASCII filter removed.
- 2026-09-27: Reproduction now prints `m.tt:3:64` (`new Error`) and
  `m.tt:3:46` (`.map`).
- 2026-09-27: Added a reversal note to TASK-200, whose Decision 6 recorded
  chunk-start columns.

## Issues and resolutions

### Issue 1: The first token test matched `=>` on the wrong line

- **Symptom**: `every_token_of_a_copied_line_maps_to_its_own_column` failed
  for `"=>"` with `(0, 8)` against `(1, 36)`.
- **Cause**: The test helper located the first `=>` in each text, which is in
  the `match` line (and its glue), not in the copied line.
- **Resolution**: The helper searches from the start of the copied line.

### Issue 2: Clippy rejected the decoded-map test type

- **Symptom**: `cargo clippy --all-targets -- -D warnings` reported
  `clippy::type_complexity` for `(String, Vec<Vec<(i64, i64, i64)>>)`.
- **Cause**: The nested tuple type was spelled inline in a test helper.
- **Resolution**: A test-local `type Decoded` alias names it.

### Issue 3: A first full `cargo test` run failed unrelated suites

- **Symptom**: Sixteen `tests/integration` cases (pipelines, `result` blocks,
  runtime runs) failed and the log was truncated.
- **Cause**: The shared disk was full (`No space left on device`) while
  other builds ran on the machine; a failing case passed when run alone.
- **Resolution**: Removed this worktree's incremental build directory,
  reran with `CARGO_INCREMENTAL=0`; the full `cargo test` then passed.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `UPDATE_EXPECT=1 cargo test --test snapshot` — no fixture contains a
  source map; no snapshot changed.

## Result

Changed files: `src/source_map.rs`, `src/main/output.rs`,
`src/lib/mapped.rs`, `tests/integration/cases_05.rs`,
`docs/tasks/TASK-200-standard-source-map.md`, `docs/tasks/INDEX.md`, and this
record. Every token copied verbatim into compiled output now has its own
source-map segment, so stack frames and debugger positions on unchanged
TypeScript lines name the `.tt` column, as `docs/ai/tt.md` already promised.
