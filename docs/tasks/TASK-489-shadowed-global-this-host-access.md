# TASK-489: Reach host globals when `globalThis` itself is shadowed

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

TASK-432 and TASK-471 write `globalThis.Error`, `globalThis.JSON`, and
`globalThis.String` in generated match guards when the file declares its
own `Error`, `JSON`, or `String`. That qualification assumed `globalThis`
always names the global object. In
`function f(o: any, globalThis: any) { const Error = 5; return match (o) { … }; }`
the fallback `throw new globalThis.Error(…)` resolved to the parameter and
failed with `TypeError: globalThis.Error is not a constructor`.

## Scope

- Included: The declared-name facts in the program syntax model
  (`src/program_syntax/projection.rs`), the host-global decision in the
  lowering plan (`src/evaluation_ir.rs`, `src/evaluation_ir/evaluation.rs`),
  and the module-top prelude in codegen (`src/codegen/core/mod.rs`,
  `src/codegen/core/emitter/mod.rs`, `src/codegen/core/emitter/pattern.rs`).
- Excluded: A file that declares both `globalThis` and the host name
  (`Error`, `JSON`, or `String`) at module scope. No identifier in that
  module resolves to the intrinsic (ECMA-262 §9.1 environment records;
  §19.1.1 `globalThis` is an ordinary global property that a module binding
  shadows), and the only other routes (`Function("return this")`, catching
  a thrown `TypeError`) depend on CSP or on engine errors. Such a file keeps
  the TASK-432 output, which `--check-types` reports as `ts2351` on the
  generated guard. Reporting it as a tt diagnostic is a possible follow-up.

## Decisions

### Decision 1: Capture the intrinsic once at module top, under a generated name

- **Context**: A declaration of `globalThis` anywhere in the file may
  shadow it at a guard. `ProgramSyntax::declared_names` is not
  scope-aware, and a precise per-site resolution would need a scope model
  the emitter does not have.
- **Alternatives considered**: (a) Always write `globalThis.X`: already
  wrong for this input. (b) Export host intrinsics from `@tt/runtime` and
  import them. That works even for module-scope shadowing, but it adds a
  runtime dependency, and build materialization, to files that use no
  pipeline, and changes the runtime module's contract. (c) Write
  `const $tt_Error = globalThis.Error;` at module top, where no function
  parameter or local can shadow `globalThis`, and use that alias at the
  guards.
- **Decision and rationale**: (c). When the file declares `globalThis`
  (or a `variant globalThis`) and a used host name is shadowed, the plan
  allocates `$tt_Error` / `$tt_JSON` / `$tt_String` through the generated
  name allocator, so the alias never collides with a user name. The
  capture is `globalThis.X` when `globalThis` is not declared at module
  scope, and `X` when `globalThis` is but `X` is not. Files that do not
  declare `globalThis` are byte-identical to before.

### Decision 2: A module-scope declaration fact from the parsed program

- **Context**: Choosing the capture needs to know which names the module
  scope binds.
- **Decision and rationale**: `ProgramSyntax::module_declared_names`
  reads the SWC module: top-level declarations (`let`/`const`/`var`
  patterns, functions, classes, enums, value namespaces, `using`), named
  default exports, imports, and import-equals, plus `var` declarations
  hoisted out of top-level blocks and loops. It does not descend into
  functions, arrows, classes, accessors, or namespace bodies, which have
  their own scopes. tt `variant` names count as module-scope.

### Decision 3: Write an alias only when its guard or helper is emitted

- **Context**: A file can shadow `globalThis` and `Error` without emitting
  any guard.
- **Decision and rationale**: The emitter records when it writes the
  guard's `Error` (`used_host_error`); `JSON` and `String` are used exactly
  when the `$tt_show` helper is emitted. The used aliases join the runtime
  import in one module-top prelude, written after the directive prologue
  (TASK-486).

## Work log

- 2026-09-28: Reproduced with `ttc -o` and Node (type stripping):
  `TypeError: globalThis.Error is not a constructor`.
- 2026-09-28: Added `module_declared_names`, the `HostGlobalAlias` plan
  fact, and the prelude. Checked runtime behavior for a parameter
  `globalThis`, a module-scope `globalThis` with a local `Error`, a nested
  `globalThis` with a module-scope `class Error`, and all three names
  shadowed; `ttc --check-types` accepts each output.
- 2026-09-28: Added `generated_guards_reach_the_host_globals_past_a_shadowed_global_this`
  (`tests/integration.rs`, runtime and type check) and
  `a_host_global_alias_is_captured_only_when_global_this_is_shadowed`
  (`tests/compile/cases_11.rs`, which also pins the unchanged outputs).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed, snapshots
  unchanged; the TASK-432 and TASK-471 tests still pass.

## Result

Changed `src/program_syntax/projection.rs`, `src/evaluation_ir.rs`,
`src/evaluation_ir/evaluation.rs`, `src/codegen/core/mod.rs`,
`src/codegen/core/emitter/mod.rs`, `src/codegen/core/emitter/pattern.rs`,
`tests/integration.rs`, and `tests/compile/cases_11.rs`.
