# TASK-548: Recover a discarded `result` block in the typed projection

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

Under `ttc --check-types`, a `result-value-discarded` error hid every type
diagnostic of its file. A file with
`result { const q = try Result.Ok(1); return q; };` as a statement and
`const z: string = 1;` reported only `result-value-discarded` (exit 1),
without `ts2322`. Other recoverable tt errors keep the file's type
diagnostics, and `docs/ai/tt.md` says a recoverable tt error does not hide
them on the typed path.

## Scope

- Included: the typed projection's recovery
  (`compile_projection_report_parsed`, `src/lib/compile.rs`) and the anchor
  of a slot `emit_expr` writes for a captured value.
- Excluded: reporting a second discarded `result` block in the same file
  before the first is fixed. The lowering plan stops at the first one in
  every path, typed or not; the projection recovers the later ones so their
  file is still type-checked.

## Decisions

### Decision 1: A discarded Result is a recoverable construct, not a projection blocker

- **Context**: The Evaluation IR raises a discarded `result` block as a
  lowering-plan failure (`EvaluationError::DiscardedResult`), so the file has
  no emit. `DiagnosticCode::blocks_projection` does not list the code, so the
  typed path went on to `compile_projection_report_parsed`, which substitutes
  only parser recovery nodes and the placement errors
  (`try-placement`, `try-crosses-value-region`, `match-placement`) and the
  invalid variant field type. With nothing to substitute, the projection had
  no emit and the file had no type diagnostics.
- **Alternatives considered**: (a) List the code in `blocks_projection`.
  That documents the loss instead of removing it: the error concerns one
  expression, and the rest of the file is valid TypeScript. (b) Make the
  plan carry discarded results as target errors instead of failing. The
  plan's other consumers (`emit_mapped`, the editor's emission) would then
  emit a discarded region that has no lowering.
- **Decision and rationale**: The diagnostic's span is the whole `result`
  block, an expression, so the projection substitutes it like a placement
  error (`undefined as any`, or `0` when shorter), and the file's other code
  is checked.

### Decision 2: Recovery repeats until the projection emits

- **Context**: The plan stops at the first discarded block, so a second one
  appears only when the recovered source is compiled, and the file still had
  no emit.
- **Decision and rationale**: Recovery repeats: while the recovered source
  has no emit, the recoverable constructs its report names that are not
  already inside a substituted one are added and the source is recovered
  again. Every round substitutes at least one construct more, so the loop
  ends. The reported diagnostics stay the original source's.

### Decision 3: A recovered construct's placeholder is `any` wherever it fits

- **Context**: With the typed projection no longer withheld, the
  `result-boundaries` practical-diagnostics fixture reported `ts2345` at
  `persist(job)`: the recovery wrote `0` over `try fetchJob()` in a
  constructor (a span shorter than `undefined as any`), so `job` became a
  `number`. A substitution is meant to leave no checker consequence behind;
  the fixture had hidden this because no type diagnostic of the file was
  reported.
- **Decision and rationale**: A span too short for `undefined as any` but
  long enough for `0 as any` gets the latter. Both are `any` and parse
  wherever the recovered expression did; `0` stays only for spans shorter
  than eight bytes.

### Decision 4: A slot written in place of a captured value carries its construct's anchor

- **Context**: The same fixture then reported `ts2454` (`'$tt_v0' is used
  before being assigned`) for a `result` block that can end without a
  success value, the checker consequence of `result-no-success-value`. It
  was not suppressed by the tt error because its origin was only "near this
  position": `Emitter::emit_expr` wrote the slot of a source replacement it
  found for the value (`export const queued = $tt_v0;`) as a bare literal,
  while the source walk writes the same replacement anchored to the value's
  construct. The same leak appeared with that block alone on `c27ad13`.
- **Decision and rationale**: `emit_expr` anchors the written replacement to
  its value's construct (`value_anchor`), as `source_range_rope` does, so a
  checker diagnostic at the slot belongs to the lowering the tt error owns.

## Work log

- 2026-09-29: Reproduced with `ttc --check-types src` in a configured
  project on `22aa04d`.
- 2026-09-29: Split `compile_projection_report_parsed` into
  `recoverable_constructs`, `outermost_recoveries`, and `recover_source`,
  added `ResultValueDiscarded` to the recoverable constructs, and made the
  recovery repeat.
- 2026-09-29: Added `a_discarded_result_does_not_hide_the_file_s_type_errors`
  (`tests/native/cases_02.rs`), with two discarded blocks and a type error.
- 2026-09-29: The full suite failed
  `cli_reports_every_practical_diagnostic_at_its_source` (`ts2345`,
  `ts2454` on `result-boundaries`); added Decisions 3 and 4
  (`src/lib/compile.rs`, `src/codegen/core/emitter/source.rs`).

## Issues and resolutions

### Issue 1: The recovered file reported checker consequences of its tt errors

- **Symptom**: `result-boundaries` reported `ts2454` and `ts2345` beside its
  three tt errors.
- **Cause**: A typed `0` placeholder, whose consequence had been hidden
  with the rest of the file's type diagnostics, and an unanchored slot write
  in `emit_expr`, which leaked for such a `result` block on its own as well.
- **Resolution**: Decisions 3 and 4. The fixture is unchanged and passes;
  `a_result_without_a_success_value_owns_its_slot_s_consequence`
  (`tests/native/cases_02.rs`) covers the block on its own.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] The new test fails without the change (no `ts2322` diagnostic).

## Result

Changed `src/lib/compile.rs`, `src/codegen/core/emitter/source.rs`,
`tests/native/cases_02.rs`, `docs/tasks/INDEX.md`, and this record.
