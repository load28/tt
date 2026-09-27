# TASK-399: Limit trigger-character completions to their registered contexts

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

The server registers `.`, `(`, `|`, `{` and `,` as completion trigger characters, but it suppressed the general completion list after a trigger only for `{` and `,`. Typing `console.log(`, `if (`, `a |` or `a ||` therefore opened the full keyword and global list, and a `.` in a comment did the same.

## Scope

- Included: the completion handler and capability registration in `editors/vscode/server/src/server.ts`, the LSP test client's completion helper, and a regression test.
- Excluded: the engine's pattern completion (`ttCompletions`) and TypeScript member completion, which answer unchanged.

## Decisions

### Decision 1: Each trigger character answers only for the context it is registered for

- **Context**: LSP 3.17 (*Completion Request*, `CompletionOptions.triggerCharacters`): "The additional characters, beyond the defaults provided by the client (typically [a-zA-Z]), that should automatically trigger a completion request." The client reports such a request with `CompletionContext.triggerKind = TriggerCharacter` (2) and the character in `triggerCharacter`; typing an identifier character is `Invoked` (1). A trigger character is therefore a request the user did not ask for, made only because the server claimed the character is meaningful. `.` is registered for member access, and `(`, `|`, `{`, `,` for tt patterns (payload fields, or-pattern alternatives, arms, and the next field or arm).
- **Alternatives considered**:
  - Add `(` and `|` to the existing `{`/`,` check. Rejected: it keeps the rule as a list of exceptions, and `.` outside a member access (in a comment or string, where the code mask hides it) would still open the general list.
  - Unregister `(` and `|`. Rejected: they start payload-field and or-pattern completion, which have no word prefix to trigger on.
- **Decision and rationale**: the trigger characters are defined once as two registries, member (`.`) and pattern (`(`, `|`, `{`, `,`), and the capability is built from them. A request with `triggerKind = TriggerCharacter` gets member completions at a member access, pattern completions where the engine reports a pattern position for a pattern trigger, and nothing anywhere else. Requests with `Invoked` or `TriggerForIncompleteCompletions` are unchanged.

### Decision 2: The test client reports the character that was typed

- **Context**: the completion helper in `server.test.ts` sent `triggerKind: 2, triggerCharacter: "."` for every request, including after `<div ti`, `if let ` and `Rect(`, which no client sends: the LSP defines `triggerCharacter` as the character that triggered the request.
- **Decision and rationale**: the helper sends `TriggerCharacter` with the preceding character when that character is registered, and `Invoked` otherwise, which is what VS Code sends for the same keystrokes. The existing assertions pass unchanged under the corrected requests.

## Work log

- 2026-09-27: Added `a trigger character completes only the context it is registered for` to `server/src/test/server.test.ts` (tt and ttx): `console.log(`, `if (`, `a |`, `a ||` and `// see user.` must return no items, and `Admin(`, `Guest |`, `match (user) {` must still return pattern items. It failed against the previous `server.ts` (`console.log(` returned the full global list).
- 2026-09-27: Introduced `MEMBER_TRIGGER_CHARACTERS` and `PATTERN_TRIGGER_CHARACTERS` in `server.ts`, built the capability from them, and replaced the `{`/`,` special case with the per-context rule. Corrected the test helper's `CompletionContext`; the previously passing `JSX optional attribute completion` case failed until then, because the helper claimed a `.` trigger after `<div ti`.

## Issues and resolutions

### Issue 1: A test helper claimed a `.` trigger for every request

- **Symptom**: `JSX optional attribute completion preserves insertion text through resolve` returned no items after the change.
- **Cause**: the helper's hard-coded `triggerCharacter: "."` described a keystroke that did not happen; with the rule in Decision 1 a `.` outside a member access correctly answers nothing.
- **Resolution**: Decision 2.

## Verification

- [x] `npm run compile` in `editors/vscode`
- [x] `node --test server/out/test/*.test.js client/out/test/*.test.js` with `target/debug` on `PATH`
- [x] `node scripts/check-task-index`

## Result

Changed files: `editors/vscode/server/src/server.ts`, `editors/vscode/server/src/test/server.test.ts`, `docs/tasks/INDEX.md`, this record. Trigger characters now open completions only in the member or pattern context they are registered for.
