# TASK-300: Refresh Result syntax across user surfaces

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: See git history.

## Purpose

The website, editor assistance, and narrative documentation still expose the
removed `<-` Result binding and semicolon-free tail syntax after the 1.0.0 beta
cutover.

## Scope

- Included: update website content, AI-facing documentation, user guides,
  editor snippets and grammar coverage, and generated website artifacts that
  still describe the removed Result surface.
- Excluded: changing the implemented Result grammar or compiler semantics.

## Decisions

### Decision 1: Audit all user-visible sources, not only the homepage example

- **Context**: the same removed syntax appears in website copy, narrative
  documents, editor completion text, and syntax grammar fixtures.
- **Alternatives considered**: patch only the reported homepage card, or align
  every active user-facing surface with the shipped language.
- **Decision and rationale**: align every active surface so users and tools do
  not receive contradictory syntax guidance.

## Work log

- 2026-08-30: Ran the environment doctor and located stale Result syntax across
  website, documentation, and editor surfaces.
- 2026-08-30: Updated the bilingual website reference and Korean essay source,
  then regenerated highlighted website data.
- 2026-08-30: Updated AI guidance, editor completions, TextMate grammar sources
  and generated grammars, grammar fixtures, and the Result fuzz generator.
- 2026-08-30: Passed the website production build, extension gate, fuzz crate
  check, and full repository CI.

## Issues and resolutions

### Issue 1: Website prerender could not bind inside the sandbox

- **Symptom**: `bun run build` completed both Vite bundles but failed with
  `listen EPERM ::1` when starting the prerender preview server.
- **Cause**: the sandbox denied the build's loopback listener.
- **Resolution**: reran the same production build with permission for its local
  preview listener; all 37 pages prerendered successfully.

### Issue 2: Grammar fixtures expected retired custom binding scopes

- **Symptom**: the new ordinary `const value = try expression;` declarations
  received TypeScript's `variable.other.constant.ts` scope while two fixtures
  still expected the removed Result binding's read-write scope.
- **Cause**: deleting the `<-` grammar correctly returned declarations to the
  base TypeScript grammar.
- **Resolution**: updated the fixtures to assert the standard constant scope
  and the tt `try` keyword scope.

### Issue 3: Direct extension test command did not select the checkout compiler

- **Symptom**: `npm test` found an unrelated compiler from the ambient PATH and
  its typed server tests timed out.
- **Cause**: the repository's extension gate intentionally prepends
  `target/debug`; the package script alone does not.
- **Resolution**: ran `./scripts/ci extension`, which builds and selects the
  checkout compiler; all 130 extension tests passed.

## Verification

- [x] Website content/build checks
- [x] Editor grammar/server checks
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

All active website, AI, editor, and fuzz surfaces now use `= try` declarations
and explicit Result returns. Historical changelog and task/design records remain
unchanged, and the AI guide retains only an intentional warning that `<-` is
not tt syntax.
