# TASK-419: Stop marking a case covered in completion when only a guarded arm handles it

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`ttCompletions` at `match (s) { Circle(r) if r > 1 => 1, |` returned `Circle`
with `covered: true`. docs/ai/tt.md (exhaustiveness): "GUARDED arms NEVER count
as covering (add an unguarded arm or `_`)". The editor sorts or dims an item
marked covered, so it steered the user away from the case they still had to
write.

## Scope

- Included: `arm_tags` and its consumer in `src/engine/completions.rs`.
- Excluded: which variant the position completes. A guarded arm's tag is
  still valid evidence for identifying the variant.

## Decisions

### Decision 1: Keep variant evidence and coverage as two separate answers

- **Context**: `arm_tags` returned one list of tags. The caller used it both
  to resolve the variant (`resolve_all`) and to mark items covered (`cases`).
  The `if` token already stopped alternative collection, but the tags written
  before the guard still reached the list when the arm's `=>` arrived.
- **Alternatives considered**: Drop a guarded arm's tags completely. The
  position would then lose evidence for which variant it is over, and
  `match (s) { Circle(r) if ok => 1, |` would offer every visible variant's
  cases.
- **Decision and rationale**: `arm_tags` now returns `ArmTags { tags, covered }`.
  Every completed arm header contributes to `tags`. Only an arm with no guard
  contributes to `covered`. This applies the same rule the analysis uses for
  `Coverage::covered` ("unguarded").

## Work log

- 2026-09-27: Reproduced the defect with `tt_completions_at`. Implemented the
  split. Added `a_guarded_arm_does_not_mark_its_case_covered`: it checks that a
  guarded arm alone leaves `Circle` uncovered, that the same variant's cases
  are still offered, and that a later unguarded `Circle(r)` covers it. The
  test fails with the guard ignored and passes with the fix.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib completions`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` and `./scripts/ci extension` (the final run at the end of TASK-421)

## Result

Only unguarded arms mark a case covered in `ttCompletions`. Changed
`src/engine/completions.rs`.
