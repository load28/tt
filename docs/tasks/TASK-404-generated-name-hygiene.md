# TASK-404: Allocate every generated binding name around the file's identifiers

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

Compiler-generated bindings could capture, or be captured by, a user
identifier with the same name. The emitted program then read the wrong
binding at runtime or did not load at all.

## Scope

- Included: Every name the TypeScript backend writes into a compiled file:
  match subject temporaries (`$tt_m`, `$tt_m_N`, `$tt_mI`, `$tt_mI_N`),
  propagation temporaries (`$tt_tN`, `$tt_rN`), the pipeline glue parameters
  (`$tt_v`, `$tt_r`, `$tt_k`), the chain label `$tt_b`, the Result exit labels
  `$tt_y_*`, the recovery binding `$tt_recovery`, duplicate-binding discards
  `$tt_discard*`, and the local names of the runtime import (`$tt_ap`,
  `$tt_fl`). The names already allocated by Evaluation IR planning (`$tt_vN`,
  `$tt_expr`, `$tt_raise`, `$tt_subject`) now use the same collision set.
- Included: The unexpected-case `throw` of a match now names that match's
  own subject, found while auditing the names (Issue 2).
- Excluded: `$tt_probe` (`src/engine/language.rs`), which the editor inserts
  into a scratch copy of the buffer for completion and never reaches compiled
  output, and `$tt_invalid_variant` (`src/parser/variants.rs`), the recovery
  name of a variant declaration that already has a reported syntax error.
  Neither is emitted for a file that compiles. `$tt_syntax_*` placeholders
  exist only in the SWC projection and are not emitted.

## Defects

1. `variant O { S(v: number), N }` / `const $tt_m = "user-m";` /
   `const r = match (O.S(1)) { S(v) => v + $tt_m, N => "" };` printed
   `1[object Object]`: the arm read the generated scrutinee `$tt_m`.
2. `const $tt_t0 = "user"; function f(): R<string> { const a = try Ok(2);
   return Ok(a + $tt_t0); }` read the `try` temporary instead of the user
   constant.
3. A user-declared `$tt_ap` or `$tt_fl` next to a pipeline gave
   `SyntaxError: Identifier '$tt_ap' has already been declared` (TypeScript:
   TS2440), and user `$tt_v`, `$tt_r`, `$tt_k` were shadowed inside member
   pipeline and postfix `flow` glue.

## Decisions

### Decision 1: One allocator, seeded with the names the file spells

- **Context**: `evaluation_ir::planning` already allocated `$tt_vN` against
  an occupied-name set, but that set is built from the SWC projection, which
  exists only when the file needs host lowering and replaces tt constructs
  with placeholders, so identifiers inside match arms were not in it. The
  backend wrote its own names as fixed strings.
- **Alternatives considered**: Renaming only in codegen with a second set
  would leave two allocators that could hand out the same name. Treating
  every `$tt_`-prefixed identifier as a user error would reject valid
  TypeScript, which breaks the first design contract (`AGENTS.md`).
- **Decision and rationale**: `src/generated_names.rs` holds the one
  allocation rule (`base`, then `base_1`, `base_2`, …, the rule the planner
  already used) and a `GeneratedNames` state. The occupied set is every
  identifier name in the file that starts with `$tt_`, collected by
  `lexer::identifier_names_with_prefix`; ProgramSyntax adds it to its SWC set,
  the planner allocates from the union, and `LoweringPlan` hands the resulting
  state to the emitter. A file without host lowering seeds the emitter from
  the lexer directly. The emitter maps each base name to one final name for
  the whole file (`GeneratedNames::stable`), so every place that wrote the same
  fixed name still writes one name. The mapping is injective (a new final
  name is never in the occupied set, which includes every final name already
  handed out), so it is a consistent renaming and preserves the program's
  scoping. Every generated name starts with `$tt_`, so only identifiers with
  that prefix can collide, and a file with no such identifier gets exactly the
  names it got before; no snapshot changed.

### Decision 2: Compare names by their ECMA-262 string value

- **Context**: ECMA-262 §12.7 (Names and Keywords) defines an identifier by
  the string value of its IdentifierName, so `$tt_m` and `$tt_m` are the
  same binding.
- **Decision and rationale**: The collector decodes `\uXXXX` and `\u{…}`
  escapes. A file that contains neither `$tt_` nor a backslash cannot spell a
  `$tt_` name and is not lexed again. JSX runs are opaque to the lexer, so
  every identifier-shaped run inside them is counted; that over-approximation
  can only rename a generated binding, never change a user one.

### Decision 3: Rename the runtime helpers with an import specifier

- **Context**: The runtime module's export names are fixed
  (`src/stdlib/runtime.ts`).
- **Decision and rationale**: When the local name differs, the import is
  written `import { $tt_ap as $tt_ap_1 }` (ECMA-262 §16.2.2, ImportSpecifier
  `ModuleExportName as ImportedBinding`), and every call uses the local name.

## Work log

- 2026-09-27: Reproduced all three defects with `ttc -p` (the `$tt_m` case
  printed `1[object Object]` under Node; the `$tt_ap` case failed with TS2440).
- 2026-09-27: Added `src/lexer/names.rs` and `src/generated_names.rs`;
  routed `allocate_generated_name`/`allocate_slot_name` through the shared
  rule; added the lexer names to `ProgramSyntax::occupied_names`; carried the
  planner's state on `LoweringPlan`; replaced every fixed name in
  `src/codegen/core/emitter/*` and the runtime import in
  `src/codegen/core/mod.rs` with `Emitter::generated_name`.
- 2026-09-27: Added `generated_names_are_allocated_around_the_files_identifiers`
  (tests/compile/cases_11.rs), and
  `generated_bindings_never_capture_user_identifiers` and
  `an_unexpected_case_reports_its_own_match_subject`
  (tests/integration/cases_05.rs). With `src/` stashed all three fail (the
  compile test on the missing `as $tt_ap_1`; the integration tests on TS2440,
  TS2365, TS2349 and TS2552); with the change they pass.

## Issues and resolutions

### Issue 1: Pipeline glue parameters shadowed user bindings

- **Symptom**: `x |> obj.add` wrote `(($tt_v, $tt_r) => ($tt_r.add)($tt_v))`,
  so a callee referring to a user `$tt_v` read the piped value.
- **Cause**: The glue names were string literals in
  `src/codegen/core/emitter/expression.rs`.
- **Resolution**: The glue asks the allocator for each name.

### Issue 2: A nested match's unexpected-case throw named the wrong subject

- **Symptom**: `match (match (s) { … }) { "a" => 1, "b" => 2 }` wrote
  `JSON.stringify($tt_m)` in the outer match's `default`, where the outer
  subject is `$tt_m_1` and `$tt_m` is out of scope (TS2552, and a
  `ReferenceError` when reached).
- **Cause**: `unexpected_throw` wrote the fixed name `$tt_m` for literal and
  case matches instead of the decision's own subject, which the tuple form
  already used.
- **Resolution**: All three forms read the subject reference the arm tests
  read (`subject_reference`, shared with `emit_place`).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` (no snapshot update needed)
- [x] `node scripts/check-task-index`

## Result

Changed files: `src/generated_names.rs` (new), `src/lexer/names.rs` (new),
`src/lexer.rs`, `src/lib.rs`, `src/program_syntax/projection.rs`,
`src/evaluation_ir.rs`, `src/evaluation_ir/evaluation.rs`,
`src/evaluation_ir/planning.rs`, `src/codegen/core/mod.rs`,
`src/codegen/core/emitter/{mod,helpers,expression,pattern,result,host,source}.rs`,
`tests/compile/cases_11.rs`, `tests/integration/cases_05.rs`.
Generated names can no longer collide with the file's identifiers; output for
files without `$tt_` identifiers is unchanged.
