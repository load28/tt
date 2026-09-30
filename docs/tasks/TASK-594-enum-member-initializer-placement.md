# TASK-594: Reject a statement value in an enum member initializer

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-594`

## Purpose

A tt value inside an enum member initializer was hoisted out of the enum.
`const P = 100; enum F { P = 7, Q = match (o) { A(n) => P + n, B => 0 } }`
printed `101` instead of `8`, with no diagnostic: the match ran before the
enum, before the member `P` existed, and its `P` named the outer constant.
A `result` block in the same position was hoisted the same way.

## Scope

- Included: the enum member initializer as an evaluation owner
  (`src/program_syntax.rs`), its target capability in the Evaluation IR,
  the `match-placement` and `try-placement` messages and `ttc explain`
  texts, `docs/ai/tt.md`, `docs/design/program-lowering.md` §7.4, and
  regression tests.
- Excluded: values inside a function written in the initializer
  (`Q = [0].map(() => match ...)[0]`) already lower in that function and
  are unchanged.

## Decisions

### Decision 1: An enum member initializer is an owner that takes no statements

- **Context**: TypeScript evaluates an enum's members in declaration order,
  and inside an initializer an unqualified member name denotes that member
  (TypeScript handbook, "Enums: Computed and constant members"; the
  TypeScript 1.8 specification §9.2 "Enum Members"). The emitted enum is a
  function whose body assigns each member in turn, and TypeScript rewrites
  a member reference in an initializer to `F.P`, including inside a nested
  function (checked with the pinned `tsc`: `Q = $tt_expr(() => P + 1)`
  emits `F.P + 1` and gives `8`). There is no statement position between
  members, so any prelude the lowering wrote went before the whole enum
  statement, where both the order and the name resolution differ.
- **Alternatives considered**: (a) A special check in the emitter for an
  enum parent. That is the kind of per-shape branch the project rules out.
  (b) Lower a `match` in place through `$tt_expr`. `match` never lowers to
  a callback by design (`docs/ai/tt.md`), and the owners that have no
  statement position reject it. (c) Model the position the way parameter
  defaults, class field initializers, and class definition positions
  already are: an `EvaluationOwner` whose target capability is
  `ExpressionBoundary(OwnerTakesNoStatements)`.
- **Decision and rationale**: (c). `evaluation_owner` returns
  `EvaluationOwner::EnumInitializer` at a `TsEnumMember` initializer edge,
  and the two places that list the statement-less owners include it. The
  consequences follow from the existing model without new branches: a
  `match` reports `match-placement` and a value-form `try` reports
  `try-placement`, each naming the enum member initializer; a `result`
  block, which owns its failure edge, runs in place through `$tt_expr`, so
  its member references keep denoting members and it runs in member order.

## Work log

- 2026-09-30: Reproduced `target/probe5-compiler/repro/r2_enum.tt` (`101`,
  no diagnostic) and the same hoisting for a `result` block. Checked the
  TypeScript enum emit for a member referenced inside a nested function.
- 2026-09-30: `src/program_syntax.rs`: `EvaluationOwner::EnumInitializer`.
  `src/evaluation_ir/planning.rs`, `src/evaluation_ir/evaluation.rs`: the
  statement-less owner lists. `src/evaluation_ir.rs`: the
  `OwnerTakesNoStatements` description. `src/lib/compile.rs`: messages.
  `src/diagnostics.rs`: `ttc explain match-placement` and `try-placement`.
- 2026-09-30: Tests
  `a_statement_value_in_an_enum_member_initializer_is_a_placement_error`
  (`tests/compile/cases_14.rs`),
  `runtime_a_result_block_in_an_enum_member_reads_the_members_in_order`
  (`tests/integration/cases_05.rs`, tsc + node: `8 8 3 r7 S`), and the
  enum case in `mixed_syntax_matrix_covers_every_host_protocol_class`
  (`src/program_syntax/tests.rs`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/program_syntax.rs`, `src/program_syntax/tests.rs`,
`src/evaluation_ir.rs`, `src/evaluation_ir/{evaluation,planning}.rs`,
`src/lib/compile.rs`, `src/diagnostics.rs`, `docs/ai/tt.md`,
`docs/design/program-lowering.md`, `tests/compile/cases_14.rs`,
`tests/integration/cases_05.rs`, `docs/tasks/INDEX.md`, and this record. A
`match` or `try` in an enum member initializer is now a placement
diagnostic, and a `result` block there runs in place.
