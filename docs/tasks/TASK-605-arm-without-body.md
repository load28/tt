# TASK-605: Keep a match arm whose body is not written yet

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-605: Keep a match arm whose body is not written yet`

## Purpose

In the guard of a match's last arm with no `=>` yet
(`return match (s) { Circle(radius) => radius, Rect(width) if wi| };`),
completion offered only tt's keywords, and hover and definition answered
nothing. It worked once a later arm or `=>` followed. TypeScript's twin,
`if (wi|)` inside the case, completes `width`, `s`, the locals, and the
globals, and hover and definition answer on every name in it.

## Scope

- Included: How the parser reads an arm whose guard or `=>` is written but
  not its body, its AST/HIR/Core IR representation
  (`Arm::missing`, `ArmBodyKind::Missing`), its emission, its diagnostic
  (`missing-arm-body`), and the tests and fixtures for them.
- Excluded: An arm whose pattern is still being typed (`Admin(name) =>
  name, Gue`), which stays a malformed arm answered by tt's pattern
  completion; a guard that runs into the next arm when no `,` separates
  them, which is read as before.

## Decisions

### Decision 1: An arm with no body is the arm written, with a missing body

- **Context**: The arm failed the arm grammar, so the match was malformed
  and the projection's recovery erased the arm (`RecoveryKind::MatchArms`
  → `ListElement`). The served text had no pattern and no guard at the
  cursor: completion fell to a probe, which could not make the arm parse
  either (`wi$tt_probe` still has no `=>`), and hover and definition have
  no probe.
- **Alternatives considered**: (a) Mend the arm in the completion probe by
  splicing ` => ...` after the guard: hover and definition still see no
  guard, and the engine would be writing arm syntax the parser owns.
  (b) A textual recovery that keeps the guard: replacements are
  byte-length preserving and no two-byte text inserts `=>` and a value;
  rewriting `if` to `=>` makes the guard the arm's value, a checker
  consequence (a `: number` return would report the guard's `boolean`).
  (c) Keep erasing the arm: the defect.
- **Decision and rationale**: As TypeScript's parser keeps a statement
  whose parts are missing and continues (`parser.ts`, `parseExpected`
  creates a missing node), and as TASK-557 keeps a pipeline whose last step
  is missing, the parser claims such an arm with a missing body: an arm
  whose guard runs to the end of its tokens without `=>`, or whose `=>` has
  nothing after it. The rule applies only where the match body already
  reads as arms (`body_reads_as_arms`, TASK-229), so no text that is
  TypeScript is claimed. The arm lowers through HIR and Core IR
  (`ArmBodyKind::Missing`), and codegen yields TypeScript's error type,
  `(undefined as any)`, where the body's value would go: the pattern binds
  its names and the guard reads them exactly as in a finished arm, so
  completion, hover, and definition in the guard are TypeScript's answers.

### Decision 2: An arm whose pattern is still being typed stays malformed

- **Context**: `Admin(name) => name, Gue` has an arm with only a pattern.
- **Alternatives considered**: Claim it with a missing body too: `Gue` is
  a prefix of a case name, and the claimed match would report
  `unknown-case` for it and project a test of a case that does not exist.
- **Decision and rationale**: Only an arm that reached its guard or `=>`
  has a finished pattern (the same boundary `partial::ArmHeader` draws).
  A pattern being typed keeps its recovery and tt's own pattern
  completion; `incomplete_match_arms_preserve_sibling_projections` keeps
  holding.

### Decision 3: `missing-arm-body` is its own rule, reported at the pattern

- **Context**: The arm was reported as `malformed-match` (tt7), which
  blocks projection and leaves its construct as tt text by contract
  (`DiagnosticCode::leaves_tt_text`). A claimed arm leaves no tt text.
- **Alternatives considered**: (a) Keep `malformed-match`: the service
  would lose the projection this task gives it. (b) Report at the whole
  arm: a checker diagnostic inside a tt error's span is owned by that
  error (`diagnostic_intersects_tt_error`), so `Cannot find name 'wi'` in
  the guard disappeared from the typed layer, where TypeScript's twin
  reports it. (c) Report zero-width where the body is missing: every
  other arm rule names the arm by its pattern, which is what a reader
  finds the arm by.
- **Decision and rationale**: A new code, `missing-arm-body` (tt52), that
  does not block projection, as `missing-pipeline-step` (tt51) does not.
  It is reported at the arm's pattern, where every other arm rule reports,
  so the guard's own checker diagnostics stay TypeScript's. Its value is
  the error type, so it has no other consequence. The claimed match is an
  ordinary match: while the last arm's guard is typed, a guarded arm does
  not cover its case, and `match-not-exhaustive` reports that as it does
  for the same finished arm.

## Work log

- 2026-09-30: Reproduced with the probe harness (`mm.tt`): the guard of
  the last arm answered `flow match result`, hover and definition `null`;
  the projection dump showed the arm erased.
- 2026-09-30: Parser (`src/parser/matches.rs`): `parse_arm_tail` takes an
  `open` mode that accepts a missing body, `guard_runs_to_end`,
  `parse_open_arm`/`parse_open_tuple_arm`, a second complete parse in
  open mode where the body reads as arms, and the open grammar in
  `recover_match_arms` so a missing body is not an erased arm.
  `Arm::missing`/`TupleArm::missing` (`src/ast.rs`), `ArmBodyKind::Missing`
  (`src/hir/mod.rs`, `src/hir/lower.rs`), its handling in
  `src/core_ir/lower.rs`, `src/evaluation_ir/evaluation.rs`,
  `src/program_syntax/projection.rs`, and its emission
  (`src/codegen/core/emitter/pattern.rs`). `DiagnosticCode::MissingArmBody`
  with its explanation (`src/diagnostics.rs`) and its report
  (`src/sema/checker.rs`).
- 2026-09-30: First reported at the whole arm; the typed layer then
  dropped `Cannot find name 'wi'` (Decision 3). Moved the span to the
  pattern.
- 2026-09-30: Re-ran the harness: the guard completes `width`, `s`,
  `limit`, and globals; hover answers `const width: number`; definition
  reaches the pattern binding; the typed layer reports `missing-arm-body`
  and TypeScript's `Cannot find name`.
- 2026-09-30: Tests: `an_arm_with_no_body_keeps_its_pattern_and_guard` and
  `a_method_named_match_with_an_arm_shaped_body_stays_typescript`
  (`tests/compile/cases_05.rs`), `the_guard_of_an_arm_with_no_body_is_served`
  (`tests/native/editor_service.rs`, which fails with the open parse
  disabled), the `missing-arm-body` diagnostic fixture, and the code
  number in `src/diagnostics/tests.rs`. Updated `docs/ai/tt.md` and
  `docs/design/lsp-architecture.md`.

## Issues and resolutions

### Issue 1: The guard's own checker diagnostic disappeared

- **Symptom**: `ttc --check-types` reported `missing-arm-body` but not
  `Cannot find name 'wi'` for `Rect(width) if wi`, while the same guard
  with `=> 0` reported it.
- **Cause**: The diagnostic spanned the whole arm, and the typed layer
  treats a checker diagnostic inside a tt error's span as that error's
  consequence.
- **Resolution**: The diagnostic spans the arm's pattern (Decision 3).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `cd editors/vscode && npm run compile && node --test "server/out/test/*.test.js" "client/out/test/*.test.js"`

## Result

Changed `src/parser/matches.rs`, `src/ast.rs`, `src/hir/mod.rs`,
`src/hir/lower.rs`, `src/core_ir/lower.rs`,
`src/evaluation_ir/evaluation.rs`, `src/program_syntax/projection.rs`,
`src/codegen/core/emitter/pattern.rs`, `src/diagnostics.rs`,
`src/diagnostics/tests.rs`, `src/sema/checker.rs`,
`tests/compile/cases_05.rs`, `tests/native/editor_service.rs`,
`tests/fixtures/diagnostic/missing-arm-body/`, `docs/ai/tt.md`,
`docs/design/lsp-architecture.md`, and the task index. An arm whose body
is being written keeps its pattern and guard in the served text, and the
arm is reported once, as `missing-arm-body`.
