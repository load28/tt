# TASK-312: Prove mixed-source structural composition

- **Status**: Complete
- **Started**: 2026-09-01
- **Completed**: 2026-09-02
- **Commit**: —

## Purpose

Prove that `.tt`, `.ttx`, `.ts`, and `.tsx` compose across compiler-owned syntax
and host boundaries. Enumerate structural equivalence classes, execute the
resulting matrices, and repair every discovered defect in its responsible layer.

## Scope

- Included: fresh-source validation of nested `match`, `result`, `try`, nested
  patterns, and spread expression hosts
- Included: parser classification and compile regressions for match operands in
  object, array, and call spreads
- Included: a self-auditing matrix for all tt constructs, their pairwise nested
  composition, TypeScript/TSX host-position classes, and directed source-kind
  interoperability
- Included: executable examples that exercise the same mixed project through
  untyped build, typed checking, declaration sidecars, and generated TypeScript
- Excluded: changes to unrelated language constructs or suppression of
  verification and TypeScript diagnostics

## Decisions

### Decision 1: Validate with a compiler built from the current commit

- **Context**: The demo invoked `target/release/ttc`, whose artifact predated
  current `main` and TASK-311 even though `./scripts/doctor` reported an
  otherwise ready environment.
- **Alternatives considered**: Modify current source from the old binary's
  diagnostics, or build `target/debug/ttc` from HEAD and rerun every reproducer.
- **Decision and rationale**: Rebuild from HEAD before diagnosis. Four reported
  lowering failures already passed with current source, preventing duplicate or
  regressive fixes to the value-region model.

### Decision 2: Treat spread punctuation as a host operator for contextual constructs

- **Context**: The parser classified the third dot in `...match` as member
  access and therefore did not claim the match construct.
- **Alternatives considered**: Require parentheses, recognize array and object
  text shapes separately, or reuse the parser's structural spread predicate.
- **Decision and rationale**: Reuse `follows_spread_operator` at contextual
  `match` recognition, matching the existing `try` contract. Object, array, and
  call spreads now enter the same host evaluation protocol without host-specific
  branches.

### Decision 3: Define completeness by compiler-owned equivalence classes

- **Context**: TypeScript permits arbitrary nesting, so a literal Cartesian
  product of all source strings is infinite and cannot be a test obligation.
- **Alternatives considered**: Maintain a large informal demo, sample random
  strings, or enumerate the finite facts that change compiler behavior.
- **Decision and rationale**: Make source kinds, tt surfaces, directed nesting,
  and `program_syntax` host enums the matrix axes. A classifier exhaustive over
  the Rust enums makes new host classes fail compilation until the gate gains a
  representative; the existing differential corpus remains the oracle for
  TypeScript syntax that contains no tt construct.

## Work log

- 2026-09-01: Reproduced failures for disjoint nested patterns, parenthesized
  `try` operands in `result`, `match` nested in `result`, `result` nested in a
  module-level `match`, and unparenthesized `match` after spread.
- 2026-09-01: Created the task record and branch from current `main` at
  `d8520b0` after `./scripts/doctor` passed.
- 2026-09-01: Built `target/debug/ttc` from HEAD and found that parenthesized
  `try`, nested `match`/`result`, and Result region composition already passed.
- 2026-09-01: Corrected the demo's nested unit patterns from alias syntax `A`
  to constructor syntax `A()`; the existing compiler behavior was correct.
- 2026-09-01: Added a failing compile regression for object, array, and call
  spread operands, then repaired contextual match recognition in
  `src/parser/mod.rs`.
- 2026-09-01: Rebuilt the local release compiler and passed the demo's source,
  typed experiment, regression, build, and generated-TypeScript checks.
- 2026-09-01: Reopened the task for the complete mixed-source objective and
  defined the finite structural matrix in
  `docs/design/mixed-source-composition-matrix.md`.
- 2026-09-01: Added an exhaustive host-enum classifier, a 36-cell directed
  value-region nesting matrix, and a four-source-kind fixture containing all
  twelve non-self import edges.
- 2026-09-01: Added typed-project, declaration-sidecar, emitted-tree, and
  TypeScript checks for the mixed-source fixture.
- 2026-09-01: Rebuilt the release compiler and passed every script in the
  Downloads demo through its existing local repository references.
- 2026-09-01: Passed the complete local CI gate and the unabridged TypeScript
  differential corpus, then audited the final diff and fixture contents.
- 2026-09-01: Reopened completion after a requirement-by-requirement audit
  found that value-region kinds and host protocol classes were proven only on
  separate axes, not as their directed cross-product.
- 2026-09-02: Added a 1,638-cell cross-product of six standalone value kinds,
  all 36 directed value nestings, and 39 representatives covering every host
  protocol, continuation, owner, and reachability class.
- 2026-09-02: Added an exhaustive parser-surface fixture whose matcher names
  every `ast::Segment` variant, so a new compiler-owned surface cannot enter
  the language without extending the gate.
- 2026-09-02: Repaired Result/static-block ownership, nested Apply planning,
  direct structured-owner emission, concise-arrow function targeting,
  parenthesized propagation ownership, and nested schedule truncation as the
  expanded matrix exposed each responsible boundary.
- 2026-09-02: Passed the complete Rust suite, local CI, unabridged TypeScript
  corpus, and every script in the Downloads demo with a release compiler built
  from the final source.
- 2026-09-02: Reopened the task after PR review identified grouping, diagnostic
  oracle, nested-slot documentation, mixed-host fallback, concise-arrow
  boundary, and mixed-source payload questions.
- 2026-09-02: Reproduced the mixed-host Apply fallback as an internal compiler
  error and the semicolon-free arrow boundary as the wrong propagation owner.
- 2026-09-02: Made structural ownership explicit for same-host children while
  preserving nested-function host barriers, and shared one concise-arrow ASI
  boundary between flow, parsing, and TypeScript projection.
- 2026-09-02: Expanded the matrix to 1,932 cells with JSX child and safe
  unparenthesized surfaces, then pinned the exact diagnostic code of every
  rejected cell.
- 2026-09-02: The expanded oracle exposed and fixed unparenthesized `yield*`
  pipeline parsing and hidden nested-propagation `MissingHost` failures.
- 2026-09-02: Added runtime payload consumption from `.tt` to `.ttx` and
  documented why isolated Result regions never use an outer nested slot.
- 2026-09-02: Passed the complete local CI gate and the unabridged TypeScript
  corpus after the review fixes, then audited the final diff.

## Issues and resolutions

### Issue 1: Four failures came from an older local binary

- **Symptom**: Parenthesized `try` and nested `result`/`match` inputs failed with
  generated TypeScript parse errors.
- **Cause**: `target/release/ttc` was older than TASK-311 on current `main`.
- **Resolution**: Revalidated with a compiler built from HEAD and rebuilt the
  release artifact after the source fix; no duplicate code changes were made.

### Issue 2: Unit nested patterns used alias syntax

- **Symptom**: Repeated `Ok(value: A)` arms were reported as duplicate `Ok`
  arms.
- **Cause**: `field: A` binds an alias; a nested unit case is written `A()`.
- **Resolution**: Corrected the external regression input to use `A()` and
  `B()`. Existing nested-pattern coverage already passed.

### Issue 3: Spread punctuation looked like member access

- **Symptom**: `...match (...) { ... }` reached TypeScript verification
  unlowered and failed parsing, while a parenthesized form passed.
- **Cause**: Generic dotted-name detection interpreted the adjacent third dot
  as property access before contextual match recognition.
- **Resolution**: Exempt structurally recognized spread operators from the
  member-access gate for `match`, as the parser already does for `try`.

### Issue 4: Nested JSX values inherited the same outer protocol

- **Symptom**: A JSX child `match` whose scrutinee was a `result` either emitted
  the JSX source twice or reached expression emission without a host rewrite.
- **Cause**: Both an enclosing tt value and its nested value inherited the same
  JSX evaluation frame from the projected TypeScript tree.
- **Resolution**: Assign every protocol frame outside an enclosing tt overlay
  only to that enclosing value. Nested values retain only operations between
  themselves and the nearest tt ancestor.

### Issue 5: One expression-only value disabled its owner's valid rewrites

- **Symptom**: A JSX return containing a statement-form `match` and an
  expression-form pipeline reached inline match emission and panicked.
- **Cause**: Target planning required every value sharing an owner to have
  statement capability before composing any of them.
- **Resolution**: Select composition actions per value. Statement-capable
  values are rewritten, while expression-only values remain in their original
  expression positions and consume earlier slots normally.

### Issue 6: Result-owned returns did not own structured arguments

- **Symptom**: `return match ...` inside `result`, and nested Result returns,
  emitted declarations inside an expression or reused an enclosing label.
- **Cause**: The return statement appeared as a distinct TypeScript owner even
  though the Result continuation semantically owned its argument and exit.
- **Resolution**: Nest values covered by a region-owned exit argument, deliver
  exact structured return arguments through the Result continuation, and give
  nested Result regions distinct control labels.

### Issue 7: Try operand sequences used the propagation span

- **Symptom**: `try (match ...)` omitted or duplicated operand source and could
  emit an unbound region break.
- **Cause**: AST-to-HIR lowering used the span from `try` through its operand as
  the operand sequence extent. Sequence delivery also assumed every assignment
  must leave a control region.
- **Resolution**: Carry an exact operand span in both try AST forms and use it
  for HIR sequences. Sequence delivery now preserves its source frame and
  falls through after assigning, while nested structured regions retain their
  own exits.

### Issue 8: The original matrices did not prove their cross-product

- **Symptom**: The host-enum matrix exercised only `match`, while the 36-cell
  value-nesting matrix exercised only one ordinary host.
- **Cause**: Source surfaces, nesting, and host protocols were validated as
  independent axes, leaving interactions between them unproved.
- **Resolution**: Generate 42 value cases and execute each against 39 finite
  host representatives. Every one of the 1,638 cells must either emit
  parseable output or return its structurally expected placement diagnostic;
  panics and verification failures are forbidden.

### Issue 9: Result ownership stopped at class static blocks

- **Symptom**: A `result` expression inside a class static block rejected its
  direct `try` even though the Result region supplied the failure boundary.
- **Cause**: Static-block placement checking consulted only the enclosing
  TypeScript function target and ignored `Place::ResultRegion`.
- **Resolution**: Make the Result region the semantic owner before applying
  the static-block no-function diagnostic.

### Issue 10: Nested Apply values disagreed about their structural host

- **Symptom**: A pipeline headed by a nested `match` could omit grouping
  source, plan the child twice, emit only an unassigned slot, or recursively
  consume its concise-arrow rewrite.
- **Cause**: Apply projection covered only the inner placeholder, descendant
  detection treated same-owner children as separate hosts, and direct Core
  values had no opaque source frame through which to consume a compose plan.
- **Resolution**: Project the complete grouped Apply span, distinguish same-
  owner from differently hosted descendants, let a statement-capable outer
  value own its same-host children, and consume one-value compose plans at the
  direct structured owner with explicit active-region guards.

### Issue 11: Nested expression-only boundaries retained unassigned slots

- **Symptom**: A pipeline containing `match` in a parameter default emitted a
  slot that no statement region could assign; nested Result and propagation
  schedules could also replay host operations already owned by the outer
  value.
- **Cause**: Every nested region received a value slot and its complete outer
  protocol even when the direct outer host admitted expressions only or
  already owned the protocol suffix.
- **Resolution**: Substitute nested slots only under a statement-capable
  structural owner, diagnose nested matches through the outer expression
  capability, and truncate nested schedules at the nearest planned ancestor
  while retaining strictly inner operations such as a surrounding call.

### Issue 12: Parenthesized statement propagation lost tt ownership

- **Symptom**: `try (match ...)` at statement position reached the TypeScript
  parser as a `try` statement without a block.
- **Cause**: Bare statement propagation rejected every parenthesized operand
  to preserve valid class/interface members named `try`, even when the operand
  contained a fully recognized tt construct.
- **Resolution**: Admit a parenthesized statement operand only when recursive
  parsing proves that it contains compiler-owned tt syntax. Plain
  `try(x);` member shapes remain byte-preserved TypeScript.

### Issue 13: Ordinary nested functions inherited outer generator targets

- **Symptom**: `try` inside a concise-arrow pipeline step nested in a generator
  was rejected as generator propagation.
- **Cause**: Both lexical target models encoded an ordinary nested function as
  absence, so lookup skipped it and selected the outer generator or
  constructor.
- **Resolution**: Record ordinary functions as explicit lexical barriers in
  both token flow and projected AST ownership. Function targets refine only
  function-body owners, preserving parameter and initializer ownership.

### Issue 14: A structured child owner could be emitted twice

- **Symptom**: A `match` declaration inside `result` emitted its statement
  region once before the declaration and again inside the initializer.
- **Cause**: A structural parent and a later source-range walk could consume
  the same owner plan independently.
- **Resolution**: Record plan consumption at the common structured-emission
  entry, make source insertion idempotent, and leave the inline occurrence to
  consume only its assigned slot.

### Issue 15: A mixed-host Apply lost its same-host structured child

- **Symptom**: A pipeline with a `match` head and another `match` inside a
  concise-arrow step reached expression emission without a host rewrite.
- **Cause**: Any differently hosted descendant made the planner skip the whole
  Apply, even when another child belonged to the Apply's own host.
- **Resolution**: Partition descendants by region ancestry. The Apply owns
  same-host structured children, while nested-function children retain their
  independent owner rewrite. Lowering records the children it structurally
  adopts so code generation does not infer ownership from Core shape.

### Issue 16: A semicolon-free concise arrow absorbed the next propagation

- **Symptom**: A generator `try` after a semicolon-free concise arrow reported
  a conditional-operation placement reason instead of generator placement.
- **Cause**: Token target lookup, parser statement recognition, and the SWC
  projection did not share the arrow's automatic-semicolon boundary. The
  generated parenthesized placeholder reattached to the preceding expression.
- **Resolution**: Define the concise-arrow boundary once in flow syntax, use it
  to claim the following `try` as a statement, and preserve the boundary with
  a projection-only separator before its placeholder.

### Issue 17: Ungrouped `yield*` included the delegate marker in a pipeline head

- **Symptom**: `yield* value |> step` emitted `yield$tt_ap(* value, step)`.
- **Cause**: Expression-head tracking treated the `*` as the first byte of the
  delegated operand.
- **Resolution**: Classify `yield*` as one prefix host operator and start the
  pipeline head after its delegate marker. A case-test colon now terminates a
  pipeline structurally instead of being mistaken for an ungrouped ternary.

### Issue 18: Hidden nested propagation produced an internal planning error

- **Symptom**: Rejected `try (try value)` cells returned
  `lowering-plan-failed` instead of their placement diagnostic.
- **Cause**: The outer propagation projection intentionally hid the inner
  overlay, leaving the child without a separate host source span.
- **Resolution**: Treat the nearest parent propagation as the diagnostic owner
  of a hidden child. The parent reports the public placement error and the
  child no longer manufactures a missing-host invariant failure.

### Issue 19: The matrix accepted any non-verification error

- **Symptom**: A rejected cell passed when it returned `lowering-plan-failed`
  instead of `try-placement` or `match-placement`.
- **Cause**: The oracle checked only that compilation returned an error whose
  message was not generated-TypeScript verification.
- **Resolution**: Derive the expected public rule from the specification-side
  capability and host facts, then assert the first diagnostic code for every
  rejected cell. Add canonical JSX child coverage and 252 safe unparenthesized
  surfaces alongside the 1,680 canonical cells.

### Issue 20: The `.tt` to `.ttx` edge consumed only a type

- **Symptom**: The directed import existed, but the `.tt` match discarded the
  imported `.ttx` payload.
- **Cause**: The fixture proved module resolution and type availability without
  using a value exported by the `.ttx` module.
- **Resolution**: Export `readTtx` from `.ttx`, import it as a value in `.tt`,
  and pass the matched payload through it.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `cargo test --test compile` — 395 passed, including 1,932 generated
  value-region/host-protocol cells
- [x] `cargo test --test native` — 63 passed
- [x] Mixed-source directed imports, typed project, declaration sidecars, and
  emitted TypeScript tree
- [x] `./scripts/ci rust`
- [x] `./scripts/ci` — agents, Rust, npm, native backend, and extension passed
- [x] `TTC_CORPUS_FULL=1 cargo test --test corpus` — full corpus passed
- [x] Local demo: `check:tt`, `check`, `check:experiments`, `check:probes`,
  `build`, and `check:generated`

## Changed files

- Compiler model: `src/ast.rs`, `src/hir/lower.rs`, `src/evaluation_ir.rs`,
  `src/program_syntax.rs`, `src/flow/mod.rs`, `src/sema.rs`, and
  `src/codegen/core.rs`
- Parser: `src/parser/mod.rs`, `src/parser/pipes.rs`, and
  `src/parser/tries.rs`
- Executable contracts: `tests/compile.rs`, `tests/integration.rs`,
  `tests/native.rs`, and `tests/fixtures/mixed-source-matrix/`
- Design and tracking: `docs/design/mixed-source-composition-matrix.md`,
  `docs/tasks/INDEX.md`, and this task record

## Result

The repository now proves all finite compiler-owned composition axes: every tt
surface, all directed value-region pairs, every host protocol class, and every
directed import edge among `.ts`, `.tsx`, `.tt`, and `.ttx`. The 1,932-cell
cross-product and exhaustive enum/surface classifiers prevent a new structural
class from entering without an executable representative. The mixed fixture,
typed paths, full corpus, local CI, and locally linked Downloads demo all pass.
