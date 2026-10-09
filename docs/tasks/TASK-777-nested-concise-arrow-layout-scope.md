# TASK-777: Give a value in a nested concise arrow body its layout scope

- **Status**: Complete
- **Started**: 2026-10-07
- **Completed**: 2026-10-07
- **Commit**: —

## Purpose

`ttc` stops with an internal compiler error on a concise arrow body that
holds a tt value and, after it, another concise arrow whose body composes a
tt value:

```tt
variant V { A, B }
declare const v: V;
export const x = () => [match (v) { A => 1, B => 2 }, () => [match (v) { A => 1, B => 2 }]];
```

```
error: internal compiler error: validate_origin broke the contract that a generated line break has a layout scope (LayoutScopeMissing)
```

The compiler built from `1cc08aa9` reports the same error, so it predates
TASK-776.

## Scope

- Included: the emission that loses the layout scope, and a case pinning it.
- Excluded: other lowering changes.

## Decisions

### Decision 1: The source walk closes arrow blocks in the order they end

- **Context**: A concise arrow body that holds a tt value becomes a block,
  and when source follows the value, the walk over that source writes the
  block's closing `;` and `}` (with the layout scope's close) where the body
  ends. `source_range_rope` listed the blocks ending in its range in the
  order of their rewrites, which is the order their bodies *start*. An outer
  arrow starts first and ends last, so the walk jumped to the outer body's
  end, closed it, and then dropped the inner block whose end it had passed:
  its scope stayed open and `validate_origin` reported the missing scope.
- **Alternatives considered**: (a) Closing every passed block when the walk
  skips over its end: the text between the two ends would land inside the
  wrong block. (b) Ordering the pending closings by where their bodies end,
  as the same walk already orders the loop bodies it closes.
- **Decision and rationale**: (b). The walk stops at each body's end in
  turn, so nested arrow blocks close innermost first.

## Work log

- 2026-10-07: Reduced the reproducer to
  `() => [match, () => [match]]`; traced the walk over the source after the
  inner value, which held both blocks with the outer one first; sorted the
  pending closings by body end in `src/codegen/core/emitter/source.rs`; added
  `tests/cases/compiler/aNestedConciseArrowBodyClosesBeforeItsOuterArrow.tt`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/aNestedConciseArrowBodyClosesBeforeItsOuterArrow.tt`
- **Observed failure**: `every_case_matches_its_baselines` panicked with
  `validate_origin broke the contract that a generated line break has a
  layout scope (LayoutScopeMissing)`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (`--test-threads=4`): every suite passes
- [x] Baseline changes reviewed and committed with the change: only the new
  case's `.ts`, `.map.txt`, `.types`, and `.stdout` (`[ 2 ] [ 4 ]`)

## Result

Complete. `src/codegen/core/emitter/source.rs` closes the arrow blocks a
source walk passes in the order their bodies end, pinned by
`aNestedConciseArrowBodyClosesBeforeItsOuterArrow`.
