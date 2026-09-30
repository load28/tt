# TASK-610: Go to a built-in tag's or field's declaration in the standard library

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-610: Go to a built-in tag's or field's declaration in the standard library`

## Purpose

Go to definition answered nothing on the built-in `Option`/`Result` names in
patterns: `Some`, `None`, `Ok`, `Err`, and the fields `value` and `error`,
in `match` arms, let-else, and `if let`. A local variant's tag and field go
to their declaration, and TypeScript's `const { error } = r` goes to
`error: E` in `@tt/std/result.ts`.

## Scope

- Included: The adapter's definition handler
  (`editors/vscode/server/src/server.ts`), the engine's definition of a
  built-in case (`Project::builtin_case_definition`), and the standard
  library module that constructs a built-in variant
  (`StdModule::constructing`).
- Excluded: References and rename of built-in names (rename stays refused,
  TASK-611), hover (unchanged).

## Decisions

### Decision 1: A tt name without a declaration of its own is the engine's definition question

- **Context**: The adapter asked `ttSymbol` first and returned null for any
  tt name without a `definition`, with the comment that a built-in case has
  no declaration to open. The built-in variants are declared by the
  standard library the engine materializes in `node_modules/@tt/std`, which
  the service resolves.
- **Alternatives considered**: Give `ttSymbol` a definition for built-ins:
  it is parse-only and answers without a toolchain, so it cannot know where
  the materialized library is or where in it a name is declared without
  reading TypeScript text by shape.
- **Decision and rationale**: A tt name with a definition goes there as
  before; one without falls through to the engine's `definition`. A field
  is copied into the destructuring its pattern lowers to (`const { value }
  = $tt_m`), so the service answers it as it answers `const { error } = r`:
  the property in the standard library's type.

### Decision 2: A built-in case goes to the standard library export that constructs it

- **Context**: A tag lowers to a string literal the emission writes
  (`case "Ok":`), with no source counterpart, and TypeScript itself answers
  nothing for a discriminant literal (`r.kind === "Ok"` has no definition).
- **Alternatives considered**: (a) The case's union member (`{ kind: "Ok";
  value: T }`): finding it would mean searching the library's text for a
  literal, a string-shape reading. (b) Nothing, as TypeScript: a local
  variant's tag goes to its declaration, so a built-in one should too.
- **Decision and rationale**: A tag's declaration in tt is the case, which
  is also its constructor (`variant Shape { Circle(r: number) }` declares
  `Shape.Circle`). The standard library declares each built-in case's
  constructor as an export named by the tag (`export const Ok` in
  `@tt/std/result`), and `StdModule::constructing` names the module of each
  built-in variant (the pairing `resolve::builtin_variants` already
  documents). When the analysis resolves the position to a case whose
  origin is `Builtin`, the engine asks the service for the definition of
  that export through a question served for the request only: the
  projection followed by `type $tt_probe = typeof
  import("@tt/std/result").Ok;`. The answer is TypeScript's, mapped like any
  other definition.

## Work log

- 2026-09-30: Reproduced with the probe harness (`gd3.tt` against `gd2.ts`):
  every built-in tag and field answered `[]`; TypeScript answered
  `result.ts` for `const { value } = r` and `const { error } = r`, and
  nothing for the literal `"Ok"`.
- 2026-09-30: Removed the adapter's early null, added
  `builtin_case_definition` and `StdModule::constructing`.
- 2026-09-30: Re-ran the harness: `Ok`/`Err` go to `result.ts`, `Some`/`None`
  to `option.ts`, and `value`/`error` (shorthand, aliased, let-else) to the
  field in the library's type.
- 2026-09-30: Tests:
  `a_builtin_tag_or_field_goes_to_its_declaration_in_the_standard_library`
  (`tests/native/editor_service.rs`: match arms, an aliased field, let-else,
  `if let`) and the extension test "a built-in tag and field go to the
  standard library" (`server.test.ts`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native a_builtin_tag`
- [x] `node --test server/out/test/server.test.js` (definition cases)
- [x] Full gate (recorded in TASK-613, run once for TASK-606 to TASK-613)

## Result

Changed `src/stdlib.rs`, `src/engine/language/project.rs`,
`tests/native/editor_service.rs`, `editors/vscode/server/src/server.ts`,
`editors/vscode/server/src/test/server.test.ts`,
`docs/design/lsp-architecture.md`, and the task index. Built-in tags and
fields go to their declarations in `@tt/std`.
