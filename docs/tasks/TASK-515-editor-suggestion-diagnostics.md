# TASK-515: Serve TypeScript's unused and deprecated suggestions in the editor

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

A `.ts` file in VS Code fades unused imports and locals and strikes through
calls to `@deprecated` declarations. A `.tt` file showed neither: the engine
dropped every service diagnostic above warning severity, and the service
session never asked the TypeScript server for diagnostic tags or related
information.

## Scope

- Included: The service session's diagnostic capabilities
  (`src/typescript/service.rs`), `ServiceDiagnostic` and its mapping
  (`src/engine/language.rs`, `src/engine/language/project.rs`), the
  `tsDiagnostics` server answer (`src/server.rs`), and the extension's
  conversion and layer merge (`editors/vscode/server/src/engine.ts`,
  `editors/vscode/server/src/server.ts`).
- Excluded: The CLI's typed report, which reports errors only.

## Decisions

### Decision 1: Carry TypeScript's severity and tags instead of a warning flag

- **Context**: `ServiceDiagnostic` had `warning: bool`, and
  `service_diagnostics` skipped `severity > 2`. A suggestion has no place in
  that shape.
- **Alternatives considered**: Adding a `hint: bool` beside `warning` would
  still lose the tag that decides between fading and striking through.
- **Decision and rationale**: `ServiceSeverity` (the four LSP 3.17
  `DiagnosticSeverity` values) and `ServiceTag` (`Unnecessary`, `Deprecated`,
  LSP 3.15 `DiagnosticTag`) replace the flag. The server answers
  `"severity": "error" | "warning" | "information" | "hint"` and `"tags"` when
  there are any; the extension maps both to LSP values.

### Decision 2: Declare the LSP 3.18 diagnostic capabilities to the TypeScript server

- **Context**: With suggestions kept, tsgo still sent no `tags`. Its client
  capability type has `tagSupport` and `relatedInformation`
  (`DiagnosticClientCapabilities` extends `DiagnosticsCapabilities` in LSP
  3.18), and the session declared `"diagnostic": {}`.
- **Decision and rationale**: The session declares
  `"relatedInformation": true` and `"tagSupport": { "valueSet": [1, 2] }`.
  This also makes pull diagnostics carry `relatedInformation`, which a comment
  in `project.rs` had attributed to the preview omitting it; the comment now
  names the capability.

### Decision 3: A suggestion survives only on text the user wrote

- **Context**: A suggestion names the text it covers. One that maps onto
  compiler glue would fade or strike text the user did not write.
- **Decision and rationale**: Information and hint diagnostics are kept only
  for an exact source mapping. Unused pattern bindings (`Circle(radius) =>`)
  map exactly and are faded, as an unused destructured name is in TypeScript.

### Decision 4: The typed layer replaces problems, not suggestions

- **Context**: `mergeTyped` removes every `source: "ts"` entry when the
  typed compiler pass answers (the default settings), and treats any entry at
  the same start position as covering a typed diagnostic.
- **Decision and rationale**: Both rules now consider errors and warnings
  only (`isProblem`). The compiler reports no suggestions, so removing them
  would erase the layer, and a faded name must not hide an error that starts
  at the same place. The same rule already applied in spirit to tt's own
  unreachable-arm hints, which could previously hide such an error.

## Work log

- 2026-09-29: Reproduced with `tsDiagnostics` on a `.tt` file holding an
  unused import, an unused local and a deprecated call: only TS2591 came
  back. Changed the severity model, the service capabilities, the server
  answer and the extension; re-ran the probe: 6133 ×2 with `unnecessary`,
  6387 with `deprecated` and its related "marked as deprecated here".
  Probed a file with match, if let, let-else, pipelines, `try` and `result`:
  only written, genuinely unused names were reported.
- 2026-09-29: Added `service_suggestions_keep_their_severity_and_tags_on_written_text`
  (`tests/native/cases_06.rs`) and the LSP case
  "unused and deprecated suggestions are published beside the type errors"
  (`editors/vscode/server/src/test/server.test.ts`).

## Issues and resolutions

### Issue 1: Suggestions arrived without tags

- **Symptom**: After keeping severity 4, `tags` was absent from every item.
- **Cause**: The session did not declare `tagSupport` (Decision 2).
- **Resolution**: Declared it.

### Issue 2: Server tests waiting for an empty publish timed out

- **Symptom**: The LSP cases that wait for `diagnostics.length === 0`
  (filesystem and config refresh) never resolved, and two engine cases that
  asserted an empty `tsDiagnostics` answer failed on 6133 / 6196.
- **Cause**: Those sources hold names that are genuinely unused
  (`const result: string = value;`, `type Item = never;`), which TypeScript
  reports as suggestions and the editor now shows. The tests meant "no
  problems", not "no suggestions".
- **Resolution**: The waits and the assertion compare errors and warnings
  only (`problems` in `server.test.ts` and `engine.test.ts`). Assertions on
  specific codes were left as they were.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `npm run compile` and `node --test "server/out/test/*.test.js" "client/out/test/*.test.js"`
  in `editors/vscode`: 220 passed after Issue 2.

## Result

Changed `src/typescript/service.rs`, `src/engine/language.rs`,
`src/engine/language/project.rs`, `src/engine/mod.rs`, `src/server.rs`,
`editors/vscode/server/src/engine.ts`, `editors/vscode/server/src/server.ts`,
and the tests in `tests/native/cases_06.rs` and
`editors/vscode/server/src/test/` (`server.test.ts`, `engine.test.ts`,
`emitmap.test.ts`, `toolchain.ts`). A `.tt` buffer now fades unused names and
strikes through deprecated calls as a `.ts` buffer does, and pull diagnostics
carry TypeScript's related information.
