# TASK-690: Emit a single-case variant as its case's object type so TypeScript names it

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-690`

## Purpose

TASK-686 Issue 4: signature help on `Pub.Item(` for `export variant Pub {
Item(n: number) }` showed `Item(n: number): { kind: "Item"; n: number; }`,
where the TypeScript twin shows `Item(n: number): Pub`. Every place
TypeScript prints the variant's type (hover, signature help, quick info,
diagnostics) printed the structure instead of the name.

## Scope

- Included: the type alias `emit_adt` writes for a variant with one case
  (`src/codegen/core/emitter/helpers.rs`), the emitted-output tests and
  baselines that pin it, a hand-written editor case, the five generated
  cases and one list line that recorded the defect, and the emitted form in
  `docs/ai/tt.md`.
- Excluded: variants with two or more cases, whose union is unchanged.

## Decisions

### Decision 1: Why TypeScript prints the alias structurally

- **Context**: ttc emitted every variant as a union with a leading `|`,
  one member per line: `export type Pub =\n  | { kind: "Item"; n: number };`.
- **Cause, confirmed with a plain-TypeScript probe** (an editor case whose
  unit is plain TypeScript, so ttc passes it through byte for byte): for
  `type Lead =\n  | { kind: "Item"; n: number };`, `type Plain = { kind:
  "Item"; n: number };`, and `type Paren = ({ kind: "Item"; n: number });`,
  hover on a `declare const` of each shows `{ kind: "Item"; n: number; }`
  for `Lead` and `Plain` / `Paren` for the others. In typescript-go,
  `parseUnionOrIntersectionType` (`internal/parser/parser.go`) builds a
  `UnionType` node whenever a leading `|` is written, even with one
  constituent. `getTypeFromUnionTypeNode` (`internal/checker/checker.go`)
  passes the alias to `getUnionTypeEx`, which returns its only constituent
  unchanged (`if len(types) == 1 { return types[0] }`), so the alias is
  never attached to it; and that constituent, an object type literal, takes
  its alias from `getAliasSymbolForTypeNode`, which looks through
  parentheses and `readonly` to a type alias declaration but not through a
  union node. The object type therefore has no alias symbol and is printed
  structurally. Written directly (or in parentheses) as the alias's type,
  the object literal gets `Pub` as its alias symbol.

### Decision 2: Emit the case's object type as the alias

- **Alternatives considered**: (a) Rewrite the displayed text in the
  engine (replace the structural type by the variant's name). The printed
  type is TypeScript's answer in hover, signature help, completion details,
  diagnostics, and declaration emit; a textual rewrite would have to find
  every one of them, would be a guess about which structural text is the
  variant, and would leave `tsc` and the `.d.ts` sidecars printing the
  structure. (b) Parenthesize (`type Pub = ({ ... })`): it works
  (`getAliasSymbolForTypeNode` skips parentheses) but writes syntax nobody
  writes. (c) Keep the union and add a second member (`| never`): a type
  trick the emission contract forbids (AGENTS.md: "no type tricks").
- **Decision and rationale**: (d) a variant with one case is emitted as
  that case's object type, the declaration a TypeScript user writes for a
  one-case tagged type: `export type Pub =\n  { kind: "Item"; n: number };`.
  The layout keeps the line break after `=` and moves the case to the
  alias's indentation, and a case written one field per line (fields with
  comments) closes at the alias's indentation. Two or more cases keep the
  leading `|` layout. The change is in the emitter, the layer that chooses
  the declaration, so every consumer of the emitted TypeScript (the
  editor, `tsc` through the content mapper, the sidecars) sees the name.

## Work log

- 2026-09-30: Reproduced with `tests/cases/editor/singleCaseVariantName.tt`
  (hover on a value of a one-case variant with a payload and of a unit-only
  variant, signature help on the constructor): all three printed the
  structure.
- 2026-09-30: Confirmed the cause with the plain-TypeScript probe above
  and read the parser and checker functions it cites.
- 2026-09-30: `emit_adt` writes no `| ` when the variant has one case and
  lays out a commented case at depth 1.
- 2026-09-30: The full `cargo test` run showed what else pins the form:
  `comments_inside_a_field_type_stay_in_the_type` in
  `tests/compile/cases_07.rs` asserted the `| {` text (updated to the new
  declaration), the editor matrix's five `variant_exported_topLevel_*`
  difference baselines became stale (deleted through
  `scripts/baseline-accept`, and the `TASK-686 Issue 4` line removed), and
  15 `variant_fieldShadowsTag_*` `.errors.txt` baselines of the compiled
  matrix (`TT_MATRIX_CASES=all TT_CASES=variant cargo test --test
  case_baselines`) now name the type in TypeScript's related information:
  `declared here on type 'Bad'` instead of `on type '{ kind: "A"; }'`.
  Nothing else in those diffs changed.
- 2026-09-30: Added `a_variant_with_one_case_is_that_case_object_type` to
  `tests/compile/cases_07.rs` (a payload case, a unit case, and a case
  laid out one field per line) and documented the form in `docs/ai/tt.md`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/singleCaseVariantName.tt` (and the removed
  `TASK-686 Issue 4` line of `tests/editor-matrix-differences.txt`, whose
  five cases then agree with their twins); also
  `a_variant_with_one_case_is_that_case_object_type` in
  `tests/compile/cases_07.rs`.
- **Observed failure**: with `src/codegen/core/emitter/helpers.rs`
  reverted, `TT_CASES=singleCaseVariantName cargo test --test editor_cases`
  failed with a modified baseline: hover showed `const made: { kind:
  "Item"; n: number; }` and `const unit: { kind: "Only"; }`, and signature
  help `Item(n: number): { kind: "Item"; n: number; }`, instead of `const
  made: Pub`, `const unit: Lone`, and `Item(n: number): Pub`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --test compile`: passes.
- [x] `TT_MATRIX_CASES=all TT_CASES=variant cargo test --test
  editor_cases` and `--test case_baselines`: pass after the baseline
  updates above.
- [x] Baseline changes reviewed and committed with the change.
- The full gate for TASK-687 to TASK-690 is recorded in TASK-688's record,
  which ran it once after the last of them.

## Result

Changed files: `src/codegen/core/emitter/helpers.rs`,
`tests/compile/cases_07.rs`, `tests/cases/editor/singleCaseVariantName.tt`
and its baseline, 15 `.errors.txt` baselines under
`tests/baselines/reference/matrix/variant/`, five deleted difference
baselines under `tests/baselines/reference/editor/matrix/variant/`,
`tests/editor-matrix-differences.txt`, `docs/ai/tt.md`,
`docs/tasks/INDEX.md`, and this record.
