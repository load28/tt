# TASK-336: Write the generated banner where the file allows it

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-336: write the generated banner below a shebang`

## Purpose

The `@generated` banner was prepended unconditionally, so every compiled
CLI entry point came out with its `#!` line on line 2 — invalid TypeScript
and no longer executable — and a byte-order mark ended up mid-file.

## Scope

- Included: Where the per-file banner is written, and the source map's
  account of the lines it adds.
- Excluded: Whether a banner is written at all (`--no-banner` already
  decides that), and the `@tt/std` module banner, whose modules the
  compiler writes itself and which can carry neither construct.

## Decisions

### Decision 1: Insert after the constructs that must come first

- **Context**: A `#!` line and a byte-order mark are only themselves when
  they are the first thing in the file. A comment above either one turns a
  runnable script into a parse error and leaves a stray U+FEFF in the middle
  of the text. Everything else at the top of a file — a license comment, a
  blank line, a directive prologue such as `"use client"` — a comment may
  precede, because a comment is not a statement and does not end a prologue.
- **Alternatives considered**: Drop the banner for files that start with
  either construct (the marker is the point); move the shebang below the
  banner (it would stop being one).
- **Decision and rationale**: Scan the emitted text for those two
  constructs and write the banner immediately after them. A shebang that
  runs to the end of the file gets a line break first, so the banner has a
  line of its own.

### Decision 2: A source map shifts only the lines that moved

- **Context**: The map was built against the emission and told that the
  banner adds one line at the top, which shifts every segment down by one.
  With the banner below a shebang, the shebang's own line does not move.
- **Alternatives considered**: Rebuild the map against the final text
  (the emission owns the offsets the mapping is expressed in).
- **Decision and rationale**: The request now carries the line the insertion
  happens at as well as its size, and segments before that line keep their
  position. With an insertion at line 0 the encoding is unchanged, which is
  every file without a shebang.

## Work log

- 2026-09-05: Reproduced with a `.tt` and a hand-written `.ts` that each
  start with `#!/usr/bin/env node`, plus a `.tt` with a byte-order mark. The
  pinned TypeScript reported `TS18026: '#!' can only be used at the start of
  a file` for both, and the emitted script failed to run under `node`. After
  the change both type-check, the script prints its output, and the mark
  stays first with exactly one occurrence.
- 2026-09-05: Confirmed the map still resolves: a thrown error inside a match
  arm of a file with a shebang reports `trace.tt:5:25` under
  `node --enable-source-maps`.

## Issues and resolutions

### Issue 1: The banner displaced a shebang and a byte-order mark

- **Symptom**: `ttc -o out src` emitted `// @generated …` on line 1 and
  `#!/usr/bin/env node` on line 2. `tsc` reported TS18026 and TS1005; `node`
  reported `SyntaxError: Invalid or unexpected token`; the executable script
  failed with a shell syntax error. A leading byte-order mark ended up at
  byte 57.
- **Cause**: The banner was prepended to the emitted text with no regard for
  what the file started with.
- **Resolution**: Decisions 1 and 2.

## Verification

- [x] Shebangs keep line 1 in `.tt` output and in hand-written `.ts`
  pass-through; the byte-order mark stays first and appears once; a shebang
  with no trailing newline still gets a line of its own
- [x] The emitted script type-checks under the pinned TypeScript and runs
- [x] The source map keeps the shebang line mapped, and a thrown error
  resolves to its `.tt` line and column
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Changed files: `src/main/build.rs`, `src/main/output.rs`, `src/source_map.rs`,
`tests/cli.rs`, and this record.
