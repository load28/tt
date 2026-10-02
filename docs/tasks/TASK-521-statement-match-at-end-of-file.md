# TASK-521: Close the hoisting block of a statement value that ends the file

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

A statement-position `match` that ends the file with no trailing line break
crashed the compiler (exit 101). `declare const x: { kind: "A" };\nmatch (x) { A => 1 }`
reported `internal compiler error: internal compiler error: validate_origin
broke the contract that a generated line break has a layout scope
(LayoutScopeMissing)`. The same happened for `if (x) match (x) { _ => 1 }`,
`lbl: match (x) { _ => 1 }`, and two statement matches without `;` at the end
of the file. The report also printed its prefix twice, while `docs/ai/tt.md`
specifies one `error: internal compiler error: ...`.

## Scope

- Included: closing an owner block that the statement-expression emission
  path opened, and the prefix of a validator failure's message.
- Excluded: the host owner span of an ASI-terminated expression statement,
  which includes the line break after it. A block around such a statement
  that is followed by another one still closes after that line break
  (`}{` on one line). That output is valid TypeScript and is unchanged.

## Decisions

### Decision 1: The statement emission that consumes an owner prelude also closes the owner's block

- **Context**: TASK-447 made the block around a hoisting owner a property of
  the owner. `within_owner_prelude` opens it, and `close_owner_blocks_at`
  closes it where the source walk (`source_range_rope`) reaches the owner's
  end. A tt value that is itself an expression statement has no opaque
  source around it, so `emit_statement_expr` emits the prelude directly and
  no source walk covers the statement. The block was closed only when a
  later source walk passed the owner's end, which is the trailing trivia of
  the file or the next statement's source. When the statement ends the file,
  no walk reaches that offset, so the rope kept an open layout scope and
  `validate_origin` raised `UnclosedScopes`. The "unless that owner's own
  prelude is the one reaching its end" rule of TASK-447 is unaffected: the
  prelude has returned by then.
- **Alternatives considered**: (a) Close every still-open owner block after
  the root body is emitted. That fixes the crash only at the end of the file
  and leaves the close attached to whatever source comes next elsewhere.
  (b) Close in `emit_statement_expr`, at the end of the source the statement
  covers, as `source_range_rope` does at the end of the span it walks.
- **Decision and rationale**: (b). The emission that consumes the plan for
  the statement is the one that passes the statement's end, so it closes the
  blocks ending there (`rewrite.source.end`). `close_owner_blocks_at` claims
  each close once, so an owner that ends later, such as one that includes a
  following `;`, is still closed by the source walk that copies it. Because
  the close now trims the statement's own rope, the prelude's trailing
  generated break no longer leaves a whitespace-only line before the `}`
  when the statement is followed by a line break. No existing test or
  fixture pinned that line.

### Decision 2: A validator failure's message carries no prefix of its own

- **Context**: TASK-221 put the `internal compiler error:` prefix in one
  place, `ice::report` (and `ice::bug_message` for `--server`). The
  `Display` of `InternalCompilerError` also wrote it, and `raise` panics
  with that text. `ice.rs` is excluded from the prefix check of TASK-221,
  so the duplicate was not caught.
- **Decision and rationale**: The `Display` now starts with the stage name,
  and `a_lowering_failure_reports_through_the_same_path` asserts that the
  rendered report contains the prefix exactly once.

## Work log

- 2026-09-29: Reproduced the crash for the four shapes and confirmed that the
  same sources with a trailing line break compile. Dumped the target pieces:
  the rope ended inside the owner's layout scope, without its `}`.
  Instrumented `close_owner_blocks_at`: no call reached the owner's end.
- 2026-09-29: `src/codegen/core/emitter/source.rs`: `emit_statement_expr`
  calls `close_owner_blocks_at(rewrite.source.end, out)` after it emits the
  owner prelude. `src/ice.rs`: removed the prefix from
  `InternalCompilerError`'s `Display` and extended the reporter test.
- 2026-09-29: Added
  `a_statement_match_that_ends_the_file_closes_the_block_it_hoists_into`
  (`tests/compile/cases_14.rs`): a script statement, `if`, `else`, `while`,
  and label bodies, and two statements, each with and without a trailing
  line break. Added the emit fixture `statement-match-at-end-of-file`
  (no trailing line break) with `UPDATE_EXPECT=1 cargo test --test snapshot`
  and read the generated file.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Changed `src/codegen/core/emitter/source.rs`, `src/ice.rs`,
`tests/compile/cases_14.rs`, and
`tests/fixtures/emit/statement-match-at-end-of-file/`. A statement value that
ends the file compiles, and a validator failure is reported with one prefix.
