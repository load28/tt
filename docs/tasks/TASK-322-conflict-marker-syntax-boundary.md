# TASK-322: Diagnose merge conflict markers at the syntax boundary

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: —

## Purpose

Prevent malformed merge conflict input from crashing the compiler's SWC parser.

## Scope

- Included: Shared lexical validation before SWC parsing, regression coverage, fuzz validation.
- Excluded: Dependency vendoring and changes to valid TypeScript semantics.

## Decisions

### Decision 1: Represent conflict markers as lexical errors

- **Context**: Arbitrary-input fuzzing found a SWC regex re-scan assertion after conflict-marker recovery.
- **Alternatives considered**: Catching panics hides the broken parser invariant; vendoring SWC adds a full dependency fork.
- **Decision and rationale**: Diagnose unresolved conflict markers before SWC recovery, using lexical isolation to preserve literals and comments. Share this validation across syntax consumers.

## Work log

- 2026-09-05: Doctor passed. Fuzzing crashed after 146,355 executions; minimization reduced the case to an eleven-byte conflict separator followed by an unterminated regex.
- 2026-09-05: Added shared `lexer::host_syntax_error` validation to projection, expression-effect, and output parsing. Conflict detection uses significant tokens and leading trivia, including line breaks inside comments; template interpolations are checked recursively.
- 2026-09-05: Added 48 malformed-input combinations across four markers, six host/trivia contexts, and both source kinds. Added 11 literal/comment/JSX preservation checks.
- 2026-09-05: `./scripts/ci rust` passed. After the final trivia refinement, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --quiet` passed again. Full required corpus validation preserved 136 valid files; three inputs were already invalid TypeScript and none were broken by compilation.
- 2026-09-05: Replayed the original 41-byte artifact and minimized 11-byte artifact successfully against the sanitizer-enabled fuzz binary.
- 2026-09-05: Both 60-second sanitizer fuzz runs passed: `compile_any_bytes` executed 199,331 units and `generated_tt_compiles` executed 21,997 units. These are fuzzer execution counts, not unique valid-program counts or exhaustive coverage of arbitrary programs.

## Issues and resolutions

### Issue 1: Merge conflict recovery crashes regex parsing

- **Symptom**: SWC `read_regexp` asserts that its reset position contains `/`, but finds `=`.
- **Cause**: Conflict-marker recovery advances to a later token while retaining the marker's token start; regex re-scanning resets to that stale position.
- **Resolution**: Diagnose all four conflict delimiters at the shared lexical boundary before SWC can enter recovery. Opaque literal/comment/JSX content is preserved. No panic catching or successful fallback output is introduced.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `TTC_CORPUS_FULL=1 TTC_REQUIRE_CORPUS=1 cargo test --test corpus -- --nocapture`
- [x] Original and minimized crash artifact replay
- [x] `cargo +nightly fuzz run compile_any_bytes -- -max_total_time=60 -verbosity=0 -print_final_stats=1`
- [x] `cargo +nightly fuzz run generated_tt_compiles -- -max_total_time=60 -verbosity=0 -print_final_stats=1`

## Result

Fixed the merge-conflict recovery crash with a shared lexical diagnostic boundary.
Changed `src/lexer.rs`, added `src/lexer/validation.rs`, updated the three SWC
parser entry points and source projection validation, and added compilation
and preservation regressions in `tests/compile/cases_09.rs`. This task record
and `INDEX.md` record the completed audit. No language feature or package
version changed.
