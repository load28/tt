# TASK-701: Hold each diagnostic code to what the editor publishes and the command line reports

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-701`

## Purpose

A tt diagnostic is one fact shown by several consumers: the VS Code
adapter publishes it after merging its text, typed, service, and hint
layers (TASK-675), and `ttc --check` and `ttc --check-types` print it. The
editor cases recorded what the adapter publishes for a handful of codes,
and no test compared it with the command line. This task asks, for a sample
of every code TASK-700's matrix covers, whether the published diagnostic
has the code, range, severity, tags, and related information the rule
gives it, and whether both command-line modes report the same diagnostic.
Any difference between the surfaces is a defect.

## Scope

- Included: the editor sample in `scripts/generate-cases`
  (`tests/cases/editor/diagnostics/`), the `@expectDiagnostic` and
  `@typedOnly` directives and the surface check in `tests/editor_cases.rs`,
  `tests/editor-diagnostic-differences.txt`, their baselines, and
  CONTRIBUTING ("Adding an editor case").
- Excluded: fixing the defects found (recorded and listed instead).

## Sources

- LSP 3.17, `Diagnostic`: `range`, `severity` (1 = Error), `code`,
  `source`, `message`, `tags` (`DiagnosticTag.Unnecessary`/`Deprecated`),
  `relatedInformation`.
- TASK-675 (the adapter path of the editor cases) and TASK-700 (the spec
  the sample is drawn from).

## Design

For each code with an example in `tests/matrix/diagnostics.mjs`, the
generator writes the invalid program of the code's first example in its
first `.tt` position and, when the example runs in `.ttx`, its first
`.ttx` position, as an editor case that asks `@diagnostics` of the unit and
carries `@expectDiagnostic: <code>` and the `[|range|]` from the spec.

The runner asks the adapter as every editor case does (the published list
is in the baseline) and then checks:

- The published entries with a tt code are exactly the case's ranges, each
  with the case's code, severity 1, and no tags.
- `ttc --check` (not for `@typedOnly`) and `ttc --check-types`, run in the
  case's project, report as many tt diagnostics in the unit as the adapter
  publishes, and each published one appears there with the same code, the
  same message, the same start, for a one-line range the same caret width,
  and, as the labels the report draws (`---` under a quoted line, or
  `= note: ... --> file:line:col`), the published related information.
- A code whose report restates TypeScript's own syntax verdict
  (`DiagnosticCode::restates_typescript_syntax`: `verify-failed`,
  `source-not-typescript`) is published as TypeScript's diagnostic in its
  place (TASK-695), so for those the published TypeScript diagnostic
  (`source: ttc`, code `ts<number>`) must be at the range, the adapter must
  publish no tt diagnostic beside it, and the command line must report the
  code where it starts.

A difference fails the case unless `tests/editor-diagnostic-differences.txt`
lists it (case name, the failure's first line, task); a listed difference
that no longer occurs fails too.

## Counts

88 editor cases: one `.tt` case for each of the 46 codes with examples,
and a `.ttx` case for the 42 whose first example runs in `.ttx`.

## Decisions

### Decision 1: The command line is read from its rendered report, the editor from what it publishes

- **Context**: The three surfaces have to be compared on the facts a reader
  sees: the published LSP diagnostic, and the printed report.
- **Alternatives considered**: compare the engine's and the server's
  structured answers (TASK-700 already does, and they are not what a reader
  sees); compare whole rendered text (the editor has no rendering).
- **Decision and rationale**: the published entry is compared field by
  field with the report's header (code and message), its `-->` start, its
  caret run, and its labels, which are the facts the renderer draws from
  the same `Diagnostic` (`src/render.rs`).

### Decision 2: Related information is a label, not a defect by itself

- **Context**: The first version required no related information, and
  `val-mutation` publishes "the read-only binding is declared here", which
  `ttc --check-types` draws as a label under the declaration.
- **Decision and rationale**: related information must equal the labels
  the command line draws; a surface that leaves the label out is the
  defect (Issue 2).

### Decision 3: A restating code is published in TypeScript's words

- **Context**: The adapter publishes TS1109 for `verify-failed` and
  `source-not-typescript`, with `restates` naming the tt code (TASK-695:
  TypeScript's reader of the same text states the same fact).
- **Decision and rationale**: that is the documented behaviour, so the
  check holds TypeScript's diagnostic to the range and the command line to
  the same start; the range TypeScript gives is what exposed TASK-700
  Issue 7 on this surface too.

### Decision 4: Differences are listed per case with the failure's first line

- **Context**: The sample is small (88 cases), unlike the matrices.
- **Decision and rationale**: `tests/editor-diagnostic-differences.txt`
  lists each difference exactly; positions in the line are stable because
  the cases are generated.

## Work log

- 2026-10-01: Added the sample, the directives, the surface check, and the
  list; ran every case with `TT_CASES=<code>`.

## Issues and resolutions

### Issue 1: The untyped check names the variant in `match-not-exhaustive`, the typed check does not

- **Symptom**: For `match (s) { Circle(r) => ..., Rect(w, h) => ... }`
  over `variant Shape { Circle, Rect, Point }`, `ttc --check` reports
  "match on variant Shape is not exhaustive: missing \"Point\"", and `ttc
  --check-types` and the editor report "match is not exhaustive: missing
  \"Point\"". `src/diagnostics.rs` says the two pipelines share one
  renderer "so their wording cannot drift apart".
- **Resolution**: Listed for both sampled cases.

### Issue 2: The untyped check leaves out where the `val` binding is declared

- **Symptom**: For `val const cfg = {...}; cfg.n = 2;`, `ttc --check-types`
  draws the label "the read-only binding is declared here" under `cfg`'s
  declaration and the editor publishes it as related information; `ttc
  --check` reports the same diagnostic without it.
- **Resolution**: Listed for both sampled cases.

### Issue 3: Defects of TASK-700 seen on the editor's surface

- **Symptom**: TASK-700 Issue 6 (the second `source-not-typescript` for a
  crossing `yield` in JSX) is printed by both command-line modes but not
  published; TASK-700 Issue 7 (an arm body's missing operand one character
  early) puts the command line's start one column before TypeScript's
  diagnostic, which the editor publishes in its place.
- **Resolution**: Listed against TASK-700. TASK-700 Issue 5
  (`result-return-nested` missing from the tt-only typed check) does not
  show here: the adapter asks for the typed layer with its types.

## Regression test (fails before the fix)

Not applicable: this task adds tests and fixes no defect.

## CI cost

The 88 cases always run (they are not sampled): about 3.5 s of wall time
per code here, 2.7 minutes for all 46 codes run one `TT_CASES` at a time,
less in one unfiltered run that shares the workers.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `UPDATE_EXPECT=1 TT_REQUIRE_EXTENSION=1 TT_CASES=<code> cargo test
  --test editor_cases` for every code with the final check: 46 runs passed
  with the listed differences; a deliberately misplaced range was
  reported. The full gate is recorded in TASK-702.
- [x] Baseline changes reviewed and committed with the change (88 new
  `.baseline` files).

## Result

Changed: `scripts/generate-cases`, `tests/editor_cases.rs`,
`tests/editor-diagnostic-differences.txt` (new), `CONTRIBUTING.md`, the
generated cases under `tests/cases/editor/diagnostics/`, and their
baselines. Two surface defects are recorded (Issues 1 and 2).
