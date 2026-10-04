# TASK-754: Separate website highlighting from artifact writes

- **Status**: In progress
- **Started**: 2026-10-04
- **Completed**: —
- **Commit**: —

## Purpose

Move topicLanguage and the two topic/section highlighting expressions into content-highlighting.ts. The module accepts content and an existing highlighter. highlight.ts keeps grammar reads, highlighter creation, writes, essay generation and sitemap construction in exactly their current order.

## Scope

- Included: `website/scripts/highlight.ts`, `website/scripts/content-highlighting.ts`.
- Excluded: Behavior changes, public API changes, baseline updates, and TASK-732.

## Decisions

### Decision 1: Extract the existing responsibility without redesign

- **Context**: The roadmap identifies a coherent operation mixed with its caller.
- **Alternatives considered**: Keeping the boundary implicit; combining the move
  with algorithm or state redesign, which would make equivalence harder to review.
- **Decision and rationale**: Preserve definitions and callers where possible,
  using the smallest visibility required by the existing consumers. See the
  [detailed brief](../design/refactoring-website-highlighting.md).

## Work log

- 2026-10-04: Started from local base `ef83f563c273c7085ae8c7100bb0217edb05e974` on
  `refactor/remaining-roadmap`. Preserved base source under
  `/tmp/tt-refactor/754` before production edits.

- 2026-10-04: The highlighting expressions, language selection and parent I/O order were reviewed unchanged. Typecheck and build passed. With identical clock input, all 67 generated and distribution artifacts match the base byte for byte with no extra files; /tmp/tt-refactor/754/{base-fixed,head-fixed}.log and base-fixed-artifacts.json. The ordinary clock also leaves generated source JSON and sitemap identical.

## Issues and resolutions

### Issue 1: Prerender build timestamps

- **Symptom**: Uncontrolled builds differed in 36 HTML files, while generated
  highlight/essay JSON, sitemap, scripts and styles matched.
- **Cause**: TanStack serializes each route match's `u` (updated-at) timestamp.
- **Resolution**: Rebuilt unchanged base and head with the same Date input via
  an untracked Node preload fixed at 2026-10-04T00:00:00Z. All 67 raw artifacts
  then matched exactly; no output normalization or production clock changes.

### Issue 2: Local prerender listener blocked by the sandbox

- **Symptom**: The initial unchanged build failed with listen EPERM on ::1.
- **Cause**: Prerendering starts a local HTTP listener.
- **Resolution**: Ran base/head builds with the supported network permission;
  no source changes were needed.

## Regression test (fails before the fix)

Not applicable: Behavior-preserving extraction; no bug is fixed.

## Verification

- [x] Moved definitions and parent changes reviewed.
- [x] Focused checks: The unchanged website typecheck/build passed before extraction; 67 artifact hashes are saved in /tmp/tt-refactor/website-base-artifacts.json. Repeat typecheck/build and compare all generated JSON/HTML/XML and distribution files byte for byte.
- [ ] Applicable complete repository gates.
- [ ] Reference expectations unchanged; task-index and whitespace checks.

## Result

Implementation and focused validation complete; final repository gates remain pending.
