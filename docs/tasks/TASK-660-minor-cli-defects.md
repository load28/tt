# TASK-660: Fix five small CLI and initializer defects, and keep one reported behaviour

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-660`

## Purpose

The round-7 CLI probe reported six small items. Five are defects fixed
here; one is the documented contract and stays.

1. A TypeScript diagnostic in a hand-written `.ts` file under
   `--check-types` printed its location without the source line or caret.
2. `--types --json-report` without TypeScript reported `"diagnostics": 1`
   although it printed no diagnostic (`reported.max(1)`).
3. `ttc src` over an in-place output that had been edited printed three
   errors, among them a misleading "multiple inputs claim this output".
4. `--symbols` printed an import's `resolved` path unnormalized
   (`src/./sub/../c.tt`).
5. `create-tt` accepted directory names that are not valid npm package
   names (`.hidden`, `_x`, `node_modules`, a core module name, more than
   214 characters) and had no way to name a directory starting with `-`.
6. `--check-types missing.tt` exits 2 while untyped modes exit 1 for a
   missing input.

## Scope

- Included: `src/main/typed.rs`, `src/main/build.rs`, `src/main/modes.rs`,
  `packages/create-tt/src/installer.js` and its README, and tests in
  `tests/cli.rs` and `packages/create-tt/test/installer.test.mjs`.
- Excluded: item 6 (Decision 5).

## Sources

- npm, `package.json` "name": at most 214 characters, cannot start with a
  dot or an underscore, no uppercase letters, URL-safe characters only.
  `validate-npm-package-name` (npm's validator): `node_modules` and
  `favicon.ico` are blacklisted, and a Node.js core module name is not
  valid for a new package (`validForNewPackages: false`).
- POSIX Utility Syntax Guidelines, Guideline 10: "The first `--` argument
  that is not an option-argument should be accepted as a delimiter
  indicating the end of options."
- `docs/ai/tt.md`, `ttc --check-types`/`--types` exit statuses: 0 clean,
  1 diagnostics reported, 2 the check could not run, 3 a write failed.

## Decisions

### Decision 1: A diagnostic in a file the snapshot does not hold quotes the file on disk

- **Context**: The renderer quotes the snapshot's text so an `--overlay`
  is never misquoted, and a hand-written `.ts` is not in the snapshot; it
  is read by TypeScript from disk.
- **Alternatives considered**: (a) Put every host file's text in the
  snapshot: the engine would read every program file for a render. (b)
  Read the file from disk when, and only when, the snapshot has no text
  for the diagnostic's path, which is where its position came from.
- **Decision and rationale**: (b).

### Decision 2: The report counts what was printed

- **Context**: `reported` feeds both the exit status and `diagnostics`; a
  blocked pass already sets `blocked`, which decides the exit status 2.
- **Decision and rationale**: `TypedReport::unchecked` takes the count;
  the no-TypeScript path reports the tt diagnostics it printed, the
  blocked-project path 1 (it printed one), and an open failure 0. The
  exit statuses do not change.

### Decision 3: An input that another input compiles to is that input's output

- **Context**: `ttc src` with `src/a.tt` writes `src/a.ts`. On the next run
  an unedited `a.ts` is recognised as an owned output and skipped; an
  edited one is collected as a host input whose job writes itself, which
  collides with `a.tt`'s job ("multiple inputs claim this output") on top
  of the refusal to overwrite it.
- **Alternatives considered**: (a) Suppress the collision error when one
  claimant is an output: it hides the rule instead of applying it. (b)
  Drop a pass-through job whose file is another job's output path, after
  collection.
- **Decision and rationale**: (b), in `build_jobs`. The run then reports
  the one real problem, the refusal to overwrite an edited output.

### Decision 4: Name a project with a name npm accepts, and honour `--`

- **Context**: The package name is derived from the directory name.
- **Alternatives considered**: (a) Reject such directory names. The name
  is only derived; the directory itself is fine. (b) Derive a valid name:
  strip leading `.`, `_`, and `-`, truncate to 214 characters, and use
  `my-tt-app` for an empty, blacklisted, or core-module result.
- **Decision and rationale**: (b), `packageName`. `--` ends option parsing,
  so `create-tt -- -dash` creates `-dash/`.

### Decision 5: `--check-types missing.tt` keeps exit status 2

- **Context**: The untyped modes exit 1 for a missing input.
- **Decision and rationale**: Not a defect. For the typed modes the
  documented statuses give 2 to "the check could not run", which is what
  a missing input is; 1 means diagnostics were reported, and none were.
  Changing it would break tools that read the documented statuses.

## Work log

- 2026-09-30: Reproduced items 1–5 by hand from the probe notes.
- 2026-09-30: Fixed each; added
  `a_type_error_in_hand_written_typescript_quotes_its_line`,
  `a_types_report_without_typescript_counts_only_what_it_printed`,
  `an_edited_output_written_in_place_is_one_refusal`,
  `symbols_resolve_an_import_to_a_normalized_path`, and two create-tt
  tests; documented the name rule and `--` in the create-tt README.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cli.rs` (the four tests above) and
  `packages/create-tt/test/installer.test.mjs` ("names a project with a
  name npm accepts for a new package", "takes every argument after -- as
  the directory").
- **Observed failure**: With the non-test changes reversed, all four CLI
  tests failed: the `ts2322` diagnostic printed `--> src/host.ts:2:26`
  with no source line or caret; the report was
  `{"checked":false,"diagnostics":1,...}`; the edited output produced
  three errors ("multiple inputs claim this output: src/a.ts and
  src/a.tt", "output would overwrite the input — pass -o <dir>", and the
  refusal); and `resolved` was `["src/./sub/b.tt",
  "src/../src/./sub/../c.tt"]`. With the old `packageName` exported for
  the test, it returned `.hidden` for `.hidden`, and `run` rejected `--`
  with "unknown argument: --".

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (the full gate, see TASK-654's record for the run)
- [x] `node --test packages/create-tt/test/*.test.mjs`
- [x] Baseline changes reviewed and committed with the change (none)

## Result

Changed files: `src/main/typed.rs`, `src/main/build.rs`,
`src/main/modes.rs`, `packages/create-tt/src/installer.js`,
`packages/create-tt/README.md`, `tests/cli.rs`,
`packages/create-tt/test/installer.test.mjs`.
