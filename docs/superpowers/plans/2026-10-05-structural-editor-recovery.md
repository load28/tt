# Structural Editor Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve independent tt/ttx editor structure and diagnostics throughout incomplete input, using parser-owned recovery.

**Architecture:** Extend the in-process SWC syntax substrate with an explicit editor recovery mode, and join its recovery facts to tt's existing syntax owners. Produce one current-revision editor projection for the engine and content mapper, with source mappings and causal diagnostics; keep strict compilation separate.

**Tech Stack:** Rust 1.98.0, vendored SWC, TypeScript 7.1.0-dev.20260826.1, the existing Rust case runners and TypeScript VS Code server tests.

**Spec:** [Approved design](../../design/structural-editor-recovery.md).

## Global Constraints

- The editor always describes the current revision, not the last valid buffer.
- Strict parsing remains the default for compilation and verification.
- Keep SWC as the local TypeScript syntax substrate and TypeScript under `src/typescript/` as the semantic backend.
- Do not reinterpret ambiguous valid TypeScript identifiers as tt keywords.
- A source error alone must not turn the whole file into an empty module.
- Genuine internal failures remain explicit failures, not recovery successes.
- Do not suppress all diagnostics in a function, all generated diagnostics, a particular TypeScript code, or the entire file because syntax is incomplete.
- Complete-program output baselines and evaluation-order runtime tests must remain unchanged.
- Keep Rust 1.98.0 and TypeScript 7.1.0-dev.20260826.1 pinned; introduce no dependency or language feature.
- Preserve user changes, including `.task-agent-disabled`. Documentation is English. Task status stays In progress until implementation and verification finish.

## Review Focus

- An unterminated template, comment, string, or regex contains apparent statement starts: do not invent boundaries inside lexical text (Task 1).
- A TypeScript generic arrow/JSX ambiguity backtracks after attempting recovery: rollback must remove speculative recovery facts (Task 1).
- A missing closing block delimiter makes apparent siblings grammatically ambiguous: preserve proven owners without inventing a scope (Task 2).
- Multiple zero-width repairs follow non-ASCII text: diagnostic and cursor mapping must retain correct byte/UTF-16 coordinates and insertion affinity (Task 3).
- A broken exported declaration affects an untouched consumer, then is repaired while an old request is pending: preserve independent exports and reject stale publication (Task 4).

## Execution and evidence

Work on `codex-structural-editor-recovery`, with TASK-759 as the parent record.
This is one coupled architectural change, delivered in four ordered commits;
the steps below are not independent parallel workstreams. Register additional
task records only if a stage becomes separately scoped work.

Before changing production code, add the behavioral assertions in Task 1 and
record their actual failure against the current production revision in
TASK-759. A missing Rust method or a missing baseline alone is not evidence
of the bug. Keep test-only changes while checking the old implementation;
never reset user files. Commit expected-output changes with their causal code.

Build the adapter with `npm --prefix editors/vscode run compile`. Dependencies
are already configured; run `npm ci --prefix editors/vscode` only if its
dependencies are missing. Set `TT_REQUIRE_EXTENSION=1 TTC_REQUIRE_TSGO=1` on
editor and integration runs so missing prerequisites cannot masquerade as a
pass. Record actual commands, failures, repairs, skips, and changed files in
TASK-759 throughout execution.

## Task 1: Parser recovery retains independent syntax

**Files:** Create `vendor/swc_ecma_parser/src/parser/recovery.rs` and
`tests/swc_editor_recovery.rs`. Modify the vendored parser's `mod.rs`,
`stmt.rs`, `expr.rs`, `class_and_fn.rs`, `pat.rs`, `object.rs`, `typescript.rs`,
and `jsx.rs` at their existing element/list/expectation routines.
Add public API assertions to `tests/compile.rs` and initial cases
`tests/cases/editor/recoveryMissingInitializer.tt`,
`recoveryMissingInitializer.ts`, `recoveryTrailingMember.tt`, and
`recoveryTrailingMember.ts`.

**Interfaces:** In the new parser recovery module, export `RecoveryMode`
(`Strict`, `Editor`), `RecoveryId(u32)`, `RecoveryContext` (source/block
statements, parameters, arguments, object/array members, type members/arguments,
JSX attributes/children), and `RecoveryKind` (missing token, missing expression,
missing type, skipped input, unterminated lexical region). Export
`RecoveryRecord { id: RecoveryId, kind: RecoveryKind, context: RecoveryContext,
span: swc_common::Span, owner: swc_common::Span, expected: String }`.
Expose `Parser::set_recovery_mode(&mut self, mode: RecoveryMode)` and
`Parser::take_recoveries(&mut self) -> Vec<RecoveryRecord>`; retain the existing
`parse_program` and `take_errors` signatures. `Strict` is the default.
Re-export these types through the existing parser exports.

- [ ] **Establish the failing behavior using current APIs.** Add
  `editor_projection_preserves_matches_after_a_missing_initializer` in
  `tests/compile.rs`: a variant, `const broken = ;`, three complete matches,
  and an unrelated `const wrong: number = "wrong";`. Require
  `compile_projection_report(...).emit.is_some()`, a nonempty mapped projection
  containing the independent declarations, and a source error. Require
  `compile_report(...).emit.is_none()` for that same input. Run
  `cargo test --test compile editor_projection_preserves_matches_after_a_missing_initializer`;
  record the actual failing assertion, not a predicted message.
- [ ] **Pin editor observations before fixing them.** Use the initial cases
  to request diagnostics, completion of a later string's `toUpperCase`, hover,
  and definition of a later binding. Twins contain equivalent TypeScript
  declarations and the same malformed host text. Run
  `TT_REQUIRE_EXTENSION=1 TTC_REQUIRE_TSGO=1 TT_CASES=recovery cargo test --test editor_cases`.
  Inspect current answers and retain the behavioral assertions above as the
  oracle; do not bless current broken answers or introduce parity exceptions.
- [ ] **Add parser invariant tests.** In `tests/swc_editor_recovery.rs`, assert
  retained later variable declarations, a zero-width missing-node record,
  strict-mode rejection (fatal or reported syntax error), nested list return to
  its enclosing context, and termination. Include unterminated lexical regions
  and generic-arrow/JSX speculative branches; complete parses must have no
  speculative recovery records. Use root integration tests, as existing
  `tests/swc_arrow_asi.rs` does, rather than depending on vendor-only test gates.
- [ ] **Implement the exported parser contract.** Centralize grammatical
  element/terminator predicates and context-stack synchronization in
  `recovery.rs`; wire each listed grammar routine into it. Reuse invalid AST
  nodes with recovery metadata, preserving valid declaration names and fields.
  Save/restore recovery state in every checkpoint and speculative clone path.
  Require token progress, context exit, or a new expected grammar slot before
  another recovery at the same position. Never scan raw lines for keywords.
- [ ] **Verify the parser boundary.** Run
  `cargo test --test swc_editor_recovery --test swc_arrow_asi --test passthrough`.
  The end-to-end projection regression remains red until Tasks 2–3; record
  that fact explicitly. Commit the parser and its passing invariant tests
  with title `TASK-759: fix(parser): retain partial host syntax in editor mode`.

## Task 2: Join recovery facts to tt and host owners

**Files:** Create `src/program_syntax/recovery.rs`. Modify `src/host_input.rs`,
`src/program_syntax.rs`, its `collector.rs`, `projection.rs`, and `tests.rs`,
`src/ast.rs`, `src/parser/parse.rs`, `src/parser/matches/arm_list.rs`,
`src/parser/variants.rs`, `src/codegen/core/mod.rs`, and
`src/evaluation_ir/evaluation.rs`. Extend the lexer validation API in
`src/lexer/validation.rs` only to expose structured lexical/delimiter facts;
preserve the existing strict validators and their callers.

**Interfaces:** Add internal `SyntaxMode { Strict, Diagnostic, Editor }` in
`program_syntax`; replace the current `tolerant: bool` argument with this enum
where that argument flows. `Diagnostic` retains the existing tolerant behavior.
Add `HostInput::editor_parser(&self, source_kind: SourceKind) -> Parser<Lexer<'_>>`.
In the new host recovery module define `RecoveryCauseId(u32)` and
`OwnerValidity { Complete, Recovered(Vec<RecoveryCauseId>), Unknown(Vec<RecoveryCauseId>) }`.
`ProgramSyntax` retains translated recovery records and exposes
`owner_validity(&self, owner: &HostOwner) -> OwnerValidity` internally.
Source spans and IDs belong to the current syntax instance; do not cache IDs
independently of its revision. Existing public strict APIs remain unchanged.

- [ ] **Add owner-model assertions that fail before integration.** In
  `src/program_syntax/tests.rs`, test three tt owners around a missing
  initializer: the damaged owner is recovered, independent owners are complete,
  and no owner is silently assigned a fabricated parent. Test an open block
  whose enclosing scope is genuinely unknown and a known block with a missing
  child expression. Run `cargo test --lib program_syntax` and record failures.
- [ ] **Add tt recovery cases.** Create
  `tests/cases/editor/recoveryMatchArm.tt`, `recoveryVariantMember.tt`, and
  `recoveryNestedDelimiters.tt` with marker queries in retained siblings and
  matching `.ts` twins where the host syntax is comparable. Exercise incomplete
  parameter/argument, array/object/type lists in the owner tests. Keep original
  tt commitment rules, including identifiers named `match` and `variant`.
- [ ] **Implement structured host ownership.** Parse through the editor
  entry point only for `SyntaxMode::Editor`; convert recovery spans through
  projection segments and `HostOrigin`. Feed delimiter facts to that parser
  instead of returning early in editor mode. Preserve strict failures and
  current diagnostic-only parsing. Join causes to the grammatical owners
  identified by parsing, not every owner whose span intersects an error.
- [ ] **Integrate tt recovery and evaluation boundaries.** Extend tt recovery
  metadata to retain committed constructs and valid arms/members, preserving
  lexical contexts. Translate these causes into the same per-model cause
  namespace. Use `OwnerValidity` before evaluation scheduling: complete and
  structurally placed recovered owners may lower; unknown owners return an
  explicit recovery requirement to the editor projection instead of guessing
  order or turning an absent overlay into an internal error. Missing overlays
  without a parser cause remain internal errors.
- [ ] **Verify strict and partial models separately.** Run
  `cargo test --lib program_syntax` and `cargo test --test passthrough`.
  Review all replacements of `tolerant` and all evaluation entry points.
  Commit with title `TASK-759: fix(analysis): retain recovery ownership across lowering`.

## Task 3: Produce one mapped current-revision editor projection

**Files:** Modify `src/lib/compile.rs`, `src/lib/compile/recovery.rs`,
`src/lib/mapped.rs`, `src/codegen/rope.rs` and `src/codegen/rope/builder.rs`,
`src/engine/projection.rs`, `src/engine/language.rs`,
`src/engine/language/service.rs`, `src/content_mapper.rs`,
`src/typescript/content_projection.rs`, and `src/typescript/mapper.rs`.
Add mapping assertions to `tests/compile.rs` and mapper assertions to
`tests/content_mapper.rs`; use the existing public API baseline runner if
the public `ProjectionReport` representation changes.

**Interfaces:** Retain `compile_projection_report(source: &str, options: &Options) -> ProjectionReport`.
Add a crate-internal shared projection result containing `MappedEmit`,
original-source diagnostics, recovery causes, emitted recovery segments, and
`ProjectionValidity { Complete, Recovered }`; consumers obtain it through one
conversion of `ProjectionReport`. The separate failure path retains original
diagnostics and an internal-failure reason where applicable. Define
`RecoverySegment { cause: RecoveryCauseId, source_start: usize, source_end: usize,
output_start: usize, output_end: usize }`; insertion has equal source endpoints.
The cause table travels with the segments. Keep public `recovered` ranges for
compatibility while internal consumers migrate to the richer representation.

- [ ] **Add failing mapping/consumer tests.** Assert the Task 1 regression,
  zero-width insertion before a retained declaration, two repairs at one
  source point, and non-ASCII text before the repair. Original copied byte
  ranges round-trip; inserted bytes resolve to their cause and never become
  editable source. In `tests/content_mapper.rs`, a provider with a damaged
  initializer still exports a separate intact declaration to its consumer.
- [ ] **Implement structural editor projection.** Consume Task 2's partial
  model and recovery requirements. Emit complete owners normally, preserve
  known declaration shape, and represent only unavailable values/unknown owners
  with category-correct editor glue. Preserve original malformed host text
  where TypeScript can consume it safely. Never obtain edit continuity by
  deleting the whole module or reusing an older version. Recovery metadata
  carries primary diagnostics even when its original text is replaced.
- [ ] **Map recovery insertions through the existing rope/anchor pipeline.**
  Accumulate source/output positions once; retain existing cursor affinity for
  original bytes. Keep synthetic recovery ranges separate from source edits,
  rename targets, and completion edits. Require both LSP and content mapper
  mappings to resolve inserted glue to the same original cause.
- [ ] **Switch consumers to the shared conversion.** Route the engine,
  service document builder, and content mapper through the same projection
  selection. Remove their divergent handling of recoverable `emit`/`withheld`
  results; preserve real compiler failure handling. A recovered projection
  cannot authorize CLI JavaScript or declaration-file output.
- [ ] **Verify and pin the now-correct observables.** Run the focused compile,
  editor, and content mapper suites with required prerequisites. Generate
  recovery editor baselines using
  `UPDATE_EXPECT=1 TT_REQUIRE_EXTENSION=1 TTC_REQUIRE_TSGO=1 TT_CASES=recovery cargo test --test editor_cases`;
  read the complete diff and TypeScript parity output before rerunning without
  `UPDATE_EXPECT`. Commit with title
  `TASK-759: fix(editor): share mapped partial projections across consumers`.

## Task 4: Preserve causal diagnostics through editing and repair

**Files:** Modify `src/engine/language/project.rs`,
`src/engine/projection.rs`, `src/typescript/content_projection.rs`, and
`editors/vscode/server/src/diagnostics.ts` only where cause-aware publication
requires it. Extend `tests/incremental.rs`, `tests/content_mapper.rs`, and
`editors/vscode/server/src/test/server.test.ts`. Add
`tests/cases/editor/recoveryJsxTag.ttx`, `recoveryJsxAttribute.ttx`,
`recoveryJsxExpression.ttx`, `recoveryProvider.tt` and meaningful TypeScript
twins; update `matchAfterUnclosedJsxTag` only for observed causal improvements.
Add malformed build cases under `tests/cases/compiler/` and update
`docs/design/compiler-architecture.md`, `docs/ai/tt.md`, and TASK-759.

**Interfaces:** Add internal diagnostic origin classification
`RecoveryOrigin { Independent, Primary(RecoveryCauseId), Derived(RecoveryCauseId) }`
over the shared cause table and `RecoverySegment`s. Classify only with
established ownership/dependency evidence; ambiguous diagnostics stay visible.
Keep existing `publishedDiagnostics(layers: DiagnosticLayers): Diagnostic[]`
and version checks. Add `assert_recovery_sequence(source_kind: SourceKind)`
inside `tests/incremental.rs`, using its existing workspace observation helpers
and explicit marker queries; this is a test helper, not a new test protocol.

- [ ] **Add failing cause-specific assertions.** Each new JSX case includes
  minimal JSX declarations, a malformed tag/attribute/expression, later tt
  owners, and an independent deliberate type mismatch. Require a primary
  syntax cause, the independent mismatch, and intact sibling marker answers.
  Ensure a real generated-code failure on complete source remains visible.
  Run focused cases before changing diagnostic classification.
- [ ] **Implement cause-aware classification and publication.** Generated
  recovery glue may produce derived diagnostics; copied unrelated source must
  remain independent. Preserve a primary source diagnostic when a recovery
  replaced the malformed text. Use existing layer merging to avoid duplicating
  that cause; do not hide diagnostics based only on a nearby span or TS code.
  Verify the same policy through actual TypeScript content-mapper diagnostics,
  not only through the engine's postprocessing.
- [ ] **Add unconditional typing regressions.** For both source kinds, run
  complete -> missing initializer -> partial member access -> restored text,
  plus JSX delimiter deletion/repair for ttx. At each state compare the edited
  workspace with a fresh workspace, and independently assert later completion,
  hover/definition, and the deliberate type mismatch. Add a multi-file export
  sequence proving an untouched consumer keeps the intact export and updates
  after repair. LSP tests use didChange and assert repaired-version publication
  cannot be replaced by the old incomplete-version result.
- [ ] **Protect strict builds and pass-through.** Compiler cases assert
  malformed programs still fail and produce no runnable placeholder output.
  Generate their baselines with
  `UPDATE_EXPECT=1 cargo test --test case_baselines`, review all changed files,
  and rerun without updating. Run existing runtime evaluation-order and
  pass-through coverage; an unrelated baseline change blocks acceptance.
- [ ] **Run the full gate and update documentation.** Run `./scripts/ci`,
  including successful `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, and `cargo test`. Run the new
  unconditional incremental and mapper regressions and built LSP adapter tests;
  no required integration skip is acceptable. Record optional corpus skips
  distinctly. Document the final ownership/projection contracts and editor
  diagnostics, remove obsolete descriptions, and mark TASK-759 and its index
  Complete only when all implementation and verification work is complete.
- [ ] **Commit and review the whole branch.** Commit the causal changes and
  their reviewed baselines with title
  `TASK-759: fix(editor): contain recovery diagnostics through edit sequences`.
  Review the full branch for strict/editor mode leakage, fabricated scope,
  lost declarations, overly broad suppression, and stale-coordinate answers.

## Plan self-review

The four tasks cover the approved spec in dependency order. Required input
families are assigned to parser/owner tests and observable editor cases;
lexical ambiguity, rollback, unknown scopes, Unicode insertion mapping, and
cross-file stale publication each have an explicit owning task. The shared
cause namespace and revision ownership connect parser facts, projection,
mapping, and diagnostics. Strict behavior and user TypeScript errors remain
separate acceptance conditions. No implementation or test pass is claimed by
this plan; execution requires plan review and an execution-method selection.
