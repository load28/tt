# TASK-770: Fix defects found by the fourth audit

- **Status**: In progress
- **Started**: 2026-10-06
- **Completed**: —
- **Commit**: —

## Purpose

A fourth audit of the CLI, the compiler, and the editor after TASK-769
reported new defects. This task fixes them in the layer that owns each,
and repeats the audit until it finds none.

## Scope

- Included: the fourth audit's CLI, compiler, and editor findings.
- Excluded: two CLI reports judged not to be defects. A `.ts` file named
  to `--check-types` is refused as "not a tt source": the typed modes take
  tt sources, and a hand-written file's diagnostics are reported through
  the project those sources open. `--check` passing on a layout that
  `-o` rejects (`x.tt` beside `x.ts`): `--check` is the tt-level check of
  the sources and takes no `-o`, so it has no output layout to judge.

## Decisions

### Decision 1: A command-line argument that is not UTF-8 is refused

- **Context**: `ttc -p "b\xff.tt"` stopped with an internal compiler error
  from `std::env::args`, exit 101.
- **Decision and rationale**: The arguments are read as `OsString`s and one
  that is not UTF-8 is refused as an invalid command line (exit 1), naming
  it, as a directory scan already refuses such a path.

### Decision 2: Each input is checked under its own project

- **Context**: `--check-types ex/src g1/src`, with a `tsconfig.json` in each
  of `ex` and `g1`, checked `ex` and dropped `g1` without a word.
- **Decision and rationale**: `--project` defaults to "the nearest one at or
  above the inputs", and one search over all inputs found a single
  configuration. Without `--project`, the typed modes now group the inputs
  by the configuration nearest to each, as a build already reads each
  file's own project (TASK-769 Decision 4), check every group, and report
  them together. A watch over several projects is refused with a message
  naming the choice, since one watch session follows one project.

### Decision 3: A named root outside the configuration gets its sidecar

- **Context**: `--types other/x.tt`, with `include: ["src"]`, checked the
  file but wrote no sidecar and reported success.
- **Decision and rationale**: A named file outside the configuration joins
  the check as a root of its own project, but declaration emit asked only
  the configured program. The host now emits for every project a requested
  module belongs to.

### Decision 4: A file that names a `.ttx` module needs the `jsx` option

- **Context**: With an unreadable `tsconfig.json`, `-p src/m.tt` where
  `m.tt` imports `./v.ttx` wrote `./v.js` and succeeded, while the `.ttx`
  file itself was refused.
- **Decision and rationale**: The option decides the spelling of every
  `.ttx` specifier, so it is needed by a `.ttx` file and by a file that
  imports one. Whether a file needs it is now read from that file (its
  extension and its tt imports) rather than from whether a `.ttx` happened
  to be among the inputs.

### Decision 5: A missing input is a command-line error in every mode

- **Context**: `--check-types missing.tt` exited 2 with an operating-system
  message, while a build exited 1 with "no such file or directory".
- **Decision and rationale**: The help says an invalid command line exits 1
  before anything runs. The typed modes now refuse a missing input (one no
  `--overlay` stands for) the way a build does. A native test that used a
  missing input to reach "could not check" now uses an input that is not a
  tt source.

### Decision 6: A source named through a file symlink is one source

- **Context**: `-o out la.tt src/x/a.tt`, `la.tt` a symlink to the second,
  wrote two outputs, while the same overlap through a directory symlink is
  refused.
- **Decision and rationale**: Source identity compared parent directories
  and file names, so two names of one file differed. Two paths that both
  exist are now the same file exactly when they canonicalize to one path.

### Decision 7: Sidecars are recorded as ttc's outputs

- **Context**: `--types -o src src` followed by `-o out src` copied
  `src/a.tt.d.ts` to the output as a hand-written declaration file.
- **Decision and rationale**: Directory scans leave out files ttc
  published, which a build marks with an ownership record. Sidecars were
  written without one. They now get the same record, so scans and the
  typed engine leave them out; overwriting a sidecar keeps today's rule.

### Decision 8: Server parameters are checked for their type

- **Context**: `"scope": "bogus"` and `"filename": 7` were accepted, and a
  `"text"` of the wrong type was reported as missing. The audit also
  reported that a buffer holding a lone surrogate (`"\ud800"`) is refused
  as malformed.
- **Decision and rationale**: `text`, `filename`, and `scope` are read
  through the typed parameter helpers, and `scope` must be one of its two
  documented values. The lone-surrogate refusal is not changed. TASK-655
  chose it: the clients replace lone surrogates with U+FFFD before writing
  (`String.prototype.toWellFormed`), and the server answers a request that
  does not decode as malformed under its id
  (`a_request_whose_parameters_do_not_decode_is_answered_under_its_id`).
  Replacing them in the server as well was tried first and dropped,
  because it reversed that contract.

### Decision 9: An operand hoisted beside a tt value keeps its syntactic role

- **Context**: A tt value is lowered into statements that run before the
  expression holding it, so the operands to its left are captured first.
  The capture took the operand's expression alone. A shorthand property
  `{ a, b: match … }` lost its key (`{ $tt_v1, b: … }`). A spread
  `[...arr, match …]` or `{ ...o, b: match … }` captured the reference,
  so the elements were read after the arm mutated them. A template
  substitution `${arr}` was converted to a string after the scrutinee ran.
  A spread argument of an optional call captured the reference too. Every
  one of these reads differently from the source.
- **Alternatives**: Exclude these operands from capture, which changes the
  evaluation order; or capture the reference and accept the drift, which
  breaks the contract that the lowering runs in source order.
- **Decision and rationale**: The projection records each operand's role
  as an `EvaluationInputMode` (`ShorthandProperty`, `SpreadElement`,
  `ObjectSpread`, `TemplateSubstitution`), and the capture does the work
  that role does at that point: it writes the key, takes the elements,
  copies the properties, or converts to a string. An array spread is taken
  by a module-local `$tt_spread` helper whose overloads keep a tuple's
  type (`[...T]`), since `const v = [...t]` widens a tuple and breaks a
  later `f(...v)` (TS2556). TypeScript reaches the same shape in its own
  emit: the ES2015 transform lowers a spread into a call to the
  `__spreadArray` helper it requests per file
  (`src/compiler/transformers/es2015.ts`, `transformAndSpreadElements`;
  `src/compiler/factory/emitHelpers.ts`, `spreadArrayHelper`).

### Decision 10: A captured name keeps its declared type

- **Context**: A callee or operand captured before a tt value became
  `const $tt_v = (name);`. That changed what TypeScript knows of it. An
  assertion call through the capture is TS2775, since the checker reads an
  assertion signature only from a name with an explicit type annotation
  (`checker.ts`, `getExplicitTypeOfSymbol` and
  `isDeclarationWithExplicitTypeAnnotation`). A `unique symbol` key read
  through a fresh `const` became another unique symbol, so `d[sym]` was
  TS7053.
- **Alternatives**: Leave entity names in place, which reads them after the
  tt value has run and breaks evaluation order; or ask the checker for an
  annotation, which an untyped build cannot do.
- **Decision and rationale**: A capture whose operand is an entity name (an
  identifier or a chain of property names, classified by SWC) is declared
  `const $tt_v: typeof name = (name);`. A type query names the reference's
  own type at that point, narrowing included, so the call or key checks
  exactly as written. The declaration then carries an explicit annotation,
  so it takes no contextual annotation. The query restates source text the
  capture also reads, so an unresolved name drew a second TS2304 there.
  Mapping the query's name to its source was tried and refused by the
  emission contract that a source byte reaches the target at most once
  (`SourceEmittedTwice`). The emission now records the query as a
  restatement (`MappedEmit::restatements`). The CLI report and the editor
  drop a diagnostic inside one, because the capture's own read reports the
  same name at its source.

### Decision 11: A callee that the call itself reads is left in place

- **Context**: `super(match …)`, `super.m(match …)`, and
  `import(x, match …)` were rejected with "cannot be lowered from this
  reference position". `eval(match …)` captured `eval`, so the call became
  an indirect eval and lost the local scope.
- **Decision and rationale**: The language fixes these callees at the call.
  `super` and `import` are not values. Whether `eval(...)` is a direct eval
  depends on the call naming `eval` (ECMAScript *PerformEval*, reached from
  a *CallExpression* whose callee is the identifier `eval`). A `super`
  member with a name or a simple-copiable key is read where the call is
  made, which matches what TypeScript does for a simple-copiable operand
  (`isSimpleCopiableExpression`). These callees are not recorded as
  inputs. Earlier arguments are still captured in order, and the call is
  rebuilt around its authored callee.

### Decision 12: A typed session releases the snapshots it no longer needs

- **Context**: Each `typedCheck` after an edit grew the TypeScript process
  by tens of megabytes. Measured over 30 edits without `skipLibCheck`, it
  grew 1133 MB, until the kernel killed it.
- **Alternatives**: Dispose every snapshot once its job is answered. This
  was tried first. A later ask in the same build then failed with "node
  handle … could not be resolved": the API carries the cached files of the
  latest snapshot forward into the next one (`api.js`, `updateSnapshot`),
  and the server no longer held them.
- **Decision and rationale**: The API keeps every snapshot until it is
  disposed (`Snapshot.dispose` sends `release`). The host now records the
  snapshots each job creates. Once the answer is written, it disposes all
  of them except the latest, which the next update builds on. A reconnect
  forgets them, since `API.close` disposes them all.

### Decision 13: A failed job starts a fresh TypeScript connection

- **Context**: After the compiler process died, every later request
  answered `EPIPE` until `reloadProjects`.
- **Decision and rationale**: A job that throws leaves the connection in
  an unknown state. The host now reconnects before answering the error,
  which is the existing path a configuration switch takes. The next
  request opens the project afresh. This matches how the VS Code
  TypeScript extension starts a new tsserver when the old one exits. The
  request that observed the failure still reports it.

### Decision 14: A tagged template's quasi is not a template of its own

- **Context**: `` tag`${x}${match …}` `` passed `"5"` instead of `5` to the
  tag. `` tag`${(x)}${match …}` `` was an internal compiler error
  (`EvaluationCountChanged`), and in a conditional branch it read a slot
  before its declaration. SWC models a tagged template's quasi as a `Tpl`
  node, so the projection opened a plain template frame inside the tag's
  call frame and recorded each substitution twice: once as a call argument
  and once as a string conversion.
- **Decision and rationale**: A `Tpl` that is the quasi of a `TaggedTpl`
  opens no frame. Its substitutions are the tag call's arguments, as
  ECMAScript evaluates them (*ArgumentListEvaluation* of a
  *TemplateLiteral*). The tag frame names each substitution by the same
  parenthesis-peeled operand span the other frames use.

### Decision 15: A nested pipeline is read from the slot its parent wrote

- **Context**: In `match ((match … |> h) ? match … : 0) { … }`, the
  scrutinee's structural emitter wrote the pipeline into its slot, and then
  the conditional operation re-emitted the pipeline from source with an
  unassigned slot for its head (`ReferenceError`).
- **Decision and rationale**: The emitter reads an already-delivered nested
  value from its slot for decisions, Result regions, and propagations.
  Pipelines (`Apply`) now get the same treatment. The emitter's operand
  collection (`collect_operand_value`) already treated them as such values.

### Decision 16: A let-else or if-let subject is a structural outer

- **Context**: `const Some(v) = c ? wrap(try r(1)) : none else …` captured
  `wrap` inside the branch and called it outside (`ReferenceError`).
  `if let Some(v) = match … ? match … : S { … }` was an internal compiler
  error.
- **Decision and rationale**: Evaluation already treats a statement
  decision's subject as an outer that owns the values in it. Two places
  did not. The nested-region boundary search now stops at a statement
  decision's subject, so a `try` in a subject branch joins a whole
  conditional operation. The codegen's set of statement-capable outers now
  includes those subjects too (`LoweringPlan::statement_decision_sources`),
  so a value the subject delivered is read from its slot rather than
  emitted again. The subject's own value is not one of those nested
  values: `if let Some(h) = k |> half` evaluates the pipeline into the
  subject temporary itself. Core wraps that value in a one-statement
  `Sequence`, so the exclusion follows such wrappers.

### Decision 17: A `try` that encloses a value bounds that value's steps

- **Context**: `match (c && (try r(result { … } |> f))) { … }` left a raw
  `try` in the output. The pipeline in the `try`'s operand and the `try`
  itself each planned the same `c && (…)` operation, one through the
  owned-children path and one through the nested-region path. The
  pipeline's copy re-emitted the `try` as source.
- **Decision and rationale**: When an owned child's steps are cut at the
  nearest enclosing structural outer, a `try` whose region is nested is
  such an outer too. The pipeline's schedule then ends inside the `try`.
  The `try`'s own operation owns the conditional step, and the pipeline is
  delivered when the `try` evaluates its operand.

### Decision 18: A claimed call frame owns the values inside it

- **Context**: In `f2(((w) => w |> h)(match …), match …)` the second value
  captured the first argument. The completed call there had claimed its
  frame (callee and parentheses), but the capture still emitted the
  pipeline inside that frame, which left `(…)(w, h)$tt_v0` (verify-failed).
- **Decision and rationale**: A capture leaves out every replacement,
  value, and statement that lies inside a completed call's claimed frame,
  when that frame lies inside the captured range. Captured parts were
  already filtered this way. A capture of the callee itself lies inside
  the frame and keeps its own values.

### Decision 19: A completed call's claim applies inside a conditional region

- **Context**: `c ? result { const z = g(match …); … } : null` called `g`
  twice. The dispatch arms performed the call, and the statement then
  applied the captured callee to the arm result again. Anchored
  replacements are suppressed inside a conditional region, so that a value
  the operation consumes is not read at its host position. The claim of a
  call completed within that region was suppressed as well.
- **Decision and rationale**: A claim whose value's compose rewrite has
  already been emitted applies at any depth. Its call has already run, and
  its frame must read the join slot.

### Decision 20: A generated operand is reported as TypeScript reports an expression

- **Context**: `"n" in match (v) { A => v, B => {} }` reported
  `'$tt_v8' is possibly 'undefined'` (TS18048), naming a generated slot.
- **Decision and rationale**: TypeScript names the operand in TS18047–18049
  only when it is an entity name. Any other expression gets TS2531–2533,
  "Object is possibly …" (`checker.ts`,
  `reportObjectPossiblyNullOrUndefinedError`). A generated name stands for
  a tt expression, so when the named operand's root is one of the file's
  generated names, the diagnostic takes that second form. The CLI report
  and the editor service share the one function
  (`semantics::unnamed_generated_operand`).

### Decision 21: An `if let` or let-else pattern is completed from its subject's type

- **Context**: At `if let Ha§(item) = o` and `const Ha§(item) = o else`,
  `patternCompletions` listed every case in scope. The same position in a
  `match (o)` listed only `o`'s cases. Picking any other case was a type
  error.
- **Decision and rationale**: A statement pattern's top level is now the
  same typed site a match arm is. Its tags come from the other
  alternatives the pattern writes. Its scrutinee is the subject after `=`,
  which `scrutinee_at` now finds for the innermost `if let` or let-else
  whose pattern holds the position. The probe keeps the tag being typed
  (or a placeholder tag when none is typed yet), because removing it would
  leave a pattern that no longer parses as the statement. A statement
  pattern offers no wildcard, since an `_` there is not a pattern the
  statement accepts.

## Work log

- 2026-10-06: Started from the fourth audit's reports (compiler, CLI,
  editor).
- 2026-10-06: Fixed the CLI findings (decisions 1–8).
- 2026-10-06: Fixed evaluation order for shorthand, spread, and template
  operands, and captures that lost a name's declared type (decisions
  9–11).
- 2026-10-06: Fixed the editor backend's snapshot leak and its recovery
  from a dead compiler (decisions 12–13).
- 2026-10-06: Fixed the compiler's internal errors and double evaluations
  in tagged templates, nested pipelines, statement subjects, `try`
  operands, and completed calls (decisions 14–19), the generated-name
  diagnostic (decision 20), and statement pattern completions (decision
  21).
- 2026-10-06: Moved the remaining findings to TASK-771: the narrowing that
  a logical condition loses when its right operand is a tt value
  (compiler), per-request cost that grows with file size and error count
  (editor E3–E5), nested patterns under a generic payload field (E6), and
  the or-pattern and unreachable-arm inconsistencies (E8–E9).

## Issues and resolutions

- **Server-side lone-surrogate replacement reversed TASK-655.** Symptom:
  `a_request_whose_parameters_do_not_decode_is_answered_under_its_id`
  failed. Cause: TASK-655 decided that clients replace lone surrogates
  and the server answers malformed under the request's id. Resolution:
  the server change was dropped (decision 8).
- **Disposing every snapshot broke later asks.** Symptom: "node handle …
  could not be resolved" in a typed build. Cause: the API carries the
  latest snapshot's cached files into the next update. Resolution: the
  latest snapshot is kept (decision 12).
- **The tagged-template quasi was recorded twice.** Symptom: the first
  version of decision 9 passed substitutions to a tag as strings.
  Cause: SWC's quasi is a `Tpl` node. Resolution: decision 14.
- **A statement subject's own pipeline was treated as nested.** Symptom:
  `if let Some(h) = k |> half` dropped `k` (`SourceOmitted`). Cause:
  decision 16's outer included the subject's own value, which Core wraps
  in a one-statement `Sequence`. Resolution: such wrappers are followed
  and the subject's value is excluded.
- **A plain pipeline was read from a slot nobody wrote.** Symptom: TS2552
  for `$tt_v1` in `try (g() |> step)`. Cause: decision 15 applied to every
  pipeline. Resolution: it applies only to a pipeline with a statement
  form, which a structural parent delivers.
- **A type query repeated an unresolved name's error.** Symptom: a second
  TS2304 "in code ttc generated". Mapping the name to its source broke
  `SourceEmittedTwice`. Resolution: the query is a restatement whose
  diagnostics are dropped (decision 10).

## Regression test (fails before the fix)

- **Path**: `tests/cli.rs` (`a_command_line_argument_that_is_not_utf8_is_refused`,
  `check_types_checks_each_input_under_its_own_project`,
  `types_writes_the_sidecar_of_a_named_file_the_configuration_leaves_out`,
  `a_tt_importing_a_ttx_needs_a_readable_jsx_option`,
  `a_missing_input_is_a_command_line_error_in_every_mode`,
  `a_source_named_twice_through_a_file_symlink_claims_two_outputs`,
  `a_build_leaves_out_the_sidecars_types_wrote_into_its_input`);
  `src/server.rs` unit tests for wrongly typed parameters;
  `tests/native/cases_10.rs`
  (`a_typed_session_recovers_after_its_compiler_process_dies`,
  `a_typed_session_releases_the_snapshots_of_earlier_edits`);
  `tests/cases/compiler/` (`aShorthandPropertyBesideATtValueKeepsItsKey`,
  `aSpreadBesideATtValueReadsItsElementsFirst`,
  `aSpreadArgumentBesideATtValueReadsItsElementsFirst`,
  `aTemplateSubstitutionBesideATtValueConvertsFirst`,
  `aCapturedNameKeepsItsDeclaredType`, `aCalleeReadAtTheCallIsNotCaptured`,
  `aDynamicImportOptionBesideATtValueIsLowered`,
  `aTaggedTemplateBesideATtValuePassesItsSubstitutionsAsValues`,
  `aConditionHoldingAPipelinedMatchIsReadFromItsSlot`,
  `aStatementSubjectConditionalLowersAsOneOperation`,
  `aConditionalOfPipedResultBlocksEvaluatesEachOnce`,
  `aTryAroundAPipelineInALogicalOperandLowersOnce`,
  `aCompletedCallCalleeKeepsItsOwnTtValues`,
  `aCompletedCallInAConditionalResultBlockRunsOnce`,
  `aPossiblyUndefinedTtValueIsReportedAsAnExpression`);
  `tests/cases/editor/` (`possiblyUndefinedTtValueIsReportedAsAnExpression`,
  `statementPatternCompletionsFollowTheSubjectType`).
- **Observed failure**: Each was run with the source changes reverted. The
  CLI tests failed on the old exit codes and outputs. The recovery test got
  `EPIPE: broken pipe` in `backendError`, and the snapshot test measured
  1133 MB of growth over 30 edits. The compiler cases failed with changed
  runtime output (`1,2,3|a` for `1,2|a`, `{ $tt_v1, b: … }`, `r5,r5,r6`,
  `[-1,-10]` calls), internal compiler errors (`EvaluationCountChanged`,
  "match reached expression emission without a host rewrite"),
  `ReferenceError`s, verify-failed parses, TS2775/TS7053/TS2556 type errors,
  or a `match-placement` rejection. The editor cases listed every case in
  scope and reported `'$tt_v0' is possibly 'undefined'` (TS18048).

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Baseline changes reviewed and committed with the change

## Result

In progress.
