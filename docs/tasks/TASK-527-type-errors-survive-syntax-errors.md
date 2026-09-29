# TASK-527: Keep a file's type errors in the editor while its TypeScript does not parse

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: `TASK-527: Keep a file's type errors in the editor while its TypeScript does not parse`

## Purpose

A TypeScript syntax error anywhere in a `.tt` buffer removed every type error
of the file from the editor, so type errors flickered away while the user
typed. The same text in a `.ts` file keeps TypeScript's syntax and type
diagnostics side by side.

## Scope

- Included: The projection the language service serves when a file's
  TypeScript does not parse, the condition under which its TypeScript
  diagnostics are reported, which layer reports a syntax error in the editor,
  where a module's helper declarations are written, and how the editor
  merges a typed pass that did not check the file (or checked it and found
  problems only elsewhere).
- Excluded: The CLI and the batch typed path (`ttc --check`,
  `ttc --check-types`, `--types`), which keep reporting only the tt-level
  diagnostic of a file whose TypeScript does not parse, as `tsc` reports only
  syntactic diagnostics while there are any (TASK-433, Issue 2). Completion
  and signature help inside an unfinished tt value (TASK-528). Code actions,
  which the native TypeScript extension owns through the content-mapper
  ownership contract.

## Decisions

### Decision 1: Serve a faithful projection when only the TypeScript is wrong

- **Context**: `compile_projection_report` withholds the emission of a file
  whose TypeScript does not parse (`source-not-typescript`, or
  `verify-failed` in a file whose constructs need no host lowering). The
  service then served the raw `emit_mapped` of the original text and
  `service_diagnostics` returned nothing unless that text parsed
  (`projection_accepts_diagnostics`) — which by definition it did not.
- **Alternatives considered**: (a) Trust the raw fallback always: it keeps
  tt text the parser could not claim as written (a stray `|>`, a
  rolled-back `try`), and TypeScript's reading of tt syntax says nothing
  about the user's code. (b) Mend the user's text (close brackets, insert
  semicolons) before serving: a heuristic that invents code, and whose
  errors would differ from what TypeScript says about the text. (c) Filter
  TypeScript's diagnostics by error-code range: the heuristic the error-layer
  contract forbids. (d) Decide by where the first parse error lands: it
  sees only one error and says nothing about the rest of the text.
- **Decision and rationale**: The condition is structural, decided where the
  projection is built. `ProjectionReport::withheld` carries the file lowered
  without an owner model over the same recovered source whenever no tt text
  is left as the user wrote it: no remaining diagnostic leaves its construct
  unlowered (`DiagnosticCode::leaves_tt_text`, the projection blockers other
  than TypeScript's own syntax verdict) and the parser rolled back no tt
  candidate into passthrough (`parser::unclaimed_candidates`). Every byte of
  that emission is then TypeScript the user wrote, glue of a claimed
  construct, or a recovery placeholder in `recovered`. The service serves it
  as `faithful` and reads it as a `.ts` file is read, with the existing
  filters (exact or anchor mapping, recovery intersection, tt-error
  ownership). Otherwise it keeps the old fallback and rule.

### Decision 2: Make the faithful projection's glue hold its own syntax

- **Context**: Faithfulness needs every piece of glue to stand where the
  user's syntax cannot change what it is. Three defects broke that in the
  no-owner lowering, found by serving the reproductions: the plan had empty
  helper names (`function (value: unknown)`, `" + ($tt_m)`, and a
  `result` boundary emitted as an uncalled `(() => { ... })`); a module's
  helpers were appended after the user's last line, so `Math.max(1,` at the
  end of the file took `function $tt_expr...` in as an argument (TS2304
  `$tt_expr`, TS2345 on the glue); and an unowned value was the placeholder
  `undefined`, whose type made `1 + try parse("2")` report TS18050.
- **Alternatives considered**: Filter the resulting diagnostics (hides real
  ones and keeps the defects); move helpers only in the no-owner lowering (a
  second placement rule for the same declarations).
- **Decision and rationale**: `LoweringPlan::without_owner_model` names its
  helpers against the source's names, as a built plan does. A module writes
  its helpers with its prelude, where no source text precedes them; a
  function declaration is hoisted, so its place does not change its meaning
  (twelve emit fixtures move the helpers from the end to the top, nothing
  else). The prelude of a module now goes after the file-level pragmas, as
  a script's does: TypeScript reads a pragma only before a file's first
  token, in a module too. A value with no lowering is
  `(undefined as any)` — TypeScript's error type, the stand-in the typed
  projection's recovery already writes — so no consequence of the stand-in
  is reported. That recovery wrote `0` into an expression node narrower than
  `undefined as any` (`try fetchJob()`), so a use of the binding reported
  TS2345 about a number; it now writes `0 as any` wherever that fits, and
  `0` only into a node narrower than eight bytes. Glue is delimiter-balanced by construction, so the user's own
  syntax error can move inside a construct's glue but can neither be hidden
  nor created by it. This reverses TASK-483's "modules keep the trailing
  function declarations"; that record says so.

### Decision 3: TypeScript reports a syntax error in the user's TypeScript

- **Context**: With the service reading the buffer, TypeScript reports the
  syntax error (`TS1005 ';' expected.`) and the compiler layers report the
  same fact (`verify-failed`, `source-not-typescript`), often at another
  byte, with a message about ttc's output.
- **Alternatives considered**: (a) Keep ttc's and drop TypeScript's
  syntactic diagnostics: the service does not mark which diagnostics are
  syntactic, so this needs the error-code heuristic; and a `.ts` user sees
  TypeScript's words. (b) Show both.
- **Decision and rationale**: The error-layer contract gives every error in
  the user's TypeScript to TypeScript; `source-not-typescript` and
  `verify-failed` restate TypeScript's verdict as the reason the file has no
  output (`DiagnosticCode::restates_typescript_syntax`). A restatement owns
  no checker consequence (`origin_intersects_tt_error` skips it), and when
  the faithful projection is served, `Project::service_restates` names the
  restated codes — not those inside a recovered span, which TypeScript
  never reads — and `tsDiagnostics` answers them as `restates`. The editor
  drops the compiler layers' copies after merging. The CLI and the batch
  typed path keep reporting them; they have no other reporter.

### Decision 4: A typed pass that did not check the buffer does not replace the service layer

- **Context**: `typedCheck` answered `"blocked": false` for a buffer that
  could not be lowered — the snapshot keeps it as a blocked file and checks
  `export {};` in its place — so the editor let its tt-only answer replace
  every service problem. And a pass that ran and found problems only in
  other files was reported as `unavailable` ("the check reported only
  outside this file"), mirroring the one-shot's stderr parsing, so the
  authoritative answer never replaced the provisional one for a clean file.
- **Alternatives considered**: Decide in the extension from the diagnostic
  codes (a guess about why the pass had nothing to say).
- **Decision and rationale**: `Snapshot::is_blocked` answers the question,
  and `typedCheck`'s `blocked` now means "the pass checked none of this
  buffer's TypeScript": the project could not be read, or the buffer could
  not be lowered. `runTypedCheck` carries it; `typedDiagnosticsFor` replaces
  the service layer only for a pass that was not blocked. A pass that was
  not blocked answers this file with its own diagnostics, none included;
  only a blocked pass with nothing of this file's is `unavailable`.

### Decision 5: An edit in glue written at a source point is an edit at that point

- **Context**: With the helpers at the head of a module, TypeScript's
  auto-import insertion for a file with no imports lands on
  `function $tt_show` — glue — and `source_edit` (TASK-526) dropped the
  edit, so accepting `helperFn` wrote no import in a file with a `match`.
- **Alternatives considered**: Keep the helpers at the end (Decision 2's
  defect); write the edit at the source's first line (TASK-526 set that
  aside: it guesses where TypeScript meant to insert); a zero-length
  mapping at the prelude (a cursor at the first source byte would then be
  served at the helper instead of the user's text).
- **Decision and rationale**: The prelude is glue written at one source
  point rather than for a construct, and the rope says so:
  `insert_lit_at_source` wraps it in `InsertedStart`/`InsertedEnd` marks,
  printed as `MappedEmit::inserted` (`InsertedGlue`), shifted with the other
  records by contextual annotation. `source_edit` maps a zero-width edit in
  such glue to its source point; any other edit in glue still refuses.

## Work log

- 2026-09-29: Reproduced with `const a: number = "x";\nconst o = { k: 1 };\no.\nexport {};\n`
  through `ttc --server`: `tsDiagnostics` answered `[]`, and `typedCheck`
  with `includeTypes` answered only `verify-failed` with `"blocked": false`.
- 2026-09-29: Printed `compile_projection_report` and the fallback emission
  for the reported edits (`o.`, `const b = `, `Math.max(1,`, `if (o.k`, an
  open `{`, an open string, and a match/variant elsewhere). Plain-TypeScript
  edits emit the source unchanged; the tt ones showed the nameless helpers
  and the uncalled `result` boundary (Decision 2).
- 2026-09-29: Added `ProjectionReport::withheld`, `emit_mapped_parsed`,
  `DiagnosticCode::{restates_typescript_syntax, leaves_tt_text}`, the
  service's `faithful` projection, `Project::service_restates`, and the
  server's `restates` and `blocked`. Served the reproductions again: the
  `.tt` answers equal the `.ts` twin's, except `Math.max(1,` at the end of a
  file with a `result` block (TS2304/TS2345 on the trailing helper) and
  `1 + try parse("2")` (TS18050) — both fixed by Decision 2.
- 2026-09-29: Moved module helpers into the prelude and the prelude after
  file pragmas (`src/codegen/core/mod.rs`); `UPDATE_EXPECT=1 cargo test
  --test snapshot` moved the helpers in twelve emit fixtures, and nothing
  else changed in them. Renamed and extended the module prelude test.
- 2026-09-29: Extension: `ValCheckResult.blocked`, `typedDiagnosticsFor`,
  `engine.tsDiagnosticsAnswer`, the `restates` drop in `validate`, and the
  `runTypedCheck` condition (Decision 4).
- 2026-09-29: Tests: `tests/native/cases_08.rs` (each edit against its
  `.ts` twin; tt constructs beside a syntax error; a rolled-back `try`
  stays unread), `server.test.ts` (the publish keeps TS2322 and TS1005 and
  no compiler restatement), `typedcheck.test.ts` (a clean file beside a
  failing one is `ok`/`[]`; an unlowerable buffer is `blocked`),
  `tests/emit_map.rs` (the placeholder), `tests/compile/cases_11.rs` (module
  prelude placement under every pragma header). The server test hangs
  instead of passing with `replacesTypes: includeTypes` put back, because
  the final publish then has no diagnostics left.

- 2026-09-29: The full Rust run failed
  `a_banner_shifts_the_map_so_positions_still_line_up`, which inferred "the
  banner is one line" from the second generated line having a mapping — now
  the unmapped helper. It now compares the map with and without the banner:
  the banner adds exactly one `;`.
- 2026-09-29: Added `an_auto_import_lands_where_the_helpers_of_a_module_stand`
  (`tests/native/cases_08.rs`); it failed with no edit (Decision 5), then
  passed with TypeScript's own `import ...;\n\n` at 0:0.
- 2026-09-29: Extension run: `the editor reports every practical diagnostic
  in result-boundaries` failed with an extra TS2345 on `persist(job)` from
  the `0` placeholder (Decision 2); passes with `0 as any`.
- 2026-09-29: Coordinator item: a typed pass that ran with findings only in
  other files answered `unavailable`; fixed with the blocked change
  (Decision 4) and `typedcheck.test.ts`'s clean-beside-failing case, which
  the engine answers with the sibling's TS2322 in `diagnostics`.

## Issues and resolutions

### Issue 1: Type errors disappeared while the TypeScript did not parse

- **Symptom**: `tsDiagnostics` answered `[]` for a buffer with `o.` on one
  line and TS2322 on another; the published list held only `verify-failed`.
- **Cause**: The service rejected the fallback projection as a whole
  because it did not parse, and the typed pass's tt-only answer replaced the
  service layer although it had checked none of the buffer.
- **Resolution**: Decisions 1 and 4.

### Issue 2: The no-owner lowering wrote invalid glue

- **Symptom**: `function (value: unknown): string {`, `" + ($tt_m)`, and
  `const r = (() => { ... })` in `emit_mapped` of a file whose TypeScript
  does not parse.
- **Cause**: `LoweringPlan::without_owner_model` left the helper names at
  their empty defaults.
- **Resolution**: Decision 2.

### Issue 3: A bracket left open at the end of a file took the helpers in

- **Symptom**: TS2304 `Cannot find name '$tt_expr'` and TS2345 on glue for
  `Math.max(1,` as the last line of a module with a `result` block.
- **Cause**: Module helpers were appended after the user's text.
- **Resolution**: Decision 2.

### Issue 4: Auto-import edits dropped in a module with helpers

- **Symptom**: `completion_resolve` of `helperFn` answered no edit in a file
  with a `match` and no imports.
- **Cause**: The insertion point was the prelude, which no mapping covers.
- **Resolution**: Decision 5.

### Issue 5: A narrow recovery placeholder reported a type error

- **Symptom**: TS2345 `Argument of type 'number' is not assignable to
  parameter of type 'Job'` in the practical `result-boundaries` fixture.
- **Cause**: The `0` stand-in for `try fetchJob()` in a constructor.
- **Resolution**: Decision 2.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --no-fail-fast`: every suite passed
  (`native` 111, `compile` 551, lib 380).
- [x] `cd editors/vscode && npm run compile && node --test "server/out/test/*.test.js" "client/out/test/*.test.js"`: 223 passed.

## Result

A `.tt` buffer whose TypeScript does not parse now shows TypeScript's own
syntax errors and the file's type errors, as the same text does in a `.ts`
file, and the compiler's restatement of the syntax error is not shown beside
them. A buffer with tt text left as written keeps the old behavior.

Changed `src/lib/compile.rs`, `src/lib/mapped.rs`, `src/diagnostics.rs`,
`src/evaluation_ir.rs`, `src/codegen/core/mod.rs`,
`src/codegen/core/emitter/source.rs`, `src/codegen/rope.rs`,
`src/codegen/rope/builder.rs`, `src/codegen/contextual.rs`,
`src/engine/language.rs`, `src/engine/language/project.rs`,
`src/engine/language/service.rs`, `src/engine/projection.rs`,
`src/engine/snapshot.rs`, `src/server.rs`,
`editors/vscode/server/src/{engine,server,ttc}.ts`, their tests
(`server.test.ts`, `typedcheck.test.ts`), `tests/native.rs`,
`tests/native/cases_08.rs`, `tests/emit_map.rs`, `tests/compile/cases_11.rs`,
`tests/cli/cases_01.rs`, twelve `tests/fixtures/emit/*/expected.ts(x)`,
`docs/design/lsp-architecture.md`, `docs/design/program-lowering.md`,
`docs/ai/tt.md`, and the TASK-483 record.
