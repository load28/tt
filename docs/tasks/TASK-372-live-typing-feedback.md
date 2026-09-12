# TASK-372: Audit live typing and editor feedback latency

> Superseded in part by TASK-373: blanket external UI ownership is replaced by method-scoped ownership, and match recognition defers recursive body construction until grammar commitment. The original wildcard recovery and nested-fallback validation were incomplete; TASK-373 records the review reproductions and corrected contracts.

- **Status**: Complete
- **Started**: 2026-09-12
- **Completed**: 2026-09-12
- **Commit**: —

## Purpose

Measure and verify the developer experience while actually typing in VS Code, including incomplete syntax, inference, suggestions, diagnostic transitions, and latency relative to TypeScript.

## Scope

- Included: Real editor typing, completion/hover/signature requests, diagnostic event traces, and architectural fixes for reproduced problems.
- Excluded: Replacing measured failures with longer timeouts or suppressing valid diagnostics.

## Decisions

Continue on the TASK-371 branch, preserving its uncommitted changes. Follow the user's explicit requirement to manipulate the VS Code desktop UI. Use actual typing, suggestion acceptance, hover commands, and the Problems panel as primary evidence. Automated tests provide regression coverage only. Tool round-trip durations are not editor latency measurements.

Preserve checker-provided insertion text, filter text, and snippet semantics across the engine and LSP boundary. A decorated completion label is presentation data, not insertion code. Do not strip punctuation from labels.

Parse match arms as independently recoverable list elements. Synchronize failed elements at the current delimiter depth after trying the actual arm grammar. Preserve valid sibling arms and their original byte mappings in the shared projection used by the language service, completion probes, and native content mapper. Normal compilation still rejects malformed syntax; original diagnostics remain visible.

Separate content-mapper document synchronization from UI feature ownership in the native extension. tt registers an external ownership lease only after its LSP client starts. Native document synchronization remains active for TypeScript consumers. Capability registrations and custom providers follow the lease through registration, late delegation, disposal, and server restart. Reject diagnostic filtering, suggestion deduplication, and disabling the native server as substitutes for this contract.

Apply the ownership extension patch in the existing release VSIX builder at the exact repository TypeScript pin, with upstream build/tests and explicit modification provenance. This supersedes TASK-258's unmodified-upstream packaging assumption; the identity required by TASK-259 remains unchanged. An unreviewed future source pin fails the patch gate. Correct the contribution's field name to the pinned public API, `inferredProjectContribution`.

The direct UI audit establishes visible behavior, not TypeScript-equivalent latency or complete parser recovery. No output-verification bypass or longer feature timeout was introduced.

## Work log

- 2026-09-12: Ran doctor successfully and inspected the prior completed-buffer editor suite.
- 2026-09-12: Switched to direct desktop UI manipulation after the user clarified the required test method. Removed the unrun scripted typing suite.
- 2026-09-12: Opened an ignored test workspace in VS Code with the debug compiler. Typed member prefixes, accepted a String method, requested hover, changed an assignment type, and inspected the Problems panel.
- 2026-09-12: Compared incomplete tt match arms with an incomplete TypeScript conditional expression in the same editor.
- 2026-09-12: Reproduced invalid optional JSX attribute insertion by selecting the tt provider's second `title?` entry and pressing Tab.
- 2026-09-12: Inspected the raw checker completion: `label: "title?"`, `insertText: "title"`, `filterText: "title"`. Fixed the insertion contract across Rust engine, JSON transport, and LSP adapter.
- 2026-09-12: Built the changed extension and opened `[Extension Development Host] task-372-fixed`. Typed `i` after `<div t`, accepted `title?` with Tab, and observed `<div title>` in the buffer. Typed `="hello"` and observed zero errors after diagnostics refreshed. This workspace exercised the tt provider alone; the original configured mapper workspace exercised both providers.

- 2026-09-12: Extracted the single-arm grammar for ordinary and tuple matches into the shared list parser; added parser-owned arm spans and exact mapping regressions for first/middle/last/tuple failures.
- 2026-09-12: Routed the native content mapper and completion probe through shared projection recovery. Updated the mapper's prior empty-module expectation for recoverable invalid field types; the original error remains reported.
- 2026-09-12: Prototyped native ownership on upstream typescript-go source, then ported and verified the patch on microsoft/TypeScript@5739027c9a7df24e27123f453a50c011b37717b6, matching the pinned npm package. No installed global extension was modified.
- 2026-09-12: A reused development window initially loaded the old installed native extension and still showed duplicates. Closed that test window and launched a new development host with both extension paths; observed a single tt error and hover, error clearing after editing the type, and an independent native error in control.ts.
- 2026-09-12: In the final pinned development host, directly entered a dot after a valid match payload beside an incomplete arm. Observed String suggestions, selected toUpperCase, and completed the call. The remaining tt error was localized to Gue. Prefix insertion also used paste because the user's active Korean input source and custom keybindings affected synthesized letter input; no timing conclusions are drawn from those tool interactions.
- 2026-09-12: Added a real extension-host ownership suite. Its first lifecycle attempt closed the provider's preview tab when opening the consumer; corrected the test to retain both tabs, then verified the complete ownership lifecycle without changing production code or timeouts.
- 2026-09-12: Integrated the reviewed source patch, upstream tests, and provenance in the existing VSIX builder. Verified the LF patch against clean CRLF upstream files. Added inferred-project coverage for the pinned contribution manifest field.

## Issues and resolutions

### Optional JSX attribute completion inserts invalid syntax — fixed

- Symptom: Accepting the tt suggestion `title?` inserted `<div title?>`, producing a verify-failed diagnostic.
- Cause: The engine and LSP transport retained only label/kind/sort text, discarding the checker's actual insertion and filtering text.
- Resolution: Carry insertion text, filtering text, and snippet semantics independently of the display label. Regression coverage checks the actual LSP completion before and after resolve. Direct UI acceptance confirmed valid insertion.

### Duplicate hover, completion, and diagnostics — fixed in the paired extension build

- Symptom: Hover displayed `const greeting: string` twice. Changing `const message: string = greeting` to `number` produced both native `ts(2322)` and `ttc(ts2322)`, at different source spans. JSX attribute suggestions appeared twice.
- Cause: Native TypeScript content-mapper services and the tt language server both supply TypeScript editor features. No explicit feature ownership contract coordinates them.
- Resolution: Add an explicit native client ownership capability and contribution lease; retain synchronization while registering UI providers only for native-owned selectors. Wire the tt claim after client readiness and ship the reviewed native client patch through the existing VSIX build. Direct UI confirmed one tt diagnostic and one inferred hover, while ordinary TypeScript still reported native TS2322. Extension-host tests exercise late delegation, unsaved consumer changes, and lease disposal. Previously installed unpatched extensions are unchanged and still require the matching new build.

### An incomplete match arm removes valid sibling-arm type context — fixed

- Symptom: Replacing `Guest => "guest"` with `Gue` removed hover and String member completion from `name` in the otherwise valid `Admin(name, level) => name` arm. The error highlighted `match`, not the incomplete arm. The comparable incomplete TypeScript conditional retained String member suggestions.
- Cause: Match parsing is all-or-nothing; the parser-owned recovery covers the complete match expression, so its projection removes valid arms too.
- Resolution: Record malformed match arm spans in parser recovery, remove only those list elements in the editor projection, and retain source maps for valid siblings. The same projection now serves native content mapping and the existing completion probe. The diagnostic points at the malformed arm. Direct UI verified `name.` suggestions beside `Gue`, selection of `toUpperCase`, and disappearance of the member error after completing the call.

## Verification

- Direct desktop UI: reproduced all three original defects; verified optional JSX insertion, sibling-arm suggestions and completion acceptance, single hover/diagnostic ownership, diagnostic clearing, and ordinary native TypeScript diagnostics. Both standalone tt and paired extension development hosts were exercised.
- `./scripts/ci rust`: passed fmt, clippy with warnings denied, the complete Rust suite, and fuzz-target compilation; `/tmp/tt-task372-final-rust.log`. Subsequently strengthened the source-map assertion and passed its focused regression (`/tmp/tt-task372-mapping-regression.log`).
- Extension unit/LSP integration: 173 passed, zero skipped; `/tmp/tt-task372-final-extension.log`.
- Patched native extension at the exact pinned source: build passed and upstream tests 15 passed; `/tmp/tt-task372-pinned-native-build.log`, `/tmp/tt-task372-pinned-native-tests.log`.
- Full paired native editor suite: 71 passed, zero failures; `/tmp/tt-task372-pinned-editor.log`, artifacts `target/editor-tests/run-yzLm76`.
- Final ownership suite: 5 passed, including inferred-project manifest delivery, late ownership, consumer synchronization, and restoration; `/tmp/tt-task372-ownership-final.log`, artifacts `target/editor-tests/run-XQvaO2`.
- `./scripts/ci agents npm`: agents passed; npm initially failed because the sandbox refused scaffold dependency downloads. The same npm gate passed with network access; `/tmp/tt-task372-final-npm-network.log`.
- Exact-pin patch application and packaging tests passed. No release was published and no user's installed extension was replaced.
- No precise latency claim: screenshots and accessibility snapshots establish visible states; their tool overhead does not measure keystroke-to-render time.

Changed files for this task: parser recovery (`src/ast.rs`, `src/parser/{matches,host,parse}.rs`), shared projection (`src/lib/compile.rs`, `src/content_mapper.rs`, `src/content_mapper/tests.rs`), engine/LSP completion transport (`src/engine/language.rs`, `src/engine/language/{service,tests}.rs`, `src/server.rs`, `editors/vscode/server/src/{engine,server}.ts`, server tests), extension registration and ownership tests (`editors/vscode/client/src/{extension,contentMapper}.ts`, `editors/vscode/test/ownership.cjs`, test runner), the native patch and packaging builder/tests, `.gitignore`, the compile diagnostic regression, extension README, task records and index. TASK-371 changes remain in the shared working tree.

## Result

The three reproduced editor defects have structural fixes and direct desktop verification. The paired native client change is part of the existing VSIX build rather than an ignored local prototype. Previously installed extensions are unchanged; the matching new pair is required to use ownership. The final inferred-project check passed. No claim of universal TypeScript-equivalent latency is made.
