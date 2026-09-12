# TASK-375: Preserve completion candidates while typing patterns

- **Status**: Complete
- **Started**: 2026-09-12
- **Completed**: 2026-09-12
- **Commit**: —

## Purpose

Verify and repair completion while typing the first match arm, subsequent arms, and payload fields in tt and ttx.

## Scope

- Included: Pattern completion candidate resolution, incomplete sibling evidence, tuple entry positions, and LSP/editor regression coverage.
- Excluded: Inventing checker type information from source text or changing normal compilation.

## Decisions

Pattern completion must retain ambiguous declaration candidates instead of selecting the first declaration without evidence. Only completed pattern headers can constrain candidates; wildcards provide no variant identity. Keep this parse-only contract available without TypeScript.

## Work log

- 2026-09-12: Ran doctor and inspected pattern completion through the engine and LSP boundary. Confirmed that one local variant works, but arbitrary first-declaration selection and incomplete sibling tags can remove applicable candidates. Created a separate branch from origin/main; TASK-374 is reserved by the separate PR #123, so this work uses TASK-375.

- 2026-09-12: Implemented compatible-candidate resolution and completed-header evidence, added delimiter and tuple contexts, and verified engine, LSP, and actual VS Code completion responses. Completed the full local gate.

## Issues and resolutions

### Issue 1: First-arm candidates depend on declaration order

- **Symptom**: With multiple variants, the first arm only offers cases from the first declaration.
- **Cause**: An empty tag set satisfied the first table entry in `resolve`.
- **Resolution**: Retain all compatible declarations until completed pattern headers supply evidence. Merge equal insertion names while retaining distinct declaration details.

### Issue 2: Sibling text removes valid case suggestions

- **Symptom**: A wildcard or unfinished sibling leaves only `_`; expression pipes can also contaminate candidate resolution.
- **Cause**: The token scan treated every apparent tag as variant identity, including wildcards and incomplete headers, and did not separate guard/body expressions from pattern alternatives.
- **Resolution**: Collect tag evidence only when a header reaches its arrow. Wildcards remain neutral. Track pattern/guard/body phases and ignore expression operators as case evidence.

### Issue 3: Tuple entries and delimiter-triggered suggestions are missing

- **Symptom**: Tuple slots produce no tt candidates; opening a match body or moving past a comma does not trigger a request.
- **Cause**: The completion context recognized payload parentheses but not tuple pattern parentheses, and the LSP trigger list omitted arm delimiters.
- **Resolution**: Recognize tuple slots as case positions. Advertise brace/comma triggers and limit these delimiter requests to actual pattern contexts.


## Verification

- Focused Rust pattern-completion tests passed (13 cases).
- Focused LSP test passed for first/subsequent arms, fields, tuples, and negative object-literal delimiter triggers in tt and ttx.
- Actual VS Code pattern matrix passed all eight groups after distinguishing VS Code's independent word suggestions from tt's negative-context assertion.
- Full `./scripts/ci` passed: agents, Rust formatting/clippy/tests, npm, website, native, and extension gates. All 174 extension tests passed.
- `git diff --check` passed.


## Result

Pattern candidates remain available across first and subsequent arms, payload fields, and tuple slots while typing. Delimiter requests are limited to pattern contexts.

Changed files: `src/engine/completions.rs`, `editors/vscode/server/src/server.ts`, `editors/vscode/server/src/test/server.test.ts`, `editors/vscode/test/patterns.cjs`, `editors/vscode/scripts/test-editor.mjs`, `editors/vscode/README.md`, and this task/index.
