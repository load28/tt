# TASK-503: Read contextual type and declaration words under TypeScript's lookahead rules

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

The token-facts machine treated `asserts` and `abstract` as type-operator prefixes wherever a type began, so a type named `asserts` or `abstract` swallowed the next line: in `.ttx`, `type asserts = number;⏎export let a: asserts⏎<p> val const text </p>` and `type A = abstract⏎<b>…</b>` had their JSX text silently changed. A sweep of the machine's other bare word lists found the same kind of misreading for `declare`, `namespace`, and `module` used as identifiers.

## Scope

- Included: The type-atom prefixes in `src/lexer/facts/types.rs`, and the `declare`, `namespace`, and `module` statement dispatch in `src/lexer/facts/statements.rs`.
- Excluded: Words whose current handling already matches TypeScript (see Decision 3).

## Decisions

### Decision 1: Type prefixes follow `parseTypeOperatorOrHigher` and `parseNonArrayType`

- **Context**: `Machine::type_atom` listed `keyof`, `typeof`, `readonly`, `unique`, `infer`, `asserts`, `new`, and `abstract` as prefixes unconditionally.
- **Alternatives considered**: Drop `asserts` and `abstract` from the list. That would misread `asserts x is T` and `abstract new () => T`.
- **Decision and rationale**: `Machine::type_operator` applies TypeScript's conditions. `keyof`, `unique`, and `readonly` (`parseTypeOperatorOrHigher`), `infer` (`parseInferType`), `typeof` (`parseTypeQuery`), and `new` are always prefixes; the pinned TypeScript confirms `type T = unique;` and `type T = infer;` are syntax errors, so they are never type names. `abstract` is a prefix only when the next token is `new` (`isStartOfFunctionTypeOrConstructorType`), and `asserts` only when the next token is an identifier or keyword on the same line (`nextTokenIsIdentifierOrKeywordOnSameLine`); the pinned TypeScript accepts `type T = abstract;` and reads `asserts⏎x` as a type reference followed by a statement.

### Decision 2: A constructor type or a type-parameter list commits the next `(` to parameters

- **Context**: With TASK-499's lookahead, `new (…)` and `<T>(…)` would have their `(` classified by `isUnambiguouslyStartOfFunctionType`, which TypeScript does not consult there: once `new`, `abstract new`, or `<` starts a type, `parseFunctionOrConstructorType` always parses a parameter list.
- **Decision and rationale**: `Type` gains `signature`, set by `new` and by a `<` in atom position, and the next `(` opens parameters when it is set.

### Decision 3: Sweep of the other contextual words

- **Context**: The task asked for a sweep of the machine's bare word lists against TypeScript's lookahead rules. Each candidate was checked against SWC through the facts oracle and, where it mattered, the pinned TypeScript.
- **Decision and rationale**:
  - `declare` began a declaration before any same-line word, so `declare instanceof C;`, `declare in o;`, and `declare as T;` were split in two. TypeScript's `isStartOfDeclaration` continues past `declare` only into a declaration keyword or another modifier; `declaration_follows` lists those tokens.
  - `namespace` and `module` began a declaration before any same-line word, so `namespace instanceof C;` and `module in o;` were split. TypeScript's `nextTokenIsIdentifierOrStringLiteralOnSameLine` requires an identifier (not a keyword) or a string.
  - Unchanged, because they already match TypeScript: `keyof`, `typeof`, `readonly`, `unique`, `infer` (always operators); `is` and `extends` (continue a type only on the same line); `in`/`out` variance and `const` type-parameter modifiers (inside a type group, where either reading gives the same facts); `accessor`, `declare`, `readonly`, and the other class-member modifiers (same line) and `static`, `get`, `set` (any line); type-literal `readonly`, `get`, and `set` (same line; the pinned TypeScript rejects `get⏎x(): T` as an accessor); parameter modifiers (same line); `satisfies` and `as` (same line); `global` (only after a modifier and before `{`); `async`, `abstract`, `type`, `interface`, and `using` at a statement start.
  - `let` as a sloppy-mode identifier followed by a keyword (`let instanceof C`) cannot appear in a module, and SWC's oracle parses modules only, so it is not changed without a way to test it.

## Work log

- 2026-09-28: Reproduced the reported `.ttx` inputs with the TASK-500 build and checked the pinned TypeScript's reading of `asserts`, `abstract`, `unique`, and `infer` as type names.
- 2026-09-28: Added `type_operator` and the `signature` flag (`src/lexer/facts/types.rs`).
- 2026-09-28: Ran a scratch oracle sweep over the words in Decision 3; `declare`, `namespace`, and `module` disagreed with SWC. Added `declaration_follows` and the identifier condition (`src/lexer/facts/statements.rs`); the sweep then agreed on every SWC-valid case.
- 2026-09-28: Added `a_contextual_type_or_statement_word_is_a_name_unless_typescript_reads_a_prefix` (`tests/compile/cases_14.rs`), `contextual_type_and_statement_words_pass_through` (`tests/passthrough.rs`), and four known shapes to the SWC oracle (`src/lexer/facts/tests.rs`), covering the names, the prefixes, generic and constructor signatures, and member modifiers across line breaks. With `abstract` forced back to a prefix, the compile test fails. Documented the rules in `docs/design/compiler-architecture.md`.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`: exit 0.
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0.
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: exit 0; 1547 tests, 0 failures.
- [x] `./scripts/ci extension`: exit 0; 210 extension tests passed, none skipped.
- [x] `scripts/check-task-index`: exit 0.
- [x] The facts oracle (`the_machine_reads_known_shapes_as_swc_does`, `the_machine_reads_the_corpus_as_swc_does`) agrees.

## Result

Changed `src/lexer/facts/types.rs`, `src/lexer/facts/statements.rs`, `src/lexer/facts/tests.rs`, `tests/compile/cases_14.rs`, `tests/passthrough.rs`, `docs/design/compiler-architecture.md`, and `docs/tasks/INDEX.md`; added this record. Contextual words in types and at statement starts are prefixes or modifiers exactly where TypeScript's lookahead makes them so.
