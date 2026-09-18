# TASK-381: Resolve PR 125 structural review findings

- **Status**: Complete
- **Started**: 2026-09-18
- **Completed**: 2026-09-18
- **Commit**: —

## Purpose

Resolve the three reproduced findings on PR 125 without input-specific exceptions or larger fixed recursion limits.

## Scope

- Included: host member ownership of `try`, complete guard evaluation regions, and stack-safe host syntax traversal.
- Excluded: unrelated compiler and editor changes.

## Decisions

### Decision 1: Preserve a negative statement ownership claim

- **Context**: `parse_try_stmt` already rejects annotation-free method signatures as host syntax, but the added expression retry overrode that proof.
- **Alternatives considered**: Special-case interface strings or add another host probe; both duplicate an existing structural distinction.
- **Decision and rationale**: Remove the statement-position expression retry. Genuine expression positions keep their existing `parse_try_expr` path; unbraced propagation statements keep the statement parser.

### Decision 2: A complete guard is an evaluation owner

- **Context**: Selecting the last structured guard child loses sibling evaluations and their conditional dependencies.
- **Alternatives considered**: Emit every child eagerly or extend the last-child schedule; neither represents conditional siblings correctly.
- **Decision and rationale**: Map each projected guard statement to its complete source extent so the existing host planner owns all guard values. Emit that owner's prelude after pattern bindings and before its test. Both Core and source-walk entry points consume each compose prelude once. Remove the guard-specific last-child emitter.
- Grouping parentheses unwrap to, but never into, a placeholder. A whole-operation replacement consumes every inline value occurrence it covers, including the condition, while each separately evaluated value retains its own source during structural emission. These rules also apply outside guards.

### Decision 3: Grow the stack at the recursive host expression visitor

- **Context**: LLDB reproduced the crash in the generated `VisitAstPath` traversal through `ParentCollector::visit_expr` and `ParenExpr`.
- **Alternatives considered**: Increase the 256 MiB reservation or reduce the regression depth; both only move the failure threshold. Replacing the complete generated visitor would duplicate SWC's AST-path semantics.
- **Decision and rationale**: Use `stacker::maybe_grow` at the expression visitor's recursion boundary. This preserves every AST path and grows only when remaining stack is low. `stacker` was already present transitively through SWC; it is now an explicit dependency. No unsafe code or input-depth exception is added.

## Work log

- 2026-09-18: Checked the existing PR branch and remote head, ran `./scripts/doctor`, and confirmed the working tree contains only the pre-existing untracked `.task-agent-disabled` file. Continued on the PR branch as explicitly requested.

- 2026-09-18: Added compiler/passthrough regressions, a whole-output guard fixture, runtime checks for logical and ternary guards, and a server-survival regression. The runtime cases use nontrivial block arms so statement lowering, rather than only expression inlining, is exercised.
- 2026-09-18: Regenerated and inspected the new TypeScript snapshot. Existing snapshots remained unchanged. Started the complete local gate. The first pass exposed the loop-test join-slot distinction; restricted compose occurrence consumption to compose owners, leaving loop owners' existing test replacement protocol intact. Its regression now passes. npm installation and website preview were blocked by sandbox network/listen restrictions, so the complete gate is rerun with the required permissions.

## Issues and resolutions

### Issue 1: Host methods are reinterpreted as propagation

- **Symptom**: `interface X { try(x); }` produces `try-placement`.
- **Cause**: The expression retry overrides a `NotTt` statement claim.
- **Resolution**: Preserve `Claim::NotTt` at statement positions; verify pure and mixed-source method signatures.

### Issue 2: Guards lose earlier structured values

- **Symptom**: Two matches joined by `&&` in a guard leave raw match syntax in the output.
- **Cause**: Guard emission selects the last child instead of planning the whole region.
- **Resolution**: Give the full guard a source-mapped host owner; share complete evaluation planning and source-replacement ownership with ordinary host expressions.

### Issue 3: Deep host syntax terminates the compiler

- **Symptom**: A variant followed by 100,000 parentheses aborts both CLI and server with stack overflow.
- **Cause**: A fixed stack reservation does not protect all recursive host syntax operations.
- **Resolution**: Grow the stack at the confirmed recursive expression-visitor boundary; check both a 100,000-level mixed-source request and the following server request.

- 2026-09-18: The unrestricted full gate passed all Rust tests but found the separate fuzz lockfile also needed the explicit `stacker` dependency. Updated only that dependency edge and reran `cargo check --manifest-path fuzz/Cargo.toml --all-targets --locked` successfully; no compiler source changed after the passing Rust test run.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (with `TTC_REQUIRE_TSGO=1`)
- [x] `./scripts/ci` (all stages; final locked fuzz check resumed after its lockfile update)
- [x] `cargo check --manifest-path fuzz/Cargo.toml --all-targets --locked`
- [x] `UPDATE_EXPECT=1 cargo test --test snapshot` and full snapshot inspection

## Result

Changed files: `Cargo.toml`, `Cargo.lock`, `fuzz/Cargo.lock`, `src/parser/parse.rs`, `src/program_syntax/{projection,protocol,visit}.rs`, `src/stack.rs`, `src/codegen/core/planning.rs`, `src/codegen/core/emitter/{pattern,source}.rs`, `tests/passthrough.rs`, `tests/compile/cases_11.rs`, `tests/integration/cases_05.rs`, `tests/cli.rs`, `tests/fixtures/emit/guard-evaluation-owner/{input.tt,expected.ts}`, and task records/index.

All three review findings are fixed with passing compiler, passthrough, runtime, snapshot, and server-survival regressions. Every local gate command passed; the full script's final failing command was resumed successfully after updating the fuzz lockfile. The output fixture was inspected in full.
