# TASK-281: Record the ratified `try`/`result` scopes consensus

- **Status**: Complete
- **Started**: 2026-08-29
- **Completed**: 2026-08-29
- **Commit**: —

## Purpose

`docs/design/try-result-scopes.md` was an unreviewed proposal. Two three-agent
deliberations reviewed it against the compiler, amended it, and agreed an
ordered implementation plan. This task records that ratified result in the
design document so an implementing agent works from an agreed, tree-checked
contract instead of a proposal.

## Scope

- Included: the design document only. Section-by-section amendments (§3–§12),
  the derived placement predicate, the completion-ownership table, the lowering
  contract, and the ordered P0–P6 / P-matrix / M0 / L0–L6 plan.
- Excluded: every compiler change. The prerequisites P0–P6 are shipped-code
  defects with their own task records; the L-slices are the language work. This
  task changes no `src/` file and adds no test.

## Decisions

### Decision 1: Two deliberations, the second re-based on `33acccc`

- **Context**: The first consensus was reached against `b3934cd`. While it was
  being applied, `origin/main` advanced to `33acccc` — `TASK-280: lower try
  through expression evaluation` (PR #87) — which merged the branch the first
  consensus assumed would stay unmerged. Applying it unchanged would have
  recorded false statements about the tree.
- **Alternatives considered**: (a) apply the first consensus and patch the
  baseline sections by hand; (b) re-run the full deliberation on the new
  baseline. (a) was cheaper but left every `path:line` citation unchecked, which
  is the failure that made the first consensus stale in the first place.
- **Decision and rationale**: (b). Three fresh agents re-validated against
  `33acccc` across five rounds. The language and ownership decisions survived
  unchanged; the baseline, sequencing, and citations did not, and the
  re-validation found six shipped defects that reading alone had missed.

### Decision 2: One completion record, two host-selected printers

- **Context**: whether a statement-bodied `result` should always be emitted as
  an arrow callback, or keep the existing statement-host slot-plus-break
  encoding alongside the expression-boundary callback.
- **Alternatives considered**: always-callback gives one emitted shape and one
  set of control-flow rules. Two printers keep the existing inlining that
  `tests/compile.rs` already pins.
- **Decision and rationale**: two printers. Always-callback either forces the
  enclosing function `async` or depends on the `is_async` source scan that this
  design already replaces, and it discards inlining the backend already
  performs. `TargetCapability` has exactly two arms, so the specification is a
  two-row table rather than a per-host list. Recorded as §6.2 with a per-host
  green condition, because testing only the expression host reports the slice
  green while the statement host is still wrong.

### Decision 3: Shipped defects are prerequisites, not plan scope

- **Context**: the deliberation found ICEs, invalid emission, silent runtime
  corruption, and a panicking public API in code already released to users.
- **Alternatives considered**: absorb them into the language slices that rewrite
  those positions anyway.
- **Decision and rationale**: file them as independent tasks that land first.
  Absorbing shipped miscompiles into a design is how the coverage gap arose; each
  prerequisite is also valuable if this proposal is abandoned.

### Decision 4: Reject constructor-, generator-, and static-block-owned propagation

- **Context**: a function-targeted `try` in these owners emits `return <Result>`.
- **Alternatives considered**: treat a generator's `return Err` as a checkable
  return channel.
- **Decision and rationale**: reject. A generator's `return` sets the iterator
  completion value, and `for...of` discards it by language design, so the `Err`
  disappears. An annotated `Generator<T, void, …>` does surface `ts2322`, but
  that is a raw TypeScript error where a tt diagnostic belongs, and unannotated
  or permissively annotated generators are silent under `--check-types` and on
  the untyped default path. The constructor case is the same shape: `new C()`
  returns the `Err` object and `instanceof` is false.

## Work log

- 2026-08-29: Deliberation I against `b3934cd`. Three agents reviewed
  independently, cross-critiqued, closed positions, drafted and ratified. Found
  a pre-existing silent miscompile: a `result` block whose tail is a `match`
  with block-arm `return`s emits those returns verbatim, so they leave the
  enclosing function while the region slot stays `undefined`, and the emitted
  TypeScript is valid so nothing reports it.
- 2026-08-29: `origin/main` advanced to `33acccc` (PR #87), invalidating the
  baseline, the sequencing, and most citations of that consensus.
- 2026-08-29: Deliberation II on `33acccc` with three fresh agents. Attribution
  was established by building a second compiler from `b3934cd` and running
  identical inputs through both, and the completion-leak mechanism by an
  instrumented third build. Ratified after two amendment rounds.
- 2026-08-29: Applied the ratified amendments to
  `docs/design/try-result-scopes.md` and inlined the ordered plan as §9.

## Issues and resolutions

### Issue 1: The first consensus went stale mid-application

- **Symptom**: after `git merge --ff-only origin/main`, the consensus's C10
  ("TASK-280 does not merge to `main` as-is"), its split baseline in §3, §12's
  instruction not to merge, and implementation slice 1 described a world that no
  longer existed, and `src/codegen/core.rs` had grown 119 lines.
- **Cause**: PR #87 merged between the deliberation and its application.
- **Resolution**: re-ran the deliberation on the new baseline rather than
  hand-patching. The re-validation required every citation to be re-checked; the
  ratifier found zero defective citations in the final Part A.

### Issue 2: The recorded fix for the completion leak was wrong

- **Symptom**: the first consensus explained the leak as a missing
  `expected_exit_calls` registration. Under that explanation the fix is one
  change.
- **Cause**: an instrumented build showed the leak is host-dependent. The
  statement host fails through `emit_body_with_exits` with an empty exits slice;
  the expression-boundary host takes the unrewritten `emit_body` branch but is
  already correct for the match-tail shape. Separately, ResultRegion projection
  emits a `0` placeholder for `region.value`, so a tail `match` never becomes its
  own overlay and never owns its `value_exits`.
- **Resolution**: §6.1 requires both the registration and the `region.value`
  projection and states they are not alternatives. §6.2 carries a per-host green
  condition so the slice cannot read green from the host that needs no change.

### Issue 3: A draft cited a function that does not exist

- **Symptom**: the consensus draft referenced `project_result_region` in three
  plan entries.
- **Cause**: the symbol was invented. The real function is `emit_result_region`
  at `src/program_syntax.rs:1078`, which collides by name with
  `src/codegen/core.rs:1832` — the exact pair the first language slice exists to
  distinguish.
- **Resolution**: both ratifiers caught it before the document was applied. Every
  occurrence now names the module.

## Verification

Documentation-only change; no Rust source or test was touched.

- [x] `git status` shows only `docs/design/try-result-scopes.md` and
      `docs/tasks/` changes
- [ ] `cargo fmt --check` — not applicable, no Rust change
- [ ] `cargo clippy --all-targets -- -D warnings` — not applicable
- [ ] `cargo test` — not applicable

## Result

Changed files:

- `docs/design/try-result-scopes.md` — replaced §3, §4.3 consequences, §4.4
  claiming, §4.5, §4.6, §5.7, §6, §7, §8, §9, §12; added to §4.2, §4.7, §10, and
  §11. Status is now a ratified consensus against baseline `33acccc`, and §9
  carries the ordered plan.
- `docs/tasks/TASK-281-try-result-scopes-consensus.md`, `docs/tasks/INDEX.md`.

Follow-up: the prerequisites P0–P6 in §9 are shipped-code defects and each needs
its own task record before the language slices L0–L6 begin. They are, in order:
the planner's swallowed failure return, host projection and source-preservation
crashes, expression-boundary Result hosting, concise-arrow propagation, unsound
function targets (constructor and generator), shipped claimer gaps, and erased
placement reasons.
