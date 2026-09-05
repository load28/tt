# TASK-335: Read the compiler's rendered diagnostics in the one-shot fallback

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-335: read the compiler's rendered diagnostics in the fallback`

## Purpose

When the editor falls back to a one-shot `ttc --check`, it parses a
diagnostic format the compiler no longer prints, so every real error is
dropped and the file appears clean.

## Scope

- Included: `parseStderr` in `editors/vscode/server/src/ttc.ts` and the
  callers that decide a run "failed" because it produced no diagnostics.
- Excluded: The engine session lifecycle (TASK-334) and what the server
  publishes when a typed layer cannot answer.

## Decisions

### Decision 1: Parse the format the compiler actually renders

- **Context**: `parseStderr` accepts only `ttc: <file>:<line>:<col>: <msg>`.
  The CLI renders `error[<code>]: <message>` followed by ` --> file:line:col`
  and a source excerpt; `ttc: ` now prefixes only CLI usage errors. So the
  parser matches nothing, `runCheckOnce` sees a non-zero exit with no
  diagnostics, and reports the run as a crash.
- **Alternatives considered**: Ask the fallback to use `--server` (that is
  the very thing the fallback exists without); add a JSON flag to the CLI
  (a new compiler surface for an editor-internal need).
- **Decision and rationale**: Read the rendered form, which is the CLI's
  documented output and is already stable enough to be snapshot-tested
  (`tests/fixtures/diagnostic/*/expected.stderr`). Keep accepting the legacy
  `ttc: file:line:col:` shape so an older compiler still works. The rendered
  form also carries the rule code and the caret extent, so the fallback now
  reports the same identity and range the engine path does.

### Decision 2: A run that reported diagnostics is not a failed run

- **Context**: `runCheckOnce` treats "non-zero exit and nothing parsed" as a
  crash. That is right, but it only ever held because nothing parsed.
- **Decision and rationale**: Unchanged in shape, but now reached only when
  the compiler really said nothing a reader could act on.

## Work log

- 2026-09-05: Confirmed against the current compiler that no line of a
  diagnostic run starts with `ttc: `, so the parser returns nothing for
  every error.

## Issues and resolutions

### Issue 1: Real errors reach the editor as an empty Problems panel

- **Symptom**: With a compiler the engine has given up on (two strikes) or
  one without `--server`, a file full of tt errors publishes zero
  diagnostics, and the output channel logs the compiler's own rendered error
  text as `exited abnormally`.
- **Cause**: Decision 1's format drift, with no test covering `parseStderr`.
- **Resolution**: Parse the rendered form, and cover both formats plus the
  no-diagnostic cases with unit tests.

## Verification

- [x] `parseStderr` unit coverage: rendered form, a wider line number's
  alignment, several diagnostics in one run, other files left alone, the
  legacy form, usage/progress lines, and warnings
- [x] End-to-end against the real compiler through a shim that refuses
  `--server`: `runCheck` returns `2:35 [match-not-exhaustive] …`
- [x] `./scripts/ci extension`: 135 passed, zero failed/cancelled/skipped
- [x] `./scripts/ci`

## Result

Changed files: `editors/vscode/server/src/ttc.ts`,
`editors/vscode/server/src/test/parse.test.ts`, and this record. The
fallback reads the compiler's rendered diagnostics — position, message and
rule code — and still accepts the legacy one-line form.
