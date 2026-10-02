# TASK-633: Fade a payload list whose bindings are all unused

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-633: Fade a payload list whose bindings are all unused`

## Purpose

An arm that binds several fields and uses none of them (`B(x, y) => 1`)
was not faded. An arm that binds one unused field is (TASK-515). TypeScript's
twin, `const { x, y } = s;`, fades the whole destructuring with 6198 "All
destructured elements are unused".

## Scope

- Included: The span a pattern's field list is written with (`TagPattern::list`,
  `InstancePattern::list` in `src/ast.rs`, set by the parsers in
  `src/parser/matches.rs`, `lets.rs`, `iflets.rs`), its HIR node
  (`Pat::Constructor::list`, `Pat::Instance::list`), the list a Core IR
  binding was written in (`Bind::list`), the emission's record of a
  destructuring that stands for a whole list (`DestructuredList`, the
  `DestructuredListStart`/`End` marks in `src/codegen/rope.rs`, written by
  `emit_bindings`), and how the service maps a diagnostic over it
  (`diagnostic_source_span`).
- Excluded: The emitted text (byte-identical), the CLI's typed report
  (errors only), and a destructuring with no single source list (see
  Decision 2).

## Decisions

### Decision 1: Record the destructuring-to-list correspondence in the emission

- **Context**: TypeScript reports 6198 over the object pattern `{ x, y }`
  the emission writes for the arm. Its names are copied from the source and
  mapped, but its braces and commas are glue, so the span has no exact
  source mapping and TASK-515's Decision 3 drops the suggestion: only a
  span with a source counterpart may be faded.
- **Alternatives considered**: (a) Map the braces to the parentheses as
  one-byte `EmitMapping`s: a mapping says the bytes were copied, and `{` is
  not `(`; every consumer that treats a mapping as copied text (rename
  edits, semantic tokens, TASK-603's "an invocation is generated when its
  `(` is not copied") would read a false fact. (b) Fade each name of the
  list instead: TypeScript reports one diagnostic over the pattern, not one
  per name. (c) Map through the arm's anchor: an anchor is where a
  diagnostic about glue is *reported*, not a counterpart, and TASK-515
  keeps suggestions off anchors on purpose.
- **Decision and rationale**: As a declared name records that a generated
  identifier stands for a source name (`DeclaredName`), the emission now
  records that an object pattern stands for a field list: the parser keeps
  the list's span, HIR gives it a node, Core IR carries it on each binding,
  and `emit_bindings` marks the `{ ... }` it writes with that list's span.
  The service maps a diagnostic whose span is exactly a recorded pattern to
  the list, as an exact origin, so the suggestion keeps TypeScript's code,
  message, and `Unnecessary` tag and covers `(x, y)`. A span that is not
  exactly a recorded pattern maps as before, so TASK-515's principle stands:
  nothing without a source counterpart is faded.

### Decision 2: Only a destructuring of a whole, single list is its counterpart

- **Context**: A list with a nested pattern (`C(v: B(x, y), w)`) is
  destructured in parts (`{ w } = $tt_m`, `{ x, y } = $tt_m.v`), and an
  or-pattern's shared bindings (`A(x) | B(x)`) are destructured once for
  several lists.
- **Decision and rationale**: Core IR gives a binding its list only when
  every entry of the list binds a name, and the emitter marks a
  destructuring only when all its bindings name the same list and none is
  shared. The nested list `(x, y)` inside `C(v: B(x, y), w)` is marked;
  the outer list and a shared destructuring are not, and a 6198 on them is
  still dropped. TypeScript reports single unused names (6133) as before.

## Work log

- 2026-09-30: Reproduced with the probe harness (`dg2.cjs w/dg1.tt` and its
  `.ts` twin): tsgo reported 6198 over `{ x, y }`; tt reported nothing for
  the arm.
- 2026-09-30: Added the list span, node, binding field, emission record,
  its shift in `src/codegen/contextual.rs`, and the service mapping.
  Re-ran the harness: 6198 with `Unnecessary` over `(x, y)` in `dg1.tt` and
  `dg3.tt`; single unused bindings unchanged.
- 2026-09-30: Tests: `a_destructuring_stands_for_the_whole_list_it_destructures`
  (`src/engine/language/tests.rs`: plain, aliased, and nested lists, and an
  or-pattern with none), `a_payload_list_whose_bindings_are_all_unused_is_faded_whole`
  (`tests/native/editor_service.rs`: a match arm, a nested list, an
  `if let`, and an `is` property list), and the extension test "a payload
  list whose bindings are all unused is faded whole" (`server.test.ts`).
  With the service lookup disabled the native test answers no 6198 at all
  and the extension test fails.
- 2026-09-30: Minor items from the review, checked and left open (below).
- 2026-09-30: Documentation comments were added only where the crate's
  `missing_docs` lint requires them (the public HIR fields; clippy fails
  without them), as in TASK-629 and TASK-631 for the public engine API.
- 2026-09-30: Full gate on the tree with TASK-629 to TASK-633 (branch
  `claude/ecstatic-dijkstra-qw5pf9` had not advanced, so no merge was
  needed): `cargo fmt --check` clean; `cargo clippy --all-targets -- -D
  warnings` clean; `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
  1744 passed, 0 failed; `npm run compile` and `node --test
  "server/out/test/*.test.js" "client/out/test/*.test.js"` 231 passed,
  0 failed, 0 skipped.

## Issues and resolutions

### Issue 1: Minor editor items left open

- **Symptom and cause**:
  - tt's keyword snippets are offered after a declaration's name
    (`const q |`), where TypeScript offers nothing: the lexer's facts mark a
    word there as a statement start (its recovery for a statement that has
    not ended), and `tt_keywords_at` trusts that fact. A same-line rule
    after an operand-ending token does not cover a binding name, which the
    facts do not mark as ending an operand; the fix belongs in the facts
    machine's statement-start rule (ECMA-262 §12.10.1: a statement begins
    after a token it cannot continue only across a line terminator), which
    the parser also reads, so it was not changed here.
  - The `match` keyword loses tt's color while an arm is malformed: the
    parse-only tokens mark only a claimed match, and a malformed arm leaves
    the match unclaimed (TASK-605 claims only a missing body).
  - `prepareRename` on a reference to a variant's name (`s: S`) answers
    null without a reason, while the declaration answers "A tt variant
    cannot be renamed.": the reference is TypeScript's name, and the rename
    is refused because an edit lands on the variant's generated
    declaration, which `rename_answer` reports without a reason.
  - Path completion entries lacked `detail`: fixed in TASK-631.
- **Resolution**: Recorded for follow-up tasks.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `cd editors/vscode && npm run compile && node --test "server/out/test/*.test.js" "client/out/test/*.test.js"`

The full gate above was run once on the merged tree for TASK-629 to
TASK-633; see the work log entry that records its results.

## Result

Changed `src/ast.rs`, `src/parser/matches.rs`, `src/parser/lets.rs`,
`src/parser/iflets.rs`, `src/hir/mod.rs`, `src/hir/lower.rs`,
`src/resolve/mod.rs`, `src/core_ir/mod.rs`, `src/core_ir/lower.rs`,
`src/codegen/core/emitter/pattern.rs`, `src/codegen/rope.rs`,
`src/codegen/rope/builder.rs`, `src/codegen/contextual.rs`,
`src/lib/mapped.rs`, `src/lib/compile.rs`, `src/engine/language.rs`,
`src/engine/language/service.rs`, `src/engine/language/project.rs`,
`src/engine/language/tests.rs`, `tests/native/editor_service.rs`,
`editors/vscode/server/src/test/server.test.ts`,
`docs/design/lsp-architecture.md`, and the task index. A field list whose
bindings are all unused is faded as TypeScript fades its destructuring.
