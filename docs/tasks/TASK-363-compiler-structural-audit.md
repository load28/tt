# TASK-363: Repair compiler boundary defects found by a parallel audit

- **Status**: Complete
- **Started**: 2026-09-11
- **Completed**: 2026-09-11
- **Commit**: `TASK-363: fix(compiler): repair audited boundary defects`

## Purpose

Audit independent compiler layers in parallel and repair reproducible defects at
the layer that owns each contract.

## Scope

- Included: TypeScript pass-through ownership for `match` members, distinct case
  evidence in variant resolution, and `.mts`/`.cts` host overlays in typed engine
  snapshots
- Excluded: New language syntax, diagnostic suppression, and unrelated editor or
  release behavior

## Decisions

### Decision 1: Keep each repair in its owning compiler layer

- **Context**: The audit found three independent failures in parsing, resolution,
  and project source classification.
- **Alternatives considered**: Add input-specific parser exclusions, suppress the
  false diagnostic, or special-case additional extensions at each consumer.
- **Decision and rationale**: Repair candidate ownership in the parser, normalize
  resolution evidence before scoring, and make the engine's TypeScript extension
  set the single source of truth. Each choice applies to the structural class of
  inputs rather than the observed reproducer.

### Decision 2: Delegate valid-TypeScript ownership to the host AST

- **Context**: Review found that reconstructing TypeScript member context from
  tt lexer tokens missed export-default expressions, angle-bracket assertions,
  and decorators.
- **Alternatives considered**: Add those introducers and prefixes to the token
  classifier, or let the existing TypeScript syntax substrate identify host
  declaration names.
- **Decision and rationale**: Parse a file with SWC when it contains a `match`
  candidate and record only identifier spans owned as function or method names.
  The tt parser excludes those exact spans and otherwise keeps its normal
  expression parsing. This removes the duplicated partial TypeScript grammar.

### Decision 3: Resolve mixed syntax through a host-compatible fixed point

- **Context**: Whole-file TypeScript parsing cannot provide AST ownership when
  the same file contains valid tt-only syntax.
- **Alternatives considered**: Restore token heuristics for mixed files, call the
  post-HIR program-syntax projection from the parser, or build a parser-owned
  byte-preserving projection.
- **Decision and rationale**: Bootstrap parsing records complete tt owner spans
  and delegates only positions that the existing expression-boundary model
  cannot prove to be operands. Each recursive region replaces confirmed tt
  nodes and unresolved candidates with category-safe, equal-length placeholders.
  A SWC syntax error restores only its smallest owning candidate; successful SWC
  AST name spans then prove host ownership. This terminates within the ambiguous
  candidate count and introduces no TypeScript modifier lists.

### Decision 4: Separate parser modes from omitted ancestor diagnostics

- **Context**: A nested parser `Program` keeps its source range but deliberately
  does not duplicate the TypeScript ancestor AST. A single ordinary-function
  wrapper therefore rejects valid `break`, `await`, or `yield` before reaching
  an ambiguous `match` declaration.
- **Alternatives considered**: Reconstruct host ancestors and labels with token
  heuristics, synthesize every ancestor capability in wrappers, or separate
  candidate errors from diagnostics caused by an isolated projection.
- **Decision and rationale**: Expression and statement regions are probed under
  plain, async, generator, and async-generator parser modes because those modes
  determine whether SWC can build an AST. Recoverable diagnostics restore only
  the candidate whose own span they intersect; diagnostics outside every
  candidate do not invalidate AST ownership evidence. This keeps arbitrary
  ancestor labels out of synthetic wrappers while ownership still requires an
  explicit host declaration node at the original span.

## Work log

- 2026-09-11: Fast-forwarded `main` to `fcaf2a0`, ran `./scripts/doctor`, and
  confirmed the baseline `cargo test` suite passes.
- 2026-09-11: Audited parser/codegen, semantic resolution, and engine/backend
  boundaries with three read-only task agents and reproduced one defect in each.
- 2026-09-11: Classified host-owned `match` member and function names before tt
  parsing while preserving tt matches inside methods, objects, and JSX hosts.
- 2026-09-11: Normalized each pattern position's resolution evidence to distinct
  case names before declaration scoring.
- 2026-09-11: Reused the engine's complete TypeScript extension set for project
  discovery and host-overlay classification.
- 2026-09-11: Added pass-through, compile, resolve, and native server regression
  coverage and ran the full Rust formatting, lint, test, and doctest gates.
- 2026-09-11: Reopened the task after PR #119 review identified three additional
  valid-TypeScript pass-through counterexamples in parser ownership.
- 2026-09-11: Replaced the token-context member classifier with SWC AST ownership
  and removed the duplicated class, object, JSX, and modifier grammar helpers.
- 2026-09-11: Ran the complete Rust CI gate after the review repair; formatting,
  clippy, unit, integration, snapshot, doctest, and fuzz compilation all passed.
- 2026-09-11: Reopened the task after follow-up review showed that whole-file
  host parsing loses ownership evidence as soon as a file contains tt syntax.
- 2026-09-11: Added recursive parser-region spans and complete owner spans for
  item and statement constructs used by the host-compatible projection.
- 2026-09-11: Implemented an error-guided match ownership fixed point and added
  mixed class, object, function, nested-match, and fully ambiguous regressions.
- 2026-09-11: Ran the complete Rust CI gate after the mixed-source repair;
  formatting, clippy, all tests, doctests, and fuzz compilation passed.
- 2026-09-11: Reversed the fixed point after CI measured repeated host parses at
  +24% to +94%; unresolved candidates now start as expression-safe probes and
  only declaration-position failures restore source text.
- 2026-09-11: Limited host delegation to positions not already proven as
  operands by the parser's expression-boundary model. Local comparison against
  `main` passed all performance budgets.
- 2026-09-11: Added a nested template-interpolation regression and removed the
  root-token shortcut so recursive parser regions participate independently.
- 2026-09-11: Reopened the task after review showed that isolated statement
  wrappers discard ancestor control capabilities such as loop, async, and
  generator context.
- 2026-09-11: Replaced the single recursive wrapper with capability-parametric
  expression and statement probes, then covered loop, async, and generator
  ancestors in one mixed-source regression.
- 2026-09-11: Ran the complete Rust CI gate and repeated the main-relative
  benchmark comparison; all correctness and performance gates passed.
- 2026-09-11: Reopened the task after review showed that a synthetic loop cannot
  preserve arbitrary ancestor labels for recursive statement probes.
- 2026-09-11: Restricted recovery errors to candidate restoration, removed the
  synthetic loop, and covered both labeled `break` and labeled `continue`.
- 2026-09-11: Ran the complete Rust CI gate and main-relative benchmark
  comparison; all correctness and performance budgets passed.

## Issues and resolutions

### Issue 1: TypeScript `match` members can be claimed as tt matches

- **Symptom**: A valid class or object method named `match` whose body starts with
  an arrow expression fails with `malformed-match` or is lowered as tt syntax.
- **Cause**: Match intent used a top-level `=>` in the body as ownership evidence,
  while a token helper independently approximated TypeScript member contexts.
- **Resolution**: SWC now records exact function-name and method-key spans from
  valid TypeScript ASTs. The tt parser excludes those spans from match-expression
  ownership without reproducing the surrounding TypeScript grammar.

### Issue 2: Repeated pattern occurrences bias variant ownership

- **Symptom**: Repeating a guarded case can turn another variant's exact case into
  an `unknown-case` diagnostic.
- **Cause**: Resolution scores occurrence counts instead of distinct case names.
- **Resolution**: Resolution now scores each distinct case name once per pattern
  position, regardless of guarded or or-pattern repetition.

### Issue 3: `.mts` and `.cts` overlays disappear from typed snapshots

- **Symptom**: Unsaved `.mts` and `.cts` host modules are unresolved or stale in
  typed checks while equivalent `.ts` and `.tsx` overlays work.
- **Cause**: Project discovery recognizes four TypeScript extensions but the
  shared host-overlay predicate recognizes only two.
- **Resolution**: Project discovery and the shared host-overlay predicate now use
  the same `TS_EXTENSIONS` constant, including `.mts` and `.cts`.

### Issue 4: Initial member classification captured JSX expression containers

- **Symptom**: Three existing program-syntax tests failed because tt matches in
  JSX attributes and children remained in the projected TypeScript.
- **Cause**: The first object-literal classifier treated braces after `=` and `>`
  as object literals without recognizing JSX opening-tag context.
- **Resolution**: The token classifier was removed. TypeScript and TSX source
  kinds now use their respective SWC grammar directly.

### Issue 5: Review exposed open-ended host grammar omissions

- **Symptom**: Export-default object methods, object methods after a TypeScript
  angle-bracket assertion, and decorated class methods named `match` failed with
  `malformed-match`.
- **Cause**: The token classifier depended on a closed list of expression
  introducers, JSX lookalikes, and member prefixes.
- **Resolution**: All three inputs are classified by the same host AST roles, and
  regression coverage fixes their byte-identical pass-through contract.

### Issue 6: Whole-file host parsing loses evidence in mixed files

- **Symptom**: A valid tt variant or match elsewhere in the file makes host
  methods and functions named `match` fail with `malformed-match` again.
- **Cause**: Host ownership was all-or-nothing: any SWC parse failure discarded
  every AST name span in the file.
- **Resolution**: The parser now projects each recursive source region, removes
  confirmed tt syntax, restores only probe candidates to which SWC assigns a
  syntax error, and accepts host ownership only from the converged AST. Host
  declarations remain source text while nested real tt matches become expression
  placeholders during classification.

### Issue 7: Candidate-by-candidate rejection regresses compilation latency

- **Symptom**: Remote performance CI measured 24% slower single-file compilation,
  94% slower first snapshots, and 77% slower one-file rechecks.
- **Cause**: The first fixed point parsed the whole file before bootstrap and
  reparsed it once for every genuine tt match candidate.
- **Resolution**: The parser now requests host proof only for structurally
  ambiguous positions, and every unresolved candidate starts as an
  expression-safe probe. Ordinary tt-heavy files need no host parse; mixed files
  perform one parse plus one retry per actual host declaration.

### Issue 8: Recursive probes lose ancestor control capabilities

- **Symptom**: A `break`, `await`, or `yield` before a host method named `match`
  blocks ownership discovery inside a nested tt statement body.
- **Cause**: Every recursive statement region was parsed as the body of an
  ordinary function, regardless of the capabilities supplied by its ancestor.
- **Resolution**: Recursive probes select the parser modes required to build an
  AST. Recoverable errors outside candidates no longer discard valid ownership
  evidence, so no synthetic loop context is required.

### Issue 9: Synthetic loops do not preserve ancestor labels

- **Symptom**: A labeled `break` or `continue` before a host method named
  `match` blocks ownership discovery inside a nested tt statement body.
- **Cause**: The recursive statement wrapper supplied an anonymous loop but had
  no structural representation of arbitrary labels from omitted ancestors.
- **Resolution**: Candidate restoration consumes only errors that intersect the
  candidate. Unknown-label diagnostics remain TypeScript concerns and cannot
  erase a declaration node that SWC successfully built at the candidate span.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Mixed TypeScript and tt files now converge on candidate-level ownership. Parser
spans build byte-preserving host projections, candidate-local SWC errors restore
only their smallest tt candidate, and the resulting SWC AST proves host function
and method names. Omitted ancestor diagnostics do not participate in ownership,
the token grammar fallback remains removed, and the complete Rust CI gate
passes.
