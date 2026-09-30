# TASK-630: Name a stored callee's signature as the source call does

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-630: Name a stored callee's signature as the source call does`

## Purpose

Signature help in `two(match (s) { ... }, |)` showed
`$tt_v1(a: number, b: string): number`, and in `new Cls(match ..., |)`
`$tt_v1(a: number, b: string): Cls`. TypeScript's twin, `two(1, |)`, shows
`two(a: number, b: string): number`. A generated name must never surface.

## Scope

- Included: The served text signature help is asked in
  (`signature_question`, `callee_name`, `tokens_holding` in
  `src/engine/language/service.rs`, used by `Project::signature_help`).
- Excluded: How the emission stores a callee (unchanged: it keeps the
  call's evaluation order), which invocation is asked about (TASK-603), and
  an optional call whose argument is lowered (`two?.(match ..., |)`), which
  the emission writes as a generated call under a null test; see Issue 1.

## Decisions

### Decision 1: Ask with the source call's callee name in the stored name's place

- **Context**: When an argument lowers to statements ahead of the call (a
  `match` argument), the emission stores the callee first so it is still
  evaluated before the arguments: `const $tt_v1 = (two); ...;
  $tt_v1($tt_v0, b)`. The call's `(` is copied from the source, so TASK-603
  rightly keeps the position in it. TypeScript labels the signature by the
  symbol of the invocation's callee expression
  (`services/signatureHelp.ts`, `createSignatureHelpItems`:
  `callTargetSymbol = typeChecker.getSymbolAtLocation(
  getExpressionFromInvocation(invocation))`, displayed with
  `symbolToDisplayParts`), which is the generated `const`.
- **Alternatives considered**: (a) Rewrite the label's prefix: the label is
  display text, the parameter spans index into it and would have to be
  shifted, and whether a prefix is a name at all (`(two)(...)` and
  `[two][0](...)` have none) is TypeScript's decision. (b) Stop storing a
  plain-identifier callee in codegen: the store is what keeps ECMAScript's
  evaluation order (the callee is evaluated before the arguments, and an
  arm may reassign it), so the editor would be changing the program to
  change a display. (c) Follow the stored name to its initializer: the
  emission's mappings say which bytes were copied, not which glue binding
  holds which source expression, so this would need a second model of the
  lowering.
- **Decision and rationale**: The engine asks where the source call's
  callee is the source's own name, as TASK-607 and TASK-610 ask questions
  served for one request. After TASK-603 chose the user's invocation, the
  engine reads the identifier that invocation's callee ends with, before
  its argument list and past a type-argument list (the ECMAScript
  `CallExpression`/`NewExpression` shape `MemberExpression TypeArguments?
  Arguments`), in the served text and, through the emit mapping of the
  `(`, in the source. When the served identifier was not copied from the
  source and the source's is an identifier, the question is the served
  text with the source's name in its place; TypeScript then resolves the
  same symbol the source call names, in the same scope (the emission keeps
  the call where the user wrote it and adds only `$tt_` names). The
  question is served for the request and the projection is served back
  before the answer is read. Where the source callee ends in no name
  (`(two)(...)`) or its name was copied (`obj.m(...)`, whose receiver alone
  is stored), nothing changes, and TypeScript answers as it answers for the
  source.

## Work log

- 2026-09-30: Reproduced with the probe harness (`cmp.cjs w/sh5.tt
  w/sh5.ts s`): `two(...)`, `two<number>(...)`, `new Cls(...)`, and a call
  nested in `new Cls(1, two(...))` answered `$tt_vN(...)`; `(two)(...)`
  and `ns.two(...)` matched the twin. Read the emission (`ttc -p`): the
  callee is stored as `const $tt_vN = (two);` and called as `$tt_vN(`.
- 2026-09-30: Added `signature_question` and used it in `signature_help`.
  Re-ran the harness: every marker but the optional call matches the twin.
- 2026-09-30: Test: `signature_help_names_a_stored_callee_as_the_source_call_does`
  (`tests/native/editor_service.rs`) compares each case with its `.ts`
  twin, including a cursor in whitespace (the probe path), type arguments,
  a parenthesized callee, a template interpolation, and a nested call.
  With the question disabled it fails with `left:
  Some(("$tt_v1(a: number, b: string): number", 1))`, `right:
  Some(("two(a: number, b: string): number", 1))`. The adapter passes the
  engine's answer through unchanged, so the engine test is the contract.

## Issues and resolutions

### Issue 1: An optional call with a lowered argument answers nothing

- **Symptom**: `two?.(match (s) { ... }, |)` answers no signature help;
  the twin answers `two(...)`.
- **Cause**: The emission lowers the optional call to a null test around a
  call it writes itself, so the call's `(` is not copied and TASK-603 moves
  past it, and no source invocation encloses the position.
- **Resolution**: Left open. It is a question of which invocation is the
  user's when the emission rewrites the call itself, not of the callee's
  name, and belongs with TASK-603's invocation model.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native signature`, `cargo test --lib signature`
- [x] Full gate (recorded in TASK-633, run once for TASK-629 to TASK-633)

## Result

Changed `src/engine/language/service.rs`, `src/engine/language/project.rs`,
`tests/native/editor_service.rs`, `docs/design/lsp-architecture.md`, and the
task index. Signature help names a stored callee as the source call names
it.
