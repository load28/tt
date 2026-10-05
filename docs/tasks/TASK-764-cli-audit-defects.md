# TASK-764: Fix defects found by a CLI audit

- **Status**: Complete
- **Started**: 2026-10-05
- **Completed**: 2026-10-05
- **Commit**: —

## Purpose

An audit of `ttc`'s command line found builds, watch rounds, and typed
checks that disagree with the documented contract or with what TypeScript
does in the same situation. Fix each in the layer that owns it.

## Scope

- Included: in-place directory builds with hand-written `.ts`, `--watch`
  re-reading the `jsx` option and the `-p` one-source rule, typed checks
  under a configuration with JSON syntax errors, the tt-only fallback's
  file set, `--node` in build and print modes, and output directories that
  are files.
- Excluded: a directory input whose files the configuration's `include`
  leaves out is still not a root (Decision 7); new diagnostics.

## Decisions

### Decision 1: An unchanged pass-through file in place is its own output

- **Context**: `ttc src` without `-o` refused the whole build when `src`
  held one hand-written `.ts`, because that file's output path is its own
  path.
- **Alternatives considered**: Keep refusing (in-place builds of any mixed
  tree are impossible); skip hand-written files in place entirely (a file
  whose relative `.tt` specifier must be rewritten would silently keep it);
  TypeScript's model.
- **Decision and rationale**: TypeScript reports an emit that would
  overwrite an input and blocks that one file (`blockEmittingOfFile` in
  typescript-go `internal/compiler/program.go`), and the rest of the
  program still emits. A pass-through file whose output equals its source
  has nothing to write, so it is left as it is; one whose specifiers would
  be rewritten is refused on its own while the other files build.

### Decision 2: A watch round re-reads the configuration it depends on

- **Context**: `--watch` read the `jsx` option once and checked the `-p`
  one-source rule once, before the loop.
- **Alternatives considered**: Restart on configuration change; re-read
  each round.
- **Decision and rationale**: typescript-go's watcher re-reads the
  configuration each round when its files changed and rebuilds everything
  when the parsed options differ (`recheckTsConfig` in
  `internal/execute/watcher.go`). Each round now re-reads the `jsx` option
  (a few small file reads) and rebuilds every output when it changes, and
  checks the `-p` rule against the round's inputs, reporting a violation
  once and printing nothing until it is resolved.

### Decision 3: A configuration with syntax errors is still the configuration

- **Context**: The TypeScript host skipped a configuration whose JSON had
  a syntax error, so it never added the `.tt` content mapper, every `.tt`
  file left the program, TypeScript reported TS18003, and the files' type
  errors were hidden.
- **Alternatives considered**: Report only the syntax error and stop;
  use TypeScript's recovered configuration.
- **Decision and rationale**: TypeScript returns the recovered object with
  the first parse diagnostic (`ParseConfigFileTextToJson` in
  `internal/tsoptions/tsconfigparsing.go`, `handleReadConfigFile` in
  `internal/api/session.go`), and `tsc` checks the program it describes.
  The host now edits that configuration, and its JSON reader recovers the
  way TypeScript's list parser does (`parseDelimitedList` in
  `internal/parser/parser.go`): the end of the text ends an open list, a
  missing comma is passed over, and a `;` after an object member is
  skipped.

### Decision 4: Without TypeScript, the inputs are the program's roots

- **Context**: When the TypeScript layer could not run, the report
  covered every `.tt` file under the project root, including files that
  were neither inputs nor in the configured program.
- **Alternatives considered**: Keep the root walk; report the inputs and
  what they import.
- **Decision and rationale**: Membership comes from TypeScript's program
  plus the inputs named by request, extended through tt imports. Without
  TypeScript's answer the requested inputs are the only roots known, which
  is also what `--check` reports on.

### Decision 5: `--node` reaches every TypeScript client the run starts

- **Context**: Build and `-p` modes start the TypeScript client for
  contextual storage typing but rejected `--node`, so their output depended
  on the `node` on `PATH`.
- **Alternatives considered**: Stop using TypeScript in those modes (loses
  contextual typing); accept `--node` and pass it through.
- **Decision and rationale**: `Options` gains `node`, the contextual
  sessions are keyed by root and node, and the CLI and the server's `print`
  request pass their node.

### Decision 6: A failed output directory names the output and the cause

- **Context**: An `-o` that is a regular file printed `ttc: afile: File
  exists (os error 17)` once per input, naming neither the output nor the
  cause (an internal ownership record was named in one form).
- **Alternatives considered**: One message for the whole run; one per
  output.
- **Decision and rationale**: `tsc --outDir afile` reports TS5033 for each
  output it could not write, naming the file and `mkdir afile: not a
  directory`. Each output now reports its own path and the path that is
  not a directory.

### Decision 7: A directory input stays governed by `include`

- **Context**: `ttc --check-types other` checks nothing when the
  configuration's `include` leaves `other/` out.
- **Alternatives considered**: Make directory inputs roots; add a
  diagnostic.
- **Decision and rationale**: Unchanged. A directory input selects what an
  emitting pass writes, and only a named file is a root by request
  (`docs/ai/tt.md`), as `tsc` checks only what its configuration includes.

## Work log

- 2026-10-05: Reproduced each defect from the audit, read the matching
  typescript-go code, and fixed it in `src/main/build.rs`,
  `src/main/output.rs`, `src/main/ownership.rs`, `src/main/command.rs`,
  `src/main/typed.rs`, `src/main/modes.rs`, `src/server.rs`,
  `src/typescript/host.mjs`, `src/typescript/contextual.rs`,
  `src/lib/compile.rs`, and `src/engine/semantics/`.

## Issues and resolutions

### Issue 1: In-place directory build refused because of a hand-written `.ts`

- **Symptom**: `ttc src` with `main.tt` and `plain.ts` printed `src/plain.ts:
  output would overwrite the input — pass -o <dir>`, wrote nothing, and
  exited 1.
- **Cause**: The ownership pre-check refused every job whose output is its
  input before anything was compiled.
- **Resolution**: Decision 1.

### Issue 2: `--watch` kept a stale `jsx` option

- **Symptom**: After `"jsx"` changed to `"preserve"`, rebuilt importers
  still wrote `./c.js` for a `.ttx` import; a one-shot build wrote `./c.jsx`.
- **Cause**: `jsx_preserve` was computed once before `watch_mode`.
- **Resolution**: Decision 2.

### Issue 3: `-p -w <dir>` printed several modules

- **Symptom**: Adding a second file to the watched directory printed it
  after the first module on stdout.
- **Cause**: The one-source rule was checked only before the loop.
- **Resolution**: Decision 2.

### Issue 4: A configuration syntax error hid every `.tt` type error

- **Symptom**: `--check-types` reported TS18003 and TS1005 and no TS2322.
- **Cause**: `host.mjs` skipped a configuration whose `readConfigFile`
  answer carried an error.
- **Resolution**: Decision 3.

### Issue 5: The tt-only fallback reported files outside the inputs

- **Symptom**: Without TypeScript, `ttc --check-types src/a.tt` reported a
  duplicate arm in `other/o.tt`, which the configuration excludes.
- **Cause**: `typed_member_sources` returned no membership without a
  TypeScript answer, and the report then covered the whole root walk.
- **Resolution**: Decision 4.

### Issue 6: `--node` rejected where a TypeScript client runs

- **Symptom**: `ttc --node <path> -p a.tt` printed `--print does not
  combine with --node`; the output depended on `PATH`.
- **Cause**: The contextual backend was created with the default node.
- **Resolution**: Decision 5.

### Issue 7: A file as the output directory gave a misleading error

- **Symptom**: `File exists (os error 17)` for the directory, per input.
- **Cause**: `create_dir_all` reports `AlreadyExists` or `ENOTDIR`, and
  the message named the directory being created.
- **Resolution**: Decision 6.

## Regression test (fails before the fix)

- **Path**: `tests/integration/cases_03.rs`
  `cli_in_place_directory_build_keeps_unchanged_pass_through_inputs`;
  `tests/cli_outputs.rs` `watch_rebuilds_every_output_when_the_jsx_option_changes`
  and `print_watch_never_prints_a_second_module`; `tests/cli.rs`
  `types_checks_tt_inputs_under_a_configuration_with_syntax_errors`,
  `a_check_without_typescript_reports_only_the_inputs_it_was_given`,
  `build_and_print_run_the_typescript_client_with_the_named_node`, and
  `an_output_directory_that_is_a_file_is_named_for_each_output`.
- **Observed failure**: Without the source changes: the in-place build
  reported `plain.ts` (assertion at `cases_03.rs:83`); the `jsx` watch test
  timed out waiting for a rebuild; the print-watch test printed the second
  module; the configuration test found no `ts2322`; the fallback test found
  `other/o.tt` in stderr; the node test failed with `--print does not
  combine with --node`; the output-directory test got `Not a directory (os
  error 20)` without the paths.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (every suite passed; the public API baseline was the one
  change, below)
- [x] Baseline changes reviewed and committed with the change:
  `tests/baselines/reference/api/ttc.api.txt` gains `Options::node`

## Result

Six of the audit's defects are fixed and pinned by regression tests; a
directory input outside the configuration's `include` stays unchecked by
design (Decision 7). Changed: `src/main/{build,command,modes,output,ownership,typed}.rs`,
`src/server.rs`, `src/lib/compile.rs`, `src/typescript/{contextual.rs,host.mjs}`,
`src/engine/semantics/{report,translate}.rs`, `docs/ai/tt.md`, and the tests
named above.
