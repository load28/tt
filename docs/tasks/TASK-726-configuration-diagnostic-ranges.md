# TASK-726: Report a configuration diagnostic at its range in `tsconfig.json`

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-726`

## Purpose

TASK-717 D1: `tsc` places an option error at the option in `tsconfig.json`
(`tsconfig.json(1,34): error TS5067`), but `ttc --check-types` printed
`--> tsconfig.json` with no line, and the server answered line 0. Contract
2 makes the user's TypeScript errors TypeScript's to report, at the place
TypeScript reports them.

## Scope

- Included: how `src/typescript/host.mjs` serves a configuration it
  rewrites and where it reports a diagnostic inside one; the regression
  case; the CLI test that pinned the old behaviour;
  `tests/typed-parity-differences.txt`; notes on TASK-388 and TASK-410.
- Excluded: which diagnostics are reported (TASK-727, TASK-728).

## Sources modelled

Pinned: microsoft/TypeScript at `5739027c9a7df24e27123f453a50c011b37717b6`.

- `tsc/internal/compiler/program.go`, `verifyCompilerOptions` (line 860
  on): an option diagnostic is created with
  `tsoptions.CreateDiagnosticForNodeInSourceFile` on the property's name or
  initializer in the configuration's own JSON source file
  (`createOptionDiagnosticInObjectLiteralSyntax`), so its range is in the
  text TypeScript parsed.
- `tsc/internal/api/proto.generated.d.ts` as shipped in the pinned package
  (`DiagnosticResponse`): `pos`/`end` are offsets in that file.

## Decisions

### Decision 1: Serve a rewritten configuration as edits to its own text, and map TypeScript's range back through them

- **Context**: The host serves `tsconfig.json` rewritten: with its own
  identity content mapper in a configured project, the user's tt mapper
  entry removed in the other arrangement, and `.tt` patterns renamed there
  (TASK-410). It served `JSON.stringify(config)`, so TypeScript's positions
  were in text the user never wrote, and TASK-388 Decision 2 reported every
  diagnostic of such a file without a position.
- **Alternatives considered**: (a) Find the option's position again in the
  user's file from the diagnostic's message or code: a second, partial
  implementation of `verifyCompilerOptions`' placement, and a guess for
  every diagnostic it does not know. (b) Serve the user's file unchanged
  and give the mapper by another channel: the API has none; the
  configuration is how a project names a content mapper. (c) Write the
  served text as the user's text with each change an edit at a recorded
  place, and translate TypeScript's range through those edits.
- **Decision and rationale**: (c). The host reads the configuration's text
  (the engine's buffer or the disk), locates the members it changes with a
  JSON-with-comments reader (`jsoncTree`: objects, arrays, strings with
  either quote, literals, `//` and `/* */` comments, trailing commas), and
  applies the same changes as before as edits: a renamed pattern replaces
  its string, a filtered `contentMappers` replaces its value or removes the
  member with its comma, and the host's mapper replaces the member's value
  or is inserted after the opening brace (a blank file becomes
  `{"contentMappers":[...]}`). Everything else, comments included, keeps
  its offset up to the edits before it. A diagnostic in such a file is
  reported at its range mapped back (`originalSpan`): an offset before an
  edit moves by the edits before it, and one inside an edit lands on the
  edited text's original start (for a start) or end (for an end). A
  diagnostic wholly inside text the host inserted has nothing the user
  wrote to point at and stays positionless, as do diagnostics with no file
  or a negative position (TASK-388 Decision 2, now narrowed). The report
  already places a TypeScript diagnostic in a file that is not a lowered
  module by converting its UTF-16 offsets against that file's text
  (TASK-352 Issue 10), so the CLI, the server, and the editor's typed layer
  get the range with no change on the Rust side. A configuration
  TypeScript cannot read without an error is served as it was, unchanged.

## Work log

- 2026-10-01: Reproduced D1: `ttc --check-types --project tsconfig.json .`
  printed `--> tsconfig.json`; `tsc -p .` printed
  `tsconfig.json(1,34): error TS5067`.
- 2026-10-01: Read `verifyCompilerOptions` and the API's diagnostic
  response; replaced the host's `JSON.stringify` rewrite with recorded
  edits (`jsoncTree`, `editedText`, `originalSpan` in
  `src/typescript/host.mjs`) and routed a configuration diagnostic through
  `originalSpan`.
- 2026-10-01: Probed the reader against `api.readConfigFile` on comments,
  trailing commas, a byte-order mark, numbers, escapes, an empty file, and
  malformed inputs (Issue 1).
- 2026-10-01: Added the case, updated the CLI test that asserted the old
  positionless output, removed the D1 signatures from
  `tests/typed-parity-differences.txt`, and noted the narrowing on TASK-388
  and TASK-410.

## Issues and resolutions

### Issue 1: An empty configuration lost the host's content mapper

- **Symptom**: The reader returned nothing for an empty `tsconfig.json`,
  which TypeScript reads as `{}` without an error, so the host would have
  served it without its mapper.
- **Cause**: The reader required a value.
- **Resolution**: A text of only trivia is an empty configuration, and the
  mapper is inserted as a whole object; checked with an empty
  `tsconfig.json` whose `a.tt` imports `./b.tt` (TS2322 reported at
  `a.tt:2:14`).

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/anOptionErrorIsReportedWhereTsconfigWritesTheOption.tt`
  (`cargo test --test case_baselines`), a JSONC configuration with a
  comment and the documented tt mapper entry before `"jsxFactory": "id1 id2"`.
- **Observed failure**: with the previous `host.mjs`, the
  `.errors.txt` baseline differed: `-   --> tsconfig.json:12:19` /
  `+  --> tsconfig.json`. `tests/cli.rs`
  `types_reports_option_diagnostics_of_a_served_configuration_at_the_option`
  (renamed from `..._without_a_position`) asserts `--> tsconfig.json:2:34`.

## Verification

- [x] `TTC_TYPED_FILTER=<the 17 D1 cases> TTC_TYPED_CASES=all cargo test
  --test corpus typescript_cases_type_check_as_typescript_does`: passes
  with the list updated (12 lines removed; 5 lines keep their D2
  signature), command line and server agreeing.
- [x] `cargo test --test cli --test native --test workflow_repairs --test
  sidecar` (`TTC_REQUIRE_TSGO=1`): 127, 163, 13, 20 passed.
- [x] An unmapped project (a foreign `contentMappers` entry) reports
  TS6046 at `tsconfig.json:2:50` and TS100024 at `3:3`.
- [ ] The full gate (`cargo test` over every suite, `TTC_TYPED_CASES=all`, every fourslash test, `TT_MATRIX_CASES=all`, `./scripts/ci agents`): not run; the coordinator asked for targeted checks only. `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass.

## Result

Changed files: `src/typescript/host.mjs`, `tests/cli.rs`,
`tests/cases/compiler/anOptionErrorIsReportedWhereTsconfigWritesTheOption.tt`
and its baselines, `tests/typed-parity-differences.txt`,
`docs/tasks/TASK-388-typed-check-diagnostic-classes.md`,
`docs/tasks/TASK-410-typed-check-content-mappers.md`, `docs/tasks/INDEX.md`,
and this record. Every D1 difference is gone.
