# TASK-686: Populate the editor matrix for every construct and triage each difference from TypeScript

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-686`

## Purpose

TASK-685 built the editor matrix and piloted it on `|>`. This task marks
the points worth asking about in every other construct's spec (`match`,
`try`, `result`, let-else, `if let`, `flow`, `val`, `variant`, and the
`.ttx` positions), writes the editor twins those constructs need, runs
every generated case, and classifies each difference from TypeScript as a
twin error (fixed in the spec), a documented tt behaviour, or a defect
(reported here with a minimal repro, not fixed).

## Scope

- Included: markers and `edit` templates in `tests/matrix/{match,try,
  result,letElse,ifLet,flow,val,variant,variants}.mjs`, the construct list
  in `tests/matrix/editor.mjs`, the generated cases and difference
  baselines, the list lines in `tests/editor-matrix-differences.txt`, and
  one renamed twin parameter in the compiled `flow` matrix.
- Excluded: fixing the defects below (the instruction for this batch is to
  report them), and the try-placement defects of TASK-679 Issues 3 and 4,
  which TASK-681 to TASK-684 address; their editor symptoms are listed so
  that a fix shows up as lines to remove.

## What the matrix covers

| Construct | Surface | Cases | Questions |
| --- | --- | --- | --- |
| `match` | `.tt` | 224 | 4,074 |
| `match` | `.ttx` | 120 | 2,254 |
| `try` (value) | `.tt` | 104 | 1,142 |
| `try` (value) | `.ttx` | 56 | 637 |
| `try` (statement) | `.tt` | 97 | 1,089 |
| `try` (statement) | `.ttx` | 14 | 210 |
| `result` | `.tt` | 112 | 1,811 |
| `result` | `.ttx` | 56 | 903 |
| let-else | `.tt` | 117 | 1,606 |
| let-else | `.ttx` | 49 | 812 |
| `if let` | `.tt` | 129 | 2,115 |
| `if let` | `.ttx` | 49 | 952 |
| `\|>` (TASK-685) | `.tt` | 176 | 2,285 |
| `\|>` (TASK-685) | `.ttx` | 88 | 1,174 |
| `flow` | `.tt` | 112 | 1,476 |
| `val` | `.tt` | 128 | 2,438 |
| `val` | `.ttx` | 56 | 1,239 |
| `variant` | `.tt` | 124 | 1,979 |
| `variant` | `.ttx` | 42 | 805 |
| **Total** | | **1,853** | **29,001** |

A question is a verb at a marker or, for `semanticTokens` and
`diagnostics`, at a unit; a semantic-token question compares the token at
every marker of the case, so the points compared are more than the
questions. By verb: hover 6,747, definition 5,278, completions 4,229,
references 3,248, rename 3,248, signature help 2,585, diagnostics 1,853,
semantic tokens 1,813. The value constructs are asked in the 16 `.tt`
host positions `tests/matrix/editor.mjs` lists (declaration initializer,
call argument, return, object property, template literal, conditional
branch, `&&`, `??`, getter, method, concise arrow, loop body, `await`
operand, module top level, match arm, if-let body) and in all 8 JSX
positions; the statement constructs in all 16 statement positions and the
component body; each row keeps the companion the compiled matrix chose.

Markers by construct: pattern bindings and their uses, aliased payload
fields, guard reads, block-arm locals, an `is` pattern's class and
destructured field, a tuple's variant constructor member, a JSX arm's
attribute (`match`); the called function and its argument, a member read
on the value form, a declaration's binding (`try`); the block's bindings
and uses, the called function (`result`); the pattern field and binding,
the called function, the `throw` constructor (let-else); the bindings of
each branch of an else-if-let chain (`if let`); steps and their arguments,
members, receivers (`flow`); `val` bindings and parameters, their uses and
member reads, the copy and the aliased original (`val`); a variant's name,
its case members, constructor arguments, and a binding matched where it
is declared (`variant`); and, in every row, the companion's operand.

## Decisions

### Decision 1: Editor twins write what a TypeScript user writes

- **Context**: The runtime twins (TASK-678 Decision 2) read payloads from
  `any` temporaries; the editor oracle needs the tt program's bindings,
  declared with their types.
- **Decision and rationale**: every `match` form has an `edit` twin that
  passes the scrutinee to an arrow typed with the scrutinee's type and
  tests the arms in order, each arm's bindings destructured inside the
  arm's own block (`if (t0.kind === "Circle") { const { r } = t0; return
  ...; }`); the operand stays an argument of the call, so `await` and
  `yield` operands stay in the host function. Shorthand bindings of
  let-else and `if let` (`Num(value) | Neg(value)`, `Err(error)`)
  destructure (`const { value } = t0`), as TypeScript would; aliased ones
  (`value: v`) keep the property read. A unit variant case is
  `{ kind: "Point" } as const`, as ttc emits it and as `docs/ai/tt.md`
  documents for the ambient form (`readonly Q: { readonly kind: "Q" }`),
  not `as Shape`. The nested `result` twin types its inner block as
  `ReturnType<typeof read>`, the Result the tt block has.

### Decision 2: Where the twin cannot be an oracle, the spec withholds the question

- **Context**: A few answers depend on something the twin cannot express.
- **Decision and rationale**: a construct or position may withhold a verb
  (`editorWithholds`), and the reason is recorded here: the `result`
  block's value type (TypeScript names it structurally, the twin's
  wrapper returns `any`), so the hover of the JSX attribute that holds it
  is not asked; signature help at a `yield` operand, which is inside no
  call, so the twin's arrow call would answer; and semantic tokens in a
  JSX spread attribute (TASK-685 Issue 2).

### Decision 3: Classification

- **Decision and rationale**: of 29,001 questions, 643 in 363 cases differ
  after the twin errors were fixed, and every one is listed in 93 lines of
  `tests/editor-matrix-differences.txt`:

  | Class | Questions | Cases | Reason |
  | --- | --- | --- | --- |
  | by-design | 126 | 63 | A step is the function value it names (`docs/ai/tt.md`, "\|>") |
  | by-design | 116 | 69 | An or-pattern binds in every alternative (`docs/ai/tt.md`, "match") |
  | by-design | 39 | 39 | A suggestion over generated code is not shown (`docs/ai/tt.md`, editor suggestions) |
  | defect | 192 | 171 | Issue 1 |
  | defect | 35 | 18 | Issue 2 |
  | defect | 68 | 68 | Issue 3 |
  | defect | 5 | 5 | Issue 4 |
  | defect | 3 | 3 | TASK-685 Issue 3 |
  | defect | 51 | 5 | TASK-679 Issue 4 (the construct does not compile) |
  | defect | 8 | 7 | TASK-679 Issue 3 (the construct does not compile) |

  The by-design suggestion rule was implemented in
  `src/engine/language/project.rs` (a suggestion "landing in glue is about
  ttc's emission") but not documented; TASK-685 added the sentence to
  `docs/ai/tt.md` that the line cites.

## Work log

- 2026-09-30: Wrote the markers and `edit` templates; the generator's
  marker check found no one-sided marker after the templates were
  complete.
- 2026-09-30: First full run of 1,853 cases (1,508 s): 1,783 differing
  cases, most from twin errors (Issues 5 to 8). Second run (1,466 s):
  709 differing questions; the rest of the twin errors fixed. Third run
  (1,458 s): 643 differing questions in 363 cases, all classified.
- 2026-09-30: Re-ran the compiled `flow` matrix after renaming its twin
  parameter (`TT_CASES=flow_ cargo test --test case_baselines`, 162 s,
  passes). Confirmed Issue 4's cause with a plain-TypeScript probe.

## Issues and resolutions

### Issue 1: Completion in a declaration's own initializer offers the declared name

- **Symptom**: 192 completion questions: the operand of a construct that
  initializes `const value`, `const top0`, `const n`, `const checked`,
  `const a`/`const b` (`result`), lists that name; TypeScript leaves the
  declaration being initialized out of its own initializer's completions.
- **Repro**:
  ```tt
  declare const input: number;
  function g() {
    const value = match (/*own*/input) { _ => 1 };
    return value;
  }
  ```
  Completion at `own` offers `value (Variable)`; the twin
  `const value = ((t: number) => 1)(/*own*/input);` does not.
- **Cause**: Not investigated; the lowering evaluates the construct before
  the declaration, so the completion probe is no longer inside the
  declaration's initializer.
- **Resolution**: Listed as a defect; not fixed here.

### Issue 2: Completion in a construct that initializes a module-level declaration offers type keywords

- **Symptom**: 35 questions at module top level (`match` and `result`
  rows): completion inside an arm or a block lists 18 type keywords
  (`abstract`, `any`, `asserts`, `bigint`, `boolean`, `declare`, `infer`,
  `keyof`, `module`, `namespace`, `never`, `number`, `object`,
  `readonly`, `string`, `symbol`, `unique`, `unknown`) that TypeScript does
  not offer in an expression.
- **Repro**:
  ```tt
  declare function f(n: number): number;
  declare const input: number;
  const top = match (input) { 1 => f(/*top*/input), _ => 0 };
  ```
  The same construct inside a function answers as TypeScript does.
- **Cause**: Not investigated; the top-level lowering (generated storage
  named after the declaration, `docs/ai/tt.md` contract) places the probe
  where TypeScript reads a type position.
- **Resolution**: Listed as a defect; not fixed here.

### Issue 3: An or-pattern's first binding is not a declaration in semantic tokens

- **Symptom**: 68 cases: at the first alternative's binding of
  `Num(value) | Neg(value)` (a `match` arm, let-else, `if let`), tt's
  token is `variable []`; the twin's `const { value } = t0` is
  `variable [declaration, local, readonly]` (at module level
  `[declaration, readonly]`), and a binding of a single-alternative
  pattern carries those modifiers in tt too.
- **Repro**: `match (t) { Num(/*bind*/value) | Neg(value) => value, _ => 0 }`
  over `variant Token { Num(value: number), Neg(value: number), End }`.
- **Resolution**: Listed as a defect; not fixed here.

### Issue 4: A single-case variant is shown structurally, not by name

- **Symptom**: `variant_exported_*` signature help on `Pub.Item(` shows
  `Item(n: number): { kind: "Item"; n: number; }`; the twin shows
  `Item(n: number): Pub`.
- **Repro**: `export variant Pub { Item(n: number) }` then `Pub.Item(`.
- **Cause**: ttc emits `export type Pub =\n  | { kind: "Item"; n: number };`,
  a one-member union with a leading `|`, and TypeScript prints such an
  alias structurally: a plain-TypeScript probe with identical `.tt` and
  `.ts` text shows `Item(n: number): { kind: "Item"; n: number; }` for the
  leading-`|` alias and `Item(n: number): Two` for
  `type Two = { kind: "Item"; n: number }`.
- **Resolution**: Listed as a defect; not fixed here.

### Issue 5: Twin arms leaked their bindings

- **Symptom**: `completions use` listed `error`, `key`, or `w` only in the
  twin.
- **Cause**: The `edit` twin destructured the last arm's fields at the
  arrow's top level, where every earlier arm could see them.
- **Resolution**: The last arm is a block, as the others are.

### Issue 6: Signature help at a `yield` operand answered for the twin's arrow

- **Symptom**: `signatureHelp operand` was null in tt and the twin's
  `(t0: Shape)` in `match` rows with the `yield` companion.
- **Cause**: `(yield input)` is inside no call in the program; in the twin
  it is the argument of the arm arrow's call.
- **Resolution**: The `yield` companion is marked `uncalled`, and signature
  help is not asked at its operand.

### Issue 7: The twin's variant typed unit cases as the union

- **Symptom**: Hover on `Shape.Point`, `Dir.North`, and the variant name
  showed `{ readonly kind: "Point" }` in tt and `Shape` in the twin.
- **Cause**: `tests/matrix/variants.mjs` wrote `{ kind: "Point" } as Shape`;
  ttc emits `as const`.
- **Resolution**: The variant's `edit` declaration follows the emitted,
  documented form (Decision 1). The compiled matrix keeps its runtime twin.

### Issue 8: Twin type errors from untyped plumbing

- **Symptom**: TS18046 (`'m' is of type 'unknown'`) and, in `.tsx` with
  `verbatimModuleSyntax`, TS1484 for a type imported as a value, only in
  the nested `result` twin; TS6133 for a `flow` twin's `first` parameter,
  which shadowed the harness function and made its import unused.
- **Resolution**: The inner block is typed as `ReturnType<typeof read>`,
  and the `flow` twins' parameter is `head` (the compiled `flow` twins
  change only by that name).

## Regression test (fails before the fix)

Not applicable: this task adds generated test cases and fixes no product
defect; the defects it finds are listed, not fixed.

## Verification

- [x] `node scripts/generate-cases --check`: 3,286 compiled cases and
  3,706 editor case files (1,853 cases) match the spec; the compiled
  cases change only in the 156 `flow` twins' parameter name.
- [x] `TT_CASES=flow_ TTC_REQUIRE_TSGO=1 cargo test --test case_baselines`:
  passes (162 s); the renamed twins print what they printed.
- [x] Every editor matrix case: `TT_MATRIX_CASES=all TT_REQUIRE_EXTENSION=1
  TTC_REQUIRE_TSGO=1 cargo test --test editor_cases` passes in 1,426 s
  (debug build, eight workers, this four-core container; about 0.77 s of
  wall time per case), and the previous full run, with `UPDATE_EXPECT`,
  wrote the same 363 difference baselines (377 KB).
- [x] Full gate, after merging nothing (`claude/ecstatic-dijkstra-qw5pf9`
  had not advanced past `e44a7e0`): `cargo fmt --check`; `cargo clippy
  --all-targets -- -D warnings`; `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1
  TTC_REQUIRE_TYPESCRIPT_CASES=1 TT_REQUIRE_EXTENSION=1
  TT_BASELINE_TRACKING_DIR=<dir> cargo test --no-fail-fast`: 1,796 passed,
  0 failed, no `SKIP`, 740 s, the editor suite 64 s with its default sample
  of 40 of 1,853 generated cases; `node scripts/check-baselines --tracking
  <dir>`: "682 compared, 3550 of unsampled matrix cases left unjudged,
  none unused"; the extension suite (`npm test` in `editors/vscode`): 238
  passed, 0 skipped; `./scripts/ci agents`: passed, with the environmental
  warnings TASK-639 recorded (rolldown not on PATH, doctor reports the
  checkout not ready).
- CI cost: a pull request's editor suite goes from about 18 s to about
  60 s (TASK-685); the nightly `exhaustive` job adds the adapter build and
  about 24 minutes for every case (it runs `--release`, which only speeds
  up ttc).

## Result

Changed files: `tests/matrix/{match,try,result,letElse,ifLet,flow,val,
variant,variants,editor}.mjs`, `scripts/generate-cases` (one expression),
`tests/cases/editor/matrix/**` (1,853 cases and their twins, 5.6 MB),
`tests/cases/conformance/matrix/flow/**` (156 twins),
`tests/baselines/reference/editor/matrix/**` (363 difference baselines),
`tests/editor-matrix-differences.txt` (93 lines), `CONTRIBUTING.md`,
`docs/tasks/INDEX.md`, and this record. Follow-ups: Issues 1 to 4 and
TASK-685 Issue 3 are defects for a later batch; the lines citing TASK-679
Issues 3 and 4 are expected to stop matching when TASK-681 to TASK-684
land, and then must be removed.
