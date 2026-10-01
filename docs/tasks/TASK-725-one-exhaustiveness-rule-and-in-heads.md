# TASK-725: One exhaustiveness rule on every surface, and `in` inside a pipeline head

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-725`

## Purpose

The round-8 probe raised two questions to decide and record, with code only
where the answer is a defect. (a) A `match` that narrowing makes exhaustive
(`if (s.kind === "Rect") return 0; match (s) { Circle(r) => r }`) was
accepted by `ttc --check-types` alone, while a build, `ttc -p` and the
bundler plugins, `ttc --check`, the content mapper, and the editor reported
`tt27` (`match-not-exhaustive`). (b) `"a" in {} |> f` bound as
`"a" in f({})`, while `x instanceof O |> f` and `x > y |> f` bound as
`f(x …)`.

## Scope

- Included: the typed report's variant exhaustiveness
  (`src/engine/semantics/report.rs`), its documentation (`docs/ai/tt.md`,
  `Options::defer_to_checker`, `sema::coverage_errors`), the pipeline-head
  tracker's `in` (`src/parser/parse.rs`, `src/parser/keywords.rs`), the
  guide's "|>" section, two cases, a note atop TASK-108, and the full gate
  for TASK-719 to TASK-725.
- Excluded: literal-union exhaustiveness, which only a checker can judge
  and which stays a typed-only diagnostic.

## Decisions

### Decision 1: The declared cases are the rule; a checker adds holes, never removes them

- **Context**: `ttc --check-types` (TASK-073, TASK-108) answered variant
  exhaustiveness from the alphabet the checker gives at the scrutinee,
  narrowing included, in place of the declared cases every other surface
  uses. The editor showed `tt27` from its text layer (`ttc --check`), and
  its typed layer only replaces a diagnostic it states itself
  (`editors/vscode/server/src/diagnostics.ts`, `mergeTyped`), so it agreed
  with the build; `ttc --check-types` was the one surface that accepted
  the program, and a CI that runs it passed a program the build rejects.
  TASK-713 and TASK-714 hold the surfaces to one tt rule and one wording.
- **Alternatives considered**: (a) Narrowing everywhere: a build, `-p`, and
  the content mapper have no types, so they cannot decide it. (b) Make the
  editor follow `--check-types`: the editor and the check would accept a
  program the build rejects. (c) The declared cases everywhere, as the
  language's rule (Rust's `match` likewise ignores earlier tests), with the
  checker adding the holes only types show: a variant reached through a
  re-export the build does not collect, a payload alphabet the declarations
  cannot resolve, and literal unions.
- **Decision and rationale**: (c). The typed report now answers every
  file's variant coverage from its declarations first, exactly as
  `ttc --check` does (`sema::coverage_errors`), and drops the checker's
  answer for a match that rule already reported, so the two never state the
  same hole twice. A program `--check-types` accepts therefore builds, and
  the editor's two layers state the same `tt27`. This reverses the part of
  TASK-108 Decision 1 that preferred the narrowed alphabet over the
  declared one; its note says so.

### Decision 2: A pipeline head extends over `in` as over every binary operator

- **Context**: `docs/ai/tt.md` defines the head as any expression and says
  a step extends over binary operators; `docs/design/pipeline-operator.md`
  §3.5 says the head extends over an assertion. The TC39 pipeline proposal
  (`tc39/proposal-pipeline-operator`, "Precedence") gives `|>` the
  precedence of `=>`, assignment, and `yield`: "looser than all other
  operators", tighter only than the comma. The head tracker reset at every
  undotted `in` for the sake of `for (k in o |> keys)` heads, which made
  `"a" in o |> f` the one binary operator a head did not extend over; the
  guide called it ambiguous and told the author to parenthesize.
- **Alternatives considered**: (a) Keep it and document the binding: an
  exception to the stated precedence, and `"a" in f(o)` is rarely meant.
  (b) Treat `in` as a separator only where it is the for-in keyword.
- **Decision and rationale**: (b). The tracker records a `for` head's
  parenthesis (`ExprFrame::ForHeader`) and whether a top-level `;` has
  ended its initializer; before that `;` an `in` is the for-in separator
  (ECMA-262 §14.7.4: a `for` initializer is `Expression[~In]`, so it never
  holds the relational `in` at its top level), and everywhere else it is
  the relational operator. `of` stays a separator: it is never an operator.

## Work log

- 2026-10-01: Reproduced (a) with `ttc`, `ttc -p`, and `ttc --check-types`
  on a narrowing-exhaustive match, and read how the editor merges its
  layers; reproduced (b) with the probe's three heads.
- 2026-10-01: Implemented Decision 2 (`src/parser/parse.rs`) and its case
  `tests/cases/compiler/aPipelineHeadExtendsOverTheInOperator.tt` (`@run`,
  with a for-in head and a `for` test after `;`).
- 2026-10-01: Implemented Decision 1 (`src/engine/semantics/report.rs`),
  updated `docs/ai/tt.md` ("match", "Workflow", "Errors"),
  `Options::defer_to_checker`, and `sema::coverage_errors`; added
  `tests/cases/compiler/aMatchCoversItsDeclaredCasesWhateverNarrowingRemoved.tt`.

- 2026-10-01: `UPDATE_EXPECT=1 cargo test --test case_baselines`: fifteen
  `.errors.txt` baselines of exhaustiveness cases whose scrutinee is an
  undeclared name (TypeScript reports it, the checker gives no alphabet)
  now carry the declared rule's `match-not-exhaustive` in their
  `ttc --check-types` section, which the `ttc --out-dir` section already
  had; read each.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/aPipelineHeadExtendsOverTheInOperator.tt`;
  `tests/cases/compiler/aMatchCoversItsDeclaredCasesWhateverNarrowingRemoved.tt`
- **Observed failure**: with `src/engine/semantics/report.rs` reverted,
  `aMatchCoversItsDeclaredCasesWhateverNarrowingRemoved`'s `.errors.txt`
  was out of date: the `ttc --check-types` section lacked
  `match on variant Shape is not exhaustive: missing "Rect"` that the
  `ttc --out-dir` section reports. With `src/parser/parse.rs` and
  `keywords.rs` reverted, `aPipelineHeadExtendsOverTheInOperator` failed:
  `"a" in o |> String` was `"a" in String(o)`, which the checker rejects
  (TS2322 "Type 'string' is not assignable to type 'object'" in the
  generated code) and the program did not run.

## Verification

- [x] The full gate (below)

## Result

Every surface demands a match's declared cases, and `--check-types` adds
only holes; a pipeline head extends over `in` outside a `for` head.
