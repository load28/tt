# TASK-620: Report an impossible match case at its pattern

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-620`

## Purpose

For `variant S { A, B }` and `match (s) { A => 1, Zzz => 2 }`, plain
`ttc --check` exits 0, and `ttc --check-types` reports the missing `"B"`
and TS2678 (`Type '"Zzz"' is not comparable to type '"A" | "B"'`)
underlining `match (s)` instead of the pattern that names `Zzz`. The report
asked for the untyped check to report the unknown case and for the typed
TS2678 to land on the pattern.

## Scope

- Included: the case label's anchor (`variant_label`,
  `src/codegen/core/emitter/pattern.rs`), the anchor's display span
  (`EmitAnchor::display`, `src/lib/mapped.rs`), the CLI/server report
  (`src/engine/semantics/report.rs`) and the editor service
  (`src/engine/language/project.rs`), `docs/ai/tt.md`, and a regression
  test (`tests/native/cases_02.rs`).
- Excluded: the untyped check (Decision 1); the `.kind === "X"` tests of a
  conditional match dispatch, let-else, and `if let`, whose TypeScript
  diagnostics (TS2367) keep their construct-level position; the rejection of
  `.ts` inputs by `--check-types` (Decision 3).

## Decisions

### Decision 1: Plain `ttc` keeps the documented name-resolution rule

- **Context**: `docs/ai/tt.md` ("Name resolution") and `ttc explain
  unknown-case` say the resolver identifies `S` from the site's evidence
  (the unique variant with the greatest partial coverage) and then reports
  only case-insensitive and near-miss names in it; "a name that is simply
  wrong rather than misspelled needs types, and is left to the checker",
  because tag patterns also describe hand-written `kind` unions. Coverage
  needs a declaration containing every tag, so this site has no coverage
  question either. With `type Ev = { kind: "A" } | { kind: "Zzz" }` in the
  same file, `match (ev) { A => 1, Zzz => 2 }` is the same text and is
  valid.
- **Alternatives considered**: Report `Zzz` (and the missing `B`) once
  partial coverage identifies `S`: plain `ttc` has no types, so that valid
  program would stop compiling.
- **Decision and rationale**: No change to the untyped check: the rule the
  report cited is the one that declines, for this reason. The typed path is
  the documented owner, and Decision 2 makes its answer precise. The doc
  now says so with this example.

### Decision 2: A case label is glue written for its pattern, and its diagnostics are shown there

- **Context**: The switch writes `case "Zzz":` inside the match's anchor,
  and the anchor's source range is `match (s)`. TS2678 lands on the string
  literal, which no mapping covers (its quotes are glue), so it took the
  match anchor. That anchor's `src`/`owner_end` also identify the match for
  every ownership rule: TS2678 on a label whose pattern ttc already
  reported as a typo is suppressed (`match_has_resolution_error`), tt
  errors owning the match suppress its glue diagnostics
  (`origin_intersects_tt_error`), and one translated class is reported once
  per construct.
- **Alternatives considered**: (a) An anchor whose `src` is the pattern:
  every ownership rule keyed on the match's keyword would stop recognizing
  the label as the match's glue, so a typo's TS2678 would be reported next
  to its unknown-case error. (b) Map the tag bytes inside the quotes: the
  diagnostic covers the quotes too, so no mapping holds it. (c) A separate
  side table of labels: a second structure with the same meaning as an
  anchor.
- **Decision and rationale**: Each variant case label is wrapped in its own
  `Match` anchor with the match's identity (`src`, `src_end`, `owner_end`
  unchanged) and the pattern's tag span as its `context`, the field an
  anchor already uses for "a second source range that explains a
  diagnostic on this glue". `EmitAnchor::display` returns that span for a
  `Match` anchor with a context and the construct's own span otherwise; the
  CLI/server report and the editor service underline `display()` and key
  their once-per-construct deduplication on its start, so each impossible
  pattern is reported once, at itself, and the ownership decisions are the
  same as before.

### Decision 3: `--check-types` rejecting a `.ts` input is intended

- **Context**: `ttc --check-types build/m.ts` prints `not a tt source
  (expected .tt, .ttx)` and exits 2.
- **Decision and rationale**: Recorded, not changed: TASK-396 (Decision on
  input errors) defines this message for the engine's inputs, and
  `Inputs::collect` documents it; TypeScript files are checked as part of
  the project the `.tt` inputs belong to.

## Work log

- 2026-09-30: Reproduced both paths with `target/p620/m.tt`.
- 2026-09-30: Read the resolver (`src/resolve/mod.rs` `identify`,
  `resolve_constructor`), the coverage owner (`src/analysis/patterns.rs`
  `resolve`), and the docs (Decision 1).
- 2026-09-30: Implemented Decision 2. `ttc --check-types` now underlines
  `Zzz` (3:38); a near-miss typo (`Circel`) still reports only
  `unknown-case`; an or-pattern reports each impossible alternative.
- 2026-09-30: Added `an_impossible_case_is_reported_at_its_pattern`
  (`tests/native/cases_02.rs`: CLI positions and the server's spans);
  `an_imported_case_without_declaration_ownership_uses_checker_evidence`
  still passes.

## Issues and resolutions

None.

## Verification

- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native an_impossible_case`,
  `--test emit_map`, `--test content_mapper`,
  `--test practical_diagnostics`, `--test snapshot`, `cargo test --lib`
- [x] Full gate for the TASK-614–620 series, run once on the tree rebased
  onto `claude/ecstatic-dijkstra-qw5pf9` (71d97cc): `cargo fmt --check`;
  `cargo clippy --all-targets -- -D warnings`;
  `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test` (1738 passed, 0
  failed); VS Code extension tests (226 passed); `npm --prefix
  integrations/unplugin test` with the debug `ttc` (18 passed); `npm
  --prefix packages/create-tt test` (25 passed); `./scripts/ci agents`
  (passed, including `node scripts/check-task-index`).

## Result

Changed `src/codegen/core/emitter/pattern.rs`, `src/lib/mapped.rs`,
`src/engine/semantics/report.rs`, `src/engine/language/project.rs`,
`docs/ai/tt.md`, `tests/native/cases_02.rs`, `docs/tasks/INDEX.md`, and
this record.
