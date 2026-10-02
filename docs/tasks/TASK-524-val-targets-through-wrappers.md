# TASK-524: Read a `val` write target through TypeScript's wrappers

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

`val const cfg = { a: 1 };` followed by `(cfg as { a: number }).a = 4;`,
`(cfg satisfies { a: number }).a = 5;`, or `(<{ a: number }>cfg).a = 6;`
compiled without a diagnostic, while `cfg!.a = 3` was reported.
`docs/ai/tt.md` says `val` errors on a val-rooted path at any depth. The
method-call probe (`src/val/calls.rs`) already read a receiver through these
wrappers; the write-target grammar (`src/val/targets.rs`) read only
parentheses and `!`.

## Scope

- Included: the write targets `val` recognizes — assignments (every
  operator), updates, `delete`, destructuring entries, and `for`-in/of
  heads — which all go through `targets::reference`, and the method-call
  probe, which now shares its path definition.
- Excluded: a path through an operator that is not a wrapper, such as
  `(0, cfg).a = 1` or `(c ? cfg : other).a = 1`.

## Decisions

### Decision 1: One definition of an access path, and the tokens only delimit it

- **Context**: Two walks answered "which binding is this path rooted at?"
  differently. The method-call probe reads the emitted TypeScript's syntax
  tree and looks through parentheses, `!`, `as`, `satisfies`, `<T>`, and
  `as const` (`unwrapped`). The write walk runs over the tt token stream,
  before emission, for the untyped check and the delegated probes alike,
  and had its own token grammar for a parenthesized target that knew only
  parentheses and `!`.
- **Alternatives considered**: (a) Teach the token grammar `as`,
  `satisfies`, and `<T>`: a second definition of the same wrappers, and a
  type after `as` has to be delimited by hand (`(cfg as any, other).a`
  roots at `other`). (b) Collect writes from the emitted syntax tree, as
  method calls are: the untyped check runs before emission, including in
  files whose other diagnostics block emission. (c) Keep the token stream
  for delimiting a reference (an identifier or a parenthesized operand, then
  member steps and `!`) and read any reference that contains a wrapper with
  SWC, through the shared definition.
- **Decision and rationale**: (c). `src/val/reference.rs` holds `unwrapped`
  and `access_path` (the root identifier and the member-step count, with
  wrappers read through at every depth); `calls.rs` and `targets.rs` both
  use it. A plain `x.a[b]` path has no wrapper and is counted from the
  tokens. `check_all` and `probes` take the source kind, because `<T>x` is
  a type assertion only outside TSX. The walk runs only in files that use
  `val`.

### Decision 2: A parenthesis after a complete operand opens arguments

- **Context**: The token grammar tried every `(` as the start of a
  parenthesized target, so `f(cfg).y = 1` reported a mutation through
  `cfg`, although it writes to what `f` returns (found while restructuring
  the grammar; `val-pass` still reports passing `cfg` to a mutable
  parameter).
- **Decision and rationale**: A `(` starts a reference only where the token
  before it does not end an expression (`TokenFacts::ends_expression`,
  TASK-491); after a complete operand it is a call's argument list.

## Work log

- 2026-09-29: Reproduced the three silent casts and the `f(cfg).y = 1`
  false positive.
- 2026-09-29: Added `src/val/reference.rs`, moved `unwrapped` and the root
  walk out of `src/val/calls.rs`, rewrote `targets::reference` over it, and
  threaded the source kind through `val::check_all`, `val::probes`, and the
  checker (`src/val.rs`, `src/val/checker.rs`, `src/lib/compile.rs`,
  `src/lib/mapped.rs`).
- 2026-09-29: Added `val_sees_a_mutation_through_every_typescript_wrapper`
  (assignment, update, `delete`, array and object destructuring, and a
  `for`-of head through each wrapper; both the reported diagnostic and the
  delegated probe) and `a_write_to_a_call_result_is_not_a_write_to_its_argument`
  (`tests/compile/cases_14.rs`). Updated `docs/ai/tt.md`.
- 2026-09-29: The full suite failed the `val-mutation` diagnostic fixture
  (Issue 1). Regenerated it with `UPDATE_EXPECT=1 cargo test --test snapshot`
  and read the diff: one added diagnostic, at `state` in
  `delete (state as { count?: number }).count;`, in both the rendered and
  the wire form.

## Issues and resolutions

### Issue 1: The `val-mutation` fixture pinned the missed diagnostic

- **Symptom**: `rendered_diagnostics_match_their_fixture` and
  `the_wire_format_matches_its_fixture` reported
  `tests/fixtures/diagnostic/val-mutation` out of date.
- **Cause**: The fixture's input already ends with
  `delete (state as { count?: number }).count;`, and its expected output had
  no diagnostic for it — the bug this task fixes. No task record describes
  the omission as intended.
- **Resolution**: Regenerated the fixture; the only change is the new
  diagnostic at 5:11.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/val.rs`, `src/val/reference.rs`, `src/val/calls.rs`,
`src/val/targets.rs`, `src/val/checker.rs`, `src/lib/compile.rs`,
`src/lib/mapped.rs`, `tests/compile/cases_14.rs`,
`tests/fixtures/diagnostic/val-mutation/`, and `docs/ai/tt.md`.
