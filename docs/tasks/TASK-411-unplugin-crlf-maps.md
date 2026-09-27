# TASK-411: Detach CRLF-terminated inline maps in the bundler plugin

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

TASK-395 made ttc end the `//# sourceMappingURL=` comment with the output's own line ending. `@openload28/unplugin-tt` matched the comment only after `\n` and before an optional `\n`, so for a CRLF source it returned `map: null` and left the base64 comment in the code, although docs/ai/tt.md says the plugin hands the map to the bundler.

## Scope

- Included: `detachInlineSourceMap` in `integrations/unplugin/index.js`.
- Excluded: ttc's output.

## Decisions

### Decision 1: Accept either line terminator around the comment and keep the output's own

- **Context**: ECMA-426 locates the map through the last `sourceMappingURL` comment line; the line terminator belongs to the output.
- **Decision and rationale**: The pattern accepts `\r\n` or `\n` before and after the comment, and the detached code keeps the terminator that preceded it, so CRLF output stays CRLF.

## Work log

- 2026-09-27: Reproduced with the plugin's `load` hook on a CRLF `.tt`. Fixed the pattern; added a plugin test that fails before the change.

## Issues and resolutions

None.

## Verification

- [x] `TTC_BINARY=target/debug/ttc npm --prefix integrations/unplugin test`: 6 passed.

## Result

Changed `integrations/unplugin/index.js` and `integrations/unplugin/test/plugin.test.mjs`.
