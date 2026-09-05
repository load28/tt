# TASK-337: Treat a closed stdout as an ordinary end, not an internal error

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-337: end quietly when stdout closes`

## Purpose

`ttc --help | head` reported an internal compiler error, told the user to
file a bug, and exited 101.

## Scope

- Included: How the CLI writes to stdout.
- Excluded: Everything written to stderr, which no pipeline closes this way.

## Decisions

### Decision 1: Handle the write error rather than the signal

- **Context**: Rust ignores `SIGPIPE`, so a write to a closed pipe returns
  an error instead of ending the process the way `cat` ends. `println!`
  panics on that error, and this binary's panic hook reports an internal
  compiler error (`crate::ice`).
- **Alternatives considered**: Restore the default signal disposition (needs
  `unsafe`, which this crate forbids); recognise the panic message in the
  hook (a string match against a standard-library message).
- **Decision and rationale**: Print through one place that inspects the
  write. `BrokenPipe` ends the run quietly with status 0 — the reader
  decided it had enough. Any other write failure is a real failure of this
  run and is reported as one, on stderr, with status 1.

## Work log

- 2026-09-05: Reproduced across every stdout-producing mode: `--help`, `-v`,
  `help all`, `explain`, `--emit-std`, and `-p` on output larger than a pipe
  buffer. Each printed the internal-compiler-error report and exited 101.
  After the change each exits 0 with an empty stderr, and the same modes
  still print their full output when nothing closes the pipe.

## Issues and resolutions

### Issue 1: A shell pipeline was reported as a compiler bug

- **Symptom**: `error: internal compiler error: failed printing to stdout:
  Broken pipe (os error 32)`, followed by "This is a bug in ttc, not in the
  code it was given" and the issue-tracker URL; exit 101.
- **Cause**: `println!`'s panic reaching the internal-error hook.
- **Resolution**: Decision 1.

## Verification

- [x] Five stdout modes exit 0 with no internal-error report when the reader
  closes the pipe after one byte
- [x] The same modes still print their complete output otherwise
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Changed files: `src/main/out.rs` (new), `src/main.rs`, `src/main/build.rs`,
`src/main/command.rs`, `src/main/modes.rs`, `tests/cli.rs`, and this record.
