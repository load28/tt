# TASK-640: Leave the rules TypeScript checks after parsing to TypeScript

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-640`

## Purpose

TASK-638 found 189 units of TypeScript's own test cases that ttc rejects with
`verify-failed`: 42 that the pinned TypeScript accepts (contract 1) and 147
whose error TypeScript's checker reports under its own code while ttc reports
it first in swc's words (contract 2). The self-check exists to catch ttc's
own emission bugs; bytes ttc copied verbatim are the user's TypeScript, and
the rules TypeScript enforces on them after parsing belong to TypeScript.

## Scope

- Included: the output self-check (`src/verify.rs`) and the projection parse
  that models a tt-bearing file's TypeScript
  (`src/program_syntax/collector.rs`): which swc errors they report inside
  copied text, and the goal (script or module) they parse with.
  `tests/passthrough-accepted.txt` and `tests/passthrough-triaged.txt`,
  `docs/ai/tt.md`, a case file and a passthrough test.
- Excluded: swc parser gaps where swc cannot read what TypeScript's parser
  reads (TASK-641), and the swc parses of other layers (`val`, sidecars,
  contextual refinement), which read the emission for facts and report
  nothing to the user.

## Sources

- TypeScript at `5739027c9a7df24e27123f453a50c011b37717b6` (the pinned
  `gitHead`), `tsc/internal`:
  - `parser/parser.go` `parseSourceFileWorker` and `reparseTopLevelAwait`:
    every file is parsed with no await context, and only a file with an
    `ExternalModuleIndicator` has its top-level statements reparsed in one;
    `isAwaitExpression` reads `await x` as an await expression anywhere.
  - `binder/binder.go` `checkContextualIdentifier`: strict-mode reserved
    words (TS1212-TS1214) and `await` (TS1262, TS1359) are reported by the
    binder, not the parser, and never in an ambient context
    (`NodeFlagsAmbient`); `await` is reserved only at the top level of a
    module or in an await context. TS1100, TS1101, TS1102, TS1344 and
    TS18012 are also binder diagnostics.
  - `checker/grammarchecks.go` and `checker/checker.go`: where each
    diagnostic behind the classified swc error kinds is reported (TS1009,
    TS1014, TS1015, TS1029-TS1048, TS1089-TS1098, TS1105-TS1123, TS1155,
    TS1162-TS1184, TS1182, TS1242-TS1277, TS1308, TS1347, TS1358, TS2206,
    TS2207, TS2483, TS2524, TS17000, TS18006, TS18010 in `grammarchecks.go`;
    TS1108, TS1113, TS1114, TS1164, TS1245, TS1257, TS1267, TS1375, TS2335,
    TS2337, TS2364, TS2369, TS2371, TS2392, TS2406, TS2410, TS2414, TS2427,
    TS2452, TS2491, TS2499, TS2660, TS2680, TS2703, TS2779, TS2815, TS4112,
    TS5076, TS17013, TS18016 in `checker.go`), each checked by the diagnostic
    name in `diagnostics/diagnostics_generated.go` against the five files.
  - `core/compileroptions.go`: `AlwaysStrict` is deprecated (TypeScript 7 is
    always strict) and `GetEmitModuleDetectionKind` is `Auto` unless
    `module` is `node16` to `nodenext`: a file is a module by its own syntax.
- ECMA-262, §13.1.1 (Static Semantics: Early Errors for identifiers): `await`
  is reserved only when the goal symbol is `Module`; strict-mode reserved
  words are early errors, not grammar productions (§12.7.2, §16.1 for the two
  goal symbols).
- swc (`vendor/swc_ecma_parser`, 45.0.0): `Parser::parse_program` parses a
  file as a script and switches to module (and strict) mode only when it
  finds module syntax; `parse_module` imposes the module goal on every file.
  Errors the parser recovers from are collected by `take_errors`; an
  unrecoverable one is the `Err` of the parse. swc names its TypeScript
  diagnostics after TypeScript's codes (`SyntaxError::TS2369` and so on).

## Decisions

### Decision 1: Report, in copied text, only what TypeScript's parser reports

- **Context**: The self-check parses the whole emission and reported the
  first swc error anywhere. Almost all 189 units fail on an swc error that
  TypeScript reports from its binder or checker, or not at all.
- **Alternatives considered**:
  (a) Skip the self-check when the emission contains no generated text: every
  corpus unit would pass, but it reverses the recorded behaviour that a file
  without tt constructs still reports through the self-check
  (`a_file_without_tt_constructs_still_reports_through_the_output_self_check`,
  `verify_rejects_invalid_passthrough_typescript`), drops the diagnostic for
  tt candidates that did not claim (`let_else_without_semicolon_is_not_recognized`
  and three more), and changes what the editor restates
  (`a_syntax_error_keeps_the_type_errors_a_ts_file_shows`). Those restate
  TypeScript's own parser verdict, which is no violation of either contract.
  (b) Drop every error swc recovered from inside copied text: swc also
  recovers from real syntax errors (`const a = 1 const b = 2;`, TS1005 in
  TypeScript's parser), which `a_recoverable_syntax_error_still_reports_plan_diagnostics`
  pins as `source-not-typescript`.
  (c) Turn swc's early errors off (`TsSyntax::no_early_errors`): also turns
  them off in generated text, where they are ttc's bugs.
  (d) Classify swc's error kinds by the TypeScript layer that enforces the
  rule, and leave to TypeScript the recovered errors inside one copied range
  whose rule TypeScript checks after parsing.
- **Decision and rationale**: (d). `verify::checked_after_parsing` lists the
  swc error kinds whose TypeScript diagnostic the pinned TypeScript reports
  only from the binder or the checker (Sources); any other kind, including
  those TypeScript's parser shares (`Identifier_expected` TS1003, `'{0}'
  expected` TS1005, TS1359 for `await`, TS8038), is read as a parse failure.
  An error is left to TypeScript only when swc recovered from it (the module
  was read), its kind is listed, and its span lies inside one copied range of
  the emit mappings (`crate::EmitMapping`); an error that touches generated
  text is ttc's and fails the check. The projection parse of a tt-bearing file
  uses the same rule over its copied segments, so such a file no longer
  reports `source-not-typescript` for TypeScript that parses. The risk that
  remains, a generated wrapper that makes a copied identifier an error (a
  copied `await` inside a generated async function), is reported by
  TypeScript's check of the emission.

### Decision 2: Parse with TypeScript's goal: a script unless the file has module syntax

- **Context**: The self-check parsed every file with `parse_module`, the
  ECMAScript module goal, which reserves `await` everywhere; TypeScript
  reserves it only at the top level of a module or in an await context, and
  reads a file without module syntax as a script.
- **Alternatives considered**: `parse_script` (rejects `import`/`export`);
  `parse_typescript_module` (module goal without strict mode: still reserves
  `await`, and no longer checks generated text under TypeScript 7's always
  strict rules); keeping `parse_module` and classifying
  `InvalidIdentInAsync` (swc reports it as unrecoverable for a parameter).
- **Decision and rationale**: `parse_program`, which is TypeScript's
  `moduleDetection: auto` rule, the default the pinned TypeScript applies
  (`GetEmitModuleDetectionKind`). A module is still parsed in strict mode, so
  generated text is held to strict rules there, and the strict-mode errors in
  copied text are the binder's (Decision 1). The projection parse converts a
  script to the module shape the collector walks; it already derived script
  facts from the tree (`is_script`).

## Work log

- 2026-09-30: Reproduced the triaged classes with `ttc --check`; read the
  pinned TypeScript's parser, binder, checker and compiler options, and
  swc's `parse_program`, `parse_module` and error collection.
- 2026-09-30: First cut (drop every recovered error in copied text, module
  goal): 117 listed units passed. With `parse_program`: 136.
- 2026-09-30: `tests/compile` failed `a_recoverable_syntax_error_still_reports_plan_diagnostics`
  (Issue 1); logged the error kind of every swc error over the whole corpus,
  mapped each kind to the file of the pinned TypeScript that reports it, and
  replaced "recovered" with Decision 1's classification. The full run then
  kept the 136 passing and reported one new difference (Issue 2).
- 2026-09-30: Applied the same rule to `program_syntax/collector.rs`; added
  `tests/cases/compiler/verbatimEarlyErrorsReachTypeScript.tt`, a passthrough
  test, and the `docs/ai/tt.md` note; removed 106 accepted and 30 triaged
  units from the lists.

## Issues and resolutions

### Issue 1: swc recovers from real syntax errors too

- **Symptom**: `const a = 1 const b = 2;` next to a `match` no longer reported
  `source-not-typescript`.
- **Cause**: swc recovers from a missing semicolon (TS1005, which TypeScript's
  parser reports), so "recovered" is not "an early error".
- **Resolution**: Decision 1's classification by error kind.

### Issue 2: `import { await as _await }` in a module

- **Symptom**: `conformance/externalModules/topLevelAwait.1.ts#index.ts`, which
  had been triaged for `throw await` across a line break, now failed with
  "`await` cannot be used as an identifier in an async context" at the import
  specifier.
- **Cause**: The first error of the unit moved: the `throw await` error is
  gone with the module goal handled, and swc reports `InvalidIdentInAsync` for
  an import specifier's imported name, which is a `ModuleExportName`, not a
  binding (ECMA-262 §16.2.2). The kind stays a parse failure because
  TypeScript's parser shares TS1359.
- **Resolution**: Triaged under TASK-641 with the new reason.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/verbatimEarlyErrorsReachTypeScript.tt`
  (`cargo test --test case_baselines`), and
  `tests/passthrough.rs::rules_typescript_checks_after_parsing_are_left_to_typescript`.
- **Observed failure**: Without the change, the case reports `modified
  baseline: .../verbatimEarlyErrorsReachTypeScript.map.txt is out of date`
  (the projection rejects `shapes.tt` with `source-not-typescript` for the
  parameter property, so no lowering exists), and the passthrough test panics
  with "compile failed: ... generated TypeScript failed to parse: `await`
  cannot be used as an identifier in an async context".

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings` (in the final gate, recorded
  in TASK-643)
- [x] `cargo test` (targeted: `--lib`, `compile`, `passthrough`,
  `case_baselines`, `snapshot`, `corpus`; the full gate is recorded in
  TASK-643)
- [x] `TTC_TYPESCRIPT_CASES=all cargo test --test corpus typescript_test_cases`:
  "15383 TypeScript unit(s), 12779 parse, 54 differ (54 listed)".
- [x] Baseline changes reviewed and committed with the change: the new case's
  `.errors.txt` shows `ttc --out-dir` succeeding and TypeScript reporting
  TS2369 at `shapes.tt:3:10`.

## Result

Changed files: `src/verify.rs`, `src/program_syntax/collector.rs`,
`tests/passthrough.rs`, `tests/passthrough-accepted.txt` (147 to 41 units),
`tests/passthrough-triaged.txt` (42 to 13 units),
`tests/cases/compiler/verbatimEarlyErrorsReachTypeScript.tt` and its four
baselines, `docs/ai/tt.md`, `docs/tasks/TASK-638-typescript-case-passthrough-parity.md`,
`docs/tasks/INDEX.md`, and this record. The 41 accepted units that remain
are files TypeScript reports whose construct swc cannot parse at all; the 13
triaged units are swc parser gaps, taken up by TASK-641.
