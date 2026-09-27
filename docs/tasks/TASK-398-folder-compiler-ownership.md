# TASK-398: Own the compiler per workspace folder end to end

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`tt.compilerPath` is a resource-scoped setting, but the language server chose compilers in three disagreeing places. A configuration or watched-file change cleared the per-folder table before reloading, so only the default compiler was reloaded and it received every folder's documents. At startup, documents opened before the configuration answer arrived were sent to an unconfigured auto-discovered compiler whose session stayed alive. Validation and sidecar refresh resolved the compiler with every workspace folder, so a file got the first folder's build, and a relative `tt.compilerPath` resolved against the server process's working directory.

## Scope

- Included: compiler ownership in `editors/vscode/server/src/server.ts`, a per-folder resolver in `ttc.ts`, the LSP test client's handling of server-to-client requests, regression tests, and the `tt.compilerPath` setting description and README.
- Excluded: the engine session model in `engine.ts` (sessions stay keyed by compiler path) and the compiler.

## Decisions

### Decision 1: One owner table, resolved from configuration before any buffer is sent

- **Context**: the LSP document synchronization handlers are synchronous, while `workspace/configuration` is a request the server must await (LSP 3.17, *Configuration Request*: "the server can fetch configuration settings from the client ... the request can fetch several configuration settings in one roundtrip"; `scopeUri` selects the resource scope). The previous code bridged this gap by guessing (`findCompiler("")`) and by recording whichever compiler the last validation computed.
- **Alternatives considered**:
  - Keep the guess and move the documents once the configured compiler is known. Rejected: the unconfigured session has already been started and has read the project, which is the defect.
  - Await settings inside each synchronization handler. Rejected: the engine's ordering guarantee is that sends are synchronous writes on one pipe (`engine.ts`), so an awaited send could overtake or be overtaken by a later change.
- **Decision and rationale**: `armCompilers` resolves one compiler per owner — every workspace folder, plus the owner of documents outside all folders — from that owner's resource-scoped settings, and swaps the whole table at once. Synchronization handlers send a buffer only when its owner already has a compiler; a buffer that arrives earlier is sent by `reloadProjectState` when the table lands, with its current text. Every request path (`validate`, `rebuildSidecar`, hover, completion, and the other engine questions) awaits `compilerOf`, which reads the same table and waits for the pending resolution when the owner is not in it yet. Nothing else computes a compiler.

### Decision 2: A re-arm keeps the previous table until the new one is complete

- **Context**: `rearmProject` cleared the table and then enumerated it to decide which sessions to reload.
- **Decision and rationale**: the previous table stays in place while the new one resolves, so edits keep reaching the session that currently holds the buffer. When the new table lands, a buffer whose compiler changed is closed on the old session, every compiler that serves an open buffer receives `reloadProjects`, and every buffer is re-opened on its owner's compiler. A generation counter discards a resolution that a later re-arm superseded.

### Decision 3: A folder's compiler is resolved relative to that folder only

- **Context**: `findCompiler(settings.compilerPath, workspaceRoots)` searched every folder for a development build or an installed package, and returned a relative configured path unchanged, so `child_process.spawn` resolved it against the server's working directory.
- **Alternatives considered**: resolving every non-absolute value against the folder. Rejected: a bare `ttc` is a command name that must be looked up on `PATH`.
- **Decision and rationale**: `folderCompiler(configured, root)` follows the command-lookup rule that `execvp` defines and Node's `child_process.spawn` inherits (POSIX `exec`: "If the file argument contains a slash character, the file argument shall be used as the pathname for this file. Otherwise, the path prefix for this file is obtained by a search of the directories passed as the environment variable PATH"): a value that contains a path separator names a file and is resolved against the owning folder, and a bare name is left for `PATH`. With no configured value, the development build and the installed package are searched from that folder only. Documents outside every folder keep the previous window-level resolution (window settings, all folders searched), because they have no folder to resolve against.

### Decision 4: The test client answers the server's requests

- **Context**: the LSP test client matched every incoming message with an `id` against its own pending requests, so a server-to-client request with a colliding id was taken for a response, and `workspace/configuration` could never be answered. Every case therefore ran without the configuration capability, which is how the defects above went unobserved.
- **Decision and rationale**: a message with both `method` and `id` is a request (JSON-RPC 2.0, *Request object*; LSP 3.17, *Base Protocol*); the client answers `workspace/configuration` from a per-test function and every other request with `null`, and only messages without `method` are matched to pending requests.

## Work log

- 2026-09-27: Added four LSP-level regression tests to `server/src/test/server.test.ts` using logging wrapper compilers that forward to the built `ttc` and record their arguments and engine requests: per-folder compilers across `workspace/didChangeConfiguration`; a document opened before the configuration answer; per-folder development builds with no configured path; a relative `tt.compilerPath` including the save-time sidecar run. All four failed against the previous `server.ts` (the configuration change re-opened `beta.tt` on the first folder's compiler; the auto-discovered build was started with `--server`; the second folder's document went to the first folder's build; the relative path never started a compiler).
- 2026-09-27: Added `folderCompiler` to `ttc.ts` with a unit test in `compilerfor.test.ts`; replaced `servedCompiler`, `compilerByRoot` and `refreshCompiler` in `server.ts` with the owner table, `compilerFor`/`compilerOf`, and `armCompilers`; updated the `tt.compilerPath` description and README.

## Issues and resolutions

None beyond the defects recorded above.

## Verification

- [x] `npm run compile` in `editors/vscode`
- [x] `node --test server/out/test/*.test.js client/out/test/*.test.js` with `target/debug` on `PATH`
- [x] `node scripts/check-task-index`

## Result

Changed files: `editors/vscode/server/src/server.ts`, `editors/vscode/server/src/ttc.ts`, `editors/vscode/server/src/test/server.test.ts`, `editors/vscode/server/src/test/compilerfor.test.ts`, `editors/vscode/package.json`, `editors/vscode/README.md`, `docs/tasks/INDEX.md`, this record. Every folder's documents are validated, served, and given sidecars by the compiler that folder configures or discovers, from startup through configuration changes.
