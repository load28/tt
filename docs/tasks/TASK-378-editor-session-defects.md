# TASK-378: Repair editor session and document ownership defects

- **Status**: Complete
- **Started**: 2026-09-14
- **Completed**: 2026-09-14
- **Commit**: —

## Purpose

An audit of the VS Code extension found the server re-arming the whole project on its own sidecar writes, caching stale declarations under a newer document version, misnaming untitled `.ttx` buffers, masking JSX text as string literals, dropping engine error replies that carry no id, and timing queued requests from the moment they were enqueued. Each is repaired at the layer that owns the contract.

## Scope

- Included: `editors/vscode` server sources and their unit tests.
- Excluded: compiler (Rust) changes, which TASK-377 covers; the Windows `\\?\` path finding, which needs a Windows run to verify.

## Decisions

### Decision 1: The server recognizes its own writes by ownership, not by glob

- **Context**: every save that refreshed a sidecar produced a `*.tt.d.ts` change the watcher classified as external, which reloaded every project and re-validated every document.
- **Alternatives considered**: excluding `*.d.ts` from the client watcher (a hand-edited declaration must still count as external).
- **Decision and rationale**: `sidecar.ts` keeps a `WriteLedger`: the output paths are recorded before `ttc --types` runs (pending state owns any event that arrives mid-write) and fingerprinted after it finishes. `watch.isExternalChange` consults the ledger; a path whose disk content still matches what the server wrote is its own, anything else is external. Deletions are always external.

### Decision 2: A declaration answer is cached only for the version it was asked for

- **Context**: `declarationsOf` read `doc.version` after awaiting the engine; documents update in place, so an in-flight answer landed under the newer version.
- **Decision and rationale**: version, text, and a cache generation are captured before the request; the answer is stored only when both are still current, the same gating the diagnostics path uses.

### Decision 3: One function names the buffer for the engine

- **Context**: validation named an untitled buffer by its URI path, so `Untitled-1` became `Untitled-1.tt` and JSX text was verified as TypeScript.
- **Decision and rationale**: `bufferPath` is the single source of the engine-facing name on every request path; for non-file URIs the kind follows `languageId`.

### Decision 4: JSX text is its own region in the code mask

- **Context**: an apostrophe in JSX text opened a string in `maskNonCode`, blanking the rest of the line and polluting member completion.
- **Decision and rationale**: the byte scanner models elements: attribute strings in the opening tag, `{...}` containers returning to code state, text between tags where quotes are ordinary text. Generic arrows are excluded by TypeScript's own JSX/arrow disambiguation rules.

### Decision 5: The protocol carries well-formed Unicode and settles in order

- **Context**: a lone surrogate in a buffer serialized as `\ud800`, which the Rust side rejects; the `id: null` error reply was dropped, the request hung, and its timer retired the shared session.
- **Decision and rationale**: every request string passes through one replacer (`String.prototype.toWellFormed` when present, otherwise an equivalent replacement of lone surrogates with U+FFFD). Because ttc answers strictly in order, an error reply without an id settles the request at the head of the queue.

### Decision 6: A request's timer starts when it reaches the head of the queue

- **Context**: timers started at enqueue time, so a queued request's budget was `timeout − wait`, and one expiry retired the session for every document.
- **Decision and rationale**: only the head request holds a timer; settlement re-arms the new head. The deadline measures the engine's own work.

## Work log

- 2026-09-14: Audited the extension against the built server over LSP stdio and confirmed each defect before changing code.
- 2026-09-14: Implemented the six decisions in `sidecar.ts`, `watch.ts`, `server.ts`, `analysis.ts`, and `engine.ts`; added regression tests in `watch.test.ts`, `sidecar.test.ts`, `server.test.ts`, `analysis.test.ts`, and `session.test.ts`; verified the new cases fail against the previous sources and pass against the new ones.

## Issues and resolutions

Each defect above is recorded with its decision; no further issues arose.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] extension unit tests (`npm test` in `editors/vscode`: 190 pass, 0 fail; baseline 174)
- [x] `./scripts/ci` (all stages)

## Result

Changed files: `editors/vscode/server/src/{analysis,engine,server,sidecar,watch}.ts` and their tests `editors/vscode/server/src/test/{analysis,server,session,sidecar,watch}.test.ts`.

Left for a later task, confirmed by the audit: on Windows the engine keys documents by `std::fs::canonicalize`, whose `\\?\` prefix does not match Node's `realpathSync` result in `ttc.ts` and produces unopenable `file://%3F/` URIs; and the TextMate grammar colors a plain `match(...)` call as the tt keyword until semantic tokens correct it.
