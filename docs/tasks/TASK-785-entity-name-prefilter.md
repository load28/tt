# TASK-785: Remove the entity-name set that slowed every compile

- **Status**: Complete
- **Started**: 2026-10-07
- **Completed**: 2026-10-07
- **Commit**: `TASK-785: Remove the entity-name set that slowed every compile`

## Purpose

The `performance` check of PR #142 failed on `5d4005c8`: `single_file`
grew 13.5% and `project_first_snapshot` 12.9% against the merge base
`1cc08aa9`, over the 10% budget. `a71c5729` passed the check.

## Scope

- Included: the cost TASK-781's decision K6 added to every compile.
- Excluded: behaviour; the emitted output is unchanged.

## Decisions

### Decision 1: An entity-name check rejects a capture by its bytes before parsing it

- **Context**: K6 recorded every entity name of the module by its source
  span while building the program syntax, so a capture could look its
  span up instead of parsing its own text. Recording mapped each
  identifier and member chain back to its source span
  (`source_span_for_projection`), which every compile paid for whether or
  not it had a capture. Callgrind counted 12.87M instructions for the
  benchmark module at `5d4005c8`, 11.44M at `1cc08aa9`.
- **Alternatives**: keep the set and build it lazily, only when a capture
  asks. That still maps every name once a module has one capture.
- **Decision and rationale**: The set is removed. `source_entity_name`
  first scans the capture's bytes and returns false at the first byte no
  entity name can hold (outside comments: ASCII letters, digits, `_`,
  `$`, `.`, `\`, whitespace, non-ASCII). A left-deep prefix such as
  `"" + a + b` holds `"` or `+` near its start, so the scan stops early
  and the chain stays linear; a text that passes is short or is a name,
  and is parsed as before. The benchmark module drops to 12.34M
  instructions.

## Work log

- 2026-10-07: Removed the entity-name set from `src/program_syntax/projection.rs`,
  `src/evaluation_ir{.rs,/evaluation.rs}` and `src/codegen/core/{mod.rs,planning.rs,emitter/}`;
  added the byte scan to `source_entity_name` in `src/program_syntax.rs`.

## Issues and resolutions

### Issue 1: Doctests failed to link

- **Symptom**: every doctest failed with `ld terminated with signal 7 [Bus error]`.
- **Cause**: the session's disk allowance was spent by stale build directories.
- **Resolution**: removed them and reran the doctests, which pass.

## Regression test (fails before the fix)

Not applicable: no behaviour changes. The cost is held by the CI
`performance` job and the linear-work test
`src/lib/scaling_tests.rs::compiling_does_linear_work_in_a_left_deep_chain_of_tt_values`,
which still passes.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Emitted output of the benchmark module is byte-identical to `1cc08aa9`

## Result

Complete. The benchmark module costs 12.34M instructions (12.87M before,
11.44M on `1cc08aa9`), its output is unchanged, and the full gate passes.
