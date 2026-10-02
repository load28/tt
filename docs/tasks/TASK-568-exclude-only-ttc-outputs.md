# TASK-568: Leave only ttc's own outputs out of the TypeScript program

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

After TASK-563, the editor's sidecar refresh for a `.ttx` file failed with
"ttc did not write it": the extension runs `ttc --types <file> -o <the
file's directory>`, and TASK-563 hid every entry of the output directory
from the program, the sources included.

## Scope

- Included: The host's layered file system listing
  (`src/typescript/host.mjs`).
- Excluded: Which directories the engine names as outputs (TASK-563).

## Decisions

### Decision 1: Hide what ttc writes, not where it writes

- **Context**: `tsc` leaves its own outputs out of a default `include`. An
  output directory of ttc can also hold the sources, when sidecars are
  written beside them.
- **Alternatives considered**: Refusing `-o` equal to a source directory
  would break the extension's documented sidecar refresh.
- **Decision and rationale**: In an output directory, the listing leaves
  out the declaration sidecars ttc names after their sources
  (`x.tt.d.ts`, `x.ttx.d.ts`, each with its `.map`, the naming `--types`
  and `--sidecar` document), and the output root's `tt/`, the support
  package directory ttc owns there (`std_placement`). Every other entry
  stays an input.

## Work log

- 2026-09-29: The merged extension suite failed "ttx saves refresh TSX
  declarations and map back to the ttx source" twice in a row; the sidecar
  refresh passes the source directory as `-o`. Narrowed the listing filter
  and added `types_written_beside_the_sources_keep_the_sources_as_inputs`
  (`tests/workflow_repairs.rs`), which fails on the TASK-563 host and
  passes now; TASK-563's `types_output_directory_is_not_a_program_input`
  still passes.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `editors/vscode`: all server and client tests

## Result

Changed `src/typescript/host.mjs` and `tests/workflow_repairs.rs`; noted the
revision at the top of the TASK-563 record.
