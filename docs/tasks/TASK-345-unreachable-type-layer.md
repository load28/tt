# TASK-345: An engine that cannot answer is not an engine that answered "none"

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-345: never publish an unreachable type layer as a clean file`

## Purpose

`engine.tsDiagnostics` returned `[]` both for "this file has no type
errors" and for "the engine did not answer". A diagnostic publish replaces
the file's complete list, so the second case cleared every type error the
editor was showing — for a file that still had them — and nothing said so
or tried again.

## Scope

- Included: What `tsDiagnostics` returns, what `validate` does with a layer
  that did not answer, and the retry and report that follow.
- Excluded: Why the engine stopped answering (TASK-334 owns the session's
  own recovery), and the typed compiler pass, which already distinguishes
  its outcomes.

## Decisions

### Decision 1: The answer type carries all three outcomes

- **Context**: The question has three outcomes — these are the errors,
  there are none, I could not tell — and the return type could express two.
  `semanticTokens`, three functions away in the same file, already returns
  `null` for "the engine is unavailable" and documents exactly this
  distinction, so the shape was established; `tsDiagnostics` had simply
  collapsed it with `result?.diagnostics ?? []`.
- **Alternatives considered**: Have the caller re-ask the engine whether it
  is alive (a second question whose answer can differ from the one that
  matters); keep a "last known good" list and republish it when the answer
  is empty (cannot tell a fixed file from an unreachable engine, so it
  would resurrect errors the user had just fixed).
- **Decision and rationale**: `Promise<EngineDiagnostic[] | null>`, with
  `null` only for "no answer at all". An engine that *answers* with an
  error — no TypeScript toolchain, a project that will not load — has said
  something definite, so that stays an empty list plus the message on
  `onError`. The two are already distinct inside `semantic()`; this reads
  them apart instead of merging them.

### Decision 2: A generation whose layer is unknown is not published

- **Context**: `server.ts` already states the rule this violated: "an LSP
  diagnostic notification replaces the file's complete list, so publishing
  a partial generation would temporarily erase every slower-layer
  diagnostic". A generation whose type layer could not answer *is* a
  partial generation; it was published anyway because the failure looked
  like an answer.
- **Alternatives considered**: Publish the tt layer and leave the type
  layer out (the erasure this task is about); republish the previous
  generation's type diagnostics (their ranges are stale against the edited
  text, and VS Code is already keeping the live ones adjusted).
- **Decision and rationale**: Do not publish. What is already on screen
  stays, with its ranges tracked by the editor, and the generation is
  retried.

### Decision 3: One retry, then publish without it and say why

- **Context**: The engine respawns on the next request, so a single retry
  covers the common case (a session that died between two validations).
  Retrying forever would leave a file's tt-level errors unpublished
  indefinitely when the engine has genuinely given up.
- **Alternatives considered**: Retry with backoff until it answers (a file
  edited once and then left alone never shows its new tt errors); publish
  immediately and only log (no recovery at all).
- **Decision and rationale**: One retry on the same debounce, carried on
  `validate`'s `attempt` parameter and through the existing
  `pendingValidation` timer so a new edit still cancels it. If the second
  attempt is also unanswered, publish without the layer and raise
  `type-layer-unreachable` once (TASK-343), naming the output channel where
  the engine's own reason was logged.

### Decision 4: The tests assert an answer instead of accepting `null`

- **Context**: Nineteen call sites in the engine and emit-map suites read
  the list directly. Under the new type each has to say what `null` means
  there.
- **Alternatives considered**: `!` at each site (silences the question the
  type exists to ask, and turns a real "the engine died" into a confusing
  property error).
- **Decision and rationale**: An `answered(value, what)` helper in the test
  toolchain. Those suites already guard on a working compiler and
  toolchain, so `null` there is a failure to report by name, which is the
  same distinction the production type now makes.

## Work log

- 2026-09-05: Reproduced against the built server: with the repository's
  TypeScript, `tsDiagnostics` on a file with `const bad: number = "text"`
  returned `[2322]` through a working compiler and `[]` — length 0, the
  same value as a clean file — through a compiler that cannot serve.
- 2026-09-05: Changed the return type in `engine.ts`, reading `no answer`
  and `answered with an error` apart from `engineRequest` directly rather
  than through `semantic()`.
- 2026-09-05: Made `typeDiagnostics` and `validate` carry the unknown, with
  the single retry and the notice.
- 2026-09-05: Added `answered()` to the test toolchain, wrapped the 19 call
  sites, and pinned the distinction in
  `server/src/test/compiler.test.ts`.

## Issues and resolutions

### Issue 1: An unreachable engine cleared the Problems panel

- **Symptom**: A `.tt` buffer with type errors showed none, with nothing in
  the Problems panel and no message, until the next keystroke.
- **Cause**: `result?.diagnostics ?? []` mapped "no answer" to an empty
  list, which `validate` published as the file's complete type layer.
- **Resolution**: Decisions 1–3.

## Verification

- [x] Reproduced before the change and pinned after it: a working engine
  reports `ts2322` for the same file a compiler that cannot serve now
  answers `null` for — `server/src/test/compiler.test.ts`
- [x] An engine that answers with an error still yields an empty layer and
  logs its reason, so a project with no TypeScript is unchanged
- [x] `node --test server/out/test/*.test.js client/out/test/*.test.js`
- [x] `./scripts/ci`

## Result

Changed files: `editors/vscode/server/src/engine.ts`,
`editors/vscode/server/src/server.ts`,
`editors/vscode/server/src/notices.ts`,
`editors/vscode/server/src/test/{toolchain,compiler,engine,emitmap}.ts`,
and this record.
