# TASK-765: Keep valid TypeScript out of tt claims and rewrites

- **Status**: Complete
- **Started**: 2026-10-05
- **Completed**: 2026-10-05
- **Commit**: `558eb746`

## Purpose

An audit of the pass-through contract found valid TypeScript claimed as a
malformed `match` or `variant`, relative specifiers left unrewritten, a
`val` parameter modifier missed after a decorator, and nested template
literals analysed in quadratic time. Fix each in the layer that owns it.

## Scope

- Included: malformed `match` and `variant` claims, import types in variant
  field types, escaped module specifiers, `val` after parameter
  decorators, and the analysis cost of nested template literals.
- Excluded: the emitter's rope appends, which still copy nested pieces
  once per level (see Result); `val this` parameters.

## Decisions

### Decision 1: A malformed `match` claim is proven against the host reading

- **Context**: `match` followed by an identifier and a brace whose body
  holds `=>` was claimed as a malformed match, so `class match extends A {
  m = () => 1 }`, `match as { f: () => void }`, and similar valid
  TypeScript failed with `malformed-match`.
- **Alternatives considered**: List the TypeScript words that may follow
  an identifier (`as`, `satisfies`, `extends`, ...) and the declaration
  keywords that may precede it; reuse the existing host-ownership probe.
- **Decision and rationale**: A list repeats TypeScript's grammar in part
  and misses positions (decorators, type parameters, mapped keys). The
  parser already asks swc whether the host owns an ambiguous `match`; a
  malformed claim is now always such a candidate, probed with its original
  text first (a malformed claim is an error either way, so valid TypeScript
  wins), and any identifier named `match` the host AST reads there owns
  it. A claim the host cannot parse falls back to the masked tt reading,
  so `match user { A => 1 }` is still reported.

### Decision 2: A malformed `variant` is committed only at a declaration position

- **Context**: `variant` followed by an identifier and `<` or `{` was
  committed anywhere, so `(variant as {})`, `<variant extends {...}>`, and
  `variant satisfies {}` were claimed.
- **Alternatives considered**: The host probe of Decision 1; TypeScript's
  rule for contextual declaration keywords.
- **Decision and rationale**: typescript-go reads `type` and `interface` as
  declarations only where a statement begins and the next token is an
  identifier on the same line (`isStartOfDeclaration`,
  `nextTokenIsIdentifierOnSameLine` in `internal/parser/parser.go`). A
  `variant` declaration is committed under the same rule, with the lexer's
  `statement_start` and `modified` facts naming the position; `export
  default variant` passes its owning `export`.

### Decision 3: Field types carry their import specifiers

- **Context**: `variant V { A(m: typeof import("./a.tt")) }` kept the `.tt`
  specifier, because field type text is copied verbatim.
- **Alternatives considered**: Parse field types as sub-programs; rewrite
  the text in codegen; lift the specifiers.
- **Decision and rationale**: The field parser lifts each specifier with
  the same import parser the rest of the file uses, HIR and Core carry them
  as import nodes, and `emit_adt` writes them through `emit_import`, so a
  field type follows every rewrite mode and `@tt/std` placement.

### Decision 4: Specifiers are read by their string value

- **Context**: `"./f.\x74t"` names `./f.tt` but was compared as raw bytes.
- **Alternatives considered**: Keep raw comparison (an unresolvable import
  in the output); decode the literal.
- **Decision and rationale**: TypeScript resolves a module specifier by its
  string value. The literal is decoded with the parser's existing string
  decoder; when the extension is written plainly the raw text before it is
  kept, otherwise the rewritten value is written as a new literal.

### Decision 5: Parameter decorators precede a `val` modifier

- **Context**: `constructor(@dec private val p: T)` left `val` in the output
  (`verify-failed`).
- **Alternatives considered**: None that keeps TypeScript's grammar.
- **Decision and rationale**: A parameter is `Decorators? Modifiers?
  BindingElement`. The `val` shape walk and both parameter readers in the
  `val` analysis skip decorators the same way.

### Decision 6: Facts that only depend on a subtree are computed once

- **Context**: 10,000 nested template literals took 21 s.
- **Alternatives considered**: Leave it (pathological input); memoize.
- **Decision and rationale**: `has_statement_form` is a pure fact of the
  immutable Core graph and is now computed once per expression when the
  file is lowered. The host probe skips a region with no `match`
  candidates in it or its interpolations, decided once per region. 10,000
  levels now take 0.8 s.

## Work log

- 2026-10-05: Reproduced each defect, read the matching typescript-go
  parser code, and fixed it in `src/parser/{parse,host,variants,vals,
  imports,literals,mod}.rs`, `src/ast.rs`, `src/hir/`, `src/core_ir/`,
  `src/codegen/core/emitter/`, `src/val.rs`, `src/val/checker.rs`, and
  `src/lib/api.rs`; profiled the nested template case with gdb stacks.

## Issues and resolutions

### Issue 1: The host probe never tried the original text of a malformed claim

- **Symptom**: With malformed claims as candidates, four of ten cases still
  failed.
- **Cause**: The probe starts from the masked tt reading, and `void 0` in
  place of `match as {...}` is valid TypeScript; the recovery node was
  also masked separately.
- **Resolution**: Malformed candidates start restored and switch to the tt
  reading only when their own text fails to parse; the recovery mask is
  skipped for them.

## Regression test (fails before the fix)

- **Path**: `tests/passthrough.rs`
  `match_as_an_identifier_beside_a_braced_arrow_is_not_a_malformed_match`,
  `match_as_an_identifier_beside_a_tt_match_stays_an_identifier`,
  `variant_as_an_identifier_outside_a_declaration_position_passes_through`;
  `tests/cases/compiler/anImportTypeInAVariantFieldTypeIsRewritten.tt`,
  `anEscapedRelativeSpecifierIsRewrittenByItsValue.tt`,
  `aValParameterAfterADecoratorIsAModifier.tt`; `src/lib/scaling_tests.rs`
  `every_request_does_linear_work_in_the_nesting_depth_of_templates`,
  `a_nested_template_with_a_host_candidate_collects_its_facts_once_per_level`.
- **Observed failure**: Without the source changes the three pass-through
  tests failed with `malformed-match`/`malformed-variant` compile errors;
  the field type kept `import("./a.tt")`; `"./f.\x74t"` stayed unrewritten;
  the decorated `val` gave `verify-failed`; the scaling tests reported
  `host region facts: 5151 units for n matches but 20301 for 2n`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (every suite passed)
- [x] Baseline changes reviewed and committed with the change: three new
  cases; no existing baseline changed

## Result

Five defects fixed and the nested-template analysis made linear. Follow-up debt: the emitter returns a rope per expression and
appends it to its parent, which copies a nested template's pieces once per
level (50,000 levels take 28 s).
