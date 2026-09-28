# TASK-500: Read an import-equals module reference as TypeScript's grammar does

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

The token-facts machine read the module reference of `import x = require("y")`, `import A = B.C`, and `export import A = B.C` as an expression, so a following line that begins with `/`, `<`, `[`, `(`, or a template continued it. Plain TypeScript failed: `import fs = require("fs")⏎/a|>b/.test("a|>b") && …` reported `lowering-plan-failed`, `import A = B.C⏎/ val const q = 2 /.test("")` lost its `val`, and `import fs = require("fs")⏎[1] |> console.log` was rejected.

## Scope

- Included: The import-equals states of `Machine::module_item` (`src/lexer/facts/statements.rs`).
- Excluded: `export = expression`, which TypeScript parses as an expression (`parseExportAssignment`) and the machine already reads as one.

## Decisions

### Decision 1: Model `parseModuleReference`

- **Context**: After the `=`, `module_item` pushed a statement expression. TypeScript's `parseImportEqualsDeclaration` instead calls `parseModuleReference`: `require(StringLiteral)` when `isExternalModuleReference` holds (`require` followed by `(`), otherwise `parseEntityNameOfTypeReference` (identifiers joined by `.`, which may cross line breaks), then `parseSemicolon`, where a line break inserts the semicolon.
- **Alternatives considered**: (a) Give the pushed expression a configuration that stops at a line break. That would still accept operators, calls, and element accesses TypeScript never reads there. (b) Model the reference grammar.
- **Decision and rationale**: (b). `ModuleState` gains `Require` (after `require`, at its `(`, whose argument is read as a call's argument list), `Entity` (after a name, which only `.` continues), `EntityDot`, and `Reference` (a complete `require(…)`). Any other token ends the declaration, and a line break before it is an automatic semicolon; the next line begins a statement, so its `/` or `<` begins an operand.

## Work log

- 2026-09-28: Reproduced the three reported inputs and the `.ttx` JSX-text variant with the TASK-499 build.
- 2026-09-28: Replaced the expression after `=` with the module-reference states in `src/lexer/facts/statements.rs`.
- 2026-09-28: Added `a_line_after_an_import_equals_declaration_starts_a_statement` (`tests/compile/cases_14.rs`), `a_line_after_an_import_equals_declaration_passes_through` (`tests/passthrough.rs`), and the shapes to the SWC oracle's known cases (`src/lexer/facts/tests.rs`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`: exit 0.
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0.
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: exit 0; 1545 tests, 0 failures.
- [x] `./scripts/ci extension`: exit 0; 210 extension tests passed, none skipped.
- [x] `scripts/check-task-index`: exit 0.
- [x] The facts oracle (`the_machine_reads_known_shapes_as_swc_does`, `the_machine_reads_the_corpus_as_swc_does`) agrees.

## Result

Changed `src/lexer/facts/statements.rs`, `src/lexer/facts/tests.rs`, `tests/compile/cases_14.rs`, `tests/passthrough.rs`, and `docs/tasks/INDEX.md`; added this record. An import-equals declaration ends where TypeScript's module-reference grammar ends it.
