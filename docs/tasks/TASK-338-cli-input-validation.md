# TASK-338: Reject CLI inputs the contract already forbids

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-338: reject CLI inputs the contract forbids`

## Purpose

Three CLI inputs are accepted and then fail somewhere far from the mistake:
a `--project` path that is not there, input roots that overlap, and a named
file that is neither a tt source nor hand-written TypeScript.

## Scope

- Included: Validation of `--project`, the overlapping-roots half of the
  documented output-collision contract, and the extension contract for a
  file named on the command line.
- Excluded: The `--check-types` backend itself, and anything about what a
  valid project then reports.

## Decisions

### Decision 1: A named `tsconfig.json` must exist, and must be a file

- **Context**: A path that is not there is carried into project identity as
  written, and its *parent directory* becomes the project root. Three
  spellings produced three different failures: a relative `./tsconfg.json`
  reached the TypeScript backend un-canonicalised and returned a Go panic
  dump reported as an internal compiler error; a bare `tsconfg.json`
  produced `ttc: No such file or directory (os error 2)` naming neither the
  flag nor the path; and an absolute one silently rooted the project at a
  directory the user never named, so ttc walked an unrelated tree and
  reported diagnostics for files that were not inputs.
- **Alternatives considered**: Fall back to discovering a config the way an
  unspecified run does (the user named one on purpose); canonicalise and
  let the backend report it (the backend's report is a Go panic).
- **Decision and rationale**: Check it where it is read, at the command
  line, and report it the way a missing input is reported — naming the flag
  and the path, exit 1. That also removes the un-canonicalised path the
  backend panicked on, because what remains is a path that exists.

### Decision 2: One input may not claim two outputs

- **Context**: `docs/ai/tt.md` states that CLI builds reject "distinct
  inputs that map to one output (`x.tt` + `x.ts`, `x.ttx` + `x.tsx`,
  overlapping roots, or a compiler support-module path)". The guard keys on
  the output path, which catches two inputs → one output but not one input
  → two outputs: `ttc -o out . src` wrote every source under `src` twice,
  at two paths, and exited 0.
- **Alternatives considered**: De-duplicate silently by keeping the first
  claim (the second output is a file the user did not ask for and would not
  know to delete); reject overlapping roots by comparing the roots (a root
  may legitimately be named twice, and the same file twice is not a
  conflict).
- **Decision and rationale**: Report when one source file is claimed by two
  *different* outputs, next to the existing check and in the same shape. The
  same input named twice still resolves to one output and stays legal.

### Decision 3: A file named on the command line is filtered like any other

- **Context**: The directory walk takes `.tt`/`.ttx` and, with
  pass-through enabled, `.ts`/`.tsx`/`.mts`/`.cts`. A file named directly
  was taken unconditionally, so `ttc -o build src/app.js` wrote TypeScript
  syntax into a file still called `.js`, and `ttc -o out notes.txt` wrote
  `out/notes.txt` with a banner. A directory containing only those files
  reports `no sources found` and exits 1, so the two paths disagreed.
- **Alternatives considered**: Keep taking any named file (the `--help`
  contract names the extensions it compiles and passes through); infer the
  language from the content.
- **Decision and rationale**: Apply the same extension test to a named file,
  and say which extensions are meant when it does not pass.

## Work log

- 2026-09-05: Reproduced all three against the current compiler.

## Issues and resolutions

### Issue 1: `--project` accepted a path that is not there

- **Symptom**: A Go panic dump reported as an internal compiler error
  (relative with `./`), a bare `No such file or directory (os error 2)`
  (bare relative), or a silent scan of an unrelated directory tree
  (absolute), which reported diagnostics for files that were never inputs.
- **Cause**: The path was used verbatim for project identity, and its parent
  directory became the project root, with no existence check anywhere.
- **Resolution**: Decision 1.

### Issue 2: Overlapping input roots wrote every source twice

- **Symptom**: `ttc -o out . src` emitted `out/src/a.ts` and `out/a.ts` for
  one source, exit 0.
- **Cause**: The collision guard keys on the output path only.
- **Resolution**: Decision 2.

### Issue 3: A named file bypassed the extension contract

- **Symptom**: `ttc -p x.js` printed TypeScript for a `.js` file; `ttc -o
  out b.txt` wrote `out/b.txt`.
- **Cause**: `collect_sources` pushes a named file unconditionally.
- **Resolution**: Decision 3.

## Verification

- [x] Each `--project` spelling reports the flag and the path and exits 1;
  a directory says to name the config itself; a valid path still checks.
  The check runs after the mode is settled, so a wrong combination of flags
  is still reported as one
- [x] Overlapping roots are reported by identity, so the same source reached
  through two roots is caught however it is spelled; the same root named
  twice still resolves to one output per source
- [x] A named `.js`/`.txt` is reported with the extensions that are meant;
  `.tt` and `.ts` still work when named directly
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Changed files: `src/main/command.rs`, `src/main/build.rs`,
`src/engine/project.rs`, `tests/cli.rs`, and this record.
