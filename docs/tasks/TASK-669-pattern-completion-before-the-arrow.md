# TASK-669: Complete an arm's pattern before its `=>` is written

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-669`

## Purpose

In a new arm with no `=>` yet, pattern completion answered wrongly:
`match (s) { Circle(|) }` (a first arm, or one followed by another arm
on the next line) offered about 1,010 globals as payload fields beside
`radius`, and a string literal being written as a pattern
(`match (x) { "|" }`, `"north" => 1, "|" }`) offered nothing, invoked or
triggered by `"`. TypeScript's twin, `case "|"`, offers the switch
expression's literals.

## Scope

- Included: `src/engine/completions.rs` (literal pattern context, the
  claimed payload site), `src/engine/language/project.rs` and
  `service.rs` (`TtCompletion::range`, finishing the items),
  `src/server.rs` (`range` in the pattern items), the adapter
  (`engine.ts`, `server.ts`: the item's `textEdit`, quote triggers at a
  pattern position), the editor case `patternCompletionBeforeArrow`, an
  extension test, the API and protocol baselines, and
  `docs/design/lsp-architecture.md`.
- Excluded: the engine's general `completion` answer at an unparsed
  payload list, which the adapter does not use at a pattern position.

## Decisions

### Decision 1: Pattern-position detection is right; the typed query at an unparsed payload list is not

- **Context**: The editor case showed `ttCompletions: pattern true` with
  `radius` at every payload marker: the parser's partial queries
  (`parser::pattern_site_at`) already find the site while the arm has no
  `=>`. The globals came from `patternCompletions`: the payload list's
  typed half asks TypeScript through the completion probe
  (`Project::field_candidates`, TASK-608), which only answers a case's
  properties when the projection destructures the payload. An arm whose
  pattern is still being written is not claimed by the parser (TASK-605
  Decision 2), so the probe landed in recovered text and TypeScript
  answered the globals of an expression position, which
  `field_candidates` accepted as field names.
- **Alternatives considered**:
  - Mend the arm in the probe with ` => ...`: TASK-605 rejected the engine
    writing arm syntax the parser owns, and a guard or later arm makes the
    mended text ambiguous.
  - Filter TypeScript's answer by shape: every global is a valid field
    name; a filter would be a guess.
- **Decision and rationale**: `Context::Field` records whether a parsed
  construct holds the payload (`claimed`: the site came from `parsed_at`,
  not the partial query), and the probe is asked only then. Otherwise
  tt's declaration table answers alone, as TASK-605 Decision 2 says an
  arm being typed is answered.

### Decision 2: A string literal starting an arm's pattern is a literal pattern position

- **Context**: `inside_text` made every string literal opaque (TASK-462),
  so the cursor between the quotes of `"|"` was never a pattern position.
  TypeScript completes `case "|"` with the switch expression's literals
  (`getStringLiteralCompletionEntries` in `services/stringCompletions.ts`
  of microsoft/TypeScript, for a string literal whose contextual type is a
  literal union), and TASK-607 already asks TypeScript that question for
  an arm slot.
- **Alternatives considered**:
  - Leave it to TypeScript through the served projection: with no `=>` the
    arm is erased by recovery and the service has no string to complete;
    with `=>` it works only because the lowering copies the literal.
  - Offer unquoted labels over the literal's contents, as TypeScript does:
    an unclosed literal (`"no|`) would stay unclosed.
- **Decision and rationale**: `Context::Literal`: a string token the cursor
  is inside (or at the end of, when unclosed) that starts an alternative
  at the top level of an arm, found with the same parse and partial
  queries as any arm site, is answered with TASK-607's literal family
  only, without tags or `_`. Each item is the literal as written in a
  pattern (`"north"`) and replaces the whole string token, carried as
  `TtCompletion::range` (LSP 3.17 `CompletionItem.textEdit`, whose range
  must contain the position). The adapter sends `textEdit` and routes a
  `"` or `'` trigger character at a pattern position to this answer
  instead of TypeScript's (TASK-631 kept each trigger to the context it is
  registered for; a quote inside a literal pattern is that context).

## Work log

- 2026-09-30: Added `tests/cases/editor/patternCompletionBeforeArrow.tt`
  from `target/probe7-editor/cases` and generated its baseline with the
  unfixed engine: the payload markers listed about 1,000 `(field)` globals
  in `patternCompletions` and `editor completion: 1010 item(s)`; the
  literal markers `ttCompletions: pattern false`, `patternCompletions:
  null`, `editor completion: 0 item(s)`.
- 2026-09-30: Implemented Decisions 1 and 2; regenerated the editor
  baselines (only this case changed) and the API and protocol baselines
  (`TtCompletion::range`, `range` in the pattern items); added the
  extension test.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/patternCompletionBeforeArrow.tt`
  (`tests/baselines/reference/editor/patternCompletionBeforeArrow.baseline`);
  `editors/vscode/server/src/test/server.test.ts`, "a literal pattern
  written in quotes completes the scrutinee's literals, replacing the
  literal".
- **Observed failure**: Without the engine change the case's baseline had
  `editor completion: 1010 item(s)` at `payloadFirstArm` and
  `payloadFirstArmMultiline` (globals such as `PerformanceServerTiming`
  as `Field`), and `editor completion: 0 item(s)` at `literalFirstArm`
  and `literalSecondArm`; the committed baseline has `radius` alone and
  the three literals, so the unfixed run reports a modified baseline.

## Verification

- [x] `UPDATE_EXPECT=1 cargo test --test editor_cases`: only the new case
  changed; `cargo test --lib engine::`: 104 passed.
- [x] `UPDATE_EXPECT=1 cargo test --test public_api`: the two reviewed
  lines.
- [x] Extension: the new test and the two trigger tests pass.
- [x] Baseline changes reviewed and committed with the change.

## Result

Changed files: `src/engine/completions.rs`,
`src/engine/language/project.rs`, `src/engine/language/service.rs`,
`src/server.rs`, `editors/vscode/server/src/engine.ts`,
`editors/vscode/server/src/server.ts`,
`editors/vscode/server/src/test/server.test.ts`, `tests/editor_cases.rs`,
`tests/cases/editor/patternCompletionBeforeArrow.tt` and its baseline,
`tests/baselines/reference/api/{ttc.api.txt,server-protocol.txt}`,
`docs/design/lsp-architecture.md`, `docs/tasks/INDEX.md`, and this record.
