# TASK-490: Decide the `val` modifier structurally, once, in the parser

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

An architecture audit found that `val` parameter detection was a token-shape heuristic. `flow::opens_parameter_list` guessed whether a `(` opened a parameter list from what followed its `)` (`=>`, `{`, or `:`), walking back through `annotated_paren` and guessing from `call_shaped`. `val::modifier_at` ran that guess again at four call sites (the parser, `uses_val`, `parse_params`, and the checker's `visit_ident` and `instantiate`). The guess was wrong in two valid TypeScript files, which lost the variable name `val`:

- `c ? (val [0]) : w => w` became `c ? ([0]) : w => w`. The `(val [0])` was read as an arrow head.
- `f(val [0])` followed by a block on the next line became `f([0])`. The call was read as an Allman method head.

## Scope

- Included: The parser's decision on `val` (`src/parser/vals.rs`, `src/parser/parse.rs`, `src/parser/host.rs`). The AST node that records it (`src/ast.rs`). The consumers that now read it: the `val` checker and probes (`src/val.rs`, `src/val/checker.rs`), compile and probe entry points (`src/lib/compile.rs`, `src/lib/mapped.rs`), HIR lowering (`src/hir/lower.rs`), and the engine's removal edit (`src/engine/projection.rs`). Removal of the flow heuristics (`src/flow/syntax.rs`, `src/flow/mod.rs`). Documentation and tests.
- Excluded: How the `val` checker models scopes (`Checker::function_body` still finds parameter scopes from the tokens). The `val` semantics are unchanged.

## Decisions

### Decision 1: Verify the audit's grammar claim before relying on it

- **Context**: The audit said that only `val [` is ambiguous: an identifier, `{`, or `...` after `val` is never valid TypeScript.
- **Alternatives considered**: Take the claim as given.
- **Decision and rationale**: Checked each shape with the pinned TypeScript 7.1 (`tsc --noEmit` on one-line files):
  - `f(val {a})`, `f(val ...a)`, and `f(val x)` are all syntax errors (TS1005).
  - `val` followed by a line break and then `{ }` is valid because ASI splits it into an expression statement and a block. The same-line rule excludes this case.
  - `f(val [0])` and `type T = [val [number]]` are valid (an element access and an indexed access type). This is the ambiguity.
  - The claim is false for decorators. `class D { @val x = 1 }`, `class D { @val [k]() {} }`, and `constructor(@val x: number)` are all valid. In these, `val` is preceded by `@`, never by `(` or `,`. So the entry-start requirement (`(`/`,` and optional parameter-property modifiers before `val`) already excludes them. `tests/passthrough.rs::val_decorators_stay_decorators` locks this.

### Decision 2: The host grammar decides every parameter-shaped `val`, not only `val [`

- **Context**: The design sketch used SWC to settle `val [` (TypeScript owns it when SWC produces a member access on `val`) and claimed the other shapes from their tokens.
- **Alternatives considered**:
  - Probe only `val [` and accept every `val x`/`val {`/`val ...` at an entry start. This silently turns invalid TypeScript such as `f(val x)` into `f(x)`. It also still needs a positive test for "TypeScript owns this bracket" that covers element access, indexed access types, and tuple elements.
  - Probe the verbatim reading and look for a `val` identifier node. This needs a second classification of which host nodes count as ownership.
- **Decision and rationale**: The parser lifts each parameter-shaped `val` tentatively and records it in `Program::host_val_candidates` (the span from the keyword to the binding start). `host::rejected_val_candidates` projects each region in its tt reading: the lifted constructs are masked as before and every `val` modifier is erased. It then parses the projection with SWC under the region's wrappers. A candidate stays a modifier only if some projection puts a formal parameter's binding at its binding start. Formal parameters are `Param`, `TsParamProp`, arrow params, the `catch` param, and `TsFnParam`. Every other candidate goes to a sorted rejection list, and the file is parsed again with those left verbatim. This happens before the `match` ownership probe, which therefore sees the settled modifiers.
  - This is sound for valid TypeScript. When `val [` at an entry start is valid TypeScript, it is an expression or a type there. Erasing `val` leaves an array literal or tuple type in the same grammatical position, and no valid program has a `val[...]` element access where the erased reading would be a parameter (`(val[0]) => x` is itself a syntax error). So TypeScript keeps it. The operator-word filter (`as`, `in`, `of`, `extends`, …) stays so that the tt reading of a remaining candidate is still well-formed TypeScript.
  - Candidates are unioned across regions: a proof from any projection that shows the binding is enough. This matters when a pipeline head re-parses bytes that its parent region masks.
  - A region the host cannot parse proves nothing, so its candidates stay identifiers.
- The decision is recorded once as `Segment::ValModifier(ValModifier { span, kind })`, with `ValModifierKind::{Declaration, Parameter}`. `parser::val_modifiers` indexes it by keyword offset. `val::check_all`, `val::probes`, `parse_params`, and the checker read that map, and `ValBinding::modifier_end` carries the AST span end to the engine's removal edit. `modifier_at`, `enclosing_open`, `uses_val`, `opens_parameter_list`, `follows_function_keyword`, and `operand_expected_before` are deleted.

### Decision 3: Signature parameters are parameters

- **Context**: TASK-452 left `val` on bodyless interface/abstract signatures unclaimed because the token model could not prove those lists.
- **Alternatives considered**: Keep excluding them by filtering `TsFnParam` out.
- **Decision and rationale**: The host AST proves them directly, so the rule is now just "a formal parameter". This includes `TsFnParam` in method, call, and construct signatures and in function types. The modifier has no semantic effect there, as before. `docs/ai/tt.md` states the rule. This reverses TASK-452 Decision 2, and its record says so.

## Work log

- 2026-09-28: Reproduced with the previous `ttc -p`. `c ? (val [0]) : w => w` emitted `c ? ([0]) : w => w`, and `f(val [0])` followed by `{` on the next line emitted `f([0])`.
- 2026-09-28: Checked the grammar claims with `tsc --noEmit` (Decision 1).
- 2026-09-28: Added `ValModifier`/`ValModifierKind` and `Program::host_val_candidates` to `src/ast.rs`. Added `src/parser/vals.rs` (the lexical shapes and `modifier_end`). Added `Parser::val_modifier_at`, the probe-then-reparse order in `lex_and_parse_with_kind`, and `val_modifiers` in `src/parser/parse.rs`. Added `rejected_val_candidates`, `ParameterCollector`, and a shared `SourceFrame` in `src/parser/host.rs`; the `match` collector now uses the same frame.
- 2026-09-28: Made the `val` analysis read the parser's map (`src/val.rs`, `src/val/checker.rs`, `src/lib/compile.rs`, `src/lib/mapped.rs`). Added `ValBinding::modifier_end` for `src/engine/projection.rs`. Deleted the flow heuristics.
- 2026-09-28: Added tests. In `tests/passthrough.rs`: `val_element_access_never_becomes_a_parameter_by_its_surroundings` (both ternary shapes, the next-line block after a call, index/tuple types) and `val_decorators_stay_decorators`. In `tests/compile/cases_07.rs`: parameter cases for an arrow in a ternary, a setter, interface/construct/call signatures, a function type, and a template interpolation. In `tests/compile/cases_08.rs`: `val_parameter_beside_an_element_access_of_a_variable_named_val`, where a real `val` parameter is still checked beside the ambiguous shapes.
- 2026-09-28: Moved the host candidate lists out of line (Issue 2). `TTC_REQUIRE_TSGO=1 cargo test` passed: 40 suites, 1445 tests, 0 failures.
- 2026-09-28: Updated `docs/ai/tt.md`, `docs/design/compiler-architecture.md`, and the TASK-452 record.

## Issues and resolutions

### Issue 1: The engine re-derived the modifier's byte span

- **Symptom**: Removing `val::modifier_end` broke `src/engine/projection.rs`, which recomputed the end of the removal edit from the source text.
- **Cause**: The engine was a fourth consumer that re-derived part of the parser's decision.
- **Resolution**: `ValBinding` now carries `modifier_end`, taken from the AST node's span.

### Issue 2: A new `Program` field tripped `clippy::large_enum_variant`

- **Symptom**: With an inline `Vec<Span>` for `val` candidates, `cargo clippy -- -D warnings` rejected `IfLetElse`, `TemplateChunk`, `ParsedMatch`, and `pipes::Attempt`, whose largest variants embed a `Program`.
- **Cause**: `Program` grew by 24 bytes, past the lint's size-difference threshold.
- **Resolution**: Both host-decided position lists (`match` and `val`) moved into one out-of-line `Option<Box<HostCandidates>>`, the same pattern as `unclaimed`. `Program` ends up 16 bytes smaller than before, and callers read the lists through `Program::host_match_candidates()` and `host_val_candidates()`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.
- [x] The two repros pass through unchanged with the new binary and failed with the previous one.

## Result

Changed `src/ast.rs`, `src/parser/{mod,parse,host,vals}.rs`, `src/val.rs`, `src/val/checker.rs`, `src/lib/{compile,mapped}.rs`, `src/hir/lower.rs`, `src/engine/projection.rs`, `src/flow/{syntax,mod}.rs`, `tests/passthrough.rs`, `tests/compile/cases_07.rs`, `tests/compile/cases_08.rs`, `docs/ai/tt.md`, `docs/design/compiler-architecture.md`, `docs/tasks/TASK-452-val-parameter-list-context.md`, `docs/tasks/INDEX.md`, and this record. `val` is now decided once, in the parser, with the host grammar as the judge of parameter position.
