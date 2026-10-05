# TASK-759: Preserve editor structure during incomplete syntax

- **Status**: Complete
- **Started**: 2026-10-05
- **Completed**: 2026-10-05
- **Commit**: `8df92671` (implementation); final corrections in the completion commit

## Purpose

Prevent incomplete syntax during typing from invalidating independent tt/ttx
constructs. Adopt TypeScript's structural recovery principles within tt's
existing syntax substrate and preserve the repository's testing contracts.

## Scope

- Included: Design, parser-owned recovery, editor projection continuity,
  causal diagnostics, and regression coverage through existing test runners.
- Excluded: New language syntax, relaxed build acceptance, debounce tuning,
  toolchain upgrades, and unrelated refactoring.

## Decisions

### Decision 1: Recover within the existing syntax ownership boundary

- **Context**: `program_syntax` uses the in-process SWC AST to establish host
  ownership and evaluation structure. A fatal parse error ends this model.
- **Alternatives considered**: Delay diagnostics; retain stale successful
  output; replace the syntax substrate with a TypeScript service call; add
  parser-owned missing/error nodes and grammatical synchronization.
- **Decision and rationale**: Adopt the last option. It addresses the
  structural failure without moving syntax ownership into the type backend.
  See [the design](../design/structural-editor-recovery.md).

### Decision 2: Reuse existing regression and parity infrastructure

- **Context**: The user explicitly requires repository-aligned tests.
- **Alternatives considered**: A new standalone typing harness or the existing
  editor cases, incremental tests, content mapper tests, and compiler cases.
- **Decision and rationale**: Extend existing suites. Editor cases compare
  engine and server answers, exercise LSP publication, and support TypeScript
  twins; deterministic edit sequences complement static baselines.

## Work log

- 2026-10-05: Ran `./scripts/doctor`; all required checks passed. Fetched
  `origin/main`; `git rev-list --left-right --count HEAD...origin/main` returned
  `0 0`. Created `codex-structural-editor-recovery` from that revision.
- 2026-10-05: Read the host parser, projection recovery, content mapper,
  service diagnostics, `CONTRIBUTING.md`, and existing editor, compiler,
  incremental, and mapper test runners. Prepared the design for review.
- 2026-10-05: Preserved the pre-existing untracked `.task-agent-disabled` file.
- 2026-10-05: Reviewed the design against the actual test runners. Static
  editor cases and deterministic incremental sequences have distinct roles;
  mapper integration uses the existing real-TypeScript process fixture.
  `./scripts/check-task-index` passed (757 records), and `git diff --check`
  passed. Rust and integration gates are pending implementation.
- 2026-10-05: The user approved the written design. Prepared
  `docs/superpowers/plans/2026-10-05-structural-editor-recovery.md` with four
  dependency-ordered stages, parser/host/projection interfaces, regression-first
  assertions, existing case and incremental runners, and full gate criteria.
  Reviewed the plan against the approved spec and the five review-focus input
  classes. Implementation remains pending plan review and execution selection.

## Issues and resolutions

### Issue 1: The default branch prefix conflicts with an existing ref

- **Symptom**: Git rejected `codex/structural-editor-recovery`.
- **Cause**: `refs/heads/codex` already exists and prevents nested branch names.
- **Resolution**: Used `codex-structural-editor-recovery` without altering the
  existing branch.

### Issue 2: One incomplete host production invalidates unrelated tt owners

- **Symptom**: Missing initializers, member names or delimiters make later tt
  constructs lose completion, navigation or their usable projection.
- **Cause**: Strict host parsing terminates the shared ownership model before
  independent statements can be lowered.
- **Resolution**: Add an explicit editor parser mode with missing nodes,
  transactional recovery records and grammatical synchronization; keep strict
  parsing for normal compilation and verification.

### Issue 3: Recovery consumes the next export or generated prelude

- **Symptom**: An open type member or expression before `export const` reports
  TS2459 in a consumer, or reports generated `let`/temporary identifiers.
- **Cause**: The unfinished production consumes a following declaration or its
  lowered statement prelude.
- **Resolution**: Use parser-owned list boundaries and missing expression/type
  insertions; split type-parameter and type-argument contexts so `<const T>`
  remains valid. Preserve source mappings across materialized editor repairs.

### Issue 4: The mapper discards an available projection

- **Symptom**: Member-access damage yields an empty mapped module while the
  engine still serves its declarations.
- **Cause**: The content mapper only selects `emit`, omitting `withheld`.
- **Resolution**: Select the same available projection in both consumers and
  apply source-aware syntax restatement to either form.

### Issue 5: Multiple syntax causes collapse during publication

- **Symptom**: Two repaired calls show one primary error; mixed raw and
  repaired syntax with the same diagnostic code loses the repaired cause.
- **Cause**: The strict lexer reports only one unmatched delimiter, and the
  LSP merger restates diagnostics by code without distinguishing occurrences.
- **Resolution**: Preserve parser-recorded independent causes and carry
  retained code/source-position pairs through engine, server and LSP layers.

### Issue 6: A missing-token range is expanded into adjacent source

- **Symptom**: Explicit zero-width diagnostics cover the next character or
  extend beyond EOF, diverging from TypeScript twins.
- **Cause**: The adapter accepts an explicit end only when strictly greater
  than the start.
- **Resolution**: Accept equal endpoints, with emoji/CRLF/EOF regression checks.

### Issue 7: Recovery can stall inside a switch clause

- **Symptom**: Mutation tests repeat the same token or EOF until terminated.
- **Cause**: Switch clauses use a statement-list entry point without the
  block/module recovery contract, and their loop assumes EOF always throws.
- **Resolution**: Share the statement-list progress contract and recognize
  EOF as an editor list terminator. Pin both minimal inputs and a switch
  editor case with a TypeScript twin.

### Issue 8: A recovered template has a value but no pending prelude

- **Symptom**: An incomplete Result/template interpolation panics during
  continued return emission.
- **Cause**: Already assigned interpolation slots leave the pending-operand
  list empty; template operand emission incorrectly treats that as no value.
- **Resolution**: Emit the ordinary template interpolation with an empty
  prelude. Keep the malformed input's original diagnostics and strict rejection.

## Regression test (fails before the fix)

- **Path**: `tests/compile.rs::editor_projection_preserves_matches_after_a_missing_initializer`.
- **Observed failure**: Against the unchanged production code at `e12014d7`,
  `cargo test --test compile editor_projection_preserves_matches_after_a_missing_initializer`
  failed at `report.emit.is_some()`: SourceNotTypeScript at byte 76, Expression
  expected. The parser regression also failed on `TS1109` at span 16..17.
  Logs: `/tmp/tt-recovery-red.log`, `/tmp/tt-recovery-parser-red.log`.

## Verification

- [x] `./scripts/doctor`
- [x] `./scripts/check-task-index`
- [x] Handwritten-file whitespace check; generated baselines retain runner-owned trailing separators
- [x] Design review
- [x] Implementation plan review and execution-method selection
- [x] Failing regression assertions observed before production changes
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`
- [x] Baseline changes reviewed and committed with the change

## Result

Implemented parser-owned recovery across host syntax, projection, mapping and
diagnostic publication. Independent tt constructs and exports remain usable
while their surrounding input is incomplete; original causes and unrelated
type errors remain visible. Strict compilation and declaration emission stay
separate from editor-only recovery. The single final review and all local
verification gates are complete.

## Implementation ledger

- 2026-10-05: Native execution approved. Required execution instructions were
  absent from the installed partial skill bundle; read the exact pinned
  upstream commit `8ca22dba9a94f28898bbce59f2537ff4d87c747d` in temporary storage.
- 2026-10-05: Baseline `cargo test` reported 418 passing library tests and one
  failure: `typescript::native::idle_tests::an_idle_host_waits_for_the_next_request_without_spending_cpu`.
  Isolated reproduction identified sandbox denial of `ps`, not a compiler
  defect. The full verification gate must run with that read-only inspection
  permitted.
- Ruling: Preserve malformed host text in the editor output when the partial
  tree can locate every tt owner. Missing AST nodes require no synthetic
  output bytes; original mappings and TypeScript syntax diagnostics remain
  authoritative. Explicit repair mappings are needed only for actual generated
  recovery text. This avoids inventing edits at zero-width AST holes; if an
  unknown owner cannot be located, it must use parser-owned recovery instead.

- 2026-10-05: Added explicit strict/editor parser modes, transactional recovery
  records, host-owned skipped-input recovery, and shared projection metadata.
  Missing initializer, trailing member, missing annotation, nested list, lexical
  region, JSX, and valid speculative-syntax parser regressions pass.
- Ruling (supersedes the earlier raw-text-only projection assumption): An open
  call followed by a generated match prelude produced TS2304 on `let` and
  `$tt_v0$a`. Materialize parser-recorded missing closing delimiters before
  lowering; restore all source coordinates and classify inserted bytes as
  glue. Keep unmodified malformed text for holes needing no insertion.
- Ruling: Reuse existing `recovered` spans and add `syntax_repairs` for repaired
  host productions instead of a second diagnostic-origin enum. These have
  separate policies: replaced text owns recovery effects; delimiter repairs
  retain primary syntax causes but do not suppress independent type errors.
  Parser-local IDs remain local to each parse; report source provenance and
  the existing content-version snapshot are the consumer contract.
- 2026-10-05: Real mapper regression initially reported duplicate tt31/TS1109.
  The mapper now defers to TypeScript only when its input still contains the
  original malformed host syntax, using the same recovery provenance as the
  engine. The real-TypeScript regression passes.
- 2026-10-05: Unicode/emoji/CRLF regression verifies every copied mapping after
  multiple zero-width insertions and confirms strict compilation still fails.
  `cargo clippy --all-targets -- -D warnings` passed. Full gates are pending.

- 2026-10-05: The initial raw open-array projection reproduced generated-name
  errors in the deterministic sequence. Added argument/array/object grammatical
  boundaries and missing-delimiter records; both engine and real mapper
  sequences now pass for tt and ttx, including JSX deletion and repair.
- 2026-10-05: Full editor cases exposed an operand diagnostic shifted to the
  enclosing match anchor in JSX. Retaining incomplete operands preserves arm
  bindings/completion; copied-source boundary mapping now crosses only generated
  whitespace and no new owner, preserving the original diagnostic token.
  Rejected whole-match masking because it removed valid arm-local completions.
  Existing arm-diagnostic and deprecated-member completion baselines are unchanged.
- 2026-10-05: `cargo test --lib` passed all 419 tests with `ps` access, including
  strict pass-through/scaling/stack invariants. Final full gate remains pending.

- 2026-10-05: Committed the coupled parser, host, mapping and editor changes as
  `8df92671`. The four plan stages share one observable projection contract and
  were validated incrementally, then committed together with their baselines.
- 2026-10-05: The single independent final review identified lost immediate
  exports after open type members, missing operands/generic arguments capturing
  the next generated prelude, mapper omission of withheld emission, and loss of
  one primary cause when two calls need repair. Fixed all four in their owning
  parser/projection/consumer layers. Unclosed lexical text remains opaque;
  ambiguous function-parameter scope is not invented by statement scanning.
- 2026-10-05: Extended parser, incremental and real-mapper regressions. Against
  archived production commit `8df92671`,
  `tests/incremental.rs::recovery_retains_immediate_exports_in_both_service_arrangements`
  failed with TS2459: `good` was declared locally but not exported. The same
  assertion now passes in engine and mapper arrangements. Log:
  `/tmp/tt759-before-review-exports-red.log`.
- 2026-10-05: Against that archived commit, the independent repaired-call
  regression in `tests/content_mapper.rs` observed one primary error instead of
  two (`/tmp/tt759-before-review-multiple-red.log`). It now passes.
- 2026-10-05: The LSP adapter expanded explicit zero-width ranges before emoji
  and at EOF. The new diagnostics unit test failed before changing `>` to `>=`
  and passes afterward (`/tmp/tt-recovery-zero-red.log`).
- 2026-10-05: A mixed raw/repaired error regression proved code-only diagnostic
  restatement removed an independent repaired primary. Added source-position
  exceptions carried from the engine through JSON-lines to LSP publication;
  the unit regression changed from failure to pass
  (`/tmp/tt-recovery-mixed-red.log`, `/tmp/tt-recovery-mixed-green.log`). Added
  `tests/cases/editor/recoveryMixedCauses.tt` for end-to-end publication.
- 2026-10-05: The first full CI run exposed strict host-only/EOF parity and
  unclaimed tt projection regressions. Restricted repair materialization to
  host lowering before following statements and retained the existing raw-tt
  rejection contract. Focused native regressions pass. Kept the existing
  unclosed-JSX hover baseline by withholding answers within replaced input.

- 2026-10-05: Full case validation found eight hover baseline differences:
  the initial replaced-input guard also bypassed valid pattern-binding fallback.
  Moved the guard into service hover so declared pattern bindings remain
  available. Reviewed all eight `.types` diffs: only synthetic
  `(property) undefined: undefined` answers disappear; real `value`/`n` binding
  types remain unchanged. Updated those expected invalid-source observations.

- 2026-10-05: Regenerated and reviewed public API/protocol baselines: the
  projection's two new metadata fields, `service_retained_syntax`, and the
  optional server `retains` response are now pinned. All three public-surface
  tests pass. The existing full gate will verify them without update mode.
- 2026-10-05: The full compiler-case baseline suite passed (315.94 seconds).
  The subsequent CLI wire-shape assertions needed the newly pinned `retains`
  field in three expected responses. Updated only those expectations; both
  focused CLI response-shape tests pass. Running the remaining integration
  binaries with `--no-fail-fast` before the final gate.
- 2026-10-05: The mutation suite exposed a missing progress boundary in SWC
  switch-clause statement lists. A process sample located repeated recovery
  in `parse_switch_stmt`; the full mutation process later exited with SIGKILL.
  Added `switch_statement_lists_advance_after_a_stray_delimiter`: unchanged
  production failed to finish within four seconds
  (`/tmp/tt-recovery-switch-red.log`). Route `parse_stmt_list_item` through the
  same recovery/progress contract as block and module lists. The new test and
  all ten parser recovery tests now finish successfully in under one second.
- 2026-10-05: A second switch invariant needed an explicit EOF list terminator;
  its focused pre-fix test also timed out. Both statement-list regressions now
  pass. Added a switch editor case whose diagnostics, hover, completion and
  definition all agree with its TypeScript twin.
- 2026-10-05: With loop termination restored, the 1,000-mutant pass completed
  and found one malformed Result/template crash. A template whose interpolation
  already has an inline slot needs no statement prelude but still delivers a
  value. Implemented that case in template operand emission instead of treating
  an empty pending-operand list as inability to emit. Recorded the minimized
  input in `fuzz/regressions/compile_any_bytes/editor-result-template-missing-operand.tt`
  and `tests/cases/compiler/editorRecoveryNestedTemplateComma.tt`. Before the
  fix, projection panicked with `returned structured value was not emitted`
  (`/tmp/tt-recovery-result-red.log`); afterward, all fuzz regressions and the
  1,000-mutant pass complete successfully in 1.27 seconds.

- 2026-10-05: Final `./scripts/ci` passed all six stages: agents, rust, npm,
  website, native and extension. This includes `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`, 420 library tests,
  11 parser recovery tests, 173 native tests and 241 extension tests (zero
  extension skips). Required TypeScript, extension and corpus integrations
  were enabled. Baseline tracking compared 5,500 observations with none unused;
  5,793 unsampled matrix baselines remain outside the normal seeded gate.
  The 1,000-input mutation sample also passed. Full log:
  `/tmp/tt-recovery-ci-complete.log`. Reviewed all baseline diffs, including
  API/protocol additions, retained zero-width ranges and removed synthetic
  hover signatures. Staged whitespace checking reports the existing runner
  format for new baseline trailing separators; handwritten files pass. Task and plan are complete.

## Changed files

- Parser recovery: `vendor/swc_ecma_parser/src/parser/{mod,macros,recovery,stmt,expr,ident,object,typescript}.rs`.
- Host and projection: `src/host_input.rs`, `src/program_syntax.rs`,
  `src/program_syntax/{collector,projection,recovery}.rs`, `src/codegen/core/mod.rs`,
  `src/lib.rs`, `src/lib/{compile,recovery}.rs`, `src/engine/projection.rs`,
  `src/codegen/core/emitter/source.rs`.
- Diagnostics and adapters: `src/typescript/mapper.rs`, `src/content_mapper.rs`,
  `src/server.rs`, `src/engine/language.rs`,
  `src/engine/language/{project,service}.rs`, `src/engine/semantics/report.rs`,
  `editors/vscode/server/src/{diagnostics,engine,lsp-projections,server}.ts`.
- Regressions: `tests/{compile,content_mapper,editor_cases,incremental,swc_editor_recovery}.rs`,
  `tests/compile/cases_05.rs`, `tests/cli/server_response_shapes.rs`, `editors/vscode/server/src/test/{diagnostics,server}.test.ts`,
  twelve `tests/cases/editor/recovery*` cases and their TypeScript twins,
  `tests/cases/compiler/editorRecoveryStillRejectsIncompleteHost.tt`,
  `tests/cases/compiler/editorRecoveryNestedTemplateComma.tt`,
  `fuzz/regressions/compile_any_bytes/editor-result-template-missing-operand.tt`, and matching
  `tests/baselines/reference/` output, diagnostic, type and mapping expectations.
- Documentation: this task and `INDEX.md`, `docs/ai/tt.md`,
  `docs/design/{compiler-architecture,structural-editor-recovery}.md`,
  `docs/superpowers/plans/2026-10-05-structural-editor-recovery.md`,
  and `editors/vscode/README.md`.
