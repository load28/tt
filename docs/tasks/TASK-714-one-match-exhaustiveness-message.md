# TASK-714: Word a match's coverage hole the same way on every path

- **Status**: Complete
- **Started**: 2026-10-01
- **Completed**: 2026-10-01
- **Commit**: see `git log --grep TASK-714`

## Purpose

TASK-701 Issue 1: for the same `match`, `ttc --check` reports "match on
variant Shape is not exhaustive: missing \"Point\"", and `ttc
--check-types`, `typedCheck`, and the editor report "match is not
exhaustive: missing \"Point\"". `src/diagnostics.rs` says the pipelines
share one renderer "so their wording cannot drift apart", but each path
built the renderer's input on its own. One message, from one source.

## Scope

- Included: which variant a coverage diagnostic names
  (`src/analysis/coverage.rs`), the import specifier the engine keeps for
  an imported variant (`src/resolve/mod.rs`, `src/engine/`, Issue 1),
  the rendering of a coverage hole into a message and closing arms
  (`src/sema/coverage.rs`, used by `src/engine/semantics/report.rs`),
  `docs/ai/tt.md`, a reversal note in TASK-120,
  `tests/editor-diagnostic-differences.txt`, and the baselines the wording
  changes.
- Excluded: which witnesses each path finds (the typed path answers from
  the narrowed type and reports only certain witnesses; that is a
  difference of data, not of wording), and the literal-union message,
  which only the typed path can give.

## Sources

- rustc's `E0004` ("non-exhaustive patterns: `Point` not covered") names
  the scrutinee's type in its note (`note: the matched value is of type
  Shape`), which is the information the variant name gives here.
- TASK-120 Decision 4 (the earlier choice this reverses), TASK-701 Issue 1.

## Decisions

### Decision 1: Keep the variant name, and name it the same way on both paths

- **Context**: Either the untyped message loses its subject or the typed
  one gains it. TASK-120 left the typed path without one because the
  checker answers with a type's constituents, not a declaration, and the
  mapping from one to the other is not unique.
- **Alternatives considered**: (a) Drop the subject everywhere: one
  wording, but every untyped report loses the variant (and its origin,
  "imported from \"./token.tt\""), the information TASK-120 called worth
  keeping. (b) Ask the checker for the scrutinee type's name: a backend
  protocol change, and an alias or a union names no tt declaration. (c)
  Name, on both paths, the variant the arms are read against, chosen by
  the untyped path's rule (the declaration the arms satisfy, else the one
  they leave least of) among the declarations that hold every tag the arms
  write and, on the typed path, every constituent the checker names.
- **Decision and rationale**: (c). The typed path has the declarations
  (its alphabet's payloads already come from them), so both paths have the
  information, and the more informative message wins. Requiring every
  checker constituent to belong to the named declaration keeps the typed
  message from naming a variant the scrutinee is not: a scrutinee of
  `Shape | Opt` matched with `Circle`, `Rect`, and `Yes` arms holds no
  single declaration's tags, so it is reported without a subject, as
  before. `subject_of` and `position_subject` are the one rule for a
  single match and for each tuple position.

### Decision 2: One function renders a coverage hole

- **Context**: The two paths each turned a `Coverage` into the message and
  the closing arms: the untyped path quoted with `"…"` and named tuple
  positions, the typed path quoted with Rust's `{:?}` and named none.
- **Alternatives considered**: share only the subject (still two
  renderings, which is how they drifted); share the whole rendering.
- **Decision and rationale**: `sema::non_exhaustive(coverage, witnesses)`
  returns the message and arms for both, with `Witnesses::All` (untyped)
  or `Witnesses::Certain` (typed) as the one difference in data the paths
  keep. A tuple match's typed message now names its positions too
  (`match on (Shape, Opt) is not exhaustive`).

## Work log

- 2026-10-01: Compared both paths on `variant Shape { Circle, Rect, Point
  }`, on a `Shape | Opt` scrutinee, and on a tuple match.
- 2026-10-01: Found Issue 1 in the regenerated baselines; the engine's
  imported declarations now carry their specifier.
- 2026-10-01: Added `subject_of` and `position_subject`, used them on both
  paths; moved the rendering to `sema::non_exhaustive`; updated
  `docs/ai/tt.md` and the TASK-120 record; removed the two listed
  differences; regenerated the baselines and read their diff.

## Issues and resolutions

### Issue 1: The engine named an imported variant without its specifier

- **Symptom**: With the subject named on the typed path,
  `importedVariantExhaustiveness` read "match on imported variant Shape is
  not exhaustive" under `ttc --check-types` and "match on variant Shape
  (imported from \"./shapes.tt\") is not exhaustive" under `ttc --check`.
- **Cause**: The command line's collector records each import's
  specifier (`ExternVariant::from`); the engine's collector
  (`engine::language::imported_variants`) kept only the declaration, so
  every engine analysis described an imported variant with
  `Origin::Imported { from: None }`.
- **Resolution**: The engine collects `resolve::ImportedVariant`
  (the declaration and the import's specifier), converted to the
  resolver's `ExternDecl` with `from` set, so the engine's analyses (the
  typed check, the editor's semantic answers, `tt_declarations`) know an
  imported variant's origin as the command line does.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/diagnostics/match-not-exhaustive_missingCase_declarationInitializer.tt`
  and `..._jsxAttribute.ttx` (`TT_REQUIRE_EXTENSION=1 cargo test --test
  editor_cases`), with their lines removed from
  `tests/editor-diagnostic-differences.txt`.
- **Observed failure**: Without the change in `src/`: "2 editor case(s)
  failed: match-not-exhaustive_missingCase_declarationInitializer: the
  editor publishes `match-not-exhaustive` at main.tt:5:17, and `ttc
  --check` does not report it there with those words and that width" (and
  the same at main.ttx:5:27).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`: the full gate is recorded in TASK-715.
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed `src/analysis/coverage.rs`, `src/analysis/mod.rs`,
`src/analysis/patterns.rs`, `src/resolve/mod.rs`,
`src/engine/language/service.rs`, `src/engine/names.rs`,
`src/engine/project.rs`, `src/engine/semantics.rs`,
`src/engine/semantics/declarations.rs`, `src/sema.rs`, `src/sema/coverage.rs`, `src/engine/semantics/report.rs`,
`docs/ai/tt.md`, the TASK-120 record,
`tests/editor-diagnostic-differences.txt`, and the baselines of the typed
coverage reports. Both paths word a coverage hole from one function and
name its variant by one rule.
