# TASK-342: Follow the window's workspace folders while it is open

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-342: follow the window's workspace folders while it is open`

## Purpose

The server read the workspace folders once, at `initialize`, and never
again. A folder added to the window afterwards was not a place the
compiler was looked for, not a place the TypeScript toolchain was looked
for, and not a root a relative `tt.sidecarDir` resolved against — until the
window was reloaded.

## Scope

- Included: The workspace-folders server capability, the change
  notification, and what the server redoes when the folders change.
- Excluded: What a relative `tt.sidecarDir` should mean for a file that
  belongs to no folder at all, and the settings scope a folder's own
  `tt.*` values are read at.

## Decisions

### Decision 1: The server declares that it wants folder changes

- **Context**: `workspaceRoots` was assigned in `onInitialize` and read by
  `findCompiler`, `findTsgo` and `resolveSidecarDir` for the life of the
  session. The notification that would have kept it current is only sent
  to a server that declared
  `capabilities.workspace.workspaceFolders.changeNotifications` — the
  client's `WorkspaceFoldersFeature.initialize` reads exactly that field
  and registers nothing without it
  (`client/node_modules/vscode-languageclient/lib/common/workspaceFolder.js:67`).
- **Alternatives considered**: Re-request the folders before each
  validation (`workspace/workspaceFolders` is a round trip per keystroke
  for something that changes a few times a day); watch the workspace file
  (a `.code-workspace` is not how a folder is added to an untitled
  workspace, and single-folder windows have no such file).
- **Decision and rationale**: Declare the capability and handle the
  notification, which is what the protocol provides for this.

### Decision 2: A folder change re-arms exactly what a settings change does

- **Context**: Both change *where* the server looks. `onDidChangeConfiguration`
  already cleared the missing-compiler warning, re-armed a compiler that
  had struck out, invalidated in-flight validations and reloaded project
  state.
- **Alternatives considered**: Only update the roots (the compiler
  resolved from the old roots stays in `servedCompiler`, so the new
  folder's own `ttc` would never be used, which is the case that motivated
  this).
- **Decision and rationale**: One `rearmProject()` used by both, so the two
  entry points cannot drift.

### Decision 3: The handler is registered only for a client that can send it

- **Context**: `connection.workspace.onDidChangeWorkspaceFolders` throws
  "Client doesn't support sending workspace folder change events" when the
  client did not declare the capability. Registering it unconditionally
  made the whole `initialized` handler fail, and took the first compiler
  resolution down with it — visible in the log as `Notification handler
  'initialized' failed`.
- **Alternatives considered**: Wrap the call in try/catch (hides a real
  failure to register behind the same silence).
- **Decision and rationale**: Read the client capability at `initialize`
  and register only when it is there, the way the configuration capability
  is already handled.

## Work log

- 2026-09-05: Confirmed the client-side gate in the installed
  `vscode-languageclient`, then drove the built server over LSP to confirm
  the notification is now received and acted on.
- 2026-09-05: Added `server/src/roots.ts` (`folderRoots`,
  `applyFolderChange`), the capability, the guarded handler, and
  `rearmProject()`.
- 2026-09-05: Added `server/src/test/roots.test.ts` and an end-to-end case
  in `server/src/test/server.test.ts` that asserts the declared capability
  and that an added folder re-validates the open buffers.

## Issues and resolutions

### Issue 1: Folders added after startup were not project roots

- **Symptom**: Opening a `.tt` file in a folder added to the window found
  neither that folder's built `ttc` nor its installed `@openload28/tt-lang`,
  and a relative `tt.sidecarDir` for its files resolved to nothing.
- **Cause**: `workspaceRoots` was written once in `onInitialize`, and the
  server never asked for the change notification that would update it.
- **Resolution**: Decisions 1 and 2.

### Issue 2: Registering the handler broke `initialized` for other clients

- **Symptom**: `Notification handler 'initialized' failed with message:
  Client doesn't support sending workspace folder change events`, and the
  first compiler resolution never ran.
- **Cause**: `onDidChangeWorkspaceFolders` throws for a client without the
  capability, and the rest of the handler followed it.
- **Resolution**: Decision 3.

## Verification

- [x] The `initialize` result declares `workspace.workspaceFolders` with
  `changeNotifications`, and a client without the capability still gets a
  working `initialized`
- [x] `workspace/didChangeWorkspaceFolders` re-validates the open buffers
- [x] `node --test server/out/test/*.test.js client/out/test/*.test.js`
- [x] `./scripts/ci`

## Result

Changed files: `editors/vscode/server/src/server.ts`,
`editors/vscode/server/src/roots.ts`,
`editors/vscode/server/src/test/roots.test.ts`,
`editors/vscode/server/src/test/server.test.ts`, and this record.
