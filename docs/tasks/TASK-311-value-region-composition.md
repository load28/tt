# TASK-311: Repair reported value-region composition defects

- **Status**: Complete
- **Started**: 2026-09-01
- **Completed**: 2026-09-01
- **Commit**: `TASK-311: repair value-region composition`

## Purpose

Repair the 17 compiler and content-mapper defects reported from the `tt-demo`
application against the latest `main`. Eliminate panics, invalid output, silent
semantic changes, and generated-code type errors without construct-specific
fallbacks or diagnostic suppression.

## Scope

- Included: statement and nested `match`, JSX-hosted values and pipelines,
  `result` contextual typing and composition, embedded `try`, async concise
  arrows, generator owners, tuple comparison subjects, and content-mapper
  standard-library resolution
- Included: the duplicate missing-case observation from the report
- Included: compiler, checker, runtime, source-preservation, and content-mapper
  regression coverage
- Excluded: release/version changes and unrelated language features

## Decisions

### Decision 1: Model nested values in the evaluation plan

- **Context**: Reports 1, 4, 6, 7, 12, 14, and 16 failed when two independently
  valid value regions shared one host expression or statement.
- **Alternatives considered**: Hoist specific syntax pairs in codegen, use an
  expression-boundary closure, or preserve nested schedules, exits, values, and
  relocations in Evaluation IR.
- **Decision and rationale**: Extend nested region placement and the lowering
  plan with the host protocol and exits that the nested value already owns.
  Target emission now composes that plan once. This keeps evaluation order,
  failure edges, and source preservation in the layer that owns them.

### Decision 2: Carry TypeScript host facts instead of reconstructing them

- **Context**: Reports 2, 3, 5, 8, 9, 11, and 13 lost JSX child identity,
  contextual types, async return semantics, or single-statement ownership.
- **Alternatives considered**: Recognize generated text after emission, add
  source-text special cases, or collect the facts from the parsed TypeScript
  host AST.
- **Decision and rationale**: ProgramSyntax now carries JSX child mode,
  contextual annotations, async awaited-return context, and whether an expanded
  exit requires a block. Generator and statement owners use the same capability
  model as ordinary functions. Isolated block-arm propagation is diagnosed from
  Core IR ownership rather than token adjacency.

### Decision 3: Preserve Result success typing and checker probes together

- **Context**: Removing an untyped Result temporary fixes empty-literal
  inference but would remove the checker probe that detects an already-Result
  success value.
- **Alternatives considered**: Drop the probe, special-case empty literals, or
  type the success temporary from the authored Result context.
- **Decision and rationale**: A contextually typed Result slot exposes its
  success type through `Extract<typeof slot, { kind: "Ok" }>["value"]`. The
  temporary retains its probe marker, so contextual literals and the existing
  nested-Result diagnostic both remain checker-proven.

### Decision 4: Keep syntax boundaries structural

- **Context**: Reports 10 and 15 came from scanners treating JSX raw text and
  comparison operators as expression or generic syntax.
- **Alternatives considered**: Match the reported spellings, or repair the
  token-boundary rules.
- **Decision and rationale**: Pipeline scanning stops at JSX raw boundaries,
  and tuple subjects count only structurally closed type-argument angles as
  nesting. Parenthesized declaration-form `try` operands are claimed by their
  declaration grammar rather than by a broad expression fallback.

## Work log

- 2026-09-01: Ran `./scripts/doctor`; the pinned Rust and TypeScript toolchains
  and local compiler artifacts were ready.
- 2026-09-01: Fast-forwarded `main` from `8dd6351` to `1d783b4` and created
  `fix/tt-value-region-composition`.
- 2026-09-01: Added direct regressions for all 16 compiler reports and the
  duplicate-case observation.
- 2026-09-01: Extended ProgramSyntax, Evaluation IR, and target emission with
  nested schedules, exits, contextual types, JSX child identity, and expanded
  statement ownership.
- 2026-09-01: Preserved Result return-shape probes while giving success values
  their authored contextual type.
- 2026-09-01: Confirmed that current `main` already materializes `@tt/std` and
  `@tt/runtime` for content-mapper projects; retained its no-`paths` integration
  contract and added contextual-literal coverage through real TypeScript 7.1.
- 2026-09-01: Normalized duplicate variant constructors in the semantic
  alphabet so coverage messages and suggested arms contain each case once.
- 2026-09-01: Passed the complete repository gate across agent contracts, Rust,
  npm packages, native TypeScript integration, and the VS Code extension.

## Issues and resolutions

### Issue 1: Hosted value regions were lowered independently

- **Symptom**: Reports 1, 4, 6, 7, 12, 14, and 16 produced ICEs, duplicate
  emission, leaked returns, fallback union pollution, or slot collisions.
- **Cause**: Nested regions discarded their host protocol and exits, while
  target emission could independently rewrite both the parent and child source
  ranges.
- **Resolution**: Nested regions retain their schedules, exits, values, and
  relocation spans. Target emission tracks active structured values and emits
  each owner rewrite once with collision-free plan slots.

### Issue 2: Host syntax facts were flattened too early

- **Symptom**: Reports 2, 5, 9, 10, 11, and 13 changed JSX children into text,
  leaked `break`, missed async concise bodies, crossed JSX, rejected generators,
  or panicked on block-arm propagation.
- **Cause**: Lowering knew only a generic value span, not whether it was a JSX
  child, a single-statement body, an async return, or an isolated arm block.
- **Resolution**: The TypeScript host model carries those distinctions through
  lowering. Emission wraps expanded single statements, restores JSX children as
  containers, and reports unsound arm propagation from Core ownership.

### Issue 3: Generated slots erased contextual typing

- **Symptom**: Reports 3, 8, and 12 widened empty arrays and literals or added an
  unreachable `Ok(undefined)` member.
- **Cause**: Unannotated generated bindings interrupted TypeScript contextual
  typing, and expression boundaries always emitted a fallback success return.
- **Resolution**: Authored annotations flow to owner slots, async returns use
  `Awaited<...>`, Result success temporaries derive the success member type, and
  Core flow completeness suppresses unreachable fallbacks.

### Issue 4: Token boundaries conflated host constructs

- **Symptom**: Reports 10 and 15 parsed through JSX or treated `<` comparison
  operators as generic nesting.
- **Cause**: The scanners lacked explicit JSX raw boundaries and structural
  angle closure.
- **Resolution**: JSX raw tokens terminate pipeline scanning, and tuple
  splitting recognizes only closed type-argument forms as nested angles.

### Issue 5: Standard modules and duplicate constructor alphabets

- **Symptom**: Report 17 required `paths` in the older nightly, and duplicate
  cases appeared twice in missing-case text and edits.
- **Cause**: The reported nightly preceded standard-package materialization;
  the semantic constructor alphabet retained declarations already rejected as
  duplicates.
- **Resolution**: The latest-main content mapper materializes standard packages
  at the project root and passes the real TypeScript integration test without
  `paths`. Semantic alphabets now keep the first constructor of each name once.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --test compile` — 387 passed
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test content_mapper` — 12 passed
- [x] `cargo test --test native` — 62 passed
- [x] `cargo test` — all unit, integration, snapshot, and documentation tests passed
- [x] `./scripts/ci` — `agents`, `rust`, `npm`, `native`, and `extension` passed

## Changed files

- `src/analysis/mod.rs`
- `src/ast.rs`
- `src/codegen/core.rs`
- `src/core_ir/lower.rs`
- `src/core_ir/mod.rs`
- `src/engine/projection.rs`
- `src/evaluation_ir.rs`
- `src/flow/mod.rs`
- `src/hir/lower.rs`
- `src/hir/mod.rs`
- `src/lib.rs`
- `src/parser/matches.rs`
- `src/parser/mod.rs`
- `src/parser/pipes.rs`
- `src/parser/results.rs`
- `src/parser/tries.rs`
- `src/program_syntax.rs`
- `src/sema.rs`
- `tests/compile.rs`
- `tests/content_mapper.rs`
- `tests/fixtures/emit/match-arm-blocks-and-guards/expected.ts`
- `tests/native.rs`
- `docs/tasks/INDEX.md`
- `docs/tasks/TASK-311-value-region-composition.md`

## Result

All reported compiler defects have structural regression coverage and pass the
complete local CI gate. The content-mapper standard-library issue was already
resolved on the updated `main`; its no-`paths` contract remains covered by the
real TypeScript 7.1 integration suite.
