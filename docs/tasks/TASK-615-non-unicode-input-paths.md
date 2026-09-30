# TASK-615: Refuse a non-Unicode input path before writing, and publish the record first

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-615`

## Purpose

`ttc -o out src` with an input named `src/bad\xff.tt` ended in an internal
compiler error (exit 101): the ownership record was built with
`serde_json::json!` over the source's canonical path, which serde cannot
serialize when it is not UTF-8. The output had already been written, so it
was left without its record and every later build refused to overwrite it.
`ttc --check-types src` panicked the same way when serializing the backend
request.

## Scope

- Included: the ownership writer (`src/main/ownership.rs`), the ownership
  rule and record path (`src/ownership.rs`), the engine's snapshot
  (`src/engine/project.rs`), `docs/ai/tt.md`, and regression tests.
- Excluded: stdout (`ttc file.tt`) and `--check`, which write no record
  and never cross the TypeScript protocol; they keep working for such a
  path.

## Decisions

### Decision 1: A source path that is not Unicode is a user error, raised before anything is written

- **Context**: A record names its owner as a JSON string, and JSON strings
  are Unicode. Rust's `Path`/`OsStr` may hold any bytes on Unix, and
  `OsString::into_string` / `Path::to_str` report when they are not valid
  Unicode (Rust std docs, `std::ffi::OsString::into_string`,
  `std::path::Path::to_str`). The emitted module is for TypeScript: Node.js
  interprets string paths as UTF-8 and returns directory entries decoded as
  UTF-8 by default (Node.js docs, "File system — File paths" and
  `fs.readdir` `encoding`), and TypeScript's paths and module specifiers
  are strings, so nothing downstream could open or import that output by
  name.
- **Alternatives considered**: (a) Record the path losslessly as bytes
  (`OsStrExt::as_bytes` on Unix, `encode_wide` on Windows): a
  platform-specific record format that publishes a module TypeScript cannot
  address. (b) Record it lossily: two names differing only in invalid bytes
  would claim each other's outputs. (c) Refuse the input with a user error
  in the ownership check.
- **Decision and rationale**: (c). `recorded_source` converts the owner
  identity once and returns the error; `check_output_owner` calls it, so
  the build's pre-write ownership pass (`compile_jobs`) refuses the whole
  build before any file is written, exactly as it does for an ownership
  conflict. `write_owned_output` builds the record through the same
  function, so it can never panic.

### Decision 2: The record is published before the output and names the bytes it replaces

- **Context**: Writing the output first and the record second left an
  output without its record whenever the second step failed (here by a
  panic; also by an I/O error or an interrupted process). Writing the record
  first has the mirror problem: an output write that fails leaves the old
  bytes under a record of the new ones, which is no longer "owned".
- **Alternatives considered**: (a) Serialize the record first, then keep
  the output-then-record order: fixes the panic but not an I/O failure or
  an interruption between the two writes. (b) Record first and restore the
  old record if the output write fails: an interruption still strands the
  output.
- **Decision and rationale**: A two-phase publication. When an owned output
  is being replaced by different bytes, the record is first written with
  `content` (the new bytes) and `replaced` (the bytes on disk); then the
  output; then the record with `content` alone. `ttc::ownership::owned_output`,
  the one rule every reader uses (build, directory scan, typed engine),
  accepts either. At every point an output ttc wrote is covered by its
  record. A new output needs only the first record write, since a record
  without its file claims nothing.

### Decision 3: The record's file name is built from the output's `OsStr`

- **Context**: `record_path` formatted the output's file name through
  `to_string_lossy`, so two outputs whose names differ only in invalid
  bytes shared one record.
- **Decision and rationale**: The name is assembled as an `OsString`
  (`OsString::push`, Rust std docs), byte for byte. An output directory
  given with `-o` may still be any path.

### Decision 4: The typed engine blocks a snapshot whose file TypeScript cannot name

- **Context**: The same input panicked `--check-types` in `job_json`,
  which serializes paths into the host's JSON protocol.
- **Decision and rationale**: `Project::update` returns `Blocked` for a
  tt file, inferred source, or overlay whose path is not Unicode, the way
  it blocks a file it cannot read; the CLI prints it and exits 2 ("could
  not check").

## Work log

- 2026-09-30: Reproduced `target/probe6-cli/p19`: exit 101 in
  `write_owned_output`, `out/bad\xff.ts` written without a record.
  `--check-types src` panicked in `src/typescript/native.rs` `job_json`.
- 2026-09-30: Added `recorded_source`, the two-phase publication, the
  `replaced` acceptance in `owned_output`, the `OsString` record path, and
  `unnameable` in `Project::update`.
- 2026-09-30: The repro now prints
  `ttc: src/bad\xff.tt: input path is not valid UTF-8, so its output cannot
  record its owner — rename the input`, exits 1, and writes nothing;
  `--check-types` prints a located error and exits 2.
- 2026-09-30: Tests
  `an_input_path_that_is_not_unicode_is_refused_before_anything_is_written`
  and `an_interrupted_publication_leaves_the_output_owned`
  (`tests/workflow_repairs.rs`), and the unit test
  `a_record_names_its_output_byte_for_byte` (`src/ownership.rs`).

## Issues and resolutions

### Issue 1: `--check-types` had the same panic

- **Symptom**: `called Result::unwrap() on an Err value: Error("path
  contains invalid UTF-8 characters")` at `src/typescript/native.rs`.
- **Cause**: Paths are serialized into the host protocol with `json!`.
- **Resolution**: Decision 4; the engine refuses the path before a request
  is built.

## Verification

- [x] `TTC_REQUIRE_TSGO=1 cargo test --test workflow_repairs` (19 passed)
- [x] `cargo test --lib ownership`
- [x] Full gate run once at the end of the TASK-614–620 series; see
  TASK-620.

## Result

Changed `src/main/ownership.rs`, `src/ownership.rs`,
`src/engine/project.rs`, `docs/ai/tt.md`, `tests/workflow_repairs.rs`,
`docs/tasks/INDEX.md`, and this record.
