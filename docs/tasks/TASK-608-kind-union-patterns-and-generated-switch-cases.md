# TASK-608: Complete a hand-written `kind` union's tags and fields, never a generated switch's cases

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-608: Complete a hand-written kind union's tags and fields, never a generated switch's cases`

## Purpose

With `type K = {kind:"Alpha"; x:number} | {kind:"Beta"}`, completion in
`match (k) { Alpha => 1, | }` offered only `_` although the exhaustiveness
diagnostic names the missing `"Beta"`, and completion in
`match (k) { Alpha(|) => 1, _ => 0 }` offered `kind`, `x`, and
`case "Beta": ...`. That last entry is TypeScript's exhaustive-switch snippet
for the switch the match lowers to (its completion data carries
`source: "SwitchCases/"` and the answer came from the completion probe,
`probe: 1`); accepting it wrote `case "Beta":` into the pattern.

## Scope

- Included: Payload field completion from TypeScript
  (`TypedSite::Field`, `Project::field_candidates`, `field_candidates`), and
  dropping a switch-cases entry whose switch the emission wrote
  (`enclosing_switch`, `ts_completions` in
  `src/engine/language/service.rs`), for every completion answer.
- Excluded: The arm-slot question, which TASK-607 introduced for both tag
  and literal matches and which answers `Beta` here; nested payload
  patterns; `if let` and let-else patterns.

## Decisions

### Decision 1: A tag slot of a hand-written union is answered by TASK-607's discriminant question

- **Context**: The declaration table knows only `variant` declarations, so
  a hand-written union had no tag candidates.
- **Alternatives considered**: Teach the declaration table object unions:
  a second reading of TypeScript types in tt.
- **Decision and rationale**: The tag family of TASK-607's question,
  `(scrutinee).kind === `, is answered by TypeScript for any scrutinee whose
  `kind` has literal constituents, declared by `variant` or by hand; this
  task pins the hand-written case in the regression tests (`Alpha
  (covered)`, `Beta`, `_`; with a typed prefix; in an empty match; after a
  guarded arm, where `Alpha` is not covered).

### Decision 2: A payload field list is completed with the selected case's properties, less the discriminant

- **Context**: The completion probe spliced into `Alpha(|)` lowers to
  `const { $tt_probe } = $tt_m;` inside the `Alpha` case, where TypeScript
  completes the properties of the narrowed scrutinee, as it does for
  `const { | } = k` in a `.ts` file. The adapter took that general answer
  only when the declaration table knew nothing, with no pattern filter.
- **Alternatives considered**: Leave the general answer: it carries
  entries that are not field names (Decision 3) and the discriminant.
- **Decision and rationale**: At a field position the engine asks through
  the probe and keeps the entries whose label tt's arm grammar reads as a
  bare name. `kind` is excluded: the tag already tests the variant ABI's
  discriminant (`VARIANT_TAG_FIELD`), and a declared variant's fields never
  include it. A field the payload already binds is excluded, and the
  declaration table's fields keep their signatures as detail.

### Decision 3: A switch-cases entry is kept only when the switch around the position is the user's

- **Context**: TypeScript adds its exhaustive-case snippet
  (`CompletionSource.SwitchCases`, `"SwitchCases/"`) when the context token
  has a case block ancestor (`services/completions.ts`,
  `getExhaustiveCaseSnippets` over `findAncestor(contextToken,
  isCaseBlock)`). In a tt file that ancestor can be the switch a match
  lowers to: in the payload, the guard, or the body of an arm.
- **Alternatives considered**: (a) Drop every switch-cases entry: a switch
  the user wrote in a `.tt` file would lose the snippet a `.ts` file has.
  (b) Drop entries by label shape (`case ...`): the label is display text.
- **Decision and rationale**: The engine lexes the served text and finds
  the innermost case block open before the position, the `{` after the `)`
  of `switch (...)`, as TypeScript's ancestor walk finds it. When the
  emission did not copy that `switch` keyword from the source (the emit
  mapping), the switch is generated and its switch-cases entry is dropped;
  a user's `switch` keeps it. This applies to every completion answer, the
  plain one and the probe's.

## Work log

- 2026-09-30: Reproduced with the probe harness (`cmp.cjs k3.tt - c`): the
  arm slot offered `_`; the payload offered `kind x case "Beta": ...`. The
  raw service item carried `data.source: "SwitchCases/"`, kind 15, and
  `insertText: "case \"Beta\":"`.
- 2026-09-30: Added the field site, the field question, and the
  switch-cases rule.
- 2026-09-30: Re-ran the harness: the arm slot offers `Beta`, `Alpha`
  (covered), `_`; the payload offers `x`.
- 2026-09-30: Tests: the hand-written union and field cases of
  `pattern_completion_offers_what_the_scrutinee_type_admits`,
  `completion_never_offers_the_cases_of_a_generated_switch` (a generated
  switch around an arm body offers no `case` entry; a hand-written switch in
  the same file still offers `case "Beta": ...`), both in
  `tests/native/editor_service.rs`, and the field case of the extension test
  "pattern completion offers what the scrutinee's type admits".

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native -- pattern_completion completion_never_offers`
- [x] Full gate (recorded in TASK-613, run once for TASK-606 to TASK-613)

## Result

Changed `src/engine/completions.rs`, `src/engine/language/project.rs`,
`src/engine/language/service.rs`, `tests/native/editor_service.rs`,
`editors/vscode/server/src/test/server.test.ts`,
`docs/design/lsp-architecture.md`, and the task index. A hand-written
union's tags and fields are completed from its type, and no completion
entry offers to write a generated switch's cases.
