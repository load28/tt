# TASK-613: Pin that a pipeline step being typed answers as its TypeScript equivalent does

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-613: Pin that a pipeline step being typed answers as its TypeScript equivalent does`

## Purpose

TASK-603 left open that, while a pipeline step is typed, hover shows `any`
mid-word and `const obj: () => any` on the step's name, and that completion
in `4 |> obj.twice |> (x …` changes from keystroke to keystroke. The task
was to compare against the TypeScript equivalent and fix the projection of
an incomplete step if it caused this, or record why the behaviour matches
TypeScript.

## Scope

- Included: A keystroke-by-keystroke comparison of hover and completion in
  `.tt` against the equivalent `.ts` source, the projection of each state,
  and a regression test that pins the equivalence.
- Excluded: Which form the emission lowers a step to (`f(x)` for an inert
  operand, `$tt_ap(x, f)` otherwise); that is the evaluation-order contract
  of `docs/design/pipeline-operator.md`, not an editor question.

## Decisions

### Decision 1: No change to the projection; the answers are TypeScript's for the equivalent program

- **Context**: A typing simulation through the real language server
  (`target/probe/typing.cjs`, typing `obj.twice |> (x => x + 1)` after
  `4 |> ` with a following line, and the same text after
  `4 |> half |> `), against tsgo typing the equivalent
  `obj.twice(half(4))` and `((x => x + 1))(obj.twice(4))`:
  - Mid-word hover: tt answers `any` for `o`, `ob`, `obj.t` … `obj.twic`,
    and TypeScript answers `any` at exactly the same states: an identifier
    that resolves to nothing is `any` to its quick info.
  - `const obj: () => any`: the projection of `4 |> obj` followed by a line
    is `obj(4)` (an inert operand is applied directly), a complete and
    faithful program, not a recovered one. TypeScript's quick info for the
    callee of a call with no applicable signature renders the error
    signature: `obj(half(4))` in a `.ts` file hovers `const obj: () =>
    any` the same way.
  - Completion: the list sizes move at the same keystrokes in both (after
    `(x =>` a parameter joins the scope, after `x ` the position is no
    longer an expression start, after `+` it is again); tt's list is
    TypeScript's plus its three expression keywords (`match`, `flow`,
    `result`, TASK-578).
- **Alternatives considered**: (a) Serve a probe or a recovery for a step
  whose name is being typed: the text is a complete step, so the projection
  already is the program the user wrote, and a stand-in would answer about
  something else. (b) Lower every step through `$tt_ap` so the step name
  is an argument (whose hover is its declared type): that changes emitted
  code and evaluation-order decisions to change a hover, and would depart
  from what TypeScript shows for the equivalent call.
- **Decision and rationale**: The behaviour is TypeScript's for the
  program as written at each keystroke, so the projection stays as it is.
  A regression test pins the equivalence: for `4 |> o`, `4 |> obj`,
  `4 |> obj.tw`, and `4 |> obj.twice`, hover and the completion labels in
  the `.tt` file equal those of `o(4)`, `obj(4)`, `obj.tw(4)`, and
  `obj.twice(4)` in a `.ts` file of the same project.

## Work log

- 2026-09-30: Wrote `target/probe/typing.cjs` (types a string one
  character at a time through the language server, asking hover before the
  cursor and completion at it) and ran it for tt and tsgo (`ty1.json`,
  `ty2.json`, `ty3.json`).
- 2026-09-30: Dumped the service projection of `4 |> obj` and
  `4 |> half |> obj` followed by a line: `obj(4)` and
  `$tt_ap(half(4), obj)`, both faithful with no recovered span.
- 2026-09-30: Test:
  `a_pipeline_step_being_typed_answers_as_its_typescript_equivalent_does`
  (`tests/native/editor_service.rs`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `cd editors/vscode && npm run compile && node --test "server/out/test/*.test.js" "client/out/test/*.test.js"`
- [x] `node scripts/check-task-index`

The full gate above ran once, on the tree with TASK-606 to TASK-613.

## Result

Changed `tests/native/editor_service.rs` and the task index. The full gate
also caught a unit test TASK-611 left pinned to the old error text; the fix
(`src/typescript/service.rs`) and its account (TASK-611, Issue 2) are in
this commit. The reported
behaviour is TypeScript's for the equivalent source and is now pinned by a
regression test; the projection is unchanged.
