# TASK-365: Audit enterprise mixed-source compatibility

- **Status**: Complete
- **Started**: 2026-09-11
- **Completed**: 2026-09-12
- **Commit**: `289351e`

## Purpose

Audit the compiler, CLI, project initializer, and documentation as one product,
then repair reproducible failures that prevent realistic TypeScript projects
from combining `.tt`, `.ttx`, `.ts`, and `.tsx` sources safely.

## Scope

- Included: Product documentation, CLI behavior, `create-tt`, compiler parsing,
  lowering, analysis, code generation, typed builds, mixed-source imports, and
  regression coverage for representative enterprise project structures
- Excluded: New language syntax, release publication, local tool reinstallation,
  and input-specific fallbacks or diagnostic suppression

## Decisions

### Decision 1: Guard external parser panics at the shared syntax boundary

- **Context**: A malformed JSX numeric entity reached an SWC `unwrap()` during
  output verification, so library and fuzz callers could abort instead of
  receiving a compiler result.
- **Alternatives considered**: Catch only at the CLI, disable verification for
  JSX, or recognize the malformed entity before SWC at the shared lexer
  boundary.
- **Decision and rationale**: Keep verification enabled and reject malformed
  numeric JSX entities before SWC, with a panic catch as a final dependency
  safety net. The shared boundary is used by compile, editor projection, and
  server paths, while quoted TypeScript text remains opaque and byte-preserved.

### Decision 2: Keep unterminated template interpolations opaque

- **Context**: Recovery for an unfinished `${` created overlapping raw and
  interpolation spans, which violated the source-preservation contract.
- **Alternatives considered**: Relax the rope validator or special-case the
  resulting byte sequence in codegen.
- **Decision and rationale**: The lexer leaves an unterminated interpolation in
  the template's raw chunk, preserving the editor buffer without overlapping
  HIR spans.

## Work log

- 2026-09-11: Fast-forwarded `main` from `09243c6` to `67173eb`, preserved the
  existing untracked `.task-agent-disabled` file, and confirmed
  `./scripts/doctor` reports a ready development environment.
- 2026-09-11: Created `codex-task-365-enterprise-compatibility-audit` from the
  updated `main` and registered this task before tracked-file changes.
- 2026-09-11: Ran the full installed TypeScript corpus and a 120-second
  `generated_tt_compiles` fuzz campaign without finding generated-code failures.
- 2026-09-11: Replayed archived fuzz inputs and reproduced a SWC JSX entity
  panic in `compile_any_bytes` for a 24-byte malformed TSX input.
- 2026-09-11: Added shared malformed-entity validation, a panic-safe SWC
  boundary, and regressions for the malformed TSX input and an identical
  sequence inside a valid string literal.
- 2026-09-11: Replayed the crash input successfully after the repair and ran
  Clippy plus the complete Rust test suite.
- 2026-09-12: A second `compile_any_bytes` fuzz campaign found a source-span
  reorder for an unterminated template interpolation; reproduced it through
  the CLI and traced it to template recovery spans.
- 2026-09-12: Changed template lexing to keep unterminated interpolations raw
  and added a regression covering the malformed editor buffer.
- 2026-09-12: Ran a 60-second `compile_any_bytes` campaign with 64,739 runs
  and no crash, then completed the full Rust test suite.
- 2026-09-12: Replayed the archived `generated_tt_compiles` crash artifact;
  the current target completed it without a failure.
- 2026-09-12: Release tests exposed two debug-only panic-injection tests that
  were incorrectly running without their injection hook.
- 2026-09-12: Gated the panic-injection helper and tests with
  `debug_assertions`; optimized CLI tests now pass without dead-code warnings.

## Issues and resolutions

### Issue 1: Malformed JSX entity crashes the SWC verifier

- **Symptom**: `<>&>&w=<>&>&w=2&(&#;;\\w\x01` caused
  `swc_ecma_parser::lexer::read_jsx_entity` to unwrap `None` and abort the
  `compile_any_bytes` fuzz target.
- **Cause**: SWC 45.0.1 assumes numeric JSX entities contain at least one
  digit. The hand lexer also intentionally gives up on incomplete JSX, so the
  malformed entity was not recognized before verification.
- **Resolution**: `host_syntax_error` now detects malformed numeric entities
  in JSX text, including incomplete JSX recovery, while skipping strings,
  comments, and template literals. `verify::parse_ts_module` catches any
  remaining SWC unwind and returns a normal validation error.

### Issue 2: Unterminated template interpolation reorders source spans

- **Symptom**: An unfinished template interpolation caused
  `validate_source_preservation` to report `SourceReordered` and panic.
- **Cause**: Lexer recovery emitted an interpolation through end-of-file while
  the nested parser also owned a suffix of that span.
- **Resolution**: Unterminated `${` sequences remain one opaque raw template
  chunk, so codegen receives non-overlapping source spans.

### Issue 3: Release test suite ran debug-only panic injection tests

- **Symptom**: `cargo test --release` failed because `TTC_PANIC_FOR_TEST` was
  intentionally inactive in optimized builds.
- **Cause**: The two panic-safety tests lacked the `debug_assertions` gate that
  documents their test-only injection contract.
- **Resolution**: Both tests are compiled only for debug test binaries.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `cargo test --release`
- [x] `./scripts/ci npm website`
- [x] `env TTC_CORPUS_FULL=1 TTC_REQUIRE_CORPUS=1 cargo test --test corpus --release`
- [x] `cargo +nightly fuzz run generated_tt_compiles -- -max_total_time=120 -rss_limit_mb=4096`
- [x] Archived `compile_any_bytes` crash replay after repair
- [x] Archived `generated_tt_compiles` crash replay after repair
- [x] `cargo +nightly fuzz run compile_any_bytes -- -max_total_time=60 -rss_limit_mb=4096`
- [x] `git diff --check`

## Result

The compiler no longer aborts on the reproduced malformed JSX entity. The
shared lexical boundary reports it as a source diagnostic, the verifier has a
dependency panic safety net, valid TSX strings remain byte-identical, and the
full available product gates pass.

Changed files: `src/lexer/validation.rs`, `src/verify.rs`,
`tests/passthrough.rs`, `docs/tasks/INDEX.md`, and this record.
