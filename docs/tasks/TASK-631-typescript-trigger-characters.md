# TASK-631: Open completion on TypeScript's trigger characters and let TypeScript decide

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-631: Open completion on TypeScript's trigger characters and let TypeScript decide`

## Purpose

A `.tt` buffer opened completion only on `. ( | { ,`, and a request sent by
any trigger character outside tt's pattern set was answered with nothing:
typing `/` in `from "./|"` or `.` in `from ".|"` listed no module, and `@`
in a JSDoc comment, `#` after `this.`, and `<` in a `.ttx` element listed
nothing. tsgo advertises `.`, `"`, `'`, backtick, `/`, `@`, `<`, `#`, space,
and `*` for completion and `( , <` for signature help, and answers each in a
`.ts` file.

## Scope

- Included: The completion and signature-help capabilities the adapter
  advertises, how the adapter routes a triggered completion request
  (`editors/vscode/server/src/server.ts`), the trigger the `completion` and
  `signatureHelp` requests carry (`engine.ts`, `src/server.rs`), the
  context the engine sends to the service (`Project::triggered_completion`,
  `Project::triggered_signature_help`, `SignatureTrigger`, and
  `ts_completions` in `src/engine/language/`), and an entry's `detail`
  (`CompletionItem::detail`).
- Excluded: tt's pattern triggers (`( | { ,`), whose behaviour is
  unchanged, and the entries the service answers.

## Decisions

### Decision 1: Advertise the union of TypeScript's trigger characters and tt's

- **Context**: LSP 3.17 `CompletionOptions.triggerCharacters` and
  `SignatureHelpOptions.triggerCharacters` are answered in `initialize`.
  The editor sends a completion request on a character only if the server
  advertised it, so a character tt does not list never reaches TypeScript,
  which owns completion for `.tt` files (the content mapper's feature
  ownership, `npm/patches/README.md`).
- **Alternatives considered**: (a) Read the characters from the service's
  own `initialize` answer: the adapter answers `initialize` before the
  engine starts (it starts lazily, and may find no toolchain), so the
  answer would have to be registered later with `client/registerCapability`,
  more protocol for a list that changes with the pinned TypeScript only.
  (b) Advertise only the characters tt had a use for: the defect.
- **Decision and rationale**: The adapter advertises tsgo's list (`.`, `"`,
  `'`, backtick, `/`, `@`, `<`, `#`, space, `*`) plus tt's pattern
  triggers, and signature help's `( , <` with retrigger `)`. The extension
  test starts the pinned tsgo and fails when it advertises a character tt
  does not, so a TypeScript upgrade that adds one is caught.

### Decision 2: Forward the request's context and let TypeScript decide

- **Context**: Most of these characters begin a completion only in some
  places: a space only after a top-level `import`, `<` only for a JSX tag,
  `/` only in a module specifier or a JSX closing tag, a quote only at an
  opening quote, `#` only for a private name in a class. TypeScript decides
  this from the request's context (`services/completions.ts`,
  `isValidTrigger`, called with the LSP `CompletionContext.triggerCharacter`)
  and answers nothing where the character does not begin a completion.
- **Alternatives considered**: (a) Decide in the adapter from the text
  before the cursor: a second implementation of `isValidTrigger`. (b) Send
  every triggered request as an invoked one: the service would list every
  global after `const q = ` and a space.
- **Decision and rationale**: A request a TypeScript trigger character sent
  goes to the engine with its character (`triggerCharacter`), and the
  engine asks the service with LSP 3.17 `CompletionContext { triggerKind:
  TriggerCharacter, triggerCharacter }`; every other request is
  `Invoked`. The adapter adds no keyword snippet to a triggered answer,
  since TypeScript's answer there is not a list of names being typed. A
  member access keeps its branch and forwards its `.`. Signature help
  forwards `SignatureHelpContext` (`triggerKind`, `triggerCharacter`,
  `isRetrigger`) the same way, so a typed `<` opens help for type arguments
  and not for a comparison (TypeScript's `characterTyped` reason uses
  syntactic owners only).

### Decision 3: tt's pattern triggers keep their meaning

- **Context**: `( | { ,` trigger pattern completion (TASK-106); TypeScript
  advertises none of them for completion.
- **Decision and rationale**: They are routed as before: a pattern position
  is answered by tt, and anywhere else they open nothing. The two sets do
  not overlap, so no character has two meanings.

### Decision 4: Carry the service's `detail` (minor item)

- **Context**: TypeScript's path entries carry `detail` (`lib` shows
  `lib.ts`); the engine dropped it, and tt's own `.tt` module entries
  (TASK-609) had none.
- **Decision and rationale**: `CompletionItem::detail` keeps the service's
  `detail`, a `.tt` module entry's is its file name, and the adapter sends
  it; resolving an entry still replaces it with the signature when there is
  one.

## Work log

- 2026-09-30: Compared capabilities with the probe harness (`caps.cjs`) and
  reproduced `from "./|"` (`/`) and `from ".|"` (`.`): tsgo answered `lib`
  and `typescript`, tt nothing.
- 2026-09-30: Implemented the capabilities, the routing, the engine and
  server trigger parameters, and the context. Compared every trigger with
  its twin (`trig.cjs w/tr.tt w/tr.ts`, `w/trx.ttx w/trx.tsx`, and
  signature help `w/ts1.tt`): each answer matches TypeScript's (`/` adds
  `shapes.tt`).
- 2026-09-30: Tests: `a_triggered_completion_answers_as_typescript_answers_its_twin`
  (`tests/native/editor_service.rs`, `/ @ # space ' <` in a `.ttx` file
  against its `.tsx` twin) fails when the engine sends every request as
  invoked (a space answered every global). The extension tests "the server
  advertises every trigger character TypeScript's server advertises" and
  "a TypeScript trigger character is answered as TypeScript answers it"
  (`server.test.ts`) both fail with the previous adapter.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native a_triggered_completion`
- [x] `node --test server/out/test/server.test.js server/out/test/completion.test.js server/out/test/engine.test.js` (105 passed)
- [x] Full gate (recorded in TASK-633, run once for TASK-629 to TASK-633)

## Result

Changed `src/engine/language.rs`, `src/engine/language/project.rs`,
`src/engine/language/service.rs`, `src/engine/mod.rs`, `src/server.rs`,
`tests/native/editor_service.rs`, `editors/vscode/server/src/server.ts`,
`editors/vscode/server/src/engine.ts`,
`editors/vscode/server/src/test/server.test.ts`,
`editors/vscode/README.md`, `docs/design/lsp-architecture.md`, and the task
index. A `.tt` buffer opens completion and signature help on the characters
a `.ts` buffer does, and TypeScript decides what they open.
