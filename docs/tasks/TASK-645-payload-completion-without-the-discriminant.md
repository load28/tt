# TASK-645: Leave the discriminant out of every completion answer in a payload pattern

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-645: Leave the discriminant out of every completion answer in a payload pattern`

## Purpose

TASK-639's `patternCompletion` baseline recorded typed completion in
`match (k) { Alpha(|) => ... }`, with `type K = { kind: "Alpha"; x: number }
| { kind: "Beta" }`, offering `kind` and `x`, while `patternCompletions` at
the same position offered only `x`. TASK-608 Decision 2 excludes the
discriminant from a payload field list; the engine's general completion
answer did not apply that rule, and the same held for a declared variant's
payload (`Circle(|)` offered `kind` and `radius`).

## Scope

- Included: `Project::triggered_completion` and `Project::field_candidates`
  in `src/engine/language/project.rs`, `is_payload_field` and
  `field_candidates` in `src/engine/language/service.rs`, and the editor
  case `patternCompletion`.
- Excluded: The VS Code adapter, which already answers a pattern position
  with `patternCompletions`; the language rule for `Alpha(kind)` itself
  (still accepted, binding the tag), and the tag and literal slots.

## Decisions

### Decision 1: One predicate decides what a payload may bind, and both answers apply it

- **Context**: The completion probe lowers `Alpha(|)` into
  `const { $tt_probe } = $tt_m;` inside the `Alpha` case, where TypeScript
  lists the properties of the narrowed case, as for `const { | } = k` in a
  `.ts` file: `kind` is one of them. TASK-608 filtered that list in
  `pattern_completions` only, in two halves (the bare-name test in
  `Project::field_candidates`, the discriminant and written fields in the
  free `field_candidates`). The engine's `completion`, which the server's
  `completion` method and every embedding ask, returned TypeScript's list
  unfiltered.
- **Alternatives considered**: (a) Change the lowering so TypeScript cannot
  offer `kind` (for example `const { kind: _, ...rest }`): the lowering is
  the program's runtime shape, and a completion question does not justify
  changing emitted code. (b) Filter by completion kind or sort text: a
  property named `kind` has the same kind and rank as `x`. (c) Leave
  `completion` as TypeScript's raw answer: the engine would give two
  answers at one position, one of which offers a name the tag already
  fixes.
- **Decision and rationale**: `is_payload_field` states the rule once: a
  label tt's pattern grammar reads as a bare field name, not
  `VARIANT_TAG_FIELD` (the discriminant the tag tests, `docs/ai/tt.md`:
  "Discriminant always `kind`"), and not a field the payload already binds.
  `pattern_completions` uses it through `field_candidates`, and
  `triggered_completion` applies it to the whole answer when
  `pattern_question` (the parser's reading of the position, the same one
  `pattern_completions` uses) says the position is a payload field site.
  The site is structural (the token stream around the cursor), not a
  string shape of the answer.

### Decision 2: `kind` stays bindable in the language

- **Context**: `Alpha(kind)` compiles to `const { kind } = $tt_m;` and is
  not a tt error (`docs/ai/tt.md`: a case may not declare a field named
  `kind`, but a pattern is not checked against the discriminant).
- **Alternatives considered**: Report `Alpha(kind)` as an error: a language
  change outside this task, and it would break valid programs.
- **Decision and rationale**: Completion does not offer what the tag
  already decides; writing it by hand stays valid.

## Work log

- 2026-09-30: Read TASK-607 and TASK-608, `pattern_completions`,
  `field_candidates`, `service_completion`, and `pattern_question`.
- 2026-09-30: Added `is_payload_field`, used it in `field_candidates`,
  dropped the bare-name filter from `Project::field_candidates` (now in the
  predicate), and filtered `triggered_completion`'s answer at a payload
  field site.
- 2026-09-30: Added the `written` (a payload that already binds `x`) and
  `ifLet` markers to `patternCompletion`; regenerated with
  `UPDATE_EXPECT=1 TT_CASES=patternCompletion cargo test --test
  editor_cases` and read the diff: `kind` is gone from `field` and
  `payload`, `written` offers nothing, `ifLet` offers `x`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/patternCompletion.tt` (`tests/editor_cases.rs`)
- **Observed failure**: With `project.rs` and `service.rs` restored to the
  previous revision, `TT_CASES=patternCompletion cargo test --test
  editor_cases` failed with "modified baseline ...
  patternCompletion.baseline is out of date": the stored
  `completion: 1 item(s)` / `x (property, 11)` at `/*field*/` against the
  answer `completion: 2 item(s)` with `kind (property, 11)`; the other
  payload markers differed the same way.

## Verification

- [x] `TTC_REQUIRE_TSGO=1 cargo test --test editor_cases` (every case)
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native -- completion` (6 passed)
- [x] Full gate, recorded in TASK-646 (run once over TASK-644 to TASK-646)
- [x] Baseline changes reviewed and committed with the change

## Result

Changed `src/engine/language/project.rs`, `src/engine/language/service.rs`,
`tests/cases/editor/patternCompletion.tt`, its baseline, and the task
index. Every completion answer in a payload pattern leaves out the
discriminant and the fields already bound.
