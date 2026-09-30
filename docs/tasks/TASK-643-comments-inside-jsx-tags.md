# TASK-643: Read a comment inside a JSX tag as trivia in the lexer facts

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-643`

## Purpose

TASK-639 (Issue 4) found that a comment inside a JSX opening tag,
`<Button /*c*/label="ok" />`, hides the element from ttc's lexer: swc reads a
JSX element there and the lexer facts do not, so the library test
`lexer::facts::tests::the_machine_reads_the_corpus_as_swc_does` failed on the
editor case `plainTsx`, and TASK-639 moved the marker out of the tag. This
task fixes the lexer and restores the marker.

## Scope

- Included: `src/lexer.rs` (`scan_jsx_opening`, `scan_jsx_closing`), a lexer
  facts test, and the editor case `plainTsx` with its baseline.
- Excluded: trivia between `<` and the tag name, where the lexer's reading
  decides between a JSX element and a type argument list or a comparison.

## Sources

- TypeScript at `5739027c`, `tsc/internal/parser/parser.go`: the attributes
  of an opening element are read with `scanJsxIdentifier` after an ordinary
  `nextToken`, and the closing `>`/`/>` with `parseExpected`, so the scanner's
  trivia (white space and both comment forms) is skipped between
  attributes, around `=`, and before the end of either tag.
  `tsc/internal/scanner/scanner.go` `ScanJsxAttributeValue` skips white space
  after `=`; a comment there falls to `Scan`, which skips it and reads an
  ordinary string literal, which `parseJsxAttributeValue` accepts.
- The JSX specification (facebook/jsx): `JSXAttributes` and the tags are
  sequences of tokens, and comments are trivia between tokens as in
  ECMAScript (ECMA-262 §12.4).

## Decisions

### Decision 1: Skip the lexer's own trivia where the tag skipped only white space

- **Context**: `scan_jsx_opening` and `scan_jsx_closing` skipped `is_ws`
  bytes between the name, the attributes, `=`, and the end of the tag, so a
  comment ended the attempt and the `<` was read as ordinary TypeScript.
- **Alternatives considered**: Skipping only block comments (a `//` comment
  in a multi-line tag is trivia too); leaving the defect and keeping markers
  out of tags (TASK-639's workaround).
- **Decision and rationale**: Use `scanner::skip_trivia`, which the lexer
  already uses between tokens (white space, line terminators, both comment
  forms, and non-ASCII white space), at each of those points. An
  unterminated block comment reaches the end of the input, where the tag is
  incomplete and the lexer falls back as before.

## Work log

- 2026-09-30: Reproduced with the corpus test and TASK-639's repro; replaced
  the four white-space loops; added `a_comment_inside_a_jsx_tag_is_trivia`
  (block and line comments between attributes, around `=`, before `/>`,
  `>`, and in a closing tag); restored `/*attribute*/` and the `attribute`
  hover in `tests/cases/editor/plainTsx.ttx` and `.tsx` and their baseline
  lines, which regenerate unchanged from TASK-639's first version.
- 2026-09-30: Merged `claude/ecstatic-dijkstra-qw5pf9` (TASK-644 to TASK-646)
  and ran the full gate over TASK-640 to TASK-643 (Verification).

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `src/lexer/facts/tests.rs`,
  `lexer::facts::tests::a_comment_inside_a_jsx_tag_is_trivia`, and
  `the_machine_reads_the_corpus_as_swc_does` over the restored
  `tests/cases/editor/plainTsx.ttx`.
- **Observed failure**: Without the change, `jsx swc only: 20..47 "<Button
  /*c*/label=\"ok\" />;"` from the new test, and `jsx swc only: 326..361`
  and `439..474 "<Button /*attribute*/label=\"ok\" />;"` from the corpus test.

## Verification

See the gate recorded at the end of this record.

## Result

Changed files: `src/lexer.rs`, `src/lexer/facts/tests.rs`,
`tests/cases/editor/plainTsx.ttx`, `tests/cases/editor/plainTsx.tsx`,
`tests/baselines/reference/editor/plainTsx.baseline`,
`docs/tasks/TASK-639-fourslash-style-editor-cases.md`, `docs/tasks/INDEX.md`,
and this record.
