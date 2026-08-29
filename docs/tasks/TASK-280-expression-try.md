# TASK-280: Lower `try` as an expression through the host evaluation protocol

- **Status**: Complete
- **Started**: 2026-08-29
- **Completed**: 2026-08-29
- **Commit**: —

## Purpose

Allow Result propagation to produce a value in TypeScript expression positions
without changing JavaScript evaluation order or returning from a generated value
region instead of the enclosing user function.

## Scope

- Included: expression `try` syntax, AST/HIR/Core representation, host evaluation
  protocol integration, source-preserving TypeScript lowering, diagnostics, tests,
  and language documentation
- Excluded: Option propagation, implicit error conversion, and changes to the
  existing Result representation

## Decisions

### Decision 1: Reuse the whole-program host evaluation protocol

- **Context**: A nested propagation point may sit behind eager, conditional,
  reference, suspension, or generated value-region boundaries.
- **Alternatives considered**: Hoist selected source shapes with parser
  heuristics; consume TypeScript's internal narrowing CFG; or represent `try` as
  a value-producing Core expression and let the existing SWC-backed host
  evaluation protocol preserve the surrounding TypeScript operation.
- **Decision and rationale**: Use the existing host evaluation protocol. It
  already owns TypeScript evaluation order and source-backed host continuations;
  syntax-specific hoists would violate the compiler-layer contract, while the
  TypeScript `FlowNode` graph is an internal type-narrowing structure rather than
  tt's target-lowering model.

### Decision 2: Give expression `try` prefix-primary precedence

- **Context**: The motivating `try total() * 1.1` must unwrap `total()` before
  multiplication, while users still need a way to propagate a conditional or
  other low-precedence expression as one value.
- **Alternatives considered**: Consume through the enclosing delimiter; require
  parentheses for every operand; or bind to one primary expression and use
  parentheses to widen the operand.
- **Decision and rationale**: Bind to the following primary expression,
  including ordinary postfix calls and member/index access. Parentheses widen
  the operand. This matches prefix-operator expectations without making the
  parser infer TypeScript precedence from delimiters.

### Decision 3: Diagnose from the typed target capability

- **Context**: A generated early return is sound in an eager argument or a
  reconstructed conditional branch, but not in a loop header, parameter
  default, class field initializer, or module owner.
- **Alternatives considered**: Maintain a parser context allowlist; attempt an
  expression-boundary callback; or derive placement from Evaluation IR.
- **Decision and rationale**: Evaluation IR records unsupported expression
  propagations after resolving the SWC host owner, schedule, and target
  capability. Semantic diagnostics consume that fact, and recovery emits a
  source-mapped placeholder without attempting unsound lowering.

## Work log

- 2026-08-29: Re-read `AGENTS.md`, confirmed `./scripts/doctor` had passed, and
  verified a clean `main` at `origin/main` before creating the task branch.
- 2026-08-29: Created the task record before implementation changes.
- 2026-08-29: Added expression propagation across AST, HIR, Core IR,
  ProgramSyntax, Evaluation IR, semantic diagnostics, and source-preserving
  TypeScript emission.
- 2026-08-29: Added prefix-primary operand parsing and expression-root parsing
  for nested tt value programs.
- 2026-08-29: Added compile, runtime, native typed-path, and content-mapper
  coverage, then updated the language guide and diagnostic explanation.
- 2026-08-29: Passed the complete local CI gate, including Rust, npm, native
  typed integration, and VS Code extension suites.

## Issues and resolutions

### Issue 1: The default `codex/` branch namespace is occupied by a branch

- **Symptom**: `git switch -c codex/task-280-expression-try` failed because
  `refs/heads/codex` already exists as a branch.
- **Cause**: Git cannot use the same ref as both a branch and a directory prefix.
- **Resolution**: Created `codex-task-280-expression-try`, matching the existing
  repository convention for Codex task branches.

### Issue 2: Expression propagation was absent from temporary numbering

- **Symptom**: The first expression-form lowering reached code generation with
  no deterministic propagation temporary.
- **Cause**: Temporary ordinal collection only visited statement propagations.
- **Resolution**: Included HIR expression propagations in the shared source-order
  ordinal traversal.

### Issue 3: The source-preservation pass double-owned expression `try`

- **Symptom**: A lowered expression propagation overlapped bytes also marked as
  pass-through source.
- **Cause**: The new Core expression node's complete span was collected as
  verbatim instead of only traversing its operand.
- **Resolution**: Treat the propagation as claimed source and recurse into its
  operand for pass-through spans.

### Issue 4: Value delivery emitted a region break for linear propagation

- **Symptom**: Direct expression propagation generated a `break` outside any
  decision region.
- **Cause**: It reused decision-arm delivery, whose continuation contract ends a
  generated region.
- **Resolution**: Emit the success payload as a linear continuation assignment;
  only decision regions emit their own break.

### Issue 5: Unsupported loop-header propagation reached target emission

- **Symptom**: `while (try condition())` reached an unscheduled-expression
  internal error.
- **Cause**: The initial implementation had no semantic bridge from Evaluation
  IR's expression-boundary capability to diagnostics and recovery.
- **Resolution**: Record every expression propagation's ultimate host
  capability, emit `try-placement`, and recover with a source-mapped
  `undefined` placeholder only on projection paths.

### Issue 6: A structured match arm eagerly emitted its inline body

- **Symptom**: An expression `try` used as a match-arm value reached inline
  emission before the arm's continuation could lower it.
- **Cause**: Match-arm emission built the fallback body before selecting the
  structured continuation path.
- **Resolution**: Select and emit the structured body first, and build the
  inline fallback only when no statement-form continuation exists.

### Issue 7: A `result` block has its own propagation target

- **Symptom**: An expression `try` in a result block's final value placed
  propagation statements inside an expression and produced invalid TypeScript.
- **Cause**: Result regions intentionally own `<-` exits and do not expose their
  final value as an ordinary TypeScript host schedule.
- **Resolution**: Added an explicit result-region placement fact and diagnose
  expression `try` there with guidance to use `<-`.

### Issue 8: Editor coverage still asserted the removed placement error

- **Symptom**: The VS Code server suite expected `return try value()` to report
  `try-placement`.
- **Cause**: The test encoded the former language rule changed by this task.
- **Resolution**: Replaced it with the remaining placement contract: a loop
  header reports the typed TypeScript control-flow boundary at `try value()`.

### Issue 9: macOS temporary paths used two spellings

- **Symptom**: Install-path tests compared `/var/...` with `/private/var/...`
  during the first complete gate run.
- **Cause**: macOS exposes `/var` through a symlink while the test process and
  spawned tools canonicalized the temporary directory differently.
- **Resolution**: Ran the complete gate with `TMPDIR=/private/tmp`, giving every
  process one canonical temporary root without changing product behavior.

### Issue 10: Interrupted editor tests left test-owned child processes

- **Symptom**: A later typed test run timed out after earlier editor test runs
  were interrupted.
- **Cause**: Interrupting the Node test harness prevented its compiler and
  language-server children from completing normal cleanup.
- **Resolution**: Identified and terminated only the orphaned test processes,
  then ran the complete gate cleanly to completion.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `TMPDIR=/private/tmp ./scripts/ci`

## Result

Expression `try` now participates in the same value-producing Core and host
evaluation protocol as other nested tt expressions. It preserves TypeScript
evaluation order in supported expression positions and reports `try-placement`
at the original expression boundary when the resolved host cannot support an
early return. Language documentation and compile, runtime, typed-native,
content-mapper, and editor coverage now record the contract.
