# TASK-590: Write no declarations from placeholders, and type a malformed variant as the error type

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: `TASK-590: Write no declarations from placeholders, and type a malformed variant as the error type`

## Purpose

With `ttc --types`, a malformed variant (`export variant X { A, 1 }`) was
reported as `malformed-variant`, but the run still replaced a correct
`.tt-types/x.tt.d.ts` with the recovery placeholder `export declare class
X {}`. Importers got a follow-on TS2339 (`Property 'A' does not exist on
type 'typeof X'`) and `any` in their own declarations
(`export declare const a: any;`), which were written too.

## Scope

- Included: the variant recovery placeholder (`src/lib/compile.rs`, with
  the type parameter list carried by `RecoveryKind::VariantDecl` from
  `src/parser/variants.rs`, `src/ast.rs`), which declarations are matched
  for writing (`src/engine/semantics/declarations.rs`), `docs/ai/tt.md`,
  a regression test.
- Excluded: importers of a *blocked* file (TypeScript that does not
  parse), which is still served as `export {}` and reported as TS2305 at
  the import; only their declarations change here (Decision 2).

## Decisions

### Decision 1: The placeholder of a malformed variant is the error type

- **Context**: A variant declares a type and a value under one name. The
  placeholder was `export class X {}`, overwritten in place when it fit
  (else `;`): a real, empty type, so every use of a case was a checker
  error about the placeholder. Expression recovery already states the rule
  this breaks: "the placeholder is `any` wherever it fits, so the code that
  uses the recovered value has no checker consequence of it", which is
  TypeScript's own treatment of what it could not resolve (the checker's
  `errorType` is an intrinsic `any`, so one error does not cascade).
- **Alternatives considered**: (a) Suppress importers' diagnostics that
  mention the placeholder: reading prose, and a diagnostic filter the
  project rules forbid. (b) An in-place `any`-typed form that declares
  both meanings (`var X:any;type X=any`, an import from an unresolvable
  module): longer than the shortest malformed variant, so the width of
  the span cannot hold it. (c) `class X { static [k: string]: any }`:
  still a type with members, still longer.
- **Decision and rationale**: The variant's span becomes `;` and the
  projection appends `declare const X: any;` and
  `type X<its parameters as written> = any;` (with `export` when the
  variant was exported) after the module, as glue anchored to the
  variant's source range (`EmitAnchor`, `AnchorKind::Variant`). Appending
  does not move any mapped byte, so the width no longer bounds the
  placeholder, and a diagnostic in that glue is owned by the recovered
  range and dropped as a recovery effect. The parameter list keeps
  `X<number>` in an importer from becoming TS2315. A name that is a
  reserved word gets no declaration.

### Decision 2: Declarations are written only from a program without placeholders in reach

- **Context**: TASK-561 writes no declarations from an unparsed
  projection. A projection with recovery placeholders is no more the
  file's own account, and an importer's declarations are emitted against
  whatever the placeholder says (`any` now, an empty class before).
- **Alternatives considered**: (a) Withhold only the recovered file's own
  declarations: the importer's `any` would still be published and
  silently weaken every consumer. (b) Withhold every declaration of the
  run whenever any file has a placeholder: keeps unrelated files' sidecars
  stale for no reason, against the per-file write contract (TASK-509).
- **Decision and rationale**: `match_declarations` leaves out every file
  that is unparsed, has recovery placeholders, or is blocked, and every
  file whose `.tt` imports reach one (the language's own module graph,
  as `typed_member_sources` walks it). Their previous sidecars stand, and
  `--json-report` does not list them as written. A `.ts` file between two
  `.tt` files is not followed; recorded below.

## Work log

- 2026-09-30: Reproduced with `target/probe5-cli/ty2`: `x.tt.d.ts` became
  `export declare class X {}`, `use.tt.d.ts` got `a: any`, and TS2339 was
  reported in `use.tt`.
- 2026-09-30: Replaced the in-place class with appended error-typed
  declarations (`declare_recovered_variants`) and added
  `reaches_placeholders`. The generic case (`variant X<T> { A(v: T), 1 }`
  imported as `X<number>`) reported TS2315 until the parameter list was
  carried by the recovery node.
- 2026-09-30: Added
  `types_keep_declarations_written_before_a_variant_stopped_parsing`
  (`tests/cli.rs`): one diagnostic, both sidecars unchanged, only
  `ok.tt.d.ts` written.

## Issues and resolutions

### Issue 1: Importers of a generic malformed variant reported TS2315

- **Symptom**: `error[ts2315]: Type 'X' is not generic.` in the importer.
- **Cause**: The error type was declared without the variant's type
  parameters.
- **Resolution**: `RecoveryKind::VariantDecl` carries the parameter list
  as written when it is balanced, and the type alias takes it.

## Remaining debt

- A declaration of a file that reaches a placeholder only through
  hand-written TypeScript (`a.tt` → `b.ts` → `x.tt`) is still written; the
  engine does not hold TypeScript's own module graph.

## Verification

- [x] `cargo test --lib --test content_mapper --test snapshot --test compile`
- [x] `cargo test --test cli types_keep_declarations` (with `TTC_REQUIRE_TSGO=1`)
- [x] Full gate and extension tests run once at the end of the
  TASK-587–592 series; see TASK-592.

## Result

Changed `src/ast.rs`, `src/parser/variants.rs`, `src/lib/compile.rs`,
`src/engine/semantics/declarations.rs`, `docs/ai/tt.md`, and
`tests/cli.rs`.
