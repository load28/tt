# TASK-274: Show the top-level `contentMappers` configuration shape

- **Status**: Pending
- **Started**: 2026-08-28
- **Completed**: —
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

## Issues and resolutions

None.

## Verification

- [ ] Documentation examples reviewed against the content-mapper fixture

## Result

Pending.
