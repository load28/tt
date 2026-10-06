# TASK-768: Fix defects found by the second audit

- **Status**: In progress
- **Started**: 2026-10-06
- **Completed**: —
- **Commit**: —

## Purpose

A second audit of `ttc --server` found quick fixes that break code, hint
ranges that miss text, completions that are empty, and overlay state that
diverges between layers. Fix each in the layer that owns it.

## Scope

- Included: the field-typo fix on a shorthand binding; a selector-deferred
  value read through a JSX capture; the source walk past skipped insertion
  points; a rejected `try` placement inside a `result` block; a `try` in a
  `for` declaration initializer inside a `result` block; a conditional
  operation whose operand holds another conditional operation; an `if let`
  bound to an unparenthesized `try`.
- Excluded: to be recorded as the task proceeds.

## Decisions

### Decision 1: A field-typo fix keeps a shorthand binding's name

- **Context**: `Circle(raduis) => raduis` was fixed to `Circle(radius) =>
  raduis`, which no longer binds `raduis`.
- **Decision and rationale**: A shorthand binding is both the field name
  and the binding name, so the replacement writes the field with the
  written name as its alias (`radius: raduis`). The `help:` line renders
  the suggestion's replacement text (TASK-213 decision 2: the CLI help and
  the editor's code action are one datum), so it now shows the edit that
  keeps the binding.

### Decision 2: A captured value whose arms are deferred reads the selector

- **Context**: A function in a JSX child that held a `match`, followed by a
  sibling `match`, stopped the compiler with an internal error.
- **Decision and rationale**: A captured value whose arm values are
  deferred to a selector is emitted through the selected arm values, as the
  same value is when it is not captured.

### Decision 3: The source walk drops insertion points it has passed

- **Context**: The same input then looped without end.
- **Decision and rationale**: Every insertion stream of the source walk
  (owner slots, `for` initializer propagations, compose insertions and
  endings, loop endings) drops the items that start before the cursor at
  the top of each step, so a point the walk skipped over cannot hold it.

### Decision 4: A rejected `try` placement in a `result` block is reported

- **Context**: `while (try r())`, `for (;; k += try r())` and
  `const { a = try r() } = o` inside a `result` block stopped the compiler
  with "unscheduled expression try reached inline emission" instead of
  `try-placement`.
- **Decision and rationale**: The planner skipped the placement check for
  any value that exits a result region. That exemption holds only for a
  value whose capability is a statement region; a value in a repeated or
  conditionally evaluated position is checked, as it is in a function body.

### Decision 5: A `for` initializer `try` in a `result` block runs before the loop

- **Context**: `for (let i = try r(); ...)` inside a `result` block emitted
  the whole propagation inside the `for` header.
- **Decision and rationale**: A propagation nested in a result region now
  keeps its host owner, so the `for` initializer rule (the prelude runs
  before the loop, the header keeps the payload declaration) applies to it
  as in a function body. The prelude's failure exit is the propagation's
  own exit target, which writes the result block's storage and breaks out
  of it inside a `result` block and returns in a function body, as before.

### Decision 6: A completed conditional operation is read through its slot

- **Context**: `(c && try r()) || match ...`, `(c ? try d : 0) ? 2 : try e`
  and `(c && try r()) ?? try s` emitted the outer condition as the inner
  operation's source with its `try` removed (`$tt_v3 = c && `).
- **Decision and rationale**: The outer operation runs after the inner
  one, which has written its result slot. A capture inside a conditional
  operation already substitutes the slots of the captures made before it;
  a completed operation is one of those, so its result slot joins the
  captured set and the capture reads the operation's source span, the
  span itself included, as that slot.

### Decision 7: A value `try` may start an `if let`'s bound expression

- **Context**: `if let E(n) = try r() {` was `stray-if-let`, while the
  reference says the body opens at the first `{` after a complete bound
  expression and `(try r())` was accepted.
- **Decision and rationale**: The scan stopped at `try` as a statement
  keyword. As in the pipeline and operand scans, a `try` not followed by
  `{` is tt's value `try`, an expression prefix; `try {` is still the
  statement and still rejected.

## Work log

- 2026-10-06: Started from the second audit's report. Fixed
  `src/resolve/`, `src/analysis/`, `src/sema.rs` (Decision 1),
  `src/codegen/core/emitter/{host,source,result}.rs` (Decisions 2, 3, 5),
  and `src/evaluation_ir/{evaluation,builder}.rs`, `src/evaluation_ir.rs`
  (Decisions 4, 5), `src/codegen/core/emitter/host.rs` (Decision 6), and
  `src/parser/iflets.rs` (Decision 7). Regenerated every `unknown-field`
  diagnostics matrix baseline (`TT_MATRIX_CASES=all`) for Decision 1.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: pending
- **Observed failure**: pending

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Baseline changes reviewed and committed with the change

## Result

In progress.
