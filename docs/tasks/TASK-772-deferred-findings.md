# TASK-772: Fix the deferred findings of earlier audits

> TASK-776 reverses the "Not changed" part of decision 2: an overlay's
> ancestor path is now a shared parent chain whose facts are folded once per
> edge, so recording it is no longer quadratic in depth.

- **Status**: Complete
- **Started**: 2026-10-06
- **Completed**: 2026-10-06
- **Commit**: —

## Purpose

Earlier audit tasks left findings for a later round. This task fixes the
ones that can be fixed in this repository, before the next audit round.

## Scope

- Included: the quadratic copy of a nested template's pieces
  (TASK-765 result); hover and definition of a nested tag under a generic
  payload (TASK-771 decision 8); completion inside a variant whose body is
  not closed, and tokens and the unused-parameter hint around a declaration
  whose name is missing (TASK-767 scope); the editor's placement and
  wording of checker errors a structured mismatch or glue explains
  (TASK-769 R3E3, TASK-767 scope).
- Excluded: tsgo's content-mapper diagnostic cost (TASK-771 decision 6),
  which is upstream.

## Decisions

### Decision 1: A deep template around a tt value does linear work

- **Context**: TASK-765 left a nested template's emission quadratic. A
  template nested 20,000 deep around one `match` took 19.0 s for `--check`
  (4,000: 0.27 s; 8,000: 1.21 s). Stack samples showed four separate costs
  that each grew with depth.
- **Decision and rationale**:
  - `Rope::append` moved every piece of the child into the parent at each
    level. Pieces are now a `VecDeque`, and the shorter side moves, so a
    rope that wraps its child level by level moves only the wrapper.
  - `FunctionTargets::new` recurses into each template interpolation and
    asked `match_owned_tokens`, which walked every expression of the file,
    once per interpolation. `HirFile::match_owned` now collects the offsets
    a match's `{` and an arm's `=>` follow once, sorted, and each token
    stream reads only the offsets inside its own extent. An offset outside
    the extent named no token of that stream before either.
  - `validate_order` asked, for each step, whether any later step is
    conditional by scanning the rest. The reverse walk now carries that
    fact.
  - `lex_region` reserved `(end - start) / 3` token slots for every region.
    A braced region (an interpolation) ends at its `}`, not at `end`, so each
    nested region reserved the rest of the file again. Only the open region
    is sized up front.
  After the change, 20,000 levels take 0.51 s and 40,000 take 1.09 s.

### Decision 2: Deep match nesting loses its cubic term; the per-overlay path stays

- **Context**: While measuring decision 1, `match` nested inside `match`
  grew cubically: depth 1,000 took 1.46 s, 2,000 took 9.8 s, and 4,000 took
  79 s. `evaluation_owner` walks a value's ancestor path and, at each
  ancestor, searched the list of decision functions on that path.
- **Decision and rationale**: The decision-function and decorated-class
  indices are collected in path order, so they are binary-searched. Depth
  4,000 now takes 23 s and 1,000 takes 0.64 s.
- **Not changed**: each overlay still records its full ancestor path
  (`record_overlay`), which is quadratic in depth. Sharing path prefixes
  would change every consumer of `FoundOverlay::parents`. Depth in the
  thousands is not written by hand; it is recorded here rather than
  restructured.

### Decision 3: A nested tag under a generic payload is answered by a typed request

- **Context**: Hover and definition on `Circle` in `Has(item: Circle(r))`
  over `Opt<Shape>` showed nothing (TASK-771 decision 8). `ttSymbol` is
  parse-only by its protocol contract, and the field's declared type `T`
  names no variant.
- **Alternatives**: Make `ttSymbol` consult the project, which would spawn
  the checker for a parse-only request on every hover; or let TypeScript's
  hover answer on the emitted `"Circle"` literal, which TypeScript does not
  resolve to a declaration.
- **Decision and rationale**: A new project-backed request, `patternSymbol`
  (`Project::pattern_symbol`), answers the same `TtSymbol` shape for a
  nested tag whose position the parse-only answer leaves empty. It asks
  TypeScript which tags the payload admits at that arm (the path of TASK-771
  decision 8), takes the one visible variant that has all of them
  (`completions::sole_owner`, shared with completion), and renders that
  variant's case as `ttSymbol` would. The VS Code server asks it only when
  `ttSymbol` is null, for hover and definition. The editor case harness
  asks it on every hover and prints it only when it answers, as the client
  shows it.

### Decision 4: A probe inside a construct that never closes is tried closed

- **Context**: Completion at `Tri(p: Poi§` in a `variant` whose body is
  never closed offered values (`Shape`, `localValue`) and no types: the
  parser recovers the whole declaration (`malformed-variant`), so the
  probe landed in recovery text.
- **Decision and rationale**: TypeScript's parser treats a missing closer
  as present after reporting it (`parseExpected`). When the probe's
  emission recovers a construct around the probe, `build_probe` closes the
  brackets that construct leaves open before the probe, right after it,
  and uses that emission when the construct then parses. When it still
  does not, the first emission stands, so nothing that answered before
  changes. An unpaired `<` is not closed: the lexer decides type arguments
  by their pair, and only a parser knows a type position.

### Decision 5: Tokens and hints around a declaration without a name already work

- **Context**: TASK-767 deferred "tokens and the unused-parameter hint
  around a declaration whose name is missing".
- **Decision and rationale**: Re-checked with `function (used, unused)`
  and a nameless `export variant { ... }` beside a named function: every
  declaration around them is tokenized and the unused parameter is hinted.
  A nameless `variant` is not tt syntax, so it passes through as
  TypeScript and draws TypeScript's errors (contract 1). Nothing changed.

### Decision 6: The editor already publishes the command line's placement and wording

- **Context**: TASK-769 (R3E3) and TASK-767 left the editor placing an
  assignability error on glue (`match (flag)`, "in code ttc generated for
  this construct") where the command line places it at the value a
  structured mismatch names (`1`, "type mismatch: expected `string`, found
  `number`").
- **Decision and rationale**: Re-checked with an editor case. The
  language service's own layer (`tsDiagnostics`) still answers in
  TypeScript's words, because the service protocol carries no structured
  mismatch. What the editor publishes is the merge of its layers
  (`editors/vscode/server/src/diagnostics.ts`, `publishedDiagnostics`), in
  which the typed pass (`typedCheck`, the command line's report) replaces
  the service's type errors (`replacesTypes`). With the default settings
  (`tt.typeDiagnostics` and `tt.typedChecks` both on) the published
  diagnostic is the command line's, at `1`, with its wording. The one
  producer the finding asked for is the typed pass. Only a user who turns
  `tt.typedChecks` off while keeping `tt.typeDiagnostics` sees the
  service's wording. Nothing changed.

## Work log

- 2026-10-06: Started from the deferred findings of TASK-765, TASK-767,
  TASK-769, and TASK-771.
- 2026-10-06: Profiled deep templates with gdb stack samples of a
  symbol-carrying release build; fixed the four costs of decision 1, then
  the cubic term found beside them (decision 2).
- 2026-10-06: Added `patternSymbol` to the engine, the JSON-lines server,
  the editor case harness, and the VS Code server (decision 3); ran
  `npm test` in `editors/vscode` (243 passed).
- 2026-10-06: Reproduced the TASK-767 items with editor cases; closed the
  probe of an unclosed construct (decision 4) and re-checked the nameless
  declaration (decision 5).
- 2026-10-06: Compared the editor's service and published diagnostics
  with `ttc --check-types` on a structured mismatch (decision 6).

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `src/lib/scaling_tests.rs`
  (`every_request_does_linear_work_in_the_nesting_depth_of_templates_around_a_match`);
  `tests/cases/editor/nestedTagHoverUnderAGenericPayload.tt`;
  `tests/cases/editor/unclosedVariantFieldTypeCompletion.tt`.
- **Observed failure**: the scaling test reported `token slots reserved for
  a braced region: 839870 units for n matches but 3279736 for 2n` with the
  lexer's sizing restored. The hover case lost both `patternSymbol` answers
  with the engine's answer stubbed out. The unclosed-variant case offered no
  `Point2` at `/*field*/` without the probe's closing step.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --no-fail-fast` with `TTC_REQUIRE_TSGO=1`: every target
  passed except the public API and server protocol baselines, which
  recorded the new `Project::pattern_symbol` and `patternSymbol` request;
  both were reviewed and accepted, and `tests/public_api.rs` now carries a
  `patternSymbol` example.
- [x] `npm test` in `editors/vscode`: 243 passed.
- [x] Baseline changes reviewed and committed with the change.
- Measurements: a template 20,000 deep around a `match` 19.0 s → 0.51 s;
  `match` nested 4,000 deep 79 s → 23 s.

## Result

Complete. Changed: `src/codegen/rope/builder.rs`, `src/codegen/rope/tests.rs`,
`src/hir/mod.rs`, `src/sema/checker.rs`, `src/core_ir/lower.rs`,
`src/evaluation_ir/validation.rs`, `src/lexer.rs`, `src/work.rs`,
`src/program_syntax.rs`, `src/engine/completions.rs`, `src/engine/names.rs`,
`src/engine/language/project.rs`, `src/engine/language/project/completion.rs`,
`src/engine/language/service.rs`, `src/server.rs`, `src/server/responses.rs`,
`editors/vscode/server/src/{engine,server}.ts`, `tests/editor_cases.rs`,
`tests/public_api.rs`, `src/lib/scaling_tests.rs`, two editor cases, and the
API baselines. The per-overlay ancestor path (decision 2) and an unpaired
`<` in a never-closed construct (decision 4) stay as recorded.
