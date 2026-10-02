# TASK-654: Read, check, and emit tt syntax of any nesting depth without overflowing the stack, in work linear in the input and output

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-654`

## Purpose

`const x = match (a) { A => match (a) { A => … } }` nested 10 000 deep
aborted `ttc` (SIGABRT, exit 134, "has overflowed its stack"), and
`ttc --server` died with it, leaving every later request unanswered. The
recursion `parse_match` → `parse_tokens_with_context` →
`parse_expression_tokens` → `ArmTailSyntax::finish` → `parse_match` had no
stack-growth boundary, although `src/stack.rs` said the compiler's stack
reservation "is not a supported syntax-depth limit". The same input was
also super-linear before it overflowed: depth 500 took 9.6 s and 1000 47 s
in a debug build, while 100 000 nested parentheses took about 1 s.

## Scope

- Included: a stack-growth boundary on every recursive traversal of tt
  syntax and of the host AST in the lexer, parser, HIR, resolution,
  analysis, sema, `val`, core IR, evaluation IR, projection, codegen, and
  the engine's editor answers; the vendored `swc_ecma_parser` and
  `swc_ecma_visit` recursion entries; the TypeScript host's syntax walks
  (`src/typescript/host.mjs`); the rescans that made nesting quadratic or
  worse; tests.
- Excluded: the emitted form. Output size grows with the square of the
  nesting depth because every nested block is indented (Decision 4).

## Sources

- `stacker` 0.1 (`maybe_grow`): the crate `swc_ecma_parser` already uses
  for the same purpose, and `crate::stack::grow` in this repository.
- Node.js/V8: a JavaScript function's recursion is bounded by V8's stack
  limit (`--stack-size`, about 984 KiB by default), and exceeding it throws
  `RangeError: Maximum call stack size exceeded`.
- TypeScript's emitter indents each nested block one level
  (`src/compiler/emitter.ts`, `increaseIndent`), as the TASK-310 readable
  control flow does here.

## Decisions

### Decision 1: Grow the stack at every recursion boundary, not a larger fixed stack

- **Context**: The compiler already runs on a 256 MiB thread; any fixed
  size is a depth limit, which `src/stack.rs` promised there is not.
- **Alternatives considered**: (a) A bigger `COMPILER_STACK_SIZE`: moves the
  limit. (b) A depth limit with a diagnostic: rejects valid TypeScript
  nesting that `tsc` accepts, against contract 1. (c) Rewrite every
  traversal iteratively: a rewrite of most of the compiler. (d) Wrap each
  function that recurses over syntax (the entry of each recursive cycle)
  in `crate::stack::grow`, which allocates a new segment only when the
  remaining stack is short.
- **Decision and rationale**: (d). Each wrapped function keeps its body as
  `*_grown` and its signature, so no caller changes. The audit covered
  every function that calls itself or a cycle back to itself over
  `Program`, `Segment`, `IfLetStmt`, `TagPattern`, `Binding`, HIR and core
  bodies, expressions, decisions and pattern plans, token trees
  (templates, JSX), and SWC `Expr`/`Stmt`/`Pat`/`TsType`. `Program`'s
  destructor grows too, since a deep `Program` is dropped recursively.
  `src/stack.rs` now says what the reservation still bounds: dropping a
  host AST, whose destructors the vendored SWC crates generate. The
  lexer facts machine's "no progress" assertion counted a retry that pops
  a frame as no progress, so closing more than 4 096 nested frames at once
  tripped it in debug builds; it now resets whenever the stack shrinks.

### Decision 2: Patch the vendored SWC recursion entries, and record them

- **Context**: The emitted TypeScript is parsed back and visited through
  SWC, and a deep `match` becomes deep blocks, function types, JSX, and
  patterns there. Upstream grows only in `parse_assignment_expr` and
  `parse_stmt`'s block path, and its generated visitors not at all.
- **Alternatives considered**: (a) Override every `visit_*` in each of
  ttc's visitors: the recursion lives in the generated
  `visit_children_with`, so every visitor would have to repeat it. (b)
  Grow at the five nodes every nested traversal passes (`Expr`, `Stmt`,
  `Pat`, `TsType`, `JSXElementChild`) in the generated `visit_with`, and
  at `parse_stmt_like`, `parse_ts_type`, `parse_jsx_element`, and
  `parse_binding_pat_or_ident` in the parser.
- **Decision and rationale**: (b), recorded in each crate's `TT-PATCH.md`.
  `swc_ecma_visit` gains a `stacker` dependency, already in the lock file
  through `swc_ecma_parser` and ttc.

### Decision 3: The TypeScript host walks syntax with an explicit stack

- **Context**: With the Rust side fixed, a 300-deep `match` of arrow arms
  failed with "the TypeScript backend failed: Maximum call stack size
  exceeded": `host.mjs` walked each source file by recursing through
  `forEachChild`.
- **Alternatives considered**: (a) Start node with a larger
  `--stack-size`: V8 does not grow its stack, so a value above the
  thread's real stack crashes the process instead of throwing. (b) Walk
  iteratively.
- **Decision and rationale**: (b): one `walkTree(root, enter)` helper, a
  pre-order walk over an explicit stack in the same order `forEachChild`
  visits, replaces the nine recursive syntax walks, and `freshLiterals`
  keeps its operands on an explicit stack. `enter` returns `true` to stop
  and `false` to skip a subtree, the two early exits the walks had. The
  contextual pass also walked the whole module once per storage slot, and
  a nested `match` has a slot per level; it now indexes a module's
  declarations, identifiers, and assignments once and looks each slot up.

### Decision 4: Linear in the input and the output, and the output stays readable

- **Context**: After the stack fixes, time still grew quadratically with
  depth (depth 250 0.3 s, 500 1.1 s, 1000 4.3 s, 2000 16.6 s, debug,
  `--check`). A profile (callgrind, depth 500) put the time in measuring
  and re-parsing the emitted text; the emitted `.ts` is 2.1 MB for a
  4.7 KB source at depth 250, because each nested `switch` block is
  indented one more level, so the output is quadratic in the depth.
- **Alternatives considered**: (a) Stop indenting past some depth: changes
  the emitted contract (TASK-310's readable control flow) for one shape of
  input, and `tsc`'s own emitter indents the same way. (b) Keep the form
  and remove every rescan that made the work grow faster than the output:
  bracket matching (`find_close`, `close_of`, the arm outline, an arm
  body's end) was a forward scan per opener, now a pair table the lexer
  fills once (`Token::balancing_close`, `Token::matching_close`); a
  `result` candidate that is an ordinary block was decided again by every
  enclosing candidate, now remembered by its `{` offset; `try`'s primary
  operand test re-lexed inside open brackets; `Ok` wrappers were grouped
  by re-inspecting the whole wrapped text at each level
  (`wrapped_delivery` inspects the body once); the projection mapped the
  same owner span back to the source once per statement (`owner_source`
  caches it).
- **Decision and rationale**: (b). Parsing, analysis, and the editor
  answers now do work linear in the nesting depth, which a work-count test
  pins; compiling is linear in the input plus the output. A depth-10 000
  compile is bounded by its multi-gigabyte output, not by the stack, so
  the depth-10 000 tests run the paths that do not emit (the editor
  answers, over the server and in process) and the compile tests nest
  every construct 300 deep on a 1 MiB thread, deeper than that thread's
  stack holds without growth.

## Work log

- 2026-09-30: Reproduced the abort and the timings with
  `target/probe7-cli/cases/01`.
- 2026-09-30: Audited the recursive traversals and added the growth
  boundaries (the files in Result); patched the vendored SWC parser and
  visitor.
- 2026-09-30: Replaced the bracket rescans with the lexer's pair table;
  added the `result` candidate memo, the `try` operand guard, the owner
  span cache, and `wrapped_delivery`.
- 2026-09-30: Found the TypeScript host's recursion with the 300-deep
  arrow-arm case; replaced it with `walkTree`. Found `parse_ts_type`
  unprotected with the same case (the storage annotation is a 300-deep
  function type); patched it.
- 2026-09-30: Profiled depth 500 with callgrind; the remaining quadratic
  term is the output's size (Decision 4).
- 2026-09-30: Added the tests below.

## Issues and resolutions

### Issue 1: The TypeScript host overflowed V8's stack

- **Symptom**: `error[other]: the TypeScript backend failed: Maximum call
  stack size exceeded` for a 300-deep arrow-arm `match` in a project with
  TypeScript installed.
- **Cause**: `host.mjs` line 751, a recursive `visit` over
  `RemoteNode.forEachChild` (found by writing the error's stack to a file
  from the host's catch).
- **Resolution**: Decision 3.

### Issue 3: The depth-10 000 server test hung

- **Symptom**: The test ran for 30 minutes and failed with `BrokenPipe`
  when the server was stopped; the server had used 83 s of CPU.
- **Cause**: `server_lines` in `tests/cli.rs` wrote all of stdin before
  reading stdout; the semantic tokens of a 10 000-deep file fill the pipe,
  so the server blocked writing while the test blocked writing.
- **Resolution**: The helper writes stdin from its own thread.

### Issue 2: A deep function type overflowed the vendored parser

- **Symptom**: The in-process test aborted with "thread has overflowed its
  stack" in `parse_ts_type` → `parse_ts_fn_or_constructor_type` →
  `parse_ts_type_ann` → `parse_ts_type` (gdb backtrace).
- **Cause**: The verifier re-parses the emitted storage annotation, a
  function type nested once per arm, and `parse_ts_type` had no growth.
- **Resolution**: Decision 2.

## Regression test (fails before the fix)

- **Path**: `src/stack.rs`
  (`a_deeply_nested_match_is_read_on_the_compiler_stack`,
  `every_nested_tt_construct_compiles_without_the_callers_stack`),
  `tests/cli.rs`
  (`a_deeply_nested_match_is_answered_and_the_session_continues`),
  `src/lib/scaling_tests.rs`
  (`every_request_does_linear_work_in_the_nesting_depth_of_matches`).
- **Observed failure**: With this task's non-test changes reversed over
  the finished tree (a reverse apply of their diff), both `src/stack.rs`
  tests and the scaling test aborted the test binary: "thread '<unknown>'
  has overflowed its stack / fatal runtime error: stack overflow,
  aborting" (SIGABRT), the scaling test already at depth 120 on the test
  thread. The server test failed with `BrokenPipe` writing the requests:
  the server had aborted on the first one. `ttc --check` of the probe's
  nested `match` took 0.93 s, 5.5 s, and 38.4 s at depths 250, 500, and
  1000 (debug build), about seven times per doubling; with the fix it
  takes 0.31 s, 1.1 s, and 4.3 s, four times per doubling, the growth of
  the indented output (Decision 4).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --no-fail-fast` with `TTC_REQUIRE_TSGO=1`,
  `TTC_REQUIRE_TYPESCRIPT_CASES=1`, and a tracking directory, then
  `node scripts/check-baselines --tracking <dir>` (292 compared, none
  unused), before and after merging the branch TASK-667–676 landed on.
- [x] Baseline changes reviewed and committed with the change (none: no
  emitted output changed)

## Result

Changed files: `src/stack.rs`, `src/ast.rs`, `src/lexer.rs`,
`src/lexer/`, `src/parser/`, `src/hir/lower.rs`, `src/resolve/mod.rs`,
`src/analysis/`, `src/sema.rs`, `src/sema/checker.rs`, `src/val.rs`,
`src/val/`, `src/flow/mod.rs`, `src/probe.rs`, `src/lib/api.rs`,
`src/core_ir/`, `src/evaluation_ir/`, `src/program_syntax/`,
`src/codegen/`, `src/engine/`, `src/sidecar.rs`,
`src/typescript/host.mjs`, `src/lib/scaling_tests.rs`, `tests/cli.rs`,
`vendor/swc_ecma_parser/` (four functions and `TT-PATCH.md`),
`vendor/swc_ecma_visit/` (`src/generated.rs`, `src/lib.rs`, `Cargo.toml`,
`TT-PATCH.md`), `Cargo.lock`.
