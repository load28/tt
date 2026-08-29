# TASK-287: Close shipped claimer gaps

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: `TASK-287: close shipped claimer gaps`

## Purpose

Item **P5** of the ratified `try`/`result` scopes plan. The contract is
[`docs/design/try-result-scopes.md`](../design/try-result-scopes.md); read §9's
`P5` entry together with the sections it depends on before starting. Do not
restate or reinterpret the design here — record only what this task decides.

## Scope

- Included: make `for` update, object/array spread, and local destructuring defaults either enter the existing placement protocol or receive the intended located rejection instead of `verify-failed`.
- Excluded: anything owned by another item of the §9 plan. This task does not
  reopen a placement, ownership, or diagnostic decision that §4–§6 and §11
  already settle; a decision that must change is a change to the design document
  first.

Files and symbols named by the plan: `src/parser/tries.rs`, AST/HIR traversal, ProgramSyntax owner projection, sema recovery.

## Decisions

Record every decision this task makes. Naming a new public diagnostic code, a
wire-compatibility choice, or a seam that the design leaves open is a decision
and belongs here with its alternatives.

### Decision 1: Claim only C-style `for` update as an expression propagation.

- **Context**: The semicolon preceding a `try` in a `for` header is not
  sufficient to distinguish Test from Update. Test already has a structural
  statement-propagation path that preserves its repeated-evaluation reason.
- **Alternatives considered**: Treat every `try` after a `for` header
  semicolon as an expression; infer the position from source text.
- **Decision and rationale**: Count top-level header separators from lexer
  tokens and claim only the position after the second separator. This keeps
  the established Test diagnostic while giving Update a located
  `try-placement` result.

### Decision 2: Recognize spread operands and binding defaults by token shape.

- **Context**: The final dot of `...try` looked like member access, while a
  binding default's `=` was treated as its declaration initializer.
- **Alternatives considered**: Special-case source spellings; relax all
  dotted `try` handling; lower destructuring defaults as owner statements.
- **Decision and rationale**: Identify three adjacent dot tokens as a spread
  operator and stop declaration-initializer detection at an enclosing binding
  delimiter. Spread expressions enter the existing evaluation protocol;
  defaults receive the existing expression-boundary `try-placement`.

## Work log

- 2026-08-30: Started reproducing unclaimed expression edges.
- 2026-08-30: Added claim/recovery coverage for C-style Update, object and
  array spread, and local destructuring defaults, plus TypeScript `try`
  member/property passthrough controls.
- 2026-08-30: Restored the pre-existing repeated-Test path after confirming
  that only Update requires expression claiming.
- 2026-08-30: Ran the required Rust checks and the full repository CI gate.

## Issues and resolutions

### Issue 1: `try` in a spread operand was ignored as a member name.

- **Symptom**: Object and array spread candidates produced no tt diagnostic
  and could reach verification without a claimed propagation.
- **Cause**: The generic dotted-property predicate classified the third dot
  of `...` as member access.
- **Resolution**: The parser now recognizes the three-token spread operator
  before applying the property-name exclusion.

### Issue 2: Broad `for` header recovery changed Test diagnostics.

- **Symptom**: The existing repeated-Test lowering diagnostic became a
  generic expression-boundary placement diagnostic.
- **Cause**: The initial recovery classified every header position after a
  semicolon as an expression.
- **Resolution**: Recovery now counts top-level header separators and applies
  only to Update after the second separator.

## Verification

Test obligation from the plan: one claim/recovery test per syntactic edge plus passthrough controls for members/properties named `try`.

Green condition: every candidate is either valid emitted TypeScript or a located `try-placement`, never “did not parse as tt try.”

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Ships to `main` alone: yes.

Changed `src/parser/mod.rs` to classify C-style Update, spread operands, and
destructuring defaults structurally. Changed `tests/compile.rs` to prove the
two accepted spread rewrites, the two placement rejections, and TypeScript
passthrough controls. Every scoped candidate now has a valid emitted result or
a located `try-placement` diagnostic.
