# TASK-267: Preserve typed exhaustiveness when pattern diagnostics are reportable

- **Status**: Complete
- **Started**: 2026-08-28
- **Completed**: 2026-08-28
- **Commit**: —

## Purpose

An unresolved pattern name currently suppresses a match's typed diagnostics
even when the corresponding source-level cause is never emitted. Make
suppression conditional on an owned, reported diagnostic.

## Scope

- Included: typed semantic reporting, per-match ownership, and exhaustiveness
  regression tests
- Excluded: declaration transport and wildcard-based subject identification

## Decisions

### Decision 1: Suppression requires a reported owner

- **Context**: `has_unresolved` is presently sufficient to discard checker and
  coverage consequences.
- **Alternatives considered**: remove suppression; retain implicit suppression;
  or model the emitted cause as the owner.
- **Decision and rationale**: Suppress only when the same pass emits the precise
  source cause. This preserves one-cause diagnostics without creating silence.

### Decision 2: Store match ownership on the reportable resolver error

- **Context**: `MatchAnalysis::has_unresolved` copied resolver state into a
  second boolean that could remain true even if diagnostic selection changed.
- **Alternatives considered**: Rename the boolean; pair it with a second
  `was_reported` flag; or make the reportable error identify its owned match.
- **Decision and rationale**: `UnresolvedName::match_owner` records the match
  keyword only for match and tuple-match sites. Diagnostic emission and
  suppression now query the same error list, so a cause-free suppression state
  cannot be represented.

## Work log

- 2026-08-28: Created from nightly audit finding 2.
- 2026-08-28: Started after TASK-266 established cross-module field
  declarations and one shared resolver-diagnostic author.
- 2026-08-28: Removed the copied `MatchAnalysis::has_unresolved` flag and
  attached an optional match owner directly to every reportable unresolved
  name; `if let` and let-else errors intentionally own no match consequence.
- 2026-08-28: Updated sema coverage and typed checker-glue suppression to query
  `PatternAnalyses::match_has_resolution_error` from that same diagnostic list.
- 2026-08-28: Added a typed CLI/server regression combining an imported field
  typo, missing cases in the owned match, and an independent non-exhaustive
  match in the same file.
- 2026-08-28: Ran `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, and `cargo test`; all passed.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Resolver errors now carry their own match ownership. Both untyped coverage and
typed checker suppression derive their recovery boundary from the same
reportable errors that produce `unknown-case` or `unknown-field`; an independent
match in the same file remains fully diagnosed. Changed files:
`src/analysis/mod.rs`, `src/sema.rs`, `src/engine/semantics.rs`, and
`tests/native.rs`.
