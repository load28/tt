# TASK-652: Execute case programs and hold their output to runtime baselines

> Issue 2's follow-up is done in [TASK-653](./TASK-653-contextual-materialization-reuse.md): a project reuses its last contextual materialization while its inputs are unchanged, and the suite's time figures below predate it.

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-652`

## Purpose

The case runner pins what ttc emits and what TypeScript says about it, but
nothing runs the emitted program. tt lowers `match`, `try`, `result`,
pipelines, let-else, and if-let into control flow, so a wrong evaluation
order, a double evaluation, a lost `this`, or a missed short circuit
type-checks and ships silently. TypeScript pins the same risk class with
evaluation tests that compile, execute, and assert on what the program
observed. This task adds that to the case runner as runtime baselines.

## Scope

- Included: `// @run: <unit>` in `tests/case_baselines.rs`, the
  `<name>.stdout` and `<name>.stderr` baselines, seed cases (three existing
  cases gain `@run`, three new companions under `tests/cases/compiler/`,
  seven evaluation cases under `tests/cases/conformance/evaluation/`),
  `CONTRIBUTING.md` ("Adding a test case"), `AGENTS.md` (the default
  regression test), and the parser fix for the one defect the seeds found
  (Issue 1: `src/lexer.rs`, `src/parser/{pipes,tries,lets,iflets,parse}.rs`).
- Excluded: the engine's per-request contextual materialization, which
  dominates the suite's new time cost (Issue 2; suggested as a follow-up
  task), and running cases in other runtimes than Node.js.

## Sources modelled

- microsoft/TypeScript `release-6.0` at `050880c`:
  `src/harness/evaluatorImpl.ts`, `evaluateTypeScript`: compile the source
  with the TypeScript compiler, fail when it reports diagnostics ("Syntax
  error in evaluation source text"), then execute the emitted JavaScript
  and return the module's exports. `src/testRunner/unittests/evaluation/`
  (`optionalCall.ts`, `usingDeclarations.ts`, `forAwaitOf.ts`,
  `asyncGenerator.ts`, `destructuring.ts`): each program pushes what it
  observed to an exported `output` array (`output.push(a); output.push(this)`,
  `"enter block"`, `"disposed"`), and the test asserts on the array.
- Node.js v22 documentation, "Command-line API" (`cli.html`), `--permission`
  ("v22.13.0: Permission Model is now stable"; file system, child process,
  worker, WASI, and addon access are restricted) and `--allow-fs-read`
  ("Multiple paths can be allowed using multiple `--allow-fs-read` flags";
  commas no longer delimit paths since v20.7.0).
- Node.js v22 documentation, "Modules: ECMAScript modules", "Mandatory file
  extensions" (a relative import must name the file's extension), and
  "Modules: Packages", "Determining module system" (a `package.json` with
  `"type": "module"` makes `.js` files ES modules).
- TypeScript's `rewriteRelativeImportExtensions` compiler option
  (TypeScript 5.7 release notes): a relative `.ts` import specifier is
  emitted as `.js`.

## Decisions

### Decision 1: A case opts into execution with `// @run: <unit>`

- **Context**: A runtime baseline needs a program that defines everything
  it calls and an entry to start.
- **Alternatives considered**: (a) Run every case whose emitted tree
  type-checks. Nearly every existing case uses `declare function` and
  `declare const` to reach a type or a diagnostic, which throws a
  `ReferenceError` at runtime, so most `.stdout` baselines would record
  noise, and each would pay the execution cost. (b) An entry chosen by
  convention (the first unit). Multi-file cases then cannot choose, and the
  choice is invisible in the case.
- **Decision and rationale**: An explicit `// @run: <unit>` naming a `.tt`,
  `.ts`, `.mts`, or `.cts` unit, as TypeScript's evaluation tests are an
  explicit, separate set. An unknown unit or an extension node cannot run
  fails the case at parse time. The directive does not vary; every
  configuration of a varied case runs and gets its own `.stdout`.

### Decision 2: Execute the pinned `tsc`'s JavaScript under Node.js

- **Context**: The emitted tree is TypeScript; something must turn it into
  JavaScript, offline and deterministically.
- **Alternatives considered**: (a) Node.js's type stripping on the emitted
  `.ts` files (unflagged in Node 22.18). It erases types only: enums,
  namespaces, and parameter properties (all valid in a `.tt` file) need
  `--experimental-transform-types`; `using` is not in Node 22's V8; and a
  `.js` specifier that ttc writes for a `.tt` import does not resolve to a
  `.ts` file. (b) TypeScript's own route, `evaluateTypeScript`: compile
  with the compiler, then execute.
- **Decision and rationale**: (b). The case's own `tsc -p tsconfig.json`
  already checks the emitted tree; a second `tsc` run in the same directory
  adds `--noEmit false`, `--rewriteRelativeImportExtensions` (so a
  `@rewriteImports: ts` tree runs too), `--rootDir .`, and an output
  directory `run/` in the case's workspace under `target/tt-tests`. The
  case's target decides the down-level emit, so `using` runs on Node 22
  through TypeScript's helpers (`usingDisposalOrder.tt`). `run/package.json`
  says `"type": "module"`, since the default configuration emits ES module
  syntax (`module: preserve`).

### Decision 3: Sandbox, environment, and timeout

- **Decision and rationale**: `node --permission
  --allow-fs-read=<run dir> <entry>.js`, with the run directory as the
  working directory: the program can read its own modules and nothing
  else, and cannot write, spawn processes, or start workers (a probe that
  called `writeFileSync` printed `ERR_ACCESS_DENIED`). Node 22's permission
  model does not cover the network (its `--allow-net` is later), so "no
  network" is the cases' own contract: nothing in a case reaches for one.
  The environment is emptied except `PATH` (and `SYSTEMROOT` where it
  exists) and `TZ=UTC`, so `NODE_OPTIONS` and the locale cannot change the
  output; stdin is closed. stdout and stderr go to files, and the runner
  polls the child and kills it after 10 seconds (a probe with an endless
  loop recorded `==== node zzProbeLoop.js timed out after 10 s ====` after
  printing its first line).

### Decision 4: `.stdout` always, `.stderr` only when there is something to say

- **Context**: TypeScript asserts on an exported `output` array. A baseline
  can hold what the program printed, so the assertion is the reviewed diff.
- **Alternatives considered**: Adopt the `output` array: a driver module
  would import the entry and serialize the array, which needs a
  serialization for values like `this` and functions, and moves the check
  out of the program's own text. Printing is what the existing runtime
  tests in `tests/integration.rs` read, and what a user sees.
- **Decision and rationale**: Cases print what they observe (the `output`
  convention, printed with `JSON.stringify` or template strings rather
  than `util.inspect`, whose formatting can change between Node versions).
  `<name>.stdout` is the program's stdout and exists for every `@run` case,
  even when empty. `<name>.stderr` exists only when stderr was not empty,
  the exit status was not 0, the run timed out, or the case was not run,
  and starts with a `==== node <entry>.js (exit N) ====` heading; it is
  compared with `compare_absent` otherwise, so a stale one fails and gets a
  `.delete` marker, as `.errors.txt` does. stderr drops Node's own frames
  (`at … (node:internal/…)`) and the `Node.js vX` line, and paths become
  `$DIR`, so a stack trace names only the emitted program's lines. Both go
  through `compare`/`compare_absent`, so tracking, `tests/baselines/local`,
  `.delete` markers, `scripts/baseline-accept`, and
  `scripts/check-baselines` cover them with no change there (the reference
  root's owner is already `case_baselines`). A case that is not `@run`
  records both as absent.

### Decision 5: A case that does not compile cleanly is not run

- **Context**: `evaluateTypeScript` refuses a source with diagnostics.
- **Decision and rationale**: When the case has an `.errors.txt`, `.stdout`
  is empty and `.stderr` is `==== not run: the case does not compile
  cleanly (see <name>.errors.txt) ====`. A regression that breaks
  compilation therefore shows in all three baselines rather than
  disappearing from the runtime ones. If `tsc` cannot emit JavaScript,
  `.stderr` records its report instead.

### Decision 6: Seeds

| Case | Pins |
| --- | --- |
| `compiler/declaratorListLaterValue.tt` (TASK-593, gains `@run`) | `t a`, then `f 11` and `g 11`: the later declarator runs after and sees the earlier one |
| `compiler/conditionalOperationNarrowing.tt` (TASK-595, gains `@run`) | a getter behind `cfg.name` is read twice (test and arm), not three times |
| `compiler/literalLeftOperandNarrowing.tt` (TASK-626, gains `@run`) | `false \|\|`, `null ??`, `undefined ??`, `0 \|\|` reach their `try`, `true &&` and `1 &&` their `match` |
| `compiler/forInitializerOperands.tt` (TASK-601) | a `for` initializer's `try`/`match` operand runs once, before the loop |
| `compiler/enumMemberResultBlock.tt` (TASK-594 companion) | a `result` block in an enum member reads the member `P` (7), not the outer `P` (100) |
| `compiler/literalMatchInEveryPosition.tt` (TASK-642 companion) | literal matches in the argument, element, and operand positions the repro used |
| `conformance/evaluation/matchEvaluationOrder.tt` | scrutinee once, guards in order, argument/operand/template order, `target().v += match` reads `v` first |
| `conformance/evaluation/tryPropagationOrder.tt` | sibling `try`s left to right, nothing after an `Err`, `result` scopes, `try` binding tightly |
| `conformance/evaluation/pipelineThisAndOrder.tt` | `this` in member steps, value before receiver, optional steps, `flow` running nothing until called |
| `conformance/evaluation/shortCircuitTtOperands.tt` | `&&`/`\|\|`/`??`/`? :`/`?.()` skip the tt value and its argument exactly where JavaScript does, `this` through `?.()` |
| `conformance/evaluation/asyncGeneratorTtConstructs.tt` | `await` in scrutinee, guard, and arm; interleaving with another task; `yield` in an arm and a scrutinee; `for await` over an async generator |
| `conformance/evaluation/usingDisposalOrder.tt` | reverse disposal around a later declarator's `match`, a block arm's own `using`, an arm that throws, and a propagating `try` |
| `conformance/evaluation/letElseIfLetDivergence.tt` | one evaluation of the initializer, `return`/`continue`/labeled `break`/`throw` exits, if-let chains, binding scope |

`compiler/enumMemberStatementValue.tt` (TASK-594) and
`compiler/mixedPatternsInEveryPosition.tt` (TASK-642) pin rejections and
emit nothing, so they cannot run; the two companions above run the
accepted forms of the same positions. Every `.stdout` was read line by line
against JavaScript's semantics for the source as written.

## Work log

- 2026-09-30: Reset onto `claude/ecstatic-dijkstra-qw5pf9` at `8d75506`,
  `npm ci`, built the extension server, fetched the TypeScript cases.
- 2026-09-30: Read `evaluatorImpl.ts` and the evaluation tests at
  `050880c`; prototyped the emit-and-run pipeline by hand (tsgo emitted
  down-levelled `using` for ES2022; `--permission` needs no warning flag on
  Node 22.22.2).
- 2026-09-30: Added `@run`, `execute`, and the two baselines to
  `tests/case_baselines.rs`; wrote the seeds. The first run rejected
  `pipelineThisAndOrder.tt` (Issue 1); fixed the parser; added the
  `try`, let-else, and if-let forms of the same input to the seeds.
- 2026-09-30: Deliberately broke a lowering (below), then reverted it.
- 2026-09-30: Probed the not-run, failing, sandboxed, and timed-out paths
  with temporary cases (deleted afterwards), and measured the cost.
- 2026-09-30: Documented `@run` in `CONTRIBUTING.md` and `AGENTS.md`.

## Issues and resolutions

### Issue 1: `as const` inside a bracket ended a pipeline step or a `try` operand

- **Symptom**: `flush("computed member", 1 |> counter[note("key", "add" as const)]);`
  failed with `error[source-not-typescript]: the TypeScript here does not
  parse: unbalanced TypeScript delimiter` at the closing `]`, and so did
  `try step(("asserted" as const), true)` at its `)`;
  `if let Hit(value: doubled) = ({ ... }) as const { ... }` was a
  `stray-if-let`. Valid TypeScript around a tt construct was rejected.
- **Cause**: The step and operand scanners end a list the operand left open
  at "a statement keyword directly in a parenthesis or an index"
  (TASK-528's rule), asking only `statement_only_keyword(word)`. `const`
  is on that list, but in `as const` it is the assertion's type, not a
  statement keyword.
- **Resolution**: `lexer::statement_keyword_at(src, tokens, k)` in
  `src/lexer.rs` answers the question for a token in place: a
  statement-only keyword that is not in a type position, i.e. not directly
  after an `as` assertion or after a `<` that opens type parameters
  (`<const T>`), the two places TypeScript's grammar puts `const` inside a
  type. Every parser site that asked the
  word-only question about a token in an operand (pipeline steps, `try`
  operands and statement expressions, let-else and if-let heads, the
  recovery span) asks it instead; the step-start checks, where no operand
  precedes the word, keep the word-only form. The let-else form already
  compiled before the fix (it reached a scanner that does not stop there)
  and is kept in the seed as a structural twin.

### Issue 3: The first form of the fix broke the open-list recovery

- **Symptom**: The first full gate failed
  `tests/compile/cases_05.rs::a_step_with_an_open_list_ends_where_typescript_ends_the_list`
  and `tests/emit_map.rs::a_value_try_without_an_owner_keeps_its_operand_mapped`:
  `const v = 1 |> add(2, \nconst w = 1;` no longer ended the step before
  `const`.
- **Cause**: The first form asked `TokenFacts::ends_expression` whether
  `const` completed an operand. The facts machine absorbs an unexpected
  `const` inside an unterminated argument list as an operand too, so the
  fact does not separate `as const` from TASK-528's open list.
- **Resolution**: The check reads the grammar position directly (the
  preceding `as`, or a type-parameter `<`), which is the same for both
  inputs' valid readings; both tests pass again.

### Issue 2: The typed baselines of the new cases dominate the suite's time

- **Symptom**: `case_baselines` took about 117 seconds on four workers
  against 28 seconds before (TASK-651's gate). Timing each case showed
  execution itself costs 0.3 to 0.6 seconds per `@run` case (6.2 seconds of
  CPU for thirteen, about 1.6 seconds of wall time), while
  `shortCircuitTtOperands` took 67 seconds, `conditionalOperationNarrowing`
  48, and `literalLeftOperandNarrowing` 43, against 3 seconds for
  `letElseIfLetDivergence` with more hovers.
- **Cause**: `.types` asks one hover per semantic token, and each engine
  request re-runs the contextual materialization (`Project::serve` calls
  `Project::update`, which calls `typescript::contextual::materialize`
  whenever a file has contextual storage slots). Files with many
  conditional operations holding tt values pay that per hover.
- **Resolution**: Recorded, not changed here: it is an engine performance
  question that also affects editor latency, suggested as a separate task
  (cache the materialization per snapshot and overlays). The seeds keep the
  conditional operations they exist to test.

## Regression test (fails before the fix)

- **Path**: `tests/case_baselines.rs`, cases
  `tests/cases/conformance/evaluation/pipelineThisAndOrder.tt`,
  `tryPropagationOrder.tt`, and `letElseIfLetDivergence.tt`
- **Observed failure**: With `src/lexer.rs` and `src/parser/` reverted,
  all three wrote an `.errors.txt` (`error[source-not-typescript] ... unbalanced
  TypeScript delimiter` at `pipelineThisAndOrder.tt:27:66` and
  `tryPropagationOrder.tt:63:56`, `error[stray-if-let]` at
  `letElseIfLetDivergence.tt:67:3`), an empty `.stdout`, and a `.stderr`
  reading `==== not run: the case does not compile cleanly (see
  pipelineThisAndOrder.errors.txt) ====`.

The runtime baselines themselves were shown to catch a silent miscompile:
with `emit_conditional_operation` (`src/codegen/core/emitter/host.rs`)
changed to put the left operand in the `if` branch for `&&` instead of `||`
(which swaps both), `shortCircuitTtOperands` still type-checked (no
`.errors.txt`) and its `.stdout` failed:

```
- 0 && match: 0 after left
- 0 || match: 3 after left scrutinee off
- 1 && match: 2 after left scrutinee on
- 1 || match: 1 after left
+ 0 && match: 2 after left scrutinee on
+ 0 || match: 0 after left
+ 1 && match: 1 after left
+ 1 || match: 3 after left scrutinee off
...
- true && false && match: false after a b
- false && ... && match: false after a
+ true && false && match: "reached" after a b scrutinee on
+ false && ... && match: "reached" after a scrutinee on
```

(A first attempt, `=== null` for the `??` test, stopped at an internal
compiler error in the contextual refiner instead, so it was not used.)

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 TTC_REQUIRE_TYPESCRIPT_CASES=1
  TT_REQUIRE_EXTENSION=1 TT_BASELINE_TRACKING_DIR=<dir> cargo test
  --no-fail-fast` with the extension's server built and the TypeScript cases
  fetched: 1775 passed, 0 failed, no `SKIP`, about 8 minutes;
  `case_baselines` 111 seconds (Issue 2). The first run of the gate found
  Issue 3; this is the run after its fix.
- [x] `node scripts/check-baselines --tracking <dir>`: "baselines: 250
  compared, none unused"; `tests/baselines/local/` stayed empty.
- [x] `cargo check --manifest-path fuzz/Cargo.toml --all-targets --locked`.
- [x] `./scripts/ci agents` passed (warnings: rolldown not on PATH, doctor
  reports the checkout not ready; both environmental).
- [x] Probes with temporary cases (deleted): a program that wrote to stderr,
  tried `writeFileSync` (`ERR_ACCESS_DENIED` on stdout), and threw after an
  `await` gave `.stderr` `==== node zzProbeThrow.js (exit 1) ====`, the
  stderr line, and the stack trace with `$DIR` paths and no `node:internal`
  frames; an endless `while (match ...)` loop timed out after 10 seconds
  with its first line in `.stdout`; a module-less top-level `await` gave the
  "not run" `.stderr`; `// @run: nope.tt` failed the case naming the units.
- [x] Baseline changes reviewed: the three modified cases' `.ts` differ only
  by the added harness (definitions for the former `declare`s and the calls)
  and by storage annotations that now name the new `Result` import
  (`Result.TErr<string>` for `import("./tt/index.js").TErr<string>`); every
  `.stdout` was checked against JavaScript's semantics of the source.
- CI time: execution adds 0.3 to 0.6 seconds per `@run` case (a `tsc` emit
  and a `node` run); the suite grows from 28 to about 111 seconds, almost
  all of it the `.types` hovers of the new cases (Issue 2).

## Result

Changed files: `tests/case_baselines.rs`, `tests/cases/compiler/{declaratorListLaterValue,conditionalOperationNarrowing,literalLeftOperandNarrowing,forInitializerOperands,enumMemberResultBlock,literalMatchInEveryPosition}.tt`,
`tests/cases/conformance/evaluation/*.tt` (seven), their baselines under
`tests/baselines/reference/` (thirteen `.stdout`, no `.stderr`),
`src/lexer.rs`, `src/parser/{pipes,tries,lets,iflets,parse}.rs`,
`CONTRIBUTING.md`, `AGENTS.md`, `docs/tasks/INDEX.md`, and this record. No
runtime miscompile was found in the seeded constructs; the one defect found
was the `as const` rejection (Issue 1). Follow-up: cache the engine's
contextual materialization across requests (Issue 2).
