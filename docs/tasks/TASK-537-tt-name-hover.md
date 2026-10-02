# TASK-537: Hover a pattern binding by its type and a tt name with its JSDoc

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

Two hover gaps on tt names. A shorthand pattern binding over a generic
payload (`Some(value)` on a `TOption<number>`) hovered as the field's
declared `value: T`, where TypeScript shows `const value: number` for
`const { value } = o`. And a variant, case or field hovered without the JSDoc
the user wrote on it, which TypeScript shows on a documented declaration.

## Scope

- Included: The service hover's positions (`Project::service_hover`,
  `declared_name_at`) and the extension's `onHover`.
- Excluded: The text of tt's own hover lines (`signature`, `detail`).

## Decisions

### Decision 1: A name that is also a binding hovers as the binding

- **Context**: `onHover` asks `ttSymbol` first and shows its declaration
  whenever it claims the position. For a shorthand pattern the symbol
  reports `binds`, and the engine's hover already answers the binding with
  its instantiated type (TASK-463 classified the name, not its type).
- **Decision and rationale**: When `binds` is set, the service hover wins;
  tt's declaration hover remains the fallback when the service has no
  answer (no toolchain).

### Decision 2: Documentation comes from TypeScript at the declaration

- **Context**: TASK-468 and TASK-479 carry a variant's, case's and field's
  JSDoc onto the names the emission declares, so TypeScript knows them. The
  service hover could not be asked at a declaration, which has no verbatim
  mapping.
- **Alternatives considered**: Reading comments out of the tt AST in the
  hover path would repeat the attachment rules the emission already
  implements.
- **Decision and rationale**: `service_hover` asks at each declared-name
  position (`to_service_names`, TASK-530) when the position has no verbatim
  mapping, and answers over the name as written (`declared_name_at`). The
  extension adds that answer's documentation to a tt name's hover by asking
  at the symbol's definition. An untitled buffer's definitions are in its
  text and are not asked.

## Work log

- 2026-09-29: Reproduced both with the editor probe's cases. After the
  change, engine hover at the variant, case and field declarations of a
  documented `variant` answers "A shape.", "A round one." and "The radius in
  cm.". Added the LSP case "a pattern binding hovers with its instantiated
  type, a documented case with its JSDoc".

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `editors/vscode`: all server and client tests, 225 passed

## Result

Changed `src/engine/language/project.rs`, `src/engine/language/service.rs`,
`editors/vscode/server/src/server.ts` and
`editors/vscode/server/src/test/server.test.ts`. Pattern bindings hover with
their instantiated types, and tt names show the JSDoc written on their
declarations.
