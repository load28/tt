# TASK-380: Color match by the construct's shape in the TextMate grammar

- **Status**: Complete
- **Started**: 2026-09-15
- **Completed**: 2026-09-16
- **Commit**: —

## Purpose

The grammar began the `match` rule on `match` followed by `(`, so a plain call such as `match("/a")` or `match(v).with(1)` was colored as the tt keyword until semantic tokens corrected it.

## Scope

- Included: `editors/vscode/syntaxes/src/tt.rules.json` and the generated grammars, plus a grammar test.
- Excluded: semantic tokens, which already classify these positions correctly.

## Decisions

### Decision 1: The rule requires the construct's full head

- **Context**: TextMate grammars cannot match balanced parentheses in general.
- **Decision and rationale**: the rule's lookahead requires `( ... ) {` with parentheses nested up to three levels, which covers every scrutinee the language tests write; a deeper scrutinee falls back to TypeScript coloring and the engine's semantic tokens. A call is never followed by `{`, so it is never claimed.

## Work log

- 2026-09-15: Changed the rule, regenerated `tt.tmLanguage.json` and `ttx.tmLanguage.json`, and added a grammar test for calls versus constructs.

## Issues and resolutions

None.

## Verification

- [x] extension unit tests (191 pass, 0 fail)
- [x] `./scripts/ci` (all stages)

## Result

Changed files: `editors/vscode/syntaxes/src/tt.rules.json`, `editors/vscode/syntaxes/tt.tmLanguage.json`, `editors/vscode/syntaxes/ttx.tmLanguage.json`, `editors/vscode/server/src/test/grammar.test.ts`.
