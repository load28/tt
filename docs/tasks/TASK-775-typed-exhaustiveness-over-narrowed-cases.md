# TASK-775: Judge a match's exhaustiveness on the typed path by the cases its value can still be

- **Status**: In progress
- **Started**: 2026-10-07
- **Completed**: —
- **Commit**: —

## Purpose

TASK-773 decision 21 kept a match's required cases at its variant's declared
cases on every surface, so after `if (s.kind === "Rect") return 0;` a match on
`s` needed a `Rect` arm that `--check-types` then rejected (TS2678). The user
chose to have the typed surfaces (`--check-types`, `--types`, the editor) read
the scrutinee's narrowed type instead: an arm for a case the value was
narrowed away from is not required there, and writing one is TypeScript's
error. Builds and `--check`, which have no types, keep requiring every
declared case, and `_` satisfies both.

This reverses TASK-773 decision 21's first half (the required cases on the
typed path); its second half (an impossible arm owns its bindings'
diagnostics, and the help line points at `_`) stays.

## Scope

- Included: the typed report's match coverage
  (`src/engine/semantics/report.rs`), the documentation of the rule
  (`docs/ai/tt.md`), and case files pinning both surfaces.
- Excluded: the untyped coverage (`sema::coverage_errors`), which stays the
  declared cases, and the checker's tag probes, which already answer the
  narrowed constituents.

## Decisions

### Decision 1: The typed verdict is the declared algorithm over the checker's alphabet

- **Context**: The typed report already asked the checker for the
  constituents of each match's scrutinee (narrowing included) and ran tt's
  usefulness algorithm over them, but only to add holes the declarations
  could not see; a declared hole was reported first and suppressed the
  typed one.
- **Alternatives considered**: Filtering the declared hole's witnesses by
  the checker's alphabet would re-derive the coverage from a message's
  parts. Using the existing typed coverage (with the checker's payload
  alphabets and only certain witnesses) as the verdict would drop holes in
  payload columns the checker was not asked about, which the declared path
  reports.
- **Decision and rationale**: For every match the checker answered, the
  verdict is `checked_coverage` with the checker's scrutinee alphabet and
  the declarations' payload alphabets, rendered with every witness, exactly
  as the declared path renders its own; the declared verdict is not
  reported for that match. Where this finds no hole, the checker's payload
  alphabets can still show one (certain witnesses only), as before. A
  match the checker did not answer keeps the declared verdict.

## Work log

- 2026-10-07: Read the typed report's coverage flow
  (`report.rs`: declared holes first, then `checked_coverage` holes only
  where none was declared) and `analysis::checked_coverage`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: pending
- **Observed failure**: pending

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Baseline changes reviewed and committed with the change

## Result

In progress.
