# TASK-680: Populate the case matrix for pipelines, flow, val, variant, and .ttx positions

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-680`

## Purpose

Extend the TASK-678 matrix to the remaining constructs (`|>`, `flow`,
`val`, `variant`) and to `.ttx`, where tt constructs stand in JSX
expression containers and in function components, each run against a
`.tsx` twin; report every disagreement as TASK-679 did.

## Scope

- Included: `tests/matrix/{pipeline,flow,val,variant}.mjs`; `jsx: true` on
  the TASK-679 constructs and a `.ttx`-only `match` form whose arms render
  elements; JSX positions and a component-body position in
  `tests/matrix/positions.mjs`; the `.ttx` surface in
  `scripts/generate-cases` (`main.ttx`, `twin.tsx`, a JSX `tsconfig.json`,
  the harness's `element` factory); `lowPrecedence` and `operand` in the
  spec; 1,669 new cases and their baselines; two new lines in
  `tests/oracle-failures.txt`; `CONTRIBUTING.md` ("The case matrix").
- Excluded: fixing defects (none new was found; see Result).

## What the spec covers

| Construct | Surface | Forms | Positions | Cases | Run against a twin | Expect a diagnostic |
| --- | --- | --- | --- | --- | --- | --- |
| `\|>` | `.tt` | 11 | 40 value | 441 | 441 | 0 |
| `\|>` | `.ttx` | 11 | 8 JSX | 88 | 88 | 0 |
| `flow` | `.tt` | 9 | 39 value | 316 | 299 | 17 |
| `val` | `.tt` | 12 | 16 statement | 162 | 128 | 34 |
| `val` | `.ttx` | 12 | component body | 84 | 56 | 28 |
| `variant` | `.tt` | 12 | 16 statement | 155 | 124 | 31 |
| `variant` | `.ttx` | 11 | component body | 63 | 42 | 21 |
| `match` | `.ttx` | 15 | 8 JSX | 120 | 120 | 0 |
| `try` | `.ttx` | 6 | 8 JSX | 64 | 56 | 8 |
| `result` | `.ttx` | 7 | 8 JSX | 64 | 56 | 8 |
| let-else | `.ttx` | 7 | component body | 49 | 49 | 0 |
| `if let` | `.ttx` | 7 | component body | 49 | 49 | 0 |
| `try` statement | `.ttx` | 2 | component body | 14 | 14 | 0 |
| **Total** | | | | **1,669** | **1,522** | **147** |

With TASK-679 the matrix holds 3,286 cases: 2,820 run against a twin and
466 expect a diagnostic.

- Pipeline forms: a named-function step, a chain of function and postfix
  steps, a postfix step, a step expression evaluated after the piped value,
  a member step called on its receiver after the value (`this` read), an
  optional-chain member step that runs and one that short-circuits, an
  optional postfix tail, a parenthesized arrow step, a step extending over
  `??`, an `as` assertion ending a step, and curried `@tt/std/option`
  steps.
- `flow` forms: two steps, a first step that is a call evaluated at
  composition, a single step, a later postfix step, a later member step and
  a later optional-chain step whose receivers are read at composition, a
  member step first (bound to its receiver), and a postfix or optional
  first step (`flow-first-step-method`).
- `val` forms: a `const` read through its paths, a `let` rebound, `val`
  parameters of a function and an arrow, a `for-of` binding, a `catch`
  binding, a spread copy that is then mutated, an alias whose original is
  mutated; and, as diagnostics, a property write and a nested increment
  (`val-mutation`), a pass to a mutable parameter (`val-pass`), and
  `items.push` on a `val` array, which only the typed check reports
  (`val-mutation`). The twin is the same program with `val` erased, which
  is the documented runtime meaning.
- `variant` forms: unit cases, payload cases, an optional field set only
  when its argument is not `undefined`, empty parentheses, a generic
  recursive variant (`.tt` only: a `<T>(` arrow in the `.tsx` twin would be
  JSX), a case tagged `kind`, a variant matched where it is declared, an
  exported variant; and, as diagnostics, a field named `kind`, a required
  field after an optional one, a duplicate case, and `export default
  variant`. The twin is the type alias and constructor object that
  `tests/matrix/variants.mjs` writes from the documented lowering, so the
  printed JSON pins property order and omitted optional fields.
- JSX positions (8): an attribute value, a child, `&&` and `? :`
  conditional rendering, a fragment child between two others, a spread
  attribute's object, a component's prop, and a nested element's child
  after a sibling; plus a function component's body for the statement
  constructs.

## Decisions

### Decision 1: `.ttx` runs through a string-rendering JSX factory

- **Context**: `.ttx` cases must execute with `.tsx` twins, and the
  default configuration keeps JSX (`jsx: preserve`), which Node cannot run.
- **Alternatives considered**: React as a dependency: a runtime the tests
  do not otherwise need, whose output (objects with `$$typeof`) prints
  poorly.
- **Decision and rationale**: `.ttx` cases carry a `tsconfig.json` with
  `jsx: react`, `jsxFactory: element`, `jsxFragmentFactory: Fragment`, and
  `verbatimModuleSyntax` (Issue 2). The harness's `element` renders an
  element to a string (`<i title='3'>4</i>`, attribute values and non-text
  children as JSON with `'` for `"`, so the runner's `\`-to-`/`
  normalization cannot alter them), declares its `JSX` namespace on the
  factory (`element.JSX`, which TypeScript consults before a global one),
  and `Show` is a function component. `// @run` and `// @twin` accept
  `.ttx`/`.tsx` units (TASK-678).

### Decision 2: Pipelines and `flow` are parenthesized where they are an operand

- **Context**: `|>` binds looser than `&&`, `||`, `??`, `? :`, and
  comparisons, so `flip() && x |> f` is `(flip() && x) |> f`, and a step
  followed by `?` is a compile error (`docs/ai/tt.md`, "|>": "PARENTHESIZE
  ternaries & arrows at head/step top level").
- **Decision and rationale**: positions that make the value an operand of
  such an operator are marked `operand`, and a construct marked
  `lowPrecedence` is written in parentheses there, as a user would write
  it. The first run without them produced 168 disagreements that were the
  spec's, not ttc's (Issue 1).

## Work log

- 2026-09-30: Wrote the four spec modules, the JSX positions, and the
  `.ttx` surface; generated 1,663 new cases. The first full run of all
  3,280 took 998 seconds; 228 cases disagreed (Issues 1 to 4).
- 2026-09-30: Fixed the spec (Issues 1, 3, 4) and the configuration
  (Issue 2); rerun of all 3,286 in 1,029 seconds: three disagreements,
  one a spec error (Issue 5) and two TASK-679 Issue 4 in JSX positions.
- 2026-09-30: Listed the two; ran the whole matrix and the gate.

## Issues and resolutions

### Issue 1: Pipelines written unparenthesized as operands

- **Symptom**: `stray-pipe` ("a step is an expression — parenthesize a
  ternary") for `x |> f ? a : b`, and runs that disagreed in loop tests
  (`rounds < 1 && x |> (n => …)` ran the arrow on `false` and looped
  again).
- **Cause**: The spec, not ttc: the documented precedence (Decision 2).
- **Resolution**: `operand` positions and `lowPrecedence` constructs.

### Issue 2: The JSX factory import was elided in fragment-only programs

- **Symptom**: every `jsxFragment` case printed `threw element is not
  defined` on both sides, with the logs differing in when it threw.
- **Cause**: TypeScript elides the `jsxFactory` import when a file's only
  JSX is a fragment (it marks only the fragment factory as referenced), so
  `element` was never imported. The two sides then failed at different
  points: the twin resolves the callee `element` before its arguments, and
  ttc's lowering had already run the children held in its storage.
- **Resolution**: `verbatimModuleSyntax` in the `.ttx` configuration, so
  no import is elided.

### Issue 3: The optional-chain `flow` twin skipped the previous step

- **Symptom**: `flow_optionalLater_*` disagreed on the short-circuiting
  input: ttc logged `add 12`, the twin did not.
- **Cause**: The twin was `receiver?.twice(first(v))`, which skips
  `first(v)` when the receiver is absent. The documents say a later
  optional-chain step is "the optional call `o?.m(v)` on each input", where
  `v` is what the previous step produced, so that step runs first; ttc was
  right.
- **Resolution**: The twin runs the previous step and then the optional
  call: `((w) => receiver?.twice(w))(first(v))`.

### Issue 4: A member step first was expected to be `flow-first-step-method`

- **Symptom**: `flow |> tools.twice |> …` compiled cleanly.
- **Cause**: The spec misread the rule: a "method step" is one starting
  with `.` (`ttc explain flow-first-step-method`: "a method step (one
  starting with `.`) or an optional-chain step"); a member step
  (`tools.twice`) is bound to its receiver and allowed first.
- **Resolution**: `memberFirst` is a runnable form; `postfixFirst`
  (`flow |> .toFixed(1)`) and `optionalFirst` (`flow |> maybe(true)?.twice`)
  are the diagnostic forms, and both report the rule.

### Issue 5: A composed function in a condition is TypeScript's TS2774

- **Symptom**: `flow_oneStep_conditionalTest_plain` did not compile
  (`This condition will always return true since this function is always
  defined`) on both sides.
- **Cause**: The form was `note(...) && (flow |> double)`, whose value is
  the named function `double`; TypeScript rejects testing it.
- **Resolution**: The single-step form is `flow |> adder(x)`. `flow` also
  skips the template-literal position, where a function's value is its
  source text, which differs between any two implementations.

### Issue 6: TASK-679's Issue 4 also holds in JSX containers

- **Symptom**: `try_siblings_jsxConditional_plain` and
  `try_siblings_jsxTernary_await` do not compile: `(try a) + (try b)` under
  `&&` and `? :` inside `{...}` is `try-placement`, like the same operation
  in a `.tt` file.
- **Resolution**: Listed under TASK-679 Issue 4.

## Regression test (fails before the fix)

Not applicable: this task adds cases and fixes no defect. Every defect it
observed is TASK-679's Issue 4, pinned by the two new lines of
`tests/oracle-failures.txt`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] Full gate on the tree merged with `claude/ecstatic-dijkstra-qw5pf9`
  at `ba58c12` (TASK-677), with every matrix case:
  `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 TTC_REQUIRE_TYPESCRIPT_CASES=1
  TT_REQUIRE_EXTENSION=1 TT_MATRIX_CASES=all TT_BASELINE_TRACKING_DIR=<dir>
  cargo test --no-fail-fast`: 1,796 passed, 0 failed, no `SKIP`, 1,487
  seconds; `case_baselines` (3,286 matrix cases and the hand-written ones)
  1,027 seconds.
- [x] `node scripts/check-baselines --tracking <dir>`: "baselines: 9250
  compared, none unused"; `tests/baselines/local/` stayed empty.
- [x] `node scripts/check-baselines --run` (the default sample, as a pull
  request runs it): "baselines: 642 compared, 3198 of unsampled matrix
  cases left unjudged, none unused", 77 seconds.
- [x] `./scripts/ci agents` passed (warnings: rolldown not on PATH, doctor
  reports the checkout not ready; both environmental).
- CI time (debug build, four workers, this container): a pull request's
  `case_baselines` runs 120 sampled matrix cases, about 40 seconds more
  than without the matrix; the nightly step runs all 3,286 in about 1,030
  seconds (0.31 seconds of wall time per case), within the `exhaustive`
  job's new 75-minute limit.
- [x] `node scripts/generate-cases --check`: "3286 generated cases match
  tests/matrix".
- [x] Baselines reviewed: the `.ttx` renders (`<p><b>2</b></p>`,
  `<>[...]</>`, `<Show …>`), pipeline and `flow` logs (value before step
  and receiver; receivers read at composition; nothing past a
  short-circuit), `val` output equal to the program without `val`, and the
  variant JSON (`{"kind":"Get","url":"a"}` without `timeout`).

## Result

Changed files: `tests/matrix/{pipeline,flow,val,variant}.mjs`,
`tests/matrix/{match,try,result,letElse,ifLet}.mjs` (`jsx: true`, the
`jsxArms` form), `tests/matrix/{positions.mjs,harness.ts}`,
`scripts/generate-cases`, `tests/cases/conformance/matrix/` (1,669 new
cases), `tests/baselines/reference/matrix/` (1,673 new baselines),
`tests/oracle-failures.txt`, `CONTRIBUTING.md`, `scripts/check-baselines`
(the wording of its unsampled count), `docs/tasks/INDEX.md`, and this
record. No new compiler defect was found in pipelines, `flow`, `val`,
`variant`, or `.ttx`: every runnable case agrees with its twin and every
diagnostic case reports its rule. The matrix totals 3,286 cases (11 MB of
case files, 1.1 MB of baselines); the list holds 28 cases for TASK-679's
four defects.
