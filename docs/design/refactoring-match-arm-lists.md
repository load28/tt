# Isolate match arm list recognition and recovery

Task: [TASK-748](../tasks/TASK-748-match-arm-lists.md). Base: `a31466d9fc542f8ddb8cc05d23fe1d7245a3c1fe`.
Program: [behavior-preserving refactoring](behavior-preserving-refactoring.md).

## Boundary and implementation

Move ArmPart, ArmOutline and its impl, outline_arms, list_arms, parse_whole_arm, parse_arm_list, recover_match_arms, parse_strict_arm_list and arms_tail into private matches::arm_list. Re-export only ArmPart and outline_arms at their original parser visibility; expose recover_match_arms, parse_strict_arm_list and arms_tail to matches. Cursor and pattern/body parsing remain with their existing owners.

## Invariants and exclusions

Preserve bracket skipping, work ticks, top-level guard/arrow precedence, trailing empty slots, strict-versus-recovery grammar choice, separator ownership, token offsets, and the lazy parsing of arm bodies. No pattern grammar or cursor redesign.

No algorithms, state ownership, request ordering, public API, dependencies, or
reference expectations change. Inspect the complete diff, compare moved bodies
and reconstruct the parent against the saved base. Unexpected observations block
completion; behavior fixes require separate tasks.

## Allowed files and validation

Production scope: `src/parser/matches.rs`, `src/parser/matches/arm_list.rs`.
Task/index/progress documentation may also change. Base source snapshots and
comparison evidence live under `/tmp/tt-refactor/748`.

Run parser unit tests plus passthrough, compile and editor_cases tests; final full Rust gate also runs fuzz and compiler baselines.

The user requested autonomous completion of the remaining roadmap on 2026-10-04.
Slices are prepared sequentially on the local branch, with focused checks before
advancing and the complete required gates before completion/publication. No
intermediate change is merged while its required gates remain pending. Rust
1.98.0 and TypeScript 7.1.0-dev.20260826.1 remain pinned. Do not regenerate baselines.
