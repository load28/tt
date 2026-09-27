# TASK-432: Reach the host `Error` and `JSON` past user declarations in generated guards

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

A match's unexpected-value guard is emitted as `throw new Error("tt match: ..." + JSON.stringify(value))`. In a file that declares its own `Error` (for example `variant Error { ... }`, a class, or an import) or `JSON`, the guard called the user's binding: `TypeError: Error is not a constructor` at runtime and TS2351 in generated code under `--check-types`. Generated glue must not depend on user bindings (docs/ai/tt.md contract 2).

## Scope

- Included: The host globals the emitter references (`Error`, `JSON`), the program-syntax declaration scan, and the lowering plan.
- Excluded: `undefined`, which TypeScript does not allow a program to redeclare (TS2397).

## Decisions

### Decision 1: Qualify a host global only when the file declares its name

- **Context**: ECMA-262 resolves an identifier through the lexical environment, so any declaration of `Error` or `JSON` in scope captures the guard; `globalThis` (ECMA-262 §19.1.1) always names the global object.
- **Alternatives considered**: Always writing `globalThis.Error` would change every file's output; renaming user bindings is not the compiler's to do.
- **Decision and rationale**: `ProgramSyntax::declared_names` collects every binding the file declares (variables, parameters, functions, classes, imports, enums, namespaces, import-equals) from the TypeScript projection, and the lowering plan adds tt `variant` names. The guard uses `globalThis.Error` / `globalThis.JSON` exactly for declared names; other files are byte-identical.

## Work log

- 2026-09-27: Reproduced with Node; implemented the declaration scan and the plan field; verified both shadowed names at runtime. Added an integration test (fails before, passes after); snapshots unchanged.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed, snapshots unchanged.

## Result

Changed `src/program_syntax/projection.rs`, `src/evaluation_ir.rs`, `src/evaluation_ir/evaluation.rs`, `src/codegen/core/planning.rs`, `src/codegen/core/mod.rs`, `src/codegen/core/emitter/mod.rs`, `src/codegen/core/emitter/pattern.rs`, and `tests/integration.rs`.
