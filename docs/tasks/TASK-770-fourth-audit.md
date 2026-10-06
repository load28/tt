# TASK-770: Fix defects found by the fourth audit

- **Status**: In progress
- **Started**: 2026-10-06
- **Completed**: —
- **Commit**: —

## Purpose

A fourth audit of the CLI, the compiler, and the editor after TASK-769
reported new defects. This task fixes them in the layer that owns each,
and repeats the audit until it finds none.

## Scope

- Included: the fourth audit's CLI, compiler, and editor findings.
- Excluded: two CLI reports judged not to be defects. A `.ts` file named
  to `--check-types` is refused as "not a tt source": the typed modes take
  tt sources, and a hand-written file's diagnostics are reported through
  the project those sources open. `--check` passing on a layout that
  `-o` rejects (`x.tt` beside `x.ts`): `--check` is the tt-level check of
  the sources and takes no `-o`, so it has no output layout to judge.

## Decisions

### Decision 1: A command-line argument that is not UTF-8 is refused

- **Context**: `ttc -p "b\xff.tt"` stopped with an internal compiler error
  from `std::env::args`, exit 101.
- **Decision and rationale**: The arguments are read as `OsString`s and one
  that is not UTF-8 is refused as an invalid command line (exit 1), naming
  it, as a directory scan already refuses such a path.

### Decision 2: Each input is checked under its own project

- **Context**: `--check-types ex/src g1/src`, with a `tsconfig.json` in each
  of `ex` and `g1`, checked `ex` and dropped `g1` without a word.
- **Decision and rationale**: `--project` defaults to "the nearest one at or
  above the inputs", and one search over all inputs found a single
  configuration. Without `--project`, the typed modes now group the inputs
  by the configuration nearest to each, as a build already reads each
  file's own project (TASK-769 Decision 4), check every group, and report
  them together. A watch over several projects is refused with a message
  naming the choice, since one watch session follows one project.

### Decision 3: A named root outside the configuration gets its sidecar

- **Context**: `--types other/x.tt`, with `include: ["src"]`, checked the
  file but wrote no sidecar and reported success.
- **Decision and rationale**: A named file outside the configuration joins
  the check as a root of its own project, but declaration emit asked only
  the configured program. The host now emits for every project a requested
  module belongs to.

### Decision 4: A file that names a `.ttx` module needs the `jsx` option

- **Context**: With an unreadable `tsconfig.json`, `-p src/m.tt` where
  `m.tt` imports `./v.ttx` wrote `./v.js` and succeeded, while the `.ttx`
  file itself was refused.
- **Decision and rationale**: The option decides the spelling of every
  `.ttx` specifier, so it is needed by a `.ttx` file and by a file that
  imports one. Whether a file needs it is now read from that file (its
  extension and its tt imports) rather than from whether a `.ttx` happened
  to be among the inputs.

### Decision 5: A missing input is a command-line error in every mode

- **Context**: `--check-types missing.tt` exited 2 with an operating-system
  message, while a build exited 1 with "no such file or directory".
- **Decision and rationale**: The help says an invalid command line exits 1
  before anything runs. The typed modes now refuse a missing input (one no
  `--overlay` stands for) the way a build does. A native test that used a
  missing input to reach "could not check" now uses an input that is not a
  tt source.

### Decision 6: A source named through a file symlink is one source

- **Context**: `-o out la.tt src/x/a.tt`, `la.tt` a symlink to the second,
  wrote two outputs, while the same overlap through a directory symlink is
  refused.
- **Decision and rationale**: Source identity compared parent directories
  and file names, so two names of one file differed. Two paths that both
  exist are now the same file exactly when they canonicalize to one path.

### Decision 7: Sidecars are recorded as ttc's outputs

- **Context**: `--types -o src src` followed by `-o out src` copied
  `src/a.tt.d.ts` to the output as a hand-written declaration file.
- **Decision and rationale**: Directory scans leave out files ttc
  published, which a build marks with an ownership record. Sidecars were
  written without one. They now get the same record, so scans and the
  typed engine leave them out; overwriting a sidecar keeps today's rule.

### Decision 8: Server parameters are checked for their type

- **Context**: `"scope": "bogus"` and `"filename": 7` were accepted, a
  `"text"` of the wrong type was reported as missing, and a buffer holding
  a lone surrogate (`"\ud800"`, which `JSON.stringify` writes) was refused
  as malformed.
- **Decision and rationale**: `text`, `filename`, and `scope` are read
  through the typed parameter helpers, `scope` from its two documented
  values. A lone surrogate escape is read as U+FFFD, as a UTF-16 decoder
  does: one UTF-16 unit either way, so positions in the buffer are kept.

## Work log

- 2026-10-06: Started from the fourth audit's reports.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: pending
- **Observed failure**: pending

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Baseline changes reviewed and committed with the change

## Result

In progress.
