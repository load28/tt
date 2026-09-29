# TASK-460: Write the separator a missing-arm edit needs after the last written arm

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

The `match-not-exhaustive` quick fixes ("add the missing arms" and "or add a final `_` arm") produced code that did not compile when the last written arm had no trailing comma and the closing `}` stood on its own line. The edit inserted whole arm lines above the `}` without writing the separator the previous arm needed, so applying it turned a hole into `malformed-match`. The same happened when the last arm was a block arm, when a line or block comment followed it, and tab-indented files received a tab plus two spaces.

## Scope

- Included: The shared arm-insertion edit (`src/diagnostics/suggestions.rs`), the parsed arm-list end the parser records for it (`src/ast.rs`, `src/parser/matches.rs`), and the plumbing that carries that fact through both exhaustiveness pipelines (`src/analysis/`, `src/probe.rs`, `src/sema/coverage.rs`, `src/engine/projection.rs`, `src/engine/semantics.rs`).
- Excluded: The wording of the suggestions, the arm text the analysis authors, and the VS Code adapter, which forwards the compiler's edit unchanged as an LSP `WorkspaceEdit` (LSP 3.17 `CodeAction.edit`, `TextEdit`).

## Decisions

### Decision 1: Record where the written arms end in the parse and author the edit from it

- **Context**: The edit needs two facts: where the last arm's final token ends, and whether a `,` token follows it. The multi-line branch did not ask either, and the one-line branch asked the text (`trim_end().ends_with(',')`), which a trailing comment defeats in both directions: `r /* , */` looks separated and `r, /* note */` does not.
- **Alternatives considered**: Re-lexing the body inside `insert_arms` and searching for the last depth-0 comma would duplicate the parser's arm grammar in the diagnostics layer and still have to guess where an expression body ends. Scanning the text for comments would be a second trivia scanner.
- **Decision and rationale**: The parser already tokenizes the arm list with the lexer, which drops comments as trivia, and the strict arm grammar makes the list's tokens exactly the arms and their separators. `arms_tail` reads the list's last token: a `,` means the last arm is separated, and the token before it (or the last token otherwise) is where the arm ends. `MatchExpr` and `TupleMatchExpr` carry this as `ArmsTail { last_start, last_end, separated }`, and the analysis, the typed probes, and `MatchSite` carry it unchanged, so both pipelines author the same edit from the same parse. When the arm is unseparated, the single edit starts at `last_end`, writes `,`, copies the text up to the insertion point unchanged (so a comment stays where it was), and then writes the arms. One edit keeps the quick fix atomic, as LSP 3.17 `TextEdit` arrays in one `WorkspaceEdit` must not overlap.

### Decision 2: Indent authored arms like the written ones

- **Context**: The multi-line branch indented new arms by the `match` keyword line's indentation plus two spaces, which mixed tabs and spaces in a tab-indented file and misaligned arms written at a different depth.
- **Alternatives considered**: Detecting the file's indentation unit from other lines is a formatting heuristic outside the match.
- **Decision and rationale**: When the last arm starts its own line, the new arms use exactly its leading whitespace. Only when no arm starts a line does the edit fall back to the keyword line's indentation plus one step, using a tab when that indentation is tab-based.

## Work log

- 2026-09-28: Reproduced with the hunter's `t3.py` against `ttc --server`: the multi-line match with an unseparated last arm, a block last arm, a trailing line comment, and a comment line before `}` all rechecked as `malformed-match`; the tab-indented file received `\t  `.
- 2026-09-28: Added `ArmsTail` to `src/ast.rs`, computed it in `parse_match_complete` with `arms_tail`, and carried it through `MatchAnalysis`, `LiteralMatch`, `TagMatch`, `MatchAnchor`, and `MatchSite`. Exported `ttc::ArmsTail` because the public probe structs now name it.
- 2026-09-28: Rewrote `insert_arms` to write the missing comma after the parsed last arm, carry the intervening text, and take the indentation from the last arm.
- 2026-09-28: Added `every_authored_arm_edit_compiles_after_the_last_written_arm` and `authored_arms_take_the_indentation_of_the_written_arms` in `tests/compile/cases_08.rs`, and `typed_missing_arm_edits_compile_after_an_unseparated_last_arm` in `tests/native.rs` for the typed pipeline. The hunter's `c1`–`c3` scripts recheck clean for every suggestion.

## Issues and resolutions

### Issue 1: The typed regression test failed on a binding name

- **Symptom**: The first version of the typed test used `Circle(r)` against a field named `radius`; the recheck reported TypeScript's `ts2339`.
- **Cause**: A payload binding names the field; `r` is not a field of `Circle`. The untyped analysis leaves that to TypeScript, as the error-layer contract says.
- **Resolution**: The tests bind `radius`, the declared field.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/ast.rs`, `src/parser/matches.rs`, `src/diagnostics/suggestions.rs`, `src/analysis/mod.rs`, `src/analysis/patterns.rs`, `src/probe.rs`, `src/lib.rs`, `src/sema/coverage.rs`, `src/engine/projection.rs`, `src/engine/semantics.rs`, `tests/compile/cases_08.rs`, and `tests/native.rs`. Every missing-arm and wildcard-arm edit now compiles after an unseparated, block, or commented last arm, in both the default and typed pipelines, and follows the file's arm indentation.
