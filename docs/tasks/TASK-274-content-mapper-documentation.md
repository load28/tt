# TASK-274: Show the top-level `contentMappers` configuration shape

- **Status**: Complete
- **Started**: 2026-08-28
- **Completed**: 2026-08-28
- **Commit**: —

## Purpose

Current snippets can be pasted inside `compilerOptions`, where TypeScript
rejects `contentMappers`. Show a complete tsconfig object and include the
content-mapper route in getting-started documentation.

## Scope

- Included: English user-facing setup documentation and complete JSON examples
- Excluded: content-mapper implementation changes

## Decisions

### Decision 1: Examples show the containing object, not a detached property

- **Context**: The key's placement is part of the configuration contract.
- **Alternatives considered**: add prose only; add a comment to the fragment;
  or show a complete minimal object.
- **Decision and rationale**: A complete minimal object is copyable and makes
  the sibling relationship with `compilerOptions` unambiguous.

## Work log

- 2026-08-28: Created from nightly audit finding 9.
- 2026-08-28: Started after TASK-273 completed diagnostic ownership parity.
- 2026-08-28: Added a direct-import content-mapper route to the manual
  compiler guide with a complete tsconfig object and CLI invocation.
- 2026-08-28: Replaced detached properties in the AI guide and VS Code README
  with complete objects that show `contentMappers` beside `compilerOptions`.
- 2026-08-28: Reviewed all three examples against the top-level shape in
  `tests/content_mapper.rs` and ran the agent/documentation gate; it passed.

## Issues and resolutions

None.

## Verification

- [x] Documentation examples reviewed against the content-mapper fixture
- [x] `./scripts/ci agents`

## Result

Every user-facing content-mapper setup route now shows a complete, copyable
tsconfig object and explicitly rejects nesting `contentMappers` under
`compilerOptions`. Changed files: `docs/getting-started.md`, `docs/ai/tt.md`,
and `editors/vscode/README.md`.
