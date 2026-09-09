# TASK-301: Add structural `is` patterns and remove match expression closures

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: See git history.

## Purpose

Implement GitHub Discussion #93's class-pattern `match` surface and make every
expression `match` lower through host-owned statements without an IIFE, an
immediately invoked callback, or a runtime expression helper.

## Scope

- Included: `is Type`, property bindings, type-only alternatives, guards,
  structural duplicate and mixture diagnostics, wildcard enforcement,
  host-owner lowering, isolated arm completion, placement diagnostics, typed
  and runtime coverage, and user-facing language documentation.
- Included: remove the existing `$tt_expr` path for all `match` expressions;
  preserve conditional reachability, repeated evaluation, evaluation order,
  references, and `this` through explicit owner protocols.
- Excluded: `is` patterns in `if let` or `let-else`, nested/rest property
  patterns, generic or `typeof` constructors, class-hierarchy reachability,
  and exhaustiveness for open class hierarchies.

## Decisions

### Decision 1: Represent class matching as a first-class pattern kind

- **Context**: Class matching has distinct syntax, runtime discrimination,
  binding syntax, and exhaustiveness rules.
- **Alternatives considered**: Reinterpret tag patterns during codegen; detect
  source strings in the backend; add an AST/HIR/Core pattern variant.
- **Decision and rationale**: Add the pattern to the parser, AST, HIR, semantic
  analysis, Core IR, and target lowering. This keeps syntax and meaning out of
  backend heuristics and applies the contract to every structurally equivalent
  input.

### Decision 2: Host protocols, not closures, own expression matches

- **Context**: IIFEs and `$tt_expr` preserve local control flow but hide it from
  the host program and violate the accepted output constraint.
- **Alternatives considered**: Keep `$tt_expr`; add a new helper; reject every
  nested expression; model each host operation as an evaluation owner.
- **Decision and rationale**: Extend the existing Evaluation IR scheduling and
  owner protocols. Conditional operators own their active region, loop tests
  evaluate the region on every iteration, and one-shot `for..of`/`for..in`
  right-hand sides evaluate before the loop. Owners that cannot admit a sound
  statement region receive a source diagnostic rather than a closure fallback.

### Decision 3: Preserve arm completion as language semantics

- **Context**: A block arm's direct `return` yields the match value, while
  nested functions keep JavaScript returns and cross-arm `break`/`continue`
  must not escape after statements are inlined.
- **Alternatives considered**: Depend on a closure boundary; rewrite every
  return token; preserve isolated completion in semantic and Core control flow.
- **Decision and rationale**: Keep direct completion and illegal crossing facts
  in the responsible flow/semantic layers, then lower leaves through the match
  continuation. Generated JavaScript control flow is not used to define tt arm
  semantics.

## Work log

- 2026-08-30: Read Discussion #93 through its latest review round and recorded
  the no-IIFE, owner-protocol, compatibility, and isolated-completion contract.
- 2026-08-30: Ran `./scripts/doctor`, fast-forwarded local `main` to
  `origin/main`, and created `task-301-is-pattern-match`.
- 2026-08-30: Added `is` pattern syntax and identity through AST, HIR,
  semantic analysis, Core IR, code generation, semantic tokens, diagnostics,
  and editor grammars.
- 2026-08-30: Removed the expression-helper path for `match`. Added explicit
  placement diagnostics for hosts without statement ownership and projection
  recovery for editor consumers.
- 2026-08-30: Extended Evaluation IR with repeated loop-test regions,
  complete conditional active-branch ownership, and slot dependencies between
  ordered sibling operations.
- 2026-08-30: Added arm-boundary control-flow checks and compile, runtime,
  mapper, syntax, and invariant regression coverage.
- 2026-08-30: Updated the language and design documentation and completed the
  Rust local CI gate.
- 2026-08-30: Reopened the task after PR review reproduced combined loop and
  conditional ownership failures, missing snapshot and documentation
  contracts, and incomplete local validation.
- 2026-08-30: Unified compose and loop rewrite selection, replacement
  registration, and nested-loop closure ordering; added full-output fixtures
  for class patterns, loop ownership, and placement diagnostics.
- 2026-08-30: Extended projection metadata and decision-subject lowering so
  matches nested in propagation calls, let-else subjects, returns, and
  optional postfix pipeline tails retain structural statement owners.
- 2026-08-30: Updated user-facing language surfaces and passed the complete
  repository CI gate plus the prerendered website production build.

## Issues and resolutions

### Issue 1: Repeated loop tests cannot be hoisted to their statement owner

- **Symptom**: Removing the expression helper would evaluate a `match` in a
  `while` or C-style `for` test only once.
- **Cause**: Evaluation IR represented repetition as a refusal but had no
  operation that could place generated statements inside the loop boundary.
- **Resolution**: Added a typed loop-test protocol. `while` and C-style `for`
  are structurally rebuilt so the match region executes before every test;
  the native `for` update remains in the header so `continue` preserves its
  behavior.

### Issue 2: Loop protocol facts leaked into unrelated `try` overlays

- **Symptom**: A repeated `try` test reported an unmapped structural span
  instead of its established located lowering diagnostic.
- **Cause**: Projected loop frames were attached to every overlay inside the
  test, although only decision overlays support the new loop operation.
- **Resolution**: Restricted loop-test protocol attachment to decision
  overlays. The existing propagation ownership rule remains unchanged.

### Issue 3: Nested short-circuit branches and later sibling values regressed

- **Symptom**: `flag && id(match ...)` and a later sibling match were rejected
  after the helper fallback was removed.
- **Cause**: The plan owned only the match value, not its complete active
  branch, and later captures still referenced source containing an already
  lowered operation.
- **Resolution**: Conditional operations now own their complete active branch
  and keep inner eager captures inside it. Later values depend on the prior
  operation's join slot instead of copying its source.

### Issue 4: Inlined arms can expose control transfers to the host

- **Symptom**: A `break` or `continue` written in an arm could target a loop
  outside the match after closure-free lowering.
- **Cause**: The old closure boundary isolated those transfers implicitly.
- **Resolution**: Semantic flow analysis now rejects only transfers that cross
  the arm boundary and keeps transfers targeting loops or switches declared
  inside the arm.

### Issue 5: Combined loop and conditional rewrites are not compositional

- **Symptom**: Conditional loop tests can ICE, nested loop tests can lose a
  closing brace, loop-side conditional operations can leave an expression
  hole, and a `for` initializer plus test can fall back to `$tt_expr`.
- **Cause**: Target rewrite selection and replacement registration are split
  across owner-wide `all` filters and duplicated compose/loop pipelines.
- **Resolution**: Select eligible values per owner, feed compose and loop
  actions through one replacement pipeline, scope replacements while emitting
  nested regions, and close loop bodies in source order. Combined conditional,
  nested, disjunctive, and initializer/test cases now share the same model.

### Issue 6: Enclosing tt placeholders hid nested match owners

- **Symptom**: `try wrap(match ...)`, a match used as a let-else subject or
  returned from its else body, and an optional postfix pipeline argument could
  reach expression emission without a structural rewrite.
- **Cause**: Whole-propagation, statement-decision, and pipeline placeholders
  hid the nested TypeScript evaluation path from the parent collector.
- **Resolution**: Added projection-only shadow paths with explicitly excluded
  synthetic protocol frames, retained real host schedules, and taught subject
  and pipeline member lowering to consume those typed paths without an IIFE.

### Issue 7: Website prerender could not bind inside the sandbox

- **Symptom**: Both Vite bundles completed, but prerender failed with
  `listen EPERM ::1`.
- **Cause**: The sandbox denied the preview server's loopback listener.
- **Resolution**: Reran the production build with permission for its local
  listener; all 37 pages prerendered successfully.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `UPDATE_EXPECT=1 cargo test --test snapshot`
- [x] `./scripts/ci`
- [x] `bun run build` in `website/`

Changed files:

- Language pipeline: `src/ast.rs`, `src/parser/matches.rs`, `src/hir/`,
  `src/resolve/mod.rs`, `src/analysis/mod.rs`, `src/sema.rs`, `src/core_ir/`,
  `src/evaluation_ir.rs`, `src/program_syntax.rs`, and `src/codegen/core.rs`.
- Public and editor surfaces: `src/lib.rs`, `src/diagnostics.rs`,
  `src/content_mapper.rs`, `src/engine/tokens.rs`, `src/probe.rs`, and
  `editors/vscode/syntaxes/`.
- Contracts and records: `tests/compile.rs`, `tests/integration.rs`,
  `tests/fixtures/emit/`, `tests/fixtures/diagnostic/`, `README.md`,
  `CHANGELOG.md`, `docs/ai/`, `docs/design/type-inference-gaps.md`,
  `website/src/content.json`, this record, and `docs/tasks/INDEX.md`.

## Result

Structural `is` patterns and closure-free match lowering now share one typed
owner protocol across ordinary expressions, conditional operations, repeated
loop tests, propagation inputs, decision subjects, and pipeline tails. Hosts
without a sound statement region receive a source diagnostic instead of an
expression closure.
