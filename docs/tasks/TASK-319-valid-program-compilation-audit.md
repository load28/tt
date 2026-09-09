# TASK-319: Audit valid-program compilation failures

- **Status**: Complete
- **Started**: 2026-09-03
- **Completed**: 2026-09-03
- **Commit**: `TASK-319: repair valid-program compilation`

## Purpose

Find valid TypeScript and tt programs that fail to compile, emit invalid
TypeScript, fail typed checking, or violate their runtime semantics. Repair each
confirmed defect in the compiler layer that owns the broken contract.

## Scope

- Included: TypeScript pass-through, tt parsing and lowering, emitted TypeScript,
  typed project checking, source-kind composition, and runtime behavior
- Excluded: New language features, release publication, and external service
  changes

## Decisions

### Decision 1: Treat validity as an end-to-end contract

- **Context**: Parser success alone does not prove that a valid program survives
  lowering, TypeScript parsing, typed checking, and execution.
- **Alternatives considered**: Audit isolated parser cases; rely on the existing
  regression suite; exercise generated and corpus inputs through every applicable
  public compiler boundary.
- **Decision and rationale**: Classify failures by the first responsible boundary
  and preserve each confirmed case as a regression at that boundary.

## Work log

- 2026-09-03: Confirmed the development environment and the complete TASK-318
  gate, then opened a focused follow-up audit for valid-program compilation.
- 2026-09-03: Ran the complete installed TypeScript corpus. All 136 valid
  files passed through unchanged; the remaining three corpus files were
  independently invalid TypeScript.
- 2026-09-03: Extended generated tt programs with TSX hosts and nested JSX
  children. The generator exposed a valid Result/JSX composition whose output
  did not parse, then completed 45,942 runs after the repair.
- 2026-09-03: Ran the arbitrary-input target against TypeScript and TSX. It
  minimized an upstream parser panic to a six-byte malformed JSX namespace
  member and replayed the artifact successfully after preflight validation.
- 2026-09-03: Re-ran the full local gate with registry and loopback access.
  Rust, npm, website, native-backend, and editor stages all passed.

## Issues and resolutions

### Issue 1: Malformed TSX could panic the verification parser

- **Symptom**: The six-byte input `<G:U.m` reached an unreachable branch in the
  TypeScript parser and terminated analysis or compilation instead of producing
  a source diagnostic.
- **Cause**: A JSX namespace name followed by member access is invalid, but the
  parser version used by the verifier assumes that AST combination is
  unreachable before it can return a recoverable parse error.
- **Resolution**: Detect that lexical JSX shape before every parser boundary,
  including projected host syntax and effect analysis, and return the same
  located source-parse failure without entering the parser.

### Issue 2: A JSX child match inside Result emitted twice

- **Symptom**: A valid `.ttx` Result body containing a JSX child `match` emitted
  the planned switch once and then emitted `let` plus the switch again inside
  the JSX expression container, producing invalid TypeScript.
- **Cause**: Ordinary bodies distinguish a planned slot occurrence from a
  standalone statement value, while the Result-specific statement emitter sent
  every extracted expression through standalone statement lowering.
- **Resolution**: Centralize the statement-expression lowering predicate and
  use it in both ordinary and Result body emitters. Planned JSX values now emit
  only their assigned slot at the authored child position.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Full installed TypeScript corpus: 136 unchanged, 3 independently
  invalid, 0 broken
- [x] Generated `.tt`/`.ttx` programs: 45,942 post-fix runs
- [x] `./scripts/ci`: agents, rust, npm, website, native, and extension passed

## Result

The audit repaired both confirmed compiler failures at their owning boundaries.
Valid generated tt/TSX composition now emits parseable TypeScript, malformed
TSX returns a located diagnostic instead of panicking, and the complete
`.tt`/`.ttx`/`.ts`/`.tsx` project and product gates pass.
