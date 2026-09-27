# TASK-447: Open a block for a value hoisted out of an unbraced body

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: see `git log --grep TASK-447`

## Purpose

A `match`, value-form `try`, or `result` inside the unbraced body of an
`if`, `else`, loop, or label was hoisted as bare statements, so only the first
generated statement stayed under the parent:
`if (c) return match (s) {...};` emitted `if (c) let $tt_v0; {...} return $tt_v0;`,
which fails verification (`'let' cannot be used as identifier`) or silently
changes control flow. The same happened for `for (...) f(match ...)`,
`if (c) f(try r); else f();`, `if (c) f(result {...}.kind)`,
`lbl: match (...) {...};`, and `while (c) match (...) {...};`.

## Scope

- Included: every host-hoisting root (expression values and `for`
  initializer propagations) in an unbraced-body owner; labeled iteration
  statements whose header hoists a value; runtime and output tests;
  `docs/ai/tt.md`.
- Excluded: statement-form `try`, which already opened its own block, and the
  `if let` projection issue (TASK-448).

## Decisions

### Decision 1: Make the block requirement a property of the host owner, not of the statement-form `try`

- **Context**: `EvaluationContext.requires_block` was computed for every
  region, but the Evaluation IR only turned it into
  `block_required_propagations` for `CoreRoot::Propagate`. The emitter had no
  block for any other hoisted prelude. The flag was also computed from the
  innermost statement on the AST path rather than from the chosen host owner.
- **Alternatives considered**: (a) wrapping each prelude by itself — leaves
  the rest of the owner (`return $tt_v0;`) outside the block; (b) special-case
  `return`/call shapes in codegen — forbidden by contract 3 and incomplete.
- **Decision and rationale**: The collector now computes `requires_block`
  from the host owner's own parent edge (`is_unbraced_body`). The Evaluation
  IR exposes `block_required_owners` for every Host region that hoists a
  prelude in front of its owner (`CoreRoot::Expr`, and a `Propagate` whose
  continuation is `ForInitialize`); only a statement-form `try`, which
  replaces its owner, stays in `block_required_propagations`. The emitter
  opens the block in the one wrapper every prelude path goes through
  (`within_owner_prelude`: owner-slot, compose, and for-initializer preludes)
  and closes it where the owner's source ends (`close_owner_blocks_at` in the
  source walk). Both braces are claimed once, and an owner's own prelude
  never closes its block even when a value ends exactly where the owner ends
  (ASI).

### Decision 2: Hoist a labeled loop's header values before its labels

- **Context**: ECMA-262 §14.8.1 / §14.13 (ContainsUndefinedContinueTarget):
  `continue lbl` is valid only when `lbl` labels an iteration statement
  directly. Wrapping `lbl: for (x of match ...)` as `lbl: { ...; for ... }`
  (or, before this task, `lbl: let v; ...; for ...`) breaks `continue lbl`.
- **Alternatives considered**: widening the host owner itself to the labeled
  statement — rejected because the `while` loop-test rewrite replaces
  `owner.start..test.start` and would erase the label.
- **Decision and rationale**: `HostOwner` gains `anchor`: the owner itself,
  or for an iteration statement the outermost of the labels directly naming
  it. Owner-slot, compose, and for-initializer rewrites insert at the anchor,
  and the block requirement is evaluated at the anchor's parent, so
  `if (c) outer: inner: for (q of match ...)` becomes
  `if (c) { ...; outer: inner: for (q of v) ... }`. A value in a loop *body*
  already has the body statement as its owner, so the block goes around the
  body, never between label and loop. A non-loop labeled statement keeps the
  block inside the label (`lbl: { ... }`), preserving `break lbl`.

## Work log

- 2026-09-27: Reproduced all six shapes with `ttc -p` and `--no-verify`;
  also found the same defect for `if (c) for (let i = try r; ...)` (the
  payload was wrapped in braces inside the `for` header) and for labeled loop
  headers in braced code.
- 2026-09-27: `src/program_syntax.rs`, `src/program_syntax/visit.rs`:
  `is_unbraced_body`, `prelude_anchor`, `HostOwner.anchor`, owner-relative
  `requires_block`; the return-exit flag reuses `is_unbraced_body`.
- 2026-09-27: `src/evaluation_ir.rs`, `src/evaluation_ir/evaluation.rs`:
  `block_required_owners`; for-initializer propagations leave
  `block_required_propagations`.
- 2026-09-27: `src/codegen/core/planning.rs`, `src/codegen/core/mod.rs`,
  `src/codegen/core/emitter/{mod,host,source}.rs`: rewrites use the anchor;
  `within_owner_prelude` / `close_owner_blocks_at`; the for-initializer
  prelude ends with a line break so the loop no longer follows `}` on the
  same line.
- 2026-09-27: Tests in `tests/compile/cases_11.rs` and
  `tests/integration/cases_05.rs` (tsc + node runtime); `docs/ai/tt.md`.

## Issues and resolutions

### Issue 2: clippy `result_large_err` after `HostOwner` grew

- **Symptom**: `InternalCompilerError` reached 136 bytes, over clippy's
  128-byte threshold, because `LoweringSubject` embedded a whole `HostOwner`.
- **Cause**: the new anchor field enlarged every copy of `HostOwner`.
- **Resolution**: `HostOwner` stores only the anchor start (the anchor always
  ends where the owner does) behind `HostOwner::anchor()`, and the ICE
  subject records a `LoweringOwner` with just the id, kind, and span it
  prints.

### Issue 1: A for-initializer `try` in an unbraced body put braces inside the `for` header

- **Symptom**: `if (c) for (let x = try r; ...)` emitted
  `if (c) const $tt_t0 = r; ... for ({ let x = $tt_t0.value; } ...)`.
- **Cause**: the propagation's block requirement was applied to the payload
  emission as if it were a statement-form `try`.
- **Resolution**: a for-initializer propagation is a hoisting root; its
  block now belongs to the owner (Decision 1).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] The runtime test type-checks with the pinned TypeScript and runs in
  node with the expected output.

## Result

Every hoisted prelude in an unbraced body now opens its own block, and
labeled loops keep their labels on the loop. Changed: `src/program_syntax.rs`,
`src/program_syntax/visit.rs`, `src/evaluation_ir.rs`,
`src/evaluation_ir/evaluation.rs`, `src/codegen/core/planning.rs`,
`src/codegen/core/mod.rs`, `src/codegen/core/emitter/mod.rs`,
`src/codegen/core/emitter/host.rs`, `src/codegen/core/emitter/source.rs`,
`src/ice.rs`,
`tests/compile/cases_11.rs`, `tests/integration/cases_05.rs`,
`docs/ai/tt.md`, `docs/tasks/INDEX.md`.
