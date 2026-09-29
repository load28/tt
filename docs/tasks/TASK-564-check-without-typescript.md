# TASK-564: `--check` does not start TypeScript

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-564: Keep --check from starting the TypeScript backend`

## Purpose

`ttc --check` writes nothing and is documented as needing no TypeScript
(`--help`: "tt-level checks; needs no TypeScript"; `docs/ai/tt.md`: the fast
tt-level check), yet with TypeScript installed it opened the TypeScript
project for every file with a contextual slot. 400 such files took 53 s
(debug build) against 1 s with no `node` on `PATH`.

## Scope

- Included: the report `--check` compiles each input with
- Excluded: `-p`, whose per-invocation cost belongs to separate work, and
  the JSON-lines server's `check` request (`src/server.rs`, edited
  concurrently elsewhere), which still calls `compile_report`

## Decisions

### Decision 1: `--check` reports over the unrefined emission

- **Context**: `compile_report` lowers, self-checks the emission, and then
  runs the standalone contextual pass, whose only effect is to annotate
  generated storage in the output. `--check` discards the output.
- **Dependency audit**: What the pass can add to a report is (a) a failure
  of the pass itself — an unreadable sibling it scans for the checker's
  filesystem, or a backend that broke its contract — and (b) the output
  self-check re-run over the annotated emission. Neither is a tt-level rule
  about the input: (a) concerns files and a process the check never needs,
  and (b) verifies output that `--check` never writes. Every tt-level
  diagnostic and the self-check of the lowered emission are computed before
  the pass and are unchanged.
- **Alternatives considered**: Use `analyze` (drops the output self-check,
  which `--check` reports today); set `defer_to_checker` (it also changes
  which exhaustiveness and `val` judgments ttc makes itself).
- **Decision and rationale**: `ttc::check_report` is `compile_report`
  without the refinement step, and the build driver uses it when `--check`
  is set. The refinement stays where the output is produced.

## Work log

- 2026-09-29: Generated 400 files under `target/probe4-cli/p3` (each a
  variant and a `match` with a join slot, nodenext `tsconfig.json`).
- 2026-09-29: `src/lib/compile.rs` — `report_parsed` takes whether to
  refine; `compile_report_parsed` refines, `check_report` does not.
  `src/main/build.rs` — `--check` compiles with `check_report`.
- 2026-09-29: Added `check_does_not_start_the_typescript_backend`
  (`tests/cli.rs`): a `node` on `PATH` that records being run; `--check`
  of a directory and of a file must leave no record, `-p` must. It fails
  without the fix ("--check started the TypeScript backend").

## Issues and resolutions

### Issue 1: `--check` scaled with TypeScript project work

- **Symptom**: `ttc --check src` over 400 files: 53.4 s with `node`,
  0.99 s without it (debug build, same head with TASK-562 and TASK-563).
- **Cause**: `compile_report` always ran the contextual pass, which opens
  the TypeScript project and asks it about every slot.
- **Resolution**: Decision 1. After: 0.62 s with `node`, 0.87 s without;
  exit 0 in all four runs.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test cli --test compile --test workflow_repairs --test practical_diagnostics`

## Result

Changed `src/lib/compile.rs`, `src/main/build.rs`, and `tests/cli.rs`.
`--check` answers from the tt layer alone, as documented.
