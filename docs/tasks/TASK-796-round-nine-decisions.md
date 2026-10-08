# TASK-796: Carry out the round-nine design decisions

- **Status**: Complete
- **Started**: 2026-10-08
- **Completed**: 2026-10-08
- **Commit**: `TASK-796: Carry out the round-nine design decisions`

## Purpose

TASK-793, TASK-794 and TASK-795 left six findings for a decision (L4, K4,
K8, K9, E6, E8, E9). The maintainer asked for the recommended choices.
This task carries out the ones that fit the current architecture and
records the two (K9, E9) that need a change to how the TypeScript host is
fed.

## Scope

- Included: L4, K4, K8, E6, E8.
- Excluded: K9 and E9 (Decision 6).

## Decisions

### Decision 1: A directory input names the files the configuration leaves out (L4)

- **Context**: `--check-types <dir>` checked only the files the
  configuration includes, while a build of the same input compiles every
  `.tt` file under it, and it said nothing about the rest.
- **Alternatives considered**: Making every file under a directory input a
  root by request was tried first. It broke the contract TASK-384 pins
  (`tests/workflow_repairs.rs::extended_config_patterns_preserve_exclusions_and_authored_config`):
  a directory input honors the configuration's `exclude`.
- **Decision and rationale**: The configuration keeps deciding, and the
  typed modes name on stderr each input file the configured program left
  out, with how to check it (name the file). The exit status is unchanged.

### Decision 2: A conversion that can run code is captured with its operand (K4)

- **Context**: A template substitution before a lowered value is already
  captured as a string (`` const $tt_v = `${part}` ``), so its `ToString`
  runs in source order. An inline object literal counted as inert and was
  not captured, although its `toString` runs at the conversion. A computed
  key is captured without `ToPropertyKey`.
- **Alternatives considered**: Converting a computed key at the capture
  would change the key's static type and so the type TypeScript gives the
  object; a helper that kept the type would need a type assertion, which
  contract 2 forbids.
- **Decision and rationale**: A template substitution's effects include its
  conversion (`template_substitution_effects`): an object literal that
  names `toString`, `valueOf` or `__proto__`, or has a computed key or a
  spread, and an array literal holding one, are not inert there. A
  computed key keeps the documented exception (docs/ai/tt.md).

### Decision 3: The typed paths let the checker decide a near-miss tag (K8)

- **Context**: Plain `ttc` has no types, so a name near a visible
  variant's case is reported by the documented evidence rule even when the
  scrutinee is a hand-written union with that tag.
- **Decision and rationale**: Plain `ttc` keeps the rule. The typed report
  already asks the checker for each match's tag alphabet (`tag_members`);
  an `unknown-case` name the checker lists for a single-subject match's
  scrutinee is not reported, and the match's coverage is then checked.

### Decision 4: Boolean members come from the checker (E6)

- **Context**: TypeScript completes `true`/`false` after
  `(scrutinee) === ` only as keywords, whatever the type.
- **Decision and rationale**: Pattern completion asks the checker's
  `LiteralQuery` (the typed check's question) in the same repaired
  projection, scoped to the one module, and adds the boolean members it
  names (`Project::literal_members`).

### Decision 5: Fields and definitions follow the scrutinee's type (E8)

- **Context**: A hand-written union sharing a tag with a visible variant
  got the variant's fields, and definition on the tag went to the
  variant's case.
- **Decision and rationale**: When TypeScript answers the field probe, its
  fields are the ones offered. Definition on an arm's tag asks the checker
  for the scrutinee's tags and answers with the variant's case only when a
  declaration owns them (`owned_case_symbol`, as nested tags already are);
  otherwise it answers as TypeScript does for that literal (nothing).
  Hover's `ttSymbol` is the parse-only answer and keeps the documented
  rule, as plain `ttc` does.

### Decision 6: K9 and E9 need projection on demand

- **Context**: The host serves every candidate `.tt` file as a module
  (`x.tt.ts`) because TypeScript sees a `.tt` file only through its
  projection; the configured program then decides membership. Filtering
  candidates by the configuration's file list (`parseConfigFile`) would
  drop a `.tt` file outside `include` that a hand-written `.ts` file
  imports. E9 re-settles every contextual slot after each edit for the same
  reason: the host is handed whole modules, not asked per file.
- **Decision and rationale**: Both need the host to ask ttc for a
  projection when TypeScript first reads a file — a change to the
  host protocol. It is left for its own task.

## Work log

- 2026-10-08: Changed `src/main/typed.rs`, `src/engine/project.rs`
  (`Project::left_out`) (L4); `src/engine/project.rs` (E6);
  `src/program_syntax.rs`, `src/program_syntax/visit.rs` (K4);
  `src/engine/semantics/report.rs` (K8);
  `src/engine/language/{service,project}.rs`,
  `src/engine/language/project/completion.rs` (E6, E8). Updated
  `docs/ai/tt.md` and `docs/design/lsp-architecture.md`, and marked the
  reversed decisions in TASK-793, TASK-794 and TASK-795.

## Issues and resolutions

### Issue 1: Checking a directory's excluded files broke a pinned contract

- **Symptom**: The full suite failed
  `extended_config_patterns_preserve_exclusions_and_authored_config` and
  `typed_watch_checks_edits_made_while_the_configuration_was_malformed`.
- **Cause**: The first L4 change made every file under a directory input a
  root, overriding the configuration's `exclude`.
- **Resolution**: Reverted it; Decision 1 records the notice that replaced
  it.

### Issue 2: The notice named files that had failed to project

- **Symptom**: 40 case baselines gained the notice for a file whose own
  error had blocked its projection.
- **Cause**: A blocked file enters the program only as a placeholder, and
  the membership map is built from projected files, so it never counted as
  checked.
- **Resolution**: `Project::left_out` skips blocked files; their errors
  already say why they were not checked.

## Regression test (fails before the fix)

- **Path**: `tests/cli.rs::a_directory_input_names_the_files_the_configuration_leaves_out`
- **Observed failure**: no message for `other/x.tt` (it was skipped silently).
- **Path**: `tests/cases/compiler/aComputedKeyIsConvertedWhereItsObjectIsBuilt.tt`
- **Observed failure**: the `.stdout` baseline differed: the inline
  object's conversion ran after the scrutinee.
- **Path**: `tests/cases/compiler/aHandWrittenTagNearAVariantCaseIsCheckedByItsType.tt`
- **Observed failure**: `--check-types` also reported `unknown-case` for
  `Betta` over the hand-written union.
- **Path**: `tests/cases/editor/booleanLiteralPatternCompletions.tt`
- **Observed failure**: no `true`/`false` entries in `patternCompletions`.
- **Path**: `tests/cases/editor/aHandWrittenUnionSharingATagUsesItsOwnFields.tt`
- **Observed failure**: `radius` offered beside `r`, and definition on the
  tag answered the variant's case.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` with `TTC_REQUIRE_TSGO=1`
- [x] Baseline changes reviewed and committed with the change (new cases
  only; the public API baseline gains `Project::left_out`)

## Result

L4, K4, K8, E6 and E8 are carried out as Decisions 1–5 describe. K9 and E9
need projection on demand in the TypeScript host (Decision 6).
