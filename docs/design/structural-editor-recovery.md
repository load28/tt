# Structural editor recovery

Task: [TASK-759](../tasks/TASK-759-structural-editor-recovery.md).
Status: Approved by the user on 2026-10-05; implementation plan pending review.

## Objective and acceptance contract

Typing an incomplete expression, delimiter, tt construct, or JSX construct
must preserve independently identifiable declarations and editor operations
elsewhere in the current buffer. Adopt TypeScript's missing-node and
context-sensitive recovery principles within tt's local syntax model.

An independent syntax error or type error must remain visible. Recovery must
not make an invalid program pass a build, change complete-program emission,
claim valid TypeScript as tt, or weaken the TypeScript pass-through contract.
The editor always describes the current revision, not the last valid buffer.

TypeScript itself can cascade when text is grammatically ambiguous. Acceptance
does not mean one diagnostic for every arbitrary malformed file. It means no
additional cascade caused solely by tt lowering a structurally independent
construct, with TypeScript twins providing a comparison where meaningful.

## Current behavior and evidence

- `src/program_syntax/collector.rs::parse_module` propagates fatal SWC parse
  failures. Its tolerant flag accepts some reported errors only when an AST
  already exists. `src/program_syntax/projection.rs::build_with` also rejects
  delimiter imbalance before parsing the projected program.
- `vendor/swc_ecma_parser/src/parser/stmt.rs::parse_block_body` propagates an
  individual statement failure out of the statement list. Later statements
  can consequently be absent from the host model.
- `src/lib/compile.rs::compile_projection_report_parsed` recovers recognized
  tt nodes by masking and reparsing. It does not model arbitrary missing host
  syntax, and byte-preserving replacement cannot represent a zero-width hole.
- `src/content_mapper.rs` serves empty text when `report.emit` is absent;
  `src/engine/language/service.rs` also considers `report.withheld`. Consumers
  therefore do not share one explicit recovery/completeness contract.

The existing `matchAfterUnclosedJsxTag` editor baseline includes a generated
spread-type diagnostic on a match arm. It was rerun successfully during the
preceding investigation: that success confirms current behavior, not the
desired recovery. Simple `--server check` probes with three later matches
and a missing initializer, open call, or trailing member dot each reported
one file-lowering failure; they did not reproduce the user's entire cascade.
The exact user buffer remains unavailable, so acceptance is expressed through
reproducible grammatical input classes rather than a claimed exact reproduction.

Reference principles: Microsoft's [TypeScript parser](https://github.com/microsoft/typescript-go/blob/main/internal/parser/parser.go)
maintains parsing contexts, creates missing nodes/lists, and resumes or returns
to an enclosing context according to grammatical list elements and terminators.
This is an architectural reference, not a promise of identical internal ASTs.

## Chosen architecture

### 1. Parsing owns recovery facts

Keep SWC as the local TypeScript syntax substrate and TypeScript under
`src/typescript/` as the semantic backend. Add an explicit editor-recovery mode
to the vendored parser; strict parsing remains the default for compilation and
verification. Extend its existing invalid-node representation where needed,
with recovery metadata that distinguishes a missing token from consumed bad
input. Do not substitute a TypeScript service request for local host parsing.

Recovery records carry a parser-local cause identity, expected syntactic
category/token, original span (including zero-width insertion positions),
grammatical context, and the affected AST owner. Parser checkpoints must roll
these records back with speculative parsing. The host adapter converts spans
to tt byte coordinates and joins owners to tt's existing source/owner model.

Lists synchronize through their grammar: module/block statements, parameters
and arguments, object/array members, type lists, JSX attributes/children, and
tt arm/case lists. A missing expression creates a missing node without eating
the next independent element. A token belonging to an enclosing context
returns control there. Every recovery iteration consumes input, exits the
context, or records a missing element and advances parser state; the same
position/context must never loop indefinitely.

Delimiter validation becomes recovery input in editor mode rather than an
early whole-file exit. Strings, comments, templates, regular expressions, and
JSX retain their lexical contexts. An unterminated lexical region must not be
split on guessed newlines. If its following boundary is unknowable, preserve
all proven structure and report that region as unknown.

The tt parser retains authority over claiming tt syntax. Extend its recovery
records with the same ownership principles after commitment to a tt construct;
do not reinterpret ambiguous valid TypeScript identifiers as tt keywords.

### 2. Host structure and projection retain known regions

`ProgramSyntax` returns the partial host model plus recovery provenance for
editor callers. Owners distinguish complete structure, recovered structure
with a known placement, and unknown placement. `evaluation_ir` and codegen
must not infer evaluation order or control-flow boundaries from a missing
parent. Lower independent complete owners normally; a dependent unknown
owner receives a category-correct editor representation tied to its cause.

Use existing source maps/anchors for copied code and generated glue. Add
explicit mapping support for recovery insertions, including zero-width holes;
do not force every repair into a same-width string or globally insert `any`.
If an unavailable value requires an editor error-type representation, its
provenance and propagation are limited to that value and its dependents.
Preserve complete declarations and the known shape of partial declarations.

The editor projection is a first-class result shared by the engine service
and content mapper. A source error alone must not turn the whole file into an
empty module. Genuine internal failures remain explicit failures, not recovery
successes. Keep build emission and editor projection distinguishable so no
placeholder projection can escape as runnable output or a successful build.

### 3. Diagnostics follow causes rather than broad suppression

Carry recovery provenance through owners and emitted mappings. Keep the primary
syntax cause and independent diagnostics. A downstream diagnostic may be
treated as a consequence only when its derivation is established by that
recovery ownership/dependency; spatial proximity alone is insufficient.
Do not suppress all diagnostics in a function, all generated diagnostics, a
particular TypeScript code, or the entire file because syntax is incomplete.

When original malformed host text remains in the projection, TypeScript owns
its syntax diagnostics. When projection must replace that text, the parser
must preserve a source diagnostic so the cause does not disappear. Emit one
primary report per actual cause across LSP and content-mapper consumers, using
the existing layer-merging contract where possible. An actual lowering defect
on complete source must still surface.

Retain version checks and the existing validation delay. Timing changes and
last-successful snapshots cannot satisfy the structural acceptance contract.

## Alternatives and rationale

Keeping only the last successful projection would answer against stale scopes
and positions. Extending diagnostic filters would hide symptoms without
restoring declarations and completion. Reparsing arbitrary source substrings
would lose parent/evaluation context. Replacing SWC with TypeScript would move
syntax ownership across an explicit architectural boundary and require a much
larger host-model migration. Parser-owned partial structure addresses the
cause at the responsible layer and fits the current separation of concerns.

## Regression coverage within the repository

### Editor cases are the primary observable contract

Add focused cases under `tests/cases/editor/` with `@diagnostics`, hover,
completion, and definition markers before and after the damaged region. Use
`@filename` units for an exported provider and its untouched consumer. Add
same-stem `.ts`/`.tsx` twins with equivalent complete declarations and the same
malformed host syntax. Include minimal JSX declarations so missing React/JSX
types do not obscure the recovery being tested.

Required families: missing initializer; trailing member dot; missing call/list
delimiter; missing block delimiter; incomplete match arm/variant member;
unclosed JSX tag/attribute/expression; nested tt inside those host contexts.
Put several complete tt constructs after the edit and an unrelated deliberate
type error before or after it. The assertions must prove that those constructs
remain usable and that the independent error remains. Include non-ASCII source
before the edit to exercise byte/UTF-16 mapping.

Use `tests/editor_cases.rs` and `tests/baselines/reference/editor/`; preserve
its engine/server equality, LSP publication, and direct `tsgo --lsp` twin
comparison. A newly generated baseline alone is not a correctness oracle.
Require expected marker answers and a failing pre-fix comparison; do not add
new parity-ignore or known-difference entries to accept the recovery defect.

### Edit sequences and real mapper integration complement static cases

Static editor cases do not express a sequence of document revisions. Extend
`tests/incremental.rs` with deterministic complete -> delete -> type -> repair
sequences, reusing its workspace observation helpers. Compare edited and fresh
workspaces at each selected intermediate state, not only after repair. Also
assert the expected valid-sibling answers: two equally broken workspaces must
not count as success. Run these targeted regressions unconditionally rather
than relying on randomized sampling.

Use `tests/content_mapper.rs` for the actual TypeScript/mapper process boundary
and multi-file declaration preservation. Use
`editors/vscode/server/src/test/server.test.ts` only for behavior requiring LSP
document changes/publication, such as a stale diagnostic arriving after the
buffer has been repaired. Do not duplicate static case coverage there.

### Strict compilation and parser invariants remain protected

Add compiler cases under `tests/cases/compiler/` to prove the malformed
programs still fail normal compilation. Keep TypeScript pass-through coverage
in `tests/passthrough.rs`. Parser-level tests cover forward progress, nested
context synchronization, speculative rollback, and missing-node spans where
those invariants cannot be observed directly through an editor case.

Run at least one new behavioral regression against unchanged production code
and record the actual failure in TASK-759 before implementation. Baselines
must be generated with the existing `UPDATE_EXPECT=1` commands, read in full,
and committed with their causal code changes. Build the extension and use
`TT_REQUIRE_EXTENSION=1` and `TTC_REQUIRE_TSGO=1` so required integration tests
cannot silently skip. Run targeted suites while iterating, then `./scripts/ci`
with at least `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
and `cargo test` passing. Record unavailable optional corpora explicitly.

## Delivery boundaries

Implement in dependency order: parser recovery and provenance; host owner and
projection integration; consumer/diagnostic integration. Each stage has its
own failing behavioral test and must preserve the strict path. Complete-program
output baselines and evaluation-order runtime tests must remain unchanged.
Update the architecture and editor documentation to describe the final
contracts, including any observable diagnostic change. Keep the pinned Rust
and TypeScript versions and preserve user changes.

This document approves no production change by itself. The next artifact is a
concrete implementation plan after design review; TASK-759 stays In progress.

## Implemented contract (TASK-759)

The implementation reuses existing `recovered` source spans and adds
`syntax_repairs` rather than introducing a second cause-table hierarchy.
Parser recovery records remain transactional and parse-local. Missing operands,
type arguments and closing tokens can supply grammar-owned insertions before
following statements; EOF and host-only text retain TypeScript's original
syntax verdict. Both service arrangements use the same current projection.

LSP restatement carries source-position exceptions for repaired primary causes.
The new recovery twins agree on completion, hover and definition. Missing-call
syntax intentionally retains tt's original unmatched-delimiter cause rather
than TypeScript's subsequent argument-expression wording. The parity inventory
records this diagnostic difference; no parity-ignore rule hides it. Other new
TypeScript twins agree on diagnostic positions, including zero-width holes.
