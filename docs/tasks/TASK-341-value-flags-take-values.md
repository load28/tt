# TASK-341: A flag that takes a value does not take an option as one

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-341: stop value-taking flags from swallowing the next option`

## Purpose

Every value-taking flag read the next argument whatever it was, so `ttc -o
--check src` built into a directory named `--check` and exited 0 without
running the check the line asked for. The mistake is silent twice over:
the value is wrong, and the option that became it is gone from the run.

## Scope

- Included: How `--out-dir`/`-o`, `--jobs`/`-j`, `--project`, `--node`,
  `--sidecar`, `--overlay`, `--source-map`, `--rewrite-imports` and
  `--emit-std` read their values, and what they say when there is none.
- Excluded: `--flag=value` spelling, a `--` end-of-options separator, and
  anything about which flags combine with which mode (TASK-338).

## Decisions

### Decision 1: A token that begins with `-` is an option, not a value

- **Context**: The parser called `it.next()` at each value-taking flag.
  Nine flags shared the hole. `ttc -o --check src` created `./--check` and
  wrote a build into it; `ttc --project --check src` used `--check` as a
  tsconfig path; `ttc -j --check src` reported "--jobs expects a positive
  number", naming the wrong problem.
- **Alternatives considered**: Reject only tokens that are *known* option
  names (leaves `ttc -o -x src` writing into a directory called `-x`, and
  makes the rule depend on the option table rather than on the shape of an
  argument); accept the value and warn (a warning does not stop the build
  it describes, and the directory is still there afterwards).
- **Decision and rationale**: Treat any token longer than one character and
  beginning with `-` as an option, the way `clap` and the GNU convention
  do. A single `-` stays a value, since that is stdin's name.

### Decision 2: The report names the flag, what it wanted, and what it found

- **Context**: A missing value was already reported per flag
  ("`--project` requires a path to a tsconfig.json"). The new case needs to
  say why an argument that *is* there was not taken.
- **Alternatives considered**: Reuse the missing-value message unchanged
  (the reader is looking at an argument and told there is none).
- **Decision and rationale**: One helper, `flag_value`, holds both cases:
  the existing message when nothing follows, and the same message plus "but
  the next argument is `--check`" when an option does. The missing-value
  wording is unchanged, so what was already pinned stays pinned.

### Decision 3: A path that really begins with `-` keeps its escape hatch

- **Context**: The rule makes `ttc -o -out src` an error, and `-out` could
  be a directory someone wants.
- **Alternatives considered**: Add `--out-dir=-out` (a spelling the parser
  does not have anywhere else, so it would be a new feature rather than
  this fix).
- **Decision and rationale**: `./-out` names the same directory and is what
  every tool with this rule expects; it is covered by a test so the hatch
  cannot close by accident.

## Work log

- 2026-09-05: Reproduced `ttc -o --check <file>` writing `--check/` and
  exiting 0.
- 2026-09-05: Added `flag_value` in `src/main/command.rs` and routed all
  nine value-taking flags through it.
- 2026-09-05: Added `value_flags_do_not_swallow_the_next_option` and
  `a_relative_path_reaches_a_directory_named_like_an_option` to
  `tests/cli.rs`, and moved `-j -1` out of the garbage-value loop — it is
  now reported as an option where a value was expected, which is the same
  rejection with the accurate reason.

## Issues and resolutions

### Issue 1: `ttc -o --check src` compiled instead of checking

- **Symptom**: A directory named `--check` appeared next to the sources,
  full of emitted TypeScript, and the command exited 0.
- **Cause**: `-o` took `it.next()` unconditionally, so `--check` never
  reached the option table.
- **Resolution**: Decision 1.

## Verification

- [x] Every value-taking flag rejects a following option, names itself and
  the argument it found, and writes nothing
- [x] `./-out` still reaches a directory whose name begins with `-`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Changed files: `src/main/command.rs`, `tests/cli.rs`, and this record.
