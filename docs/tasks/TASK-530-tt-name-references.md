# TASK-530: Find every reference to a tt variant, case, or payload field

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: —

## Purpose

Find All References was empty on a variant, case or field declaration and on
a pattern tag, and from a use such as `Color.Green(2)` it missed every
pattern that matches the case. A TypeScript developer gets every reference
to a declaration from any of its occurrences.

## Scope

- Included: `Project::references` and its service-position lookup
  (`src/engine/language/project.rs`, `src/engine/language/service.rs`), the
  pattern reference search (`src/engine/names.rs`), and the extension's
  comment on the delegation.
- Excluded: Renaming tt names, which stays refused (TASK-062 and the
  extension's `onRenameRequest`).

## Decisions

### Decision 1: Ask TypeScript at every place the emission declares a tt name

- **Context**: A tt declaration has no verbatim mapping, so
  `to_service_name` found no service position and the request was never
  made. The emission already records where it declares each name
  (`DeclaredName`: the variant's type alias and constructor object, a case's
  constructor property, a field's union and parameter properties), and
  navigation answers already map back through them.
- **Decision and rationale**: `to_service_names` returns the verbatim
  position, or every declared-name position covering the source position;
  `locations` asks at each and unions the answers. A variant's references
  therefore include both its type uses and its constructor uses.

### Decision 2: Pattern references come from tt's own resolution, keyed by the declaration

- **Context**: Pattern tags and fields lower to string literals and
  destructuring keys, which TypeScript does not relate to the declaration.
- **Alternatives considered**: Matching names textually would join two
  variants that share a case name. Asking the checker is impossible for
  the reason above.
- **Decision and rationale**: A tt name's identity is its declaration's
  location. `tt_declaration` finds it from the position (`tt_symbol_at`), or
  from TypeScript's definition when that lands on a tt declaration (a use in
  a `.ts` file). `tt_pattern_references` walks one file's resolved pattern
  names and keeps those whose definition — computed exactly as
  go-to-definition computes it — is that location. `references` unions the
  declaration, TypeScript's references asked at the declaration, and the
  pattern references of every project `.tt` file (open buffers included),
  merging duplicates.

## Work log

- 2026-09-29: Reproduced with `lib.tt` (`variant Color`), `use.tt` and
  `use.ts`: references at the declarations were empty and at `Color.Green(2)`
  missed both patterns. Implemented Decisions 1 and 2; the same probe now
  answers the declaration, both constructor calls and both patterns for the
  case from any of them, and the declaration and both patterns for the
  field.
- 2026-09-29: A shorthand pattern binding (`Green(level)`) keeps the
  binding's own references and adds the field's, matching TypeScript's
  answer for shorthand destructuring.
- 2026-09-29: Added
  `references_to_a_tt_name_reach_its_declaration_its_patterns_and_its_typescript_uses`
  (`tests/native/cases_06.rs`) and the LSP case "a case tag's references
  reach its declaration and every pattern" (`server.test.ts`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `editors/vscode`: all server and client tests; 222 of 223 passed while
  `cargo test` ran beside them, and the one failure (the sidecar re-arm
  case, which waits two seconds for a publish) passed twice alone. TASK-531
  removes that window.

## Result

Changed `src/engine/language/project.rs`, `src/engine/language/service.rs`,
`src/engine/names.rs`, `editors/vscode/server/src/server.ts`,
`tests/native/cases_06.rs` and `editors/vscode/server/src/test/server.test.ts`.
Find All References on a variant, case or field answers from its
declaration, any pattern, or any TypeScript use.
