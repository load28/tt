# TASK-700: Generate a diagnostics matrix that holds every tt code to its range, and test the examples of `ttc explain`

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-700`

## Purpose

The case matrix (TASK-678 to TASK-686) runs every construct in every host
position against a TypeScript twin, but a rejected combination is only
checked for the presence of its code. No test said, for each diagnostic
code ttc can report, that a minimal invalid program reports exactly that
code at exactly the source range it is about, that the nearest valid
program compiles cleanly, or that the examples `ttc explain` prints are
true. This task adds that matrix and the explanation doc-tests, and records
what they find.

## Scope

- Included: `tests/matrix/diagnostics.mjs` (the spec), its generation in
  `scripts/generate-cases` into `tests/cases/conformance/diagnostics/`, the
  `@expectDiagnostic`, `@typedOnly`, `@expectClean`, and `@explains`
  oracles and the stratified sample in `tests/case_baselines.rs` and
  `tests/common/`, two new tests there (`every_diagnostic_code_has_cases`,
  `every_explanation_example_is_a_case`),
  `tests/diagnostic-codes-without-cases.txt`, `*` in
  `tests/oracle-failures.txt`, two harness helpers in
  `tests/matrix/harness.ts`, an erroneous example in the explanation of
  every reachable code (`src/diagnostics.rs`), CONTRIBUTING, and the name
  of the nightly step.
- Excluded: fixing the compiler defects the matrix finds (the instruction
  for this batch is to record and list them); the editor side, which is
  TASK-701.

## Sources

- rustc's error-code explanations: `compiler/rustc_error_codes/src/error_codes/E*.md`
  each carry an erroneous example fenced as `compile_fail,E0XXX`, and
  `src/tools/tidy/src/error_codes.rs` checks that every code's explanation
  has one and that rustdoc runs it and sees that code. The explanation
  doc-test here is that rule for `ttc explain`.
- TypeScript's fourslash ranges (`[|...|]`, `tests/cases/fourslash/` in
  microsoft/TypeScript), which the editor cases already use, mark the range
  a diagnostic must cover.
- TypeScript's conformance suite and NIST SP 800-142 (all-pairs), as cited
  by TASK-678, for placing one example in every host position.

## Design

### The spec

`tests/matrix/diagnostics.mjs` exports `diagnostics`, one entry per code,
and `extraPositions`. An entry has `examples` and `explain`.

An example is a value form, a statement form, or a whole program:

- `kind: "value"` / `"statement"` reuse `build` and the host positions of
  `tests/matrix/positions.mjs` (and their `.ttx` positions when `jsx` is
  set) with the plain companion. `bad` is the invalid form with its range
  marked `[|...|]`; `tt` and `ts` are the fixed form and its twin, written
  from the construct's documented semantics as the case matrix's twins are.
- `where` says where the rule applies. `accepts` (the default) places a
  rule about the construct itself wherever the position accepts the
  construct (`position.rejects[family]` unset). `rejects` places a
  placement rule exactly where the position rejects the construct with
  this code, plus the unbraced position for a form whose `unbraced` names
  it. `all` places `if-let-placement`, which every value position applies.
- A placement example's fix lifts the construct into a declaration before
  its host (the explanations' "declare the variable before the statement" /
  "move the propagation into the nearest Result scope"); `fixes` names
  another form where lifting is not the fix (a `result` block for a `try`
  in a generator or at module top level, braces for an unbraced
  declaration), and `bads` another range where the rule reports a larger
  one (the whole unbraced statement).
- `kind: "module"` is a whole program per surface (`main.EXT`), with a
  runnable fix and twin (`run`) or a fix that only has to compile cleanly.

`extraPositions` adds the hosts the `match-placement` and `try-placement`
explanations name that the case matrix has no position for: an enum member
initializer, a class member's computed name, a class decorator's argument,
the heritage of a decorated class, and a later declarator of a C-style
`for` head.

### The oracle

An invalid case carries `@expectDiagnostic: <code>` and the marked ranges.
The runner strips the markers, compiles as for every case, and then asks
four surfaces: `ttc --out-dir` and `ttc --check-types` (their rendered
reports: the code and where each starts) and `ttc --server`'s `check` and
`typedCheck` for every `.tt`/`.ttx` unit (the code and the whole range).
Each surface must report exactly the marked ranges with that code and no
other tt code; a surface that reports a position without an end (the
protocol's `endLine: 0`) is compared by its start. `@typedOnly: true`
names a rule only the checker can decide (`result-return-nested`), which
the untyped surfaces must not report. TypeScript's own diagnostics are not
tt codes and are not judged; they stay in the `.errors.txt` baseline.

A disagreement is reported without positions, so one `*` line in
`tests/oracle-failures.txt` lists a defect that every position of an
example shows: the codes a surface reports when they are not the expected
one, how many times when the count differs, or how far the range moved
(`start +0:-1`).

A fixed case runs against its twin (`@run`/`@twin`, which already requires
a clean compile); a fixed module without a twin carries `@expectClean:
true`.

### The explanation doc-test

Every block indented by four spaces in `DiagnosticCode::explanation()` is
an example. `every_explanation_example_is_a_case` requires each block to be
held by a generated case with `@explains: <code> <n>`, whose trimmed lines
contain the block's trimmed lines contiguously, and which either reproduces
the code (`@expectDiagnostic`, with its range) or compiles cleanly
(`@expectClean`); and it requires every code not listed in
`tests/diagnostic-codes-without-cases.txt` to have one block that
reproduces it. Ten explanations had blocks before this task, eight of them
fixes only; this task adds an erroneous example to every reachable code's
explanation.

`every_diagnostic_code_has_cases` walks `DiagnosticCode::ALL` and fails for
a code no case expects unless the list names it, and for a listed code that
has a case.

### Sampling and baselines

Every generated case keeps only `.errors.txt` (absent for a fixed case).
A pull request runs every explanation case and one invalid and one fixed
case of each code, chosen by a fixed seed (`matrix::stratified`;
`TT_MATRIX_SEED` changes it); `TT_MATRIX_CASES=all`, which the nightly
`exhaustive` job sets, runs all of them, as `TT_CASES=<fragment>` runs
every matching one.

## Counts

`node scripts/generate-cases --stats`: 2,830 cases (1,388 invalid, 1,388
fixed, 54 explanation examples) over 46 codes. Invalid cases per code (the
fixed cases are as many):

| Code | Invalid | `.ttx` | Explain |
| --- | ---: | ---: | ---: |
| stray-pipe | 48 | 8 | 2 |
| malformed-pipeline-postfix | 48 | 8 | 1 |
| missing-pipeline-step | 48 | 8 | 1 |
| invalid-optional-receiver | 2 | 1 | 1 |
| stray-if-let | 17 | 1 | 1 |
| malformed-variant | 2 | 1 | 1 |
| malformed-match | 42 | 8 | 1 |
| missing-arm-body | 42 | 8 | 1 |
| result-no-success-value | 48 | 8 | 1 |
| result-value-discarded | 17 | 1 | 1 |
| result-return-nested | 48 | 8 | 1 |
| result-break-crossing | 17 | 1 | 1 |
| result-continue-crossing | 17 | 1 | 1 |
| result-yield-crossing | 47 | 8 | 1 |
| result-label-crossing | 17 | 1 | 1 |
| flow-first-step-method | 38 | 0 | 2 |
| try-placement | 21 | 0 | 1 |
| try-crosses-value-region | 48 | 8 | 1 |
| let-else-placement | 2 | 0 | 1 |
| let-else-not-diverging | 16 | 1 | 1 |
| if-let-placement | 53 | 8 | 1 |
| variant-duplicate-case | 2 | 1 | 1 |
| variant-invalid-field-type | 2 | 1 | 1 |
| variant-field-shadows-tag | 2 | 1 | 2 |
| variant-required-after-optional | 2 | 1 | 2 |
| variant-default-export | 2 | 1 | 2 |
| pattern-duplicate-binding | 42 | 8 | 2 |
| match-mixed-patterns | 42 | 8 | 1 |
| match-wildcard-not-last | 42 | 8 | 1 |
| match-or-literal-kind-mismatch | 42 | 8 | 1 |
| match-duplicate-arm | 42 | 8 | 1 |
| match-is-wildcard-required | 42 | 8 | 1 |
| match-is-empty-bindings | 42 | 8 | 1 |
| match-is-or-bindings | 42 | 8 | 1 |
| match-placement | 11 | 0 | 1 |
| match-control-crossing | 17 | 1 | 1 |
| match-nested-in-or-pattern | 43 | 8 | 2 |
| match-or-binding-mismatch | 42 | 8 | 2 |
| match-tuple-arity | 42 | 8 | 1 |
| unknown-case | 42 | 8 | 1 |
| unknown-field | 42 | 8 | 1 |
| match-not-exhaustive | 42 | 8 | 1 |
| val-mutation | 17 | 1 | 1 |
| val-pass | 17 | 1 | 1 |
| verify-failed | 2 | 1 | 1 |
| source-not-typescript | 87 | 16 | 1 |
| stray-result, lowering-plan-failed, other | 0 | 0 | 0 |

The three codes without a case are listed in
`tests/diagnostic-codes-without-cases.txt` (Issues 2 and 3, Decision 6).

## Decisions

### Decision 1: One spec drives the cases, the explanation cases, and the editor sample

- **Context**: The codes' examples, their fixes, and the explanation
  programs had to live somewhere the generator, the case runner, and the
  explanation check all read.
- **Alternatives considered**: hand-written case files per code (thousands
  of near-identical files nobody edits consistently); a table in Rust next
  to `explanation()` (the case runner and the generator would read two
  specs).
- **Decision and rationale**: `tests/matrix/diagnostics.mjs`, generated by
  the script the case matrix already uses, so the positions, the harness,
  the twins, and the drift check (`--check`) are shared. The explanation
  examples are repeated in the spec, and the test compares them with
  `explanation()` line for line, so neither can drift from the other.

### Decision 2: The range is checked on four surfaces, the whole range on the two that carry it

- **Context**: `.errors.txt` shows the rendered report, which carries a
  start and a caret run but no reliable end for a multi-line range.
- **Alternatives considered**: parse the carets (fragile across multi-line
  pictures); check one surface only (a surface that drifts would go
  unseen).
- **Decision and rationale**: the server's `check` and `typedCheck` answer
  with whole ranges, and the command line's two reports are compared by
  start. A position-only report (no end) is accepted by its start, as the
  protocol documents that the consumer then chooses the width.

### Decision 3: Placement rules are placed where the position rejects, with the fix the explanation names

- **Context**: A placement code is a fact about a position, not about a
  construct; placing it everywhere would make most of its cases valid.
- **Decision and rationale**: `where: "rejects"` reads the same `rejects`
  tables the case matrix uses, so the two matrices cannot disagree about
  where a rule applies, and the fixed program is the explanation's fix:
  the construct lifted before its host, braces for an unbraced declaration,
  a `result` block where no function can be the Result scope.

### Decision 4: A disagreement is described without positions, and `*` lists one defect across positions

- **Context**: A defect such as Issue 5 shows in every position of an
  example (48 cases); one line per case would bury the list.
- **Alternatives considered**: per-case lines with positions (48 lines that
  change with every spec edit).
- **Decision and rationale**: the observation names codes, counts, or how
  far the range moved, and a `*` line excuses each case it matches that
  observes the same thing. A run of every case a `*` line matches fails
  when none observes it any more, as a plain line fails when its case
  agrees; a case the pattern matches that observes something else still
  fails.

### Decision 5: Every reachable code's explanation gets an erroneous example

- **Context**: rustc requires one per error code; ttc had ten explanations
  with blocks, eight of them only fixes.
- **Decision and rationale**: an example is added near the top of each
  reachable explanation, written as a reader would meet the error, and is
  generated as a case with its range. The two explanations whose existing
  examples did not compile as described are corrected (Issue 9).

### Decision 6: `other` is listed, not reproduced

- **Context**: `DiagnosticCode::Other` exists so an unclassified
  `TtError` still has a code; every reporting site sets its own.
- **Decision and rationale**: listed in
  `tests/diagnostic-codes-without-cases.txt` with that reason; a reporting
  site that starts producing it will need a case and fails the list check
  only once one exists, which is the review point.

### Decision 7: Spec errors are fixed in the spec, compiler defects are listed

- **Context**: The first full run disagreed in 300-odd cases.
- **Decision and rationale**: each disagreement was read. A wrong spec
  (a TypeScript error the twin also has, a type annotation, a script-scope
  name clash) was fixed in the spec; a disagreement with the language's
  documented rules is a defect, recorded below with a minimal repro and
  listed in `tests/oracle-failures.txt` against this task. Two positions
  are left out of `source-not-typescript`'s examples (`optionalCall`,
  `topLevel`), where the form occurs twice and a file whose TypeScript does
  not parse is reported once.

## Work log

- 2026-10-01: Reset onto `claude/ecstatic-dijkstra-qw5pf9` (b3a1c9e),
  `npm ci` at the root and in `editors/vscode`, fetched the TypeScript
  cases, built the tests and the extension's server.
- 2026-10-01: Enumerated the 49 codes (`DiagnosticCode::ALL`) and probed a
  minimal repro of each with `ttc --check` / `--check-types`.
- 2026-10-01: Added the oracle and the sample to the case runner, the spec,
  and the generator; ran every code with `TT_CASES=<code>` and
  `UPDATE_EXPECT=1`; triaged each disagreement (Decision 7).
- 2026-10-01: Added the explanation examples and the doc-test; corrected two
  explanations (Issue 9).

## Issues and resolutions

### Issue 1: A malformed postfix pipeline in a construct's body is reported twice

- **Symptom**: `if let Ok(value: n) = read(1) { const v = n |> ?.toFixed`1`; }`
  (or the same pipeline in a match arm) reports
  `malformed-pipeline-postfix` at the template and also
  `source-not-typescript` at the `|>`.
- **Cause**: The pipeline blocks projection, and the construct's body,
  which still holds the `|>`, is then lowered as TypeScript.
- **Resolution**: Listed (`malformed-pipeline-postfix_taggedTemplate_ifLetBody`,
  `..._matchArm`); not fixed here.

### Issue 2: An expression the `result` block's `}` cuts off fails the lowering plan

- **Symptom**: `result { return try read(x) + }` reports
  `lowering-plan-failed` ("the generated TypeScript for it does not parse:
  Expression expected") instead of `source-not-typescript` at the `}`.
  Without the `return`, it also reports `result-no-success-value`.
  `result { const n = try read(x) *; return n; }` reports
  `source-not-typescript` correctly.
- **Cause**: The body is parsed as TypeScript only after the lowering plan
  is built from it.
- **Resolution**: Listed (`source-not-typescript_resultBodyEnd_declarationInitializer`).
  It is the one path to `lowering-plan-failed` the matrix reaches, whose
  explanation calls every report a gap, so that code is in the
  without-cases list with this reason.

### Issue 3: `stray-result` has no reporting site

- **Symptom**: No program reports it.
- **Cause**: `program.stray_results` (`src/parser/parse.rs`) is created
  empty and never filled, so `sema/checker.rs`'s loop over it never runs;
  a `result` block the parser cannot claim passes through and is reported
  as `source-not-typescript` or `verify-failed`.
- **Resolution**: Listed in `tests/diagnostic-codes-without-cases.txt`.

### Issue 4: A nested alternative's bindings are not counted when or-pattern bindings are compared

- **Symptom**: `match (o) { Done(value: Some(value: v)) | Failed(error: v) => v, _ => 0 }`
  reports `match-nested-in-or-pattern` and also `match-or-binding-mismatch`
  ("`v` is bound in `Failed(...)` but not in `Done(...)`"), which is false.
- **Resolution**: Listed (`match-nested-in-or-pattern_bindingAlternative_declarationInitializer`);
  the matrix's main example binds nothing, which reports the one code.

### Issue 5: The tt-only typed check drops `result-return-nested`

- **Symptom**: `ttc --check-types` reports `result-return-nested` for
  `result { const n = try read(x); return read(n); }`, and `ttc
  --check-types --tt-only` and the server's `typedCheck` (whose default is
  tt-only) report nothing. `--tt-only` is documented as "the tt layer of
  --check-types".
- **Resolution**: Listed as `result-return-nested_*`.

### Issue 6: A crossing `yield` in a template or JSX expression is also reported as unparsable

- **Symptom**: `` `${[...(function* () { const r = result { const n = try read(1); yield n; return n; }; })()]}` ``
  reports `result-yield-crossing` and `source-not-typescript` at the same
  `yield`; in other positions only the first.
- **Resolution**: Listed (five positions: a template literal and four JSX
  attribute or child positions).

### Issue 7: A match arm's missing operand is reported one character early

- **Symptom**: `match (n) { 1 => n +, _ => 0 }` reports
  `source-not-typescript` at the `+`; TypeScript reports TS1109
  "Expression expected" at the `,`, where the expression is missing, and
  so does ttc for the same text in a `result` block (`*;` at the `;`).
- **Resolution**: Listed as `source-not-typescript_armBody_*` and
  `source-not-typescript_explain1`.

### Issue 8: A `try` in a binary operand of a `result` block in a conditional operand emits TypeScript that does not parse

- **Symptom**: `return flip() && result { const n = try read(x) * 2; return n; };`
  reports `verify-failed` ("generated TypeScript failed to parse:
  Expression expected"); `try read(x)` as a whole initializer in the same
  place compiles. The same holds in `||`, `??`, a conditional branch, and
  the JSX conditional positions.
- **Resolution**: Listed (six `source-not-typescript_resultBody_*_fixed`
  cases).

### Issue 9: Two explanations showed fixes that do not compile

- **Symptom**: `ttc explain match-or-binding-mismatch` suggested
  `Circle(radius: r) | Square(side: r) => r`, which the rule rejects
  ("`r` is bound from field `radius` in `Circle(...)` but from field `side`
  in `Square(...)`"; `docs/ai/tt.md`: the alternatives bind the same
  (field, name) set). `ttc explain match-nested-in-or-pattern` wrote
  `Ok(value: Some(v)) => v`, which binds a field `v` that `Some(value)`
  does not have, so TypeScript reports TS2339 on the lowered code.
- **Resolution**: The explanations now split the alternatives into arms
  and write `Some(value: v)`; the doc-test holds both.

### Issue 10: The first run's spec errors

- **Symptom**: Fixed cases that did not compile: a function or array value
  in a conditional test (TS2872), `number | string[]` for an array of
  `number | string`, `unknown` storage for an enum member and a computed
  name, a unit constructor typed as its own case in a tuple scrutinee
  (TS2367), a variant named `Request` in a script (TS2451 against the DOM
  lib), an unreplaced `.EXT` specifier.
- **Resolution**: Fixed in the spec and the generator.

## Regression test (fails before the fix)

Not applicable: this task adds tests and fixes no compiler defect. The
explanation corrections are held by `every_explanation_example_is_a_case`.

## CI cost

Measured here (4 cores, debug build, `RUST_TEST_THREADS` unset, the case
runner's four workers): all 2,830 cases ran in 1,321 s of wall time over
46 `TT_CASES=<code>` runs, 0.47 s per case. A pull request runs the 54
explanation cases and one invalid and one fixed case of each of the 46
codes, 146 cases, about 70 s more than before; the nightly `exhaustive`
job (release build) runs all 2,830 beside the case matrix's 3,286. The
baselines are 1,440 `.errors.txt` files (5.9 MB); the cases are 12 MB.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `UPDATE_EXPECT=1 TT_CASES=<code> cargo test --test case_baselines every_case`
  for every code, then the same without `UPDATE_EXPECT` for every code
  (46 runs passed, the listed defects observed as
  listed); `cargo test --test case_baselines every_` (the two new tests);
  `cargo test --lib diagnostics`; `cargo test --test compile explanation`
  (the explanation test updated for Issue 9); `cargo test --test cli
  explain`; `node scripts/generate-cases --check`. The full gate over
  TASK-700 to TASK-702 is recorded in TASK-702.
- [x] Baseline changes reviewed and committed with the change (one
  `.errors.txt` per invalid and explanation case, and the six fixed cases
  of Issue 8).

## Result

Changed: `tests/matrix/diagnostics.mjs` (new), `tests/matrix/harness.ts`,
`scripts/generate-cases`, `tests/case_baselines.rs`,
`tests/common/{cases,matrix}.rs`, `tests/compile/cases_11.rs`,
`tests/oracle-failures.txt`, `tests/diagnostic-codes-without-cases.txt`
(new), `src/diagnostics.rs` (explanations), `CONTRIBUTING.md`,
`.github/workflows/ci.yml` (step name), the generated cases under
`tests/cases/conformance/diagnostics/` and their baselines under
`tests/baselines/reference/diagnostics/`. Every code `ttc explain` lists
has cases or a recorded reason it has none; eight compiler defects are
recorded and listed (Issues 1 to 8), and two explanations are corrected.
TASK-701 asks the editor and the command line about the same sample.
