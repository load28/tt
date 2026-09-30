# TASK-679: Populate the case matrix for match, try, result, let-else, and if-let

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-679`

## Purpose

Write the TASK-678 spec for the five constructs that shape control flow
(`match`, `try`, `result`, let-else, `if let`), generate their cases, run
each against its TypeScript twin, and report every disagreement: a twin
error is fixed, a compiler defect is recorded with a minimal repro and
listed in `tests/oracle-failures.txt`, not fixed here.

## Scope

- Included: `tests/matrix/{match,try,result,letElse,ifLet}.mjs`, the 1,617
  cases they generate under `tests/cases/conformance/matrix/`, their
  baselines under `tests/baselines/reference/matrix/`, and
  `tests/oracle-failures.txt`.
- Excluded: fixing the defects below (a later batch), and the other
  constructs and `.ttx` (TASK-680).

## What the spec covers

| Construct | Forms | Positions | Cases | Run against a twin | Expect a diagnostic |
| --- | --- | --- | --- | --- | --- |
| `match` | 14 | 40 value | 561 | 477 | 84 |
| `try` (value form) | 6 | 40 value | 306 | 197 | 109 |
| `try` (statement form) | 2 | 17 statement | 127 | 97 | 30 |
| `result` | 7 | 40 value | 312 | 281 | 31 |
| let-else | 7 | 17 statement | 133 | 117 | 16 |
| `if let` | 7 | 17 statement | 131 | 129 | 2 |
| `if let` as a value | 1 | 6 value | 47 | 0 | 47 |
| **Total** | | | **1,617** | **1,298** | **319** |

- `match` forms: tag patterns with bindings, aliased fields in any order,
  guards that fall through, an or-pattern, a final wildcard, nested
  patterns (`Done(value: Some(value: v))`, `Done(value: None())`), string
  literals with an or-pattern, number literals in several notations
  (`0xc9`) with a guard, boolean literals without a wildcard, a tuple
  match, `is` patterns, a block arm with an inner `return`, a hand-written
  `kind` union, and a match whose scrutinee and arm are matches.
- `try` forms: the value form on a call, binding tighter than `*`, two
  siblings, a parenthesized conditional operand, a member read on
  `(try r)`, and a `match` operand; statement forms `try r;` and
  `const v = try r;`.
- `result` forms: one `try`, two where the second reads the first, an early
  `return` in an `if`, a bare `return;`, a nested block read by `try`, a
  returned `match`, and an `if let` whose body completes the block.
- let-else forms: `const`, `let` reassigned, `var` (the one allowed as an
  unbraced body), an else that throws, an or-pattern, aliased fields, and an
  object-literal initializer; the else block uses the host's own exit
  (`return`, `continue`, `break`, a labeled `break`, `throw`).
- `if let` forms: body only, with `else`, an `else if let` chain, a nested
  pattern, an or-pattern, an object-literal head, and a body that leaves the
  host; and an `if let` written as a value (rejected everywhere).
- Value positions (40): declaration initializer, later declarator,
  property assignment, return, call and method arguments, optional calls
  (one run, one skipped), array and spread elements, object property,
  template literal, conditional test and branch, `&&`, `||`, `??`,
  parameter default, destructuring default, class field, static block,
  constructor, getter, method reading `this`, concise arrow body, `while`
  and `do`-`while` tests, `for-of` head, C-style initializer, test, and
  update, a loop body run twice, `switch` discriminant and case test,
  labeled block, `throw`, `await`, and `yield` operands, a module's top
  level, a match arm, and an `if let` body.
- Statement positions (17): function body, nested block, braced and
  unbraced `if`, `for-of` body left by `continue`, `while` body left by
  `break`, labeled block, `switch` clause, `try` and `catch` blocks,
  block-bodied arrow, method, class static block, constructor, match block
  arm, `result` body, and a module's top level.
- Companions (all 8 in every construct): none, `await`, `yield`, an
  optional chain, a spread, a throwing operand caught around it, `using`,
  `finally`.

Every documented placement rejection the spec states was reported as
documented (`match-placement` in parameter defaults, destructuring
defaults, class fields, `do`-`while` tests, C-style updates, and `switch`
case tests; `try-placement` at module top level, in parameter defaults,
class fields, loop tests, constructors, and generators;
`let-else-placement` in a match arm and for a `const`/`let` unbraced body;
`if-let-placement` in every value position; `result-yield-crossing` and
`match-control-crossing` for a `yield` in a `result` body and a block arm),
except Issues 1 and 2 below. Every runnable case whose program compiled
printed exactly what its twin printed: no runtime miscompile was found.

## Decisions

### Decision 1: Expectations follow the documents, including where they disagree with ttc

- **Context**: Where `docs/ai/tt.md` and `docs/design/try-result-scopes.md`
  state a rule, the spec states it too; where they are silent, the spec
  leaves the combination runnable.
- **Decision and rationale**: A row the documents accept is a runnable
  case, even where ttc rejects it (Issues 3 and 4); a row they reject is a
  diagnostic case, even where ttc accepts it (Issues 1 and 2). The list
  records each disagreement, and the fixing batch decides whether the
  compiler or the document moves. let-else in a class static block and a
  constructor is left runnable: the document's "position limits same as
  try" names module top level as allowed because a let-else writes no
  `return` of its own, which is also why `try`'s static-block and
  constructor rules (a generated `return`) do not carry over; ttc agrees.

## Work log

- 2026-09-30: Wrote the five spec modules and generated the matrix; the
  first run of 561 `match` cases took 319 seconds and 50 disagreed, all
  twin or harness errors (TASK-678, Issues 1 and 2); the first run of all
  1,617 took 632 seconds, 42 disagreeing.
- 2026-09-30: Fixed the twin errors, reran; 26 disagreements remained, all
  compiler defects; reduced each to a minimal repro with `ttc --check` and
  `ttc --check-types` in a scratch project under `target/`.
- 2026-09-30: Listed them in `tests/oracle-failures.txt`; the whole matrix
  then passes (`TT_MATRIX_CASES=all`).

## Issues and resolutions

Defects are recorded here and listed in `tests/oracle-failures.txt`; none
is fixed in this task.

### Issue 1: A statement-form `try` in the static block of a class inside a function is not rejected

- **Symptom**: 6 cases (`tryStatement_*_staticBlock_*`) expect
  `try-placement` and get TypeScript's TS18041 on the emitted `return`.
- **Repro**:
  ```tt
  import type { TResult } from "@tt/std";
  declare function read(n: number): TResult<number, string>;
  export function f() {
    class Holder {
      static {
        const v = try read(1);   // or `try read(1);`
      }
    }
  }
  ```
  `ttc --check-types` reports `error[ts18041]: A 'return' statement cannot
  be used inside a class static block. (in code ttc generated for this
  construct)`, and `ttc -p` emits `if (!("value" in $tt_t0)) { return
  $tt_t0; }` inside `static { }`. The same statement in a class at module
  level, and the value form in the same block (`Holder.field = try
  read(n);`), are `try-placement` ("a class static block — it has no
  enclosing function failure edge"), as documented.
- **Cause (suspected)**: The statement forms find their Result scope by
  walking out to the nearest function, crossing the class static block,
  which is its own function-like boundary for `return`
  (try-result-scopes §4.5, "class static block: Reject").
- **Resolution**: Listed; not fixed here.

### Issue 2: A `try` in a C-style `for` test is `lowering-plan-failed`, not `try-placement`

- **Symptom**: 6 cases (`try_*_forTest_*`) expect `try-placement`.
- **Repro**:
  ```tt
  export function f(flag: boolean): TResult<number, string> {
    for (; flag && try read(1); ) { break; }
    return { kind: "Ok", value: 1 };
  }
  ```
  reports `error[lowering-plan-failed]: tt host lowering could not plan this
  construct: the evaluation position 169..196 maps to no source` at `1:1`
  (the file's first line). Without the `flag &&`
  (`for (; try read(1); )`) it is `lowering-plan-failed: a propagation
  would repeat in its loop header` at the `try`. The same `try` in a
  `while` test is `try-placement` ("a repeated loop position"), as
  documented for both (try-result-scopes §4.5, "`while`/`do` test; C-style
  `for` test/update: Reject `RepeatedInOwner`").
- **Resolution**: Listed; not fixed here.

### Issue 3: A function-targeted `try` in a match block arm is rejected

- **Symptom**: 7 cases (`tryStatement_*_matchBlockArm_*`) do not compile.
- **Repro**:
  ```tt
  export function f(flag: boolean): TResult<unknown, string> {
    const a = match (flag) { true => try read(1), false => 0 };              // accepted
    const b = match (flag) { true => { const n = try read(2); return n; }, false => 0 }; // rejected
    return { kind: "Ok", value: [a, b] };
  }
  ```
  reports `error[try-placement]: `try` cannot be used here, in an isolated
  value region — it compiles to a `return`, which would complete this
  construct's value instead of returning from the enclosing function` at
  the second `try`. try-result-scopes §4.6 says of an isolated value region
  (a value-producing match arm) that the "Function target remains Legal if
  no outer ResultRegion is crossed", and the expression arm is accepted.
- **Resolution**: Listed; not fixed here. Either the block arm gains the
  function target the expression arm has, or §4.6 and `docs/ai/tt.md` say
  that a block arm is excluded.

### Issue 4: A `try` nested in an operand of a conditional operation is rejected

- **Symptom**: 7 cases (`try_siblings_{logicalAnd,logicalOr,nullish,conditionalBranch,optionalCall}_*`,
  `try_binaryOperand_optionalCall_*`, `try_memberOfValue_optionalCall_*`)
  do not compile.
- **Repro**:
  ```tt
  declare const h: ((v: unknown) => unknown) | undefined;
  export function f(flag: boolean): TResult<unknown, string> {
    const a = flag && (try read(1)) + (try read(2));   // rejected
    const b = h?.(try read(1) * 2);                    // rejected
    const c = h?.(try read(1));                        // accepted
    const d = flag && try read(1) * 2;                 // accepted
    return { kind: "Ok", value: [a, b, c, d] };
  }
  ```
  reports `error[try-placement]: `try` cannot be used in this conditional
  operation — its TypeScript control-flow boundary cannot be rebuilt without
  changing evaluation order` for `a` (both `try`s) and `b`. The documents
  state that the value form "preserves left-to-right, conditional,
  optional-call, and concise-arrow evaluation inside an enclosing function"
  (`docs/ai/tt.md`, "try") and list logical, nullish, ternary, and
  optional-call argument positions as legal (try-result-scopes §4.5); the
  single-`try` forms of the same positions are accepted and agree with
  their twins.
- **Resolution**: Listed; not fixed here.

### Issue 5: A diagnostic in generated code is placed at the whole match, not at the pattern

- **Symptom**: While the harness typed the tuple's second scrutinee as
  `{ kind: "Fast" }` (TASK-678, Issue 2), the arm `(_, Slow)` was reported
  as `error[ts2367]: This comparison appears to be unintentional ... (in
  code ttc generated for this construct)` under the whole
  `match (…, …)` head rather than at `Slow`.
- **Cause**: TypeScript's error is legitimate (the case cannot occur), but
  its span lands in generated glue and is mapped to the construct's anchor.
- **Resolution**: Recorded as an observation for the diagnostics batch; the
  harness now types the scrutinee as the variant, so no case pins it.

## Regression test (fails before the fix)

Not applicable: this task adds cases and fixes no defect. The four defects
it found are pinned by the 26 cases listed in `tests/oracle-failures.txt`,
which fail the suite when they start agreeing with their oracles.

## Verification

- [x] `node scripts/generate-cases --check`: "1617 generated cases match
  tests/matrix".
- [x] `TT_MATRIX_CASES=all cargo test --test case_baselines`: both tests
  pass; the 26 listed disagreements are observed exactly as listed.
- [x] Baselines reviewed: 1,298 `.stdout` files, read per construct for the
  logged order (scrutinee, guard, arm; `read`, then nothing after an `Err`;
  `dispose` after the value, `finally` last), 14 `.stderr` "not run" files
  for the listed runnable cases, and 333 `.errors.txt` files (319
  diagnostic cases, each naming the expected code at the construct, and the
  14 listed runnable cases).
- CI time: see TASK-678 (the default run samples 120 of these cases, about
  40 seconds; all 1,617 take about 540 seconds in a debug build on four
  workers). The full gate is recorded in TASK-680.

## Result

Changed files: `tests/matrix/{match,try,result,letElse,ifLet}.mjs`,
`tests/cases/conformance/matrix/{match,try,tryStatement,result,letElse,ifLet,ifLetValue}/`
(1,617 cases), `tests/baselines/reference/matrix/` (1,645 baselines),
`tests/oracle-failures.txt` (26 lines, Issues 1 to 4),
`docs/tasks/INDEX.md`, and this record. Follow-up: the four defects
(Issues 1 to 4) and the diagnostic span (Issue 5) for the fixing batch.
