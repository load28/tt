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
  bound to an unparenthesized `try`; a `try` in a match scrutinee or a
  pipeline head inside a `result` block; a `try` in a function written in
  an isolated value region inside a `result` block; a CommonJS module's
  storage annotations naming the ECMAScript-syntax standard library; a
  TypeScript older than the API ttc drives.
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

### Decision 8: A scrutinee and a pipeline head belong to the enclosing scope

- **Context**: Inside a `result` block, `match (try r()) { ... }` and
  `(try n()) |> f` were `try-crosses-value-region`, while in a function
  body the same `try` returns from the function. The design table
  (`docs/design/try-result-scopes.md` §4.6) listed the scrutinee as an
  isolated value region.
- **Alternatives considered**: Document the restriction; treat the
  scrutinee and the head as the enclosing scope's code.
- **Decision and rationale**: A region is isolated because it owns its
  value exits: an arm's `return` yields the arm value, and a step runs
  inside the pipeline's lowering. The scrutinee and the head run before
  any arm or step, unconditionally, exactly as they do in a function body,
  where the backend already lowers their `try` in the enclosing scope. The
  checker now visits them in the enclosing place; the lowering needed no
  change. The reference and the design table state the rule.

### Decision 9: A `try` in a nested function does not cross a value region

- **Context**: `m |> (x => try x)`, `match (n) { _ => (() => try n)() }`
  and `` `${(() => try n)()}` `` inside a `result` block were
  `try-crosses-value-region`, though the `try` targets the arrow written
  in the region (§4.6: a function nested inside `result` owns its `try`).
- **Decision and rationale**: The value `try` check judged only the place,
  not the target. The checker now keeps the stack of enclosing `result`
  blocks and asks the file's function targets for the innermost function
  around the `try`; it crosses the region only when that function opens
  before the nearest `result` block (or there is none). The statement
  `try` already made the same judgment through `in_function`.

### Decision 10: A CommonJS module's annotations name `tt/cjs/`

- **Context**: A storage annotation the checker printed as an import type
  (`import("@tt/std").TOption<number>`) was rewritten to `./tt/index.js`
  in a CommonJS module, whose other standard-library imports go to
  `./tt/cjs/` (TASK-617).
- **Decision and rationale**: The annotation is rewritten by compiling it
  as a one-line module, which has no CommonJS syntax of its own, so the
  rewrite chose the ECMAScript specifiers. The host module's emission
  already records that it is CommonJS (`MappedEmit::commonjs`); the
  annotation's rewrite now uses the specifiers that module uses, as the
  emitter does for the module's own imports.

### Decision 11: A TypeScript older than 7.1 is refused at resolution

- **Context**: With `typescript@7.0.2` (what `npm i -D typescript@7`
  installs) or the latest `@typescript/native-preview`
  (`7.0.0-dev.20260707.2`), every typed mode failed with "the TypeScript
  backend failed: api.readConfigFile is not a function" and an empty
  location. The documentation requires the 7.1 line.
- **Alternatives considered**: Detect the missing method in the host after
  it starts; check the client package's version where the toolchain is
  resolved.
- **Decision and rationale**: Both clients lack the project API the host
  opens a project through (`readConfigFile` is absent from their
  `dist/api/sync/api.js`; present in 7.1). Resolution is where "no
  TypeScript" is already reported, so an older client is reported there
  the same way: its version, the 7.1 requirement, and the one install
  instruction, and the typed layer degrades as when TypeScript is missing.
  An unreadable version is not guessed at.

## Work log

- 2026-10-06: Started from the second audit's report. Fixed
  `src/resolve/`, `src/analysis/`, `src/sema.rs` (Decision 1),
  `src/codegen/core/emitter/{host,source,result}.rs` (Decisions 2, 3, 5),
  and `src/evaluation_ir/{evaluation,builder}.rs`, `src/evaluation_ir.rs`
  (Decisions 4, 5), `src/codegen/core/emitter/host.rs` (Decision 6), and
  `src/parser/iflets.rs` (Decision 7), `src/sema.rs`,
  `src/sema/checker.rs`, `src/flow/syntax.rs`, `docs/ai/tt.md` and
  `docs/design/try-result-scopes.md` (Decisions 8, 9), and
  `src/typescript/contextual.rs` (Decision 10), and
  `src/typescript/toolchain.rs` (Decision 11). Updated
  `tests/compile/cases_08.rs`, which pinned the shorthand-breaking edit
  (Decision 1). Regenerated every `unknown-field`
  diagnostics matrix baseline (`TT_MATRIX_CASES=all`) for Decision 1.

## Issues and resolutions

### Issue 1: `--dependencies` does not list `node_modules/@tt/std`

- **Symptom**: The audit reported that a project importing `@tt/std` with
  the package present on disk did not list the package's files.
- **Cause**: The engine serves its own standard-library package at
  `node_modules/@tt/std` in the layered file system the compiler reads
  (`engine::projection::served_std_packages`), so the files on disk are
  never read and do not affect the compile; an ordinary package such as
  `node_modules/foo` is listed.
- **Resolution**: Not a defect; no change.

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
