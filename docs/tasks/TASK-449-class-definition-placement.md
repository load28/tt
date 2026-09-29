# TASK-449: Reject a match the class definition evaluates instead of hoisting it before the class

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

A `match` inside a class member's decorator was hoisted in front of the class
declaration. Pinned `tsc --strict` rejected the output (TS7034/TS7005 on the
hoisted `let $tt_v0;`), and at runtime the match ran before the class
heritage and the earlier computed member names: `match, extends, key1`
instead of TypeScript's `extends, key1, match`.

## Scope

- Included: The evaluation owner of values in class decorators, member
  decorators, computed member names, and class heritage
  (`src/program_syntax.rs`, `src/program_syntax/visit.rs`,
  `src/program_syntax/collector.rs`); the capability and diagnostics that
  consume it (`src/evaluation_ir/`, `src/lib/compile.rs`,
  `src/diagnostics.rs`); `docs/ai/tt.md`.
- Excluded: Modelling a class definition as an ordered evaluation protocol
  frame that captures earlier decorators and heritage into temporaries.

## Decisions

### Decision 1: Class definition positions are an owner with no statement position

- **Context**: `evaluation_owner` walks the AST path to the nearest function
  body, parameter list, class field initializer, or static block. A member
  decorator or computed member name has none of these between it and the
  class, so the class declaration statement became its host owner and the
  value was hoisted before the class. The evaluation protocol has no frame
  for a class, so nothing captured the heritage or earlier members.
- **Alternatives considered**: (a) Add a class protocol frame whose earlier
  positions are captured into temporaries. This needs new eager positions,
  source rewrites of decorator and heritage text, and still cannot be
  order-correct because the order depends on the decorator mode (below).
  (b) Reject only member decorators. Computed member names have the same
  structural defect (reproduced: `[match (s) {...}]() {}` printed
  `match, extends, key1` and failed strict `tsc` the same way).
- **Decision and rationale**: A new `EvaluationOwner::ClassDefinition` owns
  every position a class definition evaluates itself: class decorators,
  class body edges not already owned by a method body, parameter list, field
  initializer, or static block (member decorators and computed member
  names), and the heritage of a decorated class. It takes no statements, like
  a class field initializer, so the existing `match-placement` and
  `try-placement` diagnostics report it.

### Decision 2: Keep hoisting only an undecorated class's heritage

- **Context**: The task asked whether hoisting a class decorator's match is
  order-preserving. Pinned `tsc` evaluates, with standard decorators, class
  decorators, then heritage, then member decorators and computed names in
  source order (`classdec, extends, key1, memberdec, key2`); with
  `experimentalDecorators` it evaluates heritage and computed names first and
  every decorator after the class body (`extends, key1, key2, memberdec,
  classdec`). ttc does not see the decorator mode.
- **Decision and rationale**: A class decorator's match is first only under
  standard decorators, so hoisting it is not order-correct in general and it
  is rejected. The heritage of an undecorated class is evaluated first under
  both modes, so it keeps lowering before the class; the heritage of a
  decorated class follows the class decorators under standard decorators and
  is rejected. The collector records which classes on the path carry
  decorators so the owner walk can tell the two apart.

## Work log

- 2026-09-27: Reproduced member decorator, computed name, class decorator,
  and heritage cases with pinned `tsc` and node; measured the evaluation
  order of both decorator modes with a plain TypeScript probe.
- 2026-09-27: Added `EvaluationOwner::ClassDefinition`, the decorated class
  fact, capability and message arms, `ttc explain` text, and `docs/ai/tt.md`
  placement rules.
- 2026-09-27: Added five hosts to the value-by-host matrix in
  `tests/compile.rs`, a class-definition owner case to the program syntax
  owner inventory, and a regression test for the reported program in
  `tests/compile/cases_11.rs`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/program_syntax.rs`, `src/program_syntax/visit.rs`,
`src/program_syntax/collector.rs`, `src/program_syntax/tests.rs`,
`src/evaluation_ir.rs`, `src/evaluation_ir/evaluation.rs`,
`src/evaluation_ir/planning.rs`, `src/lib/compile.rs`, `src/diagnostics.rs`,
`docs/ai/tt.md`, `tests/compile.rs`, and `tests/compile/cases_11.rs`. A match
or `try` in a decorator, a computed member name, or a decorated class's
heritage now reports its placement; an undecorated class's heritage still
lowers before the class.
