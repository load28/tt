# TASK-405: Write the runtime import before generated text at the top of the file

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`docs/ai/tt.md` says the `@tt/runtime` import is written at the top of the
output, after a shebang or a directive prologue. When the file began with a
tt construct whose output is generated text, the import landed after that
text instead, glued to its last line.

## Scope

- Included: The insertion model of `Rope::insert_lit_at_source`
  (`src/codegen/rope/builder.rs`), its caller in `src/codegen/core/mod.rs`,
  and the leading byte-order mark in `directive_prologue_end`
  (`src/codegen/core/planning.rs`).
- Excluded: Which helpers are imported and the import specifier.

## Defects

1. `variant S { A, B }` / `const xs = [1].map(x => x |> String);` emitted
   `};import { $tt_ap } from "./tt/runtime.js";` after the variant's
   declarations, in `.tt` and `.ttx` alike.
2. A file starting with a byte-order mark got the import in front of the
   mark, which left a U+FEFF in the middle of the output.

## Decisions

### Decision 1: Insert after the leading source pieces that precede the prologue end

- **Context**: The old rule inserted before "the first source byte at or
  after `at` printed at the top level". A variant's declarations are
  generated text inside an anchor; they carry no `Src` piece, so the search
  skipped them and stopped at the first pass-through byte after the variant.
- **Alternatives considered**: Treating an anchor's `src` offset as its
  position would fix variants but not unanchored glue such as an owner
  prelude (`let $tt_v0;`), and it would still reason about pieces the import
  does not have to follow.
- **Decision and rationale**: The only output that must precede the import
  is the source before `at` — a byte-order mark, a shebang, and the directive
  prologue, all copied through as source pieces. The rope's pieces are in
  output order, so the import is inserted after the leading top-level `Src`
  pieces that end at or before `at` (splitting the one that straddles `at`)
  and before everything else, generated text included. When `at` is the end
  of the source and the output does not end with a line break, the break is
  now part of the inserted text instead of a separate trailing piece, so the
  rule needs no exception for it.

### Decision 2: A byte-order mark stays first

- **Context**: The CLI's banner already treats a leading U+FEFF as part of
  the file's top that nothing may precede (`write_banner` in
  `src/main/build.rs`). ECMA-262 §12.2 lists U+FEFF as white space, so a
  mark moved after an import is still valid code, but it is no longer a
  byte-order mark and the output gains a stray character.
- **Decision and rationale**: `directive_prologue_end` starts after a
  leading U+FEFF, then applies the shebang and directive rules unchanged.

## Work log

- 2026-09-27: Reproduced both defects with `ttc -p` on `.tt` and `.ttx`
  inputs, with and without `"use client"` and a byte-order mark.
- 2026-09-27: Rewrote `insert_lit_at_source`, moved the trailing-break case
  into the inserted text, and made `directive_prologue_end` skip a leading
  byte-order mark.
- 2026-09-27: Added
  `the_runtime_import_precedes_generated_text_at_the_top_of_the_file`
  (tests/compile/cases_10.rs), covering `.tt` and `.ttx`, a directive, a
  byte-order mark, and a byte-order mark followed by a shebang. It fails with
  the previous `src/` (the import after the variant) and passes now.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` (no snapshot update needed)
- [x] `node scripts/check-task-index`

## Result

Changed files: `src/codegen/rope/builder.rs`, `src/codegen/core/mod.rs`,
`src/codegen/core/planning.rs`, `tests/compile/cases_10.rs`. The runtime
import is written before all generated text, after only a byte-order mark,
a shebang, and a directive prologue.
