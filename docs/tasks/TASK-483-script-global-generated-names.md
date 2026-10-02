# TASK-483: Keep generated declarations of script files from colliding in the shared global scope

> TASK-518 resolves the runtime residual named under Scope: `ttc` now writes
> `tt/runtime.ts` only when an emitted output imports it, so a project of
> scripts no longer gets an unused runtime file.
>
> TASK-527 reverses Decision 3's "modules keep ... the trailing function
> declarations": a module now writes `$tt_show`, `$tt_raise`, and `$tt_expr`
> with its prelude, and a module's prelude goes after the file-level pragmas
> as a script's does.

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

Two `.tt` files with no `import` or `export` that each wrote a top-level
`match` failed to type-check together: `ttc --check-types`, `tsc -p .
--runExternalCode`, and `tsc` over `ttc -o` output all reported TS2451
`Cannot redeclare block-scoped variable '$tt_v0'` and TS2393 for the
duplicate `$tt_show`. TypeScript reads a file without a top-level `import` or
`export` as a script, and the top-level declarations of every script share
one global scope (TypeScript Handbook, "Modules"; ECMA-262 §16.1.7
GlobalDeclarationInstantiation). The compiler's generated storage and
helpers collided across files, so the emitted TypeScript was not error-free
(contract 2). A pipeline made it worse: its `@tt/runtime` import turned the
script into a module, and its globals disappeared for every other file.

## Scope

- Included: Script detection and the classification of a script's
  top-level statements from the parsed program (`src/program_syntax.rs`,
  `src/program_syntax/{collector,projection,visit}.rs`); the lowering plan
  that encloses or names the storage (`src/evaluation_ir.rs`,
  `src/evaluation_ir/evaluation.rs`, `src/generated_names.rs`); a grammar of
  TypeScript's file-level pragmas (`src/lexer/pragmas.rs`); helper emission
  for scripts (`src/codegen/core/{mod,planning}.rs`,
  `src/codegen/core/emitter/mod.rs`); `docs/ai/tt.md`,
  `docs/design/program-lowering.md` §4.4, `docs/design/pipeline-operator.md`,
  and a reversal note on TASK-212.
- Excluded: Module files, whose top level is private, keep their output
  byte for byte. `ttc`'s build still materializes `tt/runtime.ts` when any
  input uses a pipeline (`ModuleScan::uses_pipeline` is a fact about the
  source), so a project of scripts gets an unused runtime file. Two layout
  quirks of existing owner-block emission that scripts now reach more often
  are left as they were: a generated blank line inside an opened block keeps
  its indentation, and a lowered C-style `for` test closes its owner block
  on the loop's last line (`}\n}}`). Both are valid TypeScript.

## Decisions

### Decision 1: A script stays a script, decided from the parsed program

- **Context**: Any fix had to decide whether a script may become a module.
- **Alternatives considered**: (a) Emit `export {}` or keep the runtime
  import so every script with a tt construct becomes a module. It removes
  all collisions, but the file's declarations stop being global, and
  whether a file is a module would depend on which tt constructs it uses.
  (b) Keep each file the kind TypeScript reads it as.
- **Decision and rationale**: (b). `ProgramSyntax` reads it off the SWC
  module: a top-level `import`, `export`, `export =`, `import x =
  require(...)`, or `export` modifier makes a module, as TypeScript's
  `isFileProbablyExternalModule` does; `import x = N.M` does not.
  `import.meta`, and a top-level `await`, `for await`, or `await using`,
  also make a module, because only the module goal allows them. A file that
  TypeScript treats as a module through configuration only
  (`moduleDetection: "force"`, `.mts`, `jsx: react-jsx`) is lowered as a
  script, and that output is valid in a module too.

### Decision 2: Each top-level statement of a script gets the storage its declarations allow

- **Context**: A top-level statement's lowering declares storage (value
  slots, captures, inline subjects, a let-else temporary) that the
  statement reads. In a script that position is the global scope shared
  with every other script. The constraints: no user-visible semantic change
  (a `var` written in user code keeps its scope), no IIFE (tt.md guarantees
  match never emits one), modules unchanged, scripts stay scripts, and no
  generated global can collide.
- **Alternatives considered**, each checked with the pinned TypeScript 7.1
  (`tsc --strict --target es2022`) and node:

  | Option | Result |
  |---|---|
  | Compute a declaration's value in a function (`const total = (() => { ... })()`) | A user `var z` in the value becomes local: `typeof z` is `undefined` instead of `number`; also breaks tt.md's no-IIFE guarantee |
  | Split the declaration, assign in a block (`let total; { ...; total = v; }`) | `const` cannot be split. A split `let` reports TS7034/TS7005 when a closure reads it, and silently accepts `split = 3` where the unsplit `let` reports TS2322 |
  | Global `var` storage with one type in every script | Different per-site types give TS2403 (`Subsequent variable declarations must have the same type`); a shared `unknown`/`any` erases the user binding's type |
  | Salt names with a hash of the path or the content | The path differs between the CLI and the content mapper and is absent for `compile()`; identical files or a hash collision still clash, so uniqueness is probabilistic |
  | Report a tt error for this shape in scripts | Single-script programs that compile today would fail |
  | Wrap every top-level statement in a block | A block hides the `const`/`let`/`class` binding the statement declares |
  | Derive names from the global the statement declares | Unique by construction (below); nothing in user code moves |

- **Decision and rationale**: `ProgramSyntax` classifies each statement of a
  script's global statement list (`GlobalStatement`):
  - `Enclose`, a statement that declares no lexical global (an expression,
    a control statement, a `var` declaration, a `var` let-else, a lexical
    declaration that binds nothing): the prelude and the statement form one
    block, through the owner-block mechanism an unbraced `if` body already
    uses. A `var` in the block still binds the global.
  - `Binding(name)`, a statement that declares a lexical global (`let`,
    `const`, `using`, `class`, a function declaration, a `const`/`let`
    let-else that binds a name), with `name` its first binding. Its binding
    must stay global, so the storage stays at the top level, and every
    generated name its prelude declares there is `{stem}${name}`
    (`$tt_v0$total`, `$tt_t0$value`, `$tt_subject$total`), allocated through
    `generated_names` so it still avoids every identifier the file writes
    (`$tt_v0_1$total` when the file already names `$tt_v0$total`). This is a
    program-level uniqueness guarantee rather than a spelling convention:
    TypeScript rejects a second declaration of a block-scoped global anywhere
    in the program (TS2451), so a valid program declares `name` exactly once,
    and a name derived from that binding's identity cannot be derived by any
    other statement of any other script. A generated stem has no `$` after
    its first byte, so a derived name has one reading. The user's code in
    the value is emitted exactly where it was, so a `var` inside it keeps
    its global scope, and no function is introduced.
  A let-else is projected as a block, so its classification comes from the
  Core decision: its binding mode and the first name its pattern binds.

### Decision 3: A script declares its helpers as identical typed `var`s after its file-level pragmas

- **Context**: `$tt_show`, `$tt_raise`, and `$tt_expr` were trailing function
  declarations, a second script's copy is TS2393, and the `@tt/runtime`
  helpers cannot be imported without Decision 1's problem. Host-global
  aliases (TASK-489) were top-level `const`s.
- **Alternatives considered**: (a) Function declarations: duplicates across
  scripts are TS2393. (b) A helper per private scope that uses it: every
  function would carry a copy. (c) One `var` per helper, written the same way
  with the same explicit type in every script: TypeScript accepts repeated
  `var` declarations of one type across files (two scripts checked with no
  diagnostics), and JavaScript accepts repeated `var` declarations. TASK-212
  set (c) aside because the binding is global; in a script every
  declaration is global.
- **Decision and rationale**: (c) for scripts; modules keep the import, the
  trailing function declarations, and `const` aliases. A `var` is assigned
  where it is written: after its first top-level use, `$tt_ap(1, String)`
  throws `TypeError: $tt_ap is not a function`. So the helpers go before
  the first statement. Written between `// @ts-expect-error` and its
  statement, they would take over the directive (TS2578 unused directive,
  and the suppressed TS2322 reappears), and a JSDoc would document the
  helper instead. They therefore go after the directive prologue and the
  file-level pragmas and before the first statement's own leading comments.
  `crate::lexer::pragmas` recognizes the pragmas with the grammar TypeScript
  documents for what it reads only before a file's first token:
  - triple-slash directives `/// <reference ... />`, `/// <amd-module ... />`,
    `/// <amd-dependency ... />`, "only valid at the top of their containing
    file" (TypeScript Handbook, "Triple-Slash Directives");
  - `// @ts-check` and `// @ts-nocheck` at the top of a file (TypeScript
    Handbook, "Type Checking JavaScript Files"; TSConfig reference
    `checkJs`);
  - the JSX factory pragmas `@jsx`, `@jsxFrag`, `@jsxImportSource`,
    `@jsxRuntime` in a block comment (TSConfig reference `jsxFactory`,
    `jsxFragmentFactory`, `jsxImportSource`).
  It reports every leading comment with its pragma kind (`leading_comments`),
  and `after_file_pragmas` returns the start of the line after the last
  pragma, so other code can reuse it. `$tt_raise` keeps its `never` return
  through the explicit annotation, and `$tt_ap`/`$tt_fl` carry the types
  `src/stdlib/runtime.ts` gives them. This reverses TASK-212's decision for
  scripts; that record says so.

## Work log

- 2026-09-28: Reproduced with `src/a.tt`/`src/b.tt`: TS2451 for `$tt_v0`,
  TS2393 for `$tt_show`. Listed every generated top-level declaration of a
  script.
- 2026-09-28: First implementation (binding-derived names, text scan for
  pragmas) was withdrawn after review asked for structural fixes; a second
  one computed declaration values in functions (kept on the local branch
  `wt-483-iife-attempt`) and was withdrawn because it changed the scope of a
  user `var` and broke tt.md's no-IIFE guarantee. Collected the evidence in
  the table above; the user chose binding-derived names with the
  program-level uniqueness argument, and a pragma grammar in the lexer.
- 2026-09-28: Rebuilt on the branch head after TASK-482..490 and 492. Merged
  with TASK-486's parsed directive prologue (`module_import_position`) and
  TASK-489's host-global aliases (a `var` in scripts). Added script
  detection including top-level `await`, `GlobalStatement`, the name
  derivation in the lowering plan and for let-else temporaries,
  `src/lexer/pragmas.rs`, and the script helper prelude.
- 2026-09-28: Updated expectations of the compile tests whose snippets are
  scripts (top-level slots now carry the declared binding), made the
  runtime-import tests compile a module, adjusted the content-mapper and
  evaluation unit tests, and reviewed the nine updated emit snapshots:
  helpers move to the top as `var`s, statements without a lexical global
  are enclosed with their prelude, and slots of declarations carry the
  binding. User text is unchanged.
- 2026-09-28: Added regression tests: `lexer::pragmas` unit tests for every
  documented pragma and for statement comments that are not pragmas; a
  `program_syntax` unit test for the classification; compile tests for each
  statement class, a `var` in a declaration's value, let-else, name
  allocation around file identifiers, helper placement after a shebang,
  triple-slash directives, `@ts-nocheck`, `@ts-check`, a directive prologue,
  and `@jsxImportSource`, with a JSDoc, `@ts-expect-error`, or `@ts-ignore`
  still directly above the first statement, and which files are modules; an
  integration test that type-checks two scripts and a plain `.ts` consumer
  with `tsc` (including a `@ts-expect-error` first statement) and runs the
  emitted JavaScript in one global context with `vm.runInThisContext`,
  reading a `var` declared inside a top-level `const`'s match arm; a native
  test that checks a `.tt` and a `.ttx` script with a `.ts` consumer through
  `ttc --check-types`.

## Issues and resolutions

### Issue 1: A script's runtime import made it a module

- **Symptom**: A plain `.ts` script reading a global of a `.tt` script that
  used `|>` reported TS2304 `Cannot find name`.
- **Cause**: The pipeline's `import { $tt_ap } from "@tt/runtime"` is a
  module indicator.
- **Resolution**: Decisions 1 and 3.

### Issue 2: The comment-stripping pass removed a line of a test string

- **Symptom**: The integration test lost its `// @ts-expect-error` source
  line after removing comments the change had added.
- **Cause**: The pass matched lines by their leading text, and the line was
  a continuation of a Rust string literal.
- **Resolution**: Restored the line; the test asserts the directive is still
  directly above its statement in the output.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `./scripts/ci extension`
- [x] `UPDATE_EXPECT=1 cargo test --test snapshot`, diff reviewed
- [x] The two-script reproduction passes `ttc -o out` + `tsc` and
  `ttc --check-types`; the emitted scripts run in one shared global context.

## Result

Changed `src/program_syntax.rs`, `src/program_syntax/{collector,projection,visit,tests}.rs`,
`src/evaluation_ir.rs`, `src/evaluation_ir/{evaluation,tests}.rs`,
`src/generated_names.rs`, `src/lexer.rs`, `src/lexer/pragmas.rs`,
`src/codegen/core/{mod,planning}.rs`, `src/codegen/core/emitter/mod.rs`,
`src/content_mapper/tests.rs`, `tests/compile/cases_{01,02,05,06,07,10,11}.rs`,
`tests/integration/cases_03.rs`, `tests/native/cases_03.rs`, nine
`tests/fixtures/emit/*/expected.ts(x)` snapshots, `docs/ai/tt.md`,
`docs/design/program-lowering.md`, `docs/design/pipeline-operator.md`,
`docs/tasks/TASK-212-pipeline-helper-global-collision.md`, and the task
index. Scripts of one program no longer collide on generated declarations,
a script stays a script, user code keeps its scopes, and modules are
unchanged.
