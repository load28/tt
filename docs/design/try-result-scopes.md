# Design: one `try`, `result` as a nested Result scope

- **Status**: Implemented on the Result-scope branch. The language model and
  implementation order below remain the acceptance contract.
- **Baseline**: `33acccc` (`TASK-280: lower try through expression evaluation`,
  PR #87). Every claim in this document was checked against that tree.
- **Audience**: An implementing agent. This document is the full brief. Do not
  reconstruct the model from chat history; this file is the source.
- **Provenance**: Recorded by TASK-281. Two three-agent deliberations reviewed
  and amended the original proposal — the first against `b3934cd`, the second
  re-based on `33acccc` after PR #87 merged. All three reviewers signed the
  re-based consensus.

## 1. One-sentence model

`try expr` unwraps a `Result`. On `Ok` it is the payload. On `Err` it ends
the **nearest Result scope** with that `Err`. A function that returns
`Result` is one such scope. `result { ... }` is a smaller scope: an
expression whose value is a `Result`, used when the enclosing function
should keep running.

## 2. Why this exists

Today tt has two spellings for the same operation:

| Surface | Unwraps `Ok` | On `Err` |
|---|---|---|
| `try expr` | yes | returns from the **enclosing function** |
| `const x <- expr;` inside `result { }` | yes | the **`result` block** becomes that `Err` |

`result` was not invented because a second operator was needed. It exists
because the **exit target** is different. `try` currently always leaves the
function, so the function's return type must be `Result`. Nested
`Result.andThen` chains need a local `Result` scope that does *not* force
the outer function to return `Result`. That local scope is the `result`
block.

The current design then forbids `try` inside `result` (it would `return`
from the block's lowering, which users read as "leave the function") and
invents `<-` plus a Rust-style last expression with no semicolon. Those two
spellings are what feel unlike TypeScript.

This proposal keeps two scopes and uses **one operator**.

## 3. Current baseline: `33acccc`

`33acccc` is the only implementation baseline. It contains TASK-280's expression-position `try`, prefix-primary operand grammar, HIR `Expr::Try`, Core `Propagate`, and host-evaluation lowering in function scope. Statement and expression forms both target `ExitTarget::EnclosingFunction`; neither has nearest-`result` identity yet (`src/ast.rs:429-440`, `src/hir/mod.rs:402-408`, `src/core_ir/lower.rs:185-193`, `src/core_ir/mod.rs:292-306`).

The current `result` form still uses top-level `<-` bindings and a semicolon-free tail expression. Direct `try` in a result body is rejected with help to use `<-` (`src/sema.rs:451-466`, `docs/ai/tt.md:65-77`). Those are current rules to migrate, not the proposed final language.

The baseline also contains defects that must be repaired independently before Result-scope language work begins:

| Defect | Dual-binary attribution | Required disposition |
|---|---|---|
| C-style `for` declaration-init/test projection ICE; discarded-Result source-preservation ICE | Pre-existing on `b3934cd` and `33acccc` | Located diagnostic or valid TypeScript; never unwind |
| Expression-boundary `result` containing value-form `try`; pipeline concise-arrow invalid emit | Regressions introduced by #87 | Located diagnostic or valid TypeScript; never invalid emit |
| Constructor and generator propagation | Statement form pre-existing; expression reach widened by #87 | Reject both forms at the `try` span |
| Lowering-plan failures escaping public consumers | Failure mode pre-existing; `analyze` path exposed by current lowering integration | Return located diagnostics to every client |

The attribution is established by running identical inputs through binaries built from `b3934cd` and `33acccc`; it is not inferred from source history. These repairs are the independent prerequisites P0–P6 in §9.

## 4. Proposed language

### 4.1 `try` — one operation, expression

`try expr` is always an expression.

- Operand must be a `Result` (`TOk<T> | TErr<E>` / `TResult<T, E>`).
- `Ok` → the payload (`T`), used as a value in the surrounding expression.
- `Err` → end the **nearest Result scope** with that `Err`. Code after the
  `try` in that scope does not run.
- Option is still unsupported.
- No implicit `From`-style error conversion. Error unions remain a
  TypeScript inference fact.

Tightness is unchanged from TASK-280: prefix `try` binds to the following
primary, including calls and member/index postfixes.

```tt
try total() * 1.1          // (try total()) * 1.1
try (flag ? left() : right())
const n = try parse(raw);
try validate();            // expression statement: discard Ok, propagate Err
return Result.Ok(try total() * 1.1);
```

There is no separate statement `try` and expression `try` in the language
model. `try validate();` is an expression statement. `const n = try parse(raw);`
is an initializer. Both are the same `try`.

### 4.2 Result scopes — two, nested

A Result scope is the region an `Err` from `try` leaves.

1. **Function scope.** A user-written function (including methods, concise
   arrows after they become blocks, and functions written *inside* a
   `result` block) is a Result scope when `try` in it is allowed to emit
   `return` from that function. On `Err`, that function returns the `Err`.
   The function's return type must be `Result`-compatible. This is today's
   function-level `try`.

2. **`result` block scope.** The block is itself an expression of type
   `Result`. On `Err`, the block evaluates to that `Err` and the enclosing
   function continues. The enclosing function need not return `Result`.

Nearest scope is lexical:

- `try` in a function body, not inside a `result` block → leaves the
  function.
- `try` in a `result` block body, not inside a nested function → leaves
  that block.
- `try` in a function nested inside a `result` block → leaves **that
  nested function**, not the block. Same rule as today's "a `try` inside a
  function you write there is fine."

Nested `result` blocks are allowed. Inner `try` leaves the inner block.

Constructors, generators, async generators, and class static blocks are not legal function Result scopes. A function-targeted `try` is rejected in a constructor, including before or in `super(...)`, because returning an object replaces the constructed instance or violates derived-constructor initialization. It is rejected in a generator or async generator because `return Err` is iterator completion; ordinary `for...of`/`for await...of` consumption can discard that completion and silently lose the error. It is rejected in a class static block because no enclosing function failure edge exists.

These restrictions do not reject a nested `result` block in those owners. When the block is the nearest Result scope, the block carries the failure edge. `using` and `await using` declarations are Legal hosts when their surrounding owner otherwise permits propagation; normal disposal still runs on the failure path.

### 4.3 `result { }` — TypeScript-shaped body, `async`/`await` analogue

The TypeScript analogue is an async IIFE:

```ts
const data = await (async () => {
  const user = await getUser(id);
  return { user };
})();
```

Proposed tt:

```tt
const data = result {
  const user = try getUser(id);
  return { user };
};
```

| async/await | this design |
|---|---|
| `async { ... }` / async IIFE | `result { ... }` |
| `await expr` unwraps or leaves the async context | `try expr` unwraps or leaves the Result scope |
| `return x` wraps `x` in a `Promise` | `return x` wraps `x` in `Ok` |
| the IIFE is an expression | the `result` block is an expression |

Consequences:

- The body is a TypeScript-shaped statement list. Statements may end with `;`; there is no semicolon-free tail-expression success channel.
- A result-owned `return x` completes the block with `Ok(x)`. A bare `return;` completes it with `Ok(undefined)`.
- Reachable fallthrough is `result-no-success-value`; it is not implicit `Ok(undefined)` in this design.
- A claimed Result expression in `HostContinuation::Discard` (`src/program_syntax.rs:623-640`) is `result-value-discarded`, because discarding the expression also discards its `Err`. This stronger diagnostic is primary and may suppress a redundant missing-success diagnostic on the same block.
- `<-` is removed. During the migration release, old Result bindings receive a located diagnostic and an applicable `= try` edit.
- `return alreadyAResult` does not flatten. It means `Ok(alreadyAResult)`. Typed paths diagnose a definitely Result-shaped returned expression as `result-return-nested` and suggest `return try <expr>;`; untyped compilation does not guess.
- `break`, `continue`, a user label, or `yield` may not cross outward from a ResultRegion. Each has a named tt diagnostic.
- `throw` crosses normally. JavaScript `finally` runs on success and failure paths and may override a pending completion under ordinary abrupt-completion rules.
- A claimed Result body does not create a uniform variable environment. In a statement host, `var` and function declarations have the scope of the same host statements. An expression-boundary arrow confines them, and TypeScript reports an outside reference. Direct `eval` and Annex-B block functions are not specially prohibited.

The async analogy is directional, not identical: `try` unwraps or completes the nearest Result scope and result-owned `return` wraps `Ok`, but Results do not adopt Promise/thenable flattening.

### 4.4 Claiming and the passthrough contract

Invariant: every valid TypeScript file remains a valid `.tt` file.
Unrecognized shapes pass through byte-for-byte.

`result` is a common TypeScript identifier. These are valid TypeScript and
must not be claimed:

- `result { ... }` with no tt construct inside (identifier expression
  statement plus a block, or a labeled-looking ASI pair).
- `function f(): result { ... }` (return type named `result`).
- `class result { ... }`, `const result = 1`, etc.

Today the claim marker is `<-`, which is never valid TypeScript in a
declarator (`const x <- e` cannot be a legal initializer).

A candidate `result { B }` is claimed if and only if one speculative, lossless parse of `B` finds a successfully parsed tt `try` whose nearest lexical Result scope is that candidate block.

The parse reuses the existing statement and expression try claimers and the shipped prefix-primary operand rule (`src/parser/tries.rs:28-43`, `src/parser/tries.rs:74-133`). It does not use a token-presence scan or a second lookahead grammar. Dotted `obj.try()`, TypeScript `try { ... } catch`, property/member names, labels, and ASI-separated TypeScript forms cannot claim the block.

The nearest-scope walk descends through ordinary statements and expressions, including loops, TypeScript `try`, `if let`, `match`, pipelines, templates, and JSX. It stops at a nested `result`, nested function-like, method, accessor, constructor, or class boundary. It may see a `try` through an isolated value region; claiming and legality are separate, so that block remains claimed and the crossing then receives `try-crosses-value-region`.

A `result { return 1; }` with no qualifying `try` is not claimed and passes through. A leftover `<-` is not a claim marker after removal. Preserve TypeScript `a < -b` and every other unclaimed source byte-for-byte.

### 4.5 Placement is a derived predicate

A `Propagate` is legal if and only if the host protocol can carry its failure to the nearest Result scope without changing JavaScript evaluation order, count, receiver/`this` semantics, disposal, or constructor initialization. Placement is derived from `EvaluationContext` (`src/program_syntax.rs:421-430`), `OwnerReach` and `EvaluationOwner` (`src/program_syntax.rs:379-401`), `owner_reach` (`src/program_syntax.rs:479-534`), and `TargetCapability` and `ExpressionBoundaryReason` (`src/evaluation_ir.rs:225-267`); analysis reports the reason as `try-placement`.

| Host/path | Final classification |
|---|---|
| Function-body statement/initializer; call, `new`, optional-call argument; logical/nullish/ternary/comma/object/array/template/tagged-template/JSX value position | Legal where each edge is `Same` or an explicitly modeled conditional step |
| C-style `for` initializer, including a declaration initializer; `for-in`/`for-of` RHS; `switch` discriminant | Legal; each is evaluated once. `owner_reach` marks Test/Update/Body `Repeated` and deliberately omits Init (`src/program_syntax.rs:483-493`) |
| `using`/`await using` initializer | Legal; disposal is preserved on `Err` |
| `while`/`do` test; C-style `for` test/update | Reject `RepeatedInOwner` |
| `switch` case test; destructuring default; optional-chain member/index tail | Reject as `UnmodeledConditional` until that edge is modeled |
| Concise-arrow body, including parenthesized bodies and pipeline-step arrows | Legal by converting that arrow to a block whose return target remains the arrow |
| Module/namespace function target; parameter/class initializer; class static block; decorator | Reject: no legal function failure edge for that owner |
| Constructor, `super(...)`, or before `super()` function target | Reject `try-placement` |
| Generator or async-generator function target | Reject `try-placement` |
| A whole `result` block in a parameter default, class field, static block, constructor, generator, or module expression position | Legal when the block itself is the nearest Result scope and its host capability can print the block |
| A path from `try` to its nearest Result scope that crosses an isolated value region | Reject `try-crosses-value-region`; see §4.6 |

“Loop header” is not a placement class. Declaration Init is `OwnerReach::Same`; Test and Update are `Repeated`. The existing C-style declaration-init ICE is therefore a projection defect, not evidence that the Legal row should change.

Do not legalize an illegal function-targeted `Propagate` by wrapping it in `$tt_expr`: that callback would be the wrong Result scope. `$tt_expr` is valid only as the printer for a ResultRegion that is itself at an expression boundary.

### 4.6 Completion ownership by lexical region

| Location | `return x` means | `try` targets |
|---|---|---|
| Ordinary Result-returning function | TypeScript return from that function; write `Result.Ok(x)` explicitly | That function, when placement permits |
| Statement, initializer, `if`, loop, `switch`, or TypeScript `try` body directly owned by `result` | Complete the block with `Ok(x)` | This block |
| Inline `if let` body or let-else `else` inside `result` | Complete the block with `Ok(x)` | This block; let-else divergence is relative to block completion |
| Isolated value region: value-producing match arm/scrutinee, pipeline step, template interpolation | That isolated region owns its value exits; a match block-arm `return` yields the arm value | Function target remains Legal if no outer ResultRegion is crossed; a `try` targeting an outer `result` is rejected as `try-crosses-value-region` |
| Function/method/accessor nested inside `result` | TypeScript return from that nested function-like | That nested function-like, subject to owner restrictions |
| Constructor/generator/async generator nested inside `result` | TypeScript constructor/iterator completion | Function-targeted `try` is rejected; a nested `result` remains Legal |
| Nested `result` | Complete the inner block with `Ok(x)` | The inner block |

Success-exit collection is structural. It resets at nested function/class boundaries and does not descend into a nested isolated value region, because that region owns its exits. It does descend into inline `DecisionKind::IfLet` and let-else bodies. Claim scanning may cross an isolated region, after which placement rejects only the propagation whose nearest Result scope lies outside it.

Let-else is Legal in a result body once Result completion capture exists. Every `else` path must complete the block; outward `break` or `continue` does not satisfy that rule.

### 4.7 Types

Unchanged policy: ttc does not infer or union error types. `tsc` sees the
lowered `return` / slot assignments and infers `TResult<T, E1 | E2>`.

- Function with several `try`s: inferred as today.
- `result` block with several `try`s: the block expression is assignable to
  `TResult<Success, E1 | E2>` the same way today's `<-` bindings union.

Typed paths add `result-return-nested`. Projection asks the TypeScript backend whether a returned expression is definitely Result-shaped; tt owns the construct-specific code, source span, and `return try <expr>;` suggestion. The implementation follows the existing checker-fact seam used by literal-match exhaustiveness (`src/engine/projection.rs:40-55`, `src/typescript/backend.rs:197-205`, `src/engine/semantics.rs:1151-1187`). Untyped ttc remains silent when it lacks that fact.

## 5. Examples the language must accept or reject

### 5.1 Function scope only

```tt
function load(id: string): TResult<User, E> {
  const user = try getUser(id);
  return Result.Ok(user);
}
```

`getUser` `Err` → `load` returns that `Err`. Nothing after the `try` runs.

### 5.2 `result` scope only (outer function is not Result)

```tt
function page(id: string): void {
  const data = result {
    const user = try getUser(id);
    return { user };
  };
  if (data.kind === "Err") return;
  render(data.value);
}
```

`getUser` `Err` → `page` continues. `data` is that `Err`.

### 5.3 Both scopes inside one function

This is the example that distinguishes the two boxes.

```tt
function load(id: string): TResult<Page, E> {
  const user = try getUser(id);
  // Err → load returns Err. The rest of load does not run.

  const extra = result {
    const company = try getCompany(user.companyId);
    const plan = try getPlan(company.id);
    return { company, plan };
  };
  // getCompany / getPlan Err → load continues. extra is that Err.

  if (extra.kind === "Err") {
    return Result.Ok({ user, extra: null });
  }
  return Result.Ok({ user, extra: extra.value });
}
```

### 5.4 Expression `try` in a function

```tt
function amount(): TResult<number, string> {
  return Result.Ok(Math.round(try total() * 1.1));
}

function callOrder(flag: boolean): TResult<number, string> {
  return Result.Ok(call(first(), flag && try second(), third()));
}

const f = (): TResult<number, string> => Result.Ok(try read());
```

Evaluation order is JavaScript order. `first()` before `second()` before
`third()`. The concise arrow becomes a block so the generated `return` has
a home.

### 5.5 Expression `try` inside `result`

```tt
const data = result {
  return Math.round(try total() * 1.1);
};
```

`Err` ends the block, not the enclosing function.

### 5.6 Nested function inside `result`

```tt
result {
  const inner = (): TResult<number, E> => {
    return Result.Ok(try step());
  };
  return try inner();
}
```

`try step()` leaves `inner`. `try inner()` leaves the `result` block.

### 5.7 Rejected

```tt
try getUser(id);                                  // no Result scope at module top level
function f() { while (try ready()) work(); }     // repeated host
function f() { for (; try ready(); ) work(); }   // repeated host
function f() { switch (x) { case try key(): break; } } // unmodeled conditional
function f({ x = try seed() }) {}                 // unmodeled destructuring default
function f(x = try seed()) {}                     // function-targeted parameter owner
class C { field = try seed(); }                   // function-targeted field owner
class D extends B { constructor() { super(); try seed(); } } // constructor target
function* values() { yield try seed(); }          // generator target; Err would vanish from for...of
class S { static { try seed(); } }                // no function Result scope

result {
  return match (x) { A => try a(), _ => 0 };
} // try-crosses-value-region

result { const x <- get(); return x; }            // removed syntax; migration diagnostic
result { return 1; }                              // no inner tt try: not claimed
```

Also add accepted examples and tests for `for (const x of try xs())`, `for (let i = try n();;)`, `switch (try x())`, `using x = try acquire()`, `await using x = try acquireAsync()`, and a whole claimed `result { return try seed(); }` expression inside a parameter, class field, constructor, static block, and generator. The accepted examples assert runtime behavior, including disposal on `Err`.

## 6. Lowering contract

Use one semantic Result completion record and two host-selected printers. Do not add a second propagation machine.

### 6.1 Core ownership

- `Propagate` retains evaluate-once success extraction and a failure edge. Add `ExitTarget::ResultRegion(ResultRegionId)` and retain `ExitTarget::EnclosingFunction` only for legal function scopes (`src/core_ir/mod.rs:292-306`).
- `ResultRegion` owns its statement body, stable identity, structural `is_async` fact, and Result-owned success/failure completions (`src/core_ir/mod.rs:343-355`). Reuse `HostExit { statement, argument, captured_break }` (`src/program_syntax.rs:96-106`) for source-backed success exits.
- Register ResultRegion projection calls in `expected_exit_calls`. Today it accepts only `OverlayMarker::DecisionCallExpression` (`src/program_syntax.rs:1574-1578`), so Result-owned returns are not captured without this addition.
- Project `region.value`, not the current `0` placeholder (`src/program_syntax.rs:1078-1108`). This makes a tail value-producing Decision its own `DecisionCallExpression` and gives that Decision ownership of its arm `value_exits`.
- **Both preceding fixes are required.** Registration captures result-owned item and inline-if-let returns; value projection prevents ResultRegion from stealing match-arm returns that belong to the Decision. Neither is an alternative to the other.
- Replace the source-span await scan (`src/core_ir/lower.rs:403-410`) with structural `is_async` that stops at nested function-like and class boundaries.
- Validation proves that every Result-targeted exit names an enclosing live region and every Result-owned completion has exactly one destination.

### 6.2 One completion, two printers

| Host | Required lowering | Per-host green condition |
|---|---|---|
| Statement host: `emit_result_region_continued` (`src/codegen/core.rs:1871-1929`) | Project `region.value`; register the ResultRegion call for HostExit capture; write success/failure through a slot and labelled break where needed | Tail match assigns `Ok(1)`/`Ok(2)`; item `return 42` and inline-if-let return assign `Ok(...)`; no path returns a raw arm/body value from the user function |
| Expression boundary: `emit_result_region` (`src/codegen/core.rs:1832-1868`) | Give the printer a real continuation so plain result-owned returns are rewritten inside the lexical arrow | Plain `return 99` yields `Ok(99)`, not raw `99`; tail match remains `Ok(1)` as a regression guard |

Both greens require runtime assertions, emitted-TypeScript parse verification, source-map checks, and typed-clean output. Testing only the expression-host match tail does not green this slice because that shape is already confined by its nested `$tt_expr` arrow today.

The statement printer uses `emit_body_with_exits` to assign `slot = Ok(value)` or the propagated `Err` and break the ResultRegion label (`src/codegen/core.rs:1027-1069`). The expression printer returns the same completion from a lexical arrow. The arrow preserves `this`, `super`, `arguments`, and `new.target`; `finally` has ordinary JavaScript precedence in both encodings.

### 6.3 Placement, diagnostics, and recovery

Evaluation IR computes capability and preserves its `ExpressionBoundaryReason`; analysis/sema emits `try-placement` or `try-crosses-value-region`. Codegen does not invent policy, and `lib.rs` must not erase the reason (`src/lib.rs:1001-1018`). Rejected-expression recovery may emit `undefined` only to continue diagnostics.

`codegen::lowering_plan` must return overlay parse and `MissingHost` failures instead of calling `ice::bug!` (`src/codegen/core.rs:49-76`). Every consumer—`ttc::analyze`, `ttc::compile_report`, CLI, JSON server, content mapper, and Engine/Snapshot—must convert the failure to a located tt diagnostic. The server's per-request catch (`src/server.rs:112-117`) is resilience, not success; the compile-report path at `src/server.rs:441` must produce diagnostics rather than a bug-only payload.

## 7. Removed and migrated surface

Remove `<-` Result bindings, the semicolon-free tail requirement, the direct-result-body `try` ban, the result-body let-else ban, and copy that says `try` always returns from the enclosing function.

Replace the old missing-tail rule with `result-no-success-value`. Retire `result-tail-semicolon` across `DiagnosticCode`, `ttc explain`, server JSON, snapshots, and the content-mapper allowlist (`src/diagnostics.rs:121`, `src/content_mapper.rs:93-116`). A one-release explain alias may point to the new rule but is not emitted.

Remove `ResultBind` and `AnchorKind::ResultBind` only after source maps, checker anchors, semantic tokens, and content mapping have replacements (`src/lib.rs:552`). Preserve passthrough for identifier `result`, TypeScript `try {}`, members named `try`, and spaced `a < -b`.

The shape accepted on `33acccc` in which a value-producing isolated region contains a function-targeted `try` while lexically inside `result` becomes `try-crosses-value-region`. It must fail loudly at the `try` span. For one release, the diagnostic states that #87 accepted function-level propagation through this position and supplies an applicable nested-function extraction edit that preserves conditional evaluation and the old function-level target. The edit is offered only when the compiler can prove the generated helper preserves captures, evaluation count/order, and host syntax; every such migrated shape must still receive located help if applicability cannot be proven. There is no scope opt-out. This migration diagnostic ships in the same release that removes `<-`.

## 8. Compiler-layer contract (AGENTS.md)

- AST/parser own one expression-`try` surface, speculative Result claiming, statement-bodied Result syntax, and lossless recovery. Unifying `TryStmt`/`TryExpr` must preserve ternary-balance recovery and parse-first/validate-second behavior.
- HIR carries lexical ownership but not host placement policy.
- Core IR owns ResultRegion identity, Result-targeted exits, structural suspension, and validation.
- ProgramSyntax/Evaluation IR own host projection, reach, continuation capability, placement reason, and structural exit capture.
- Resolve/analysis/sema own nearest-scope resolution and named tt diagnostics.
- Codegen consumes the validated plan and emits source-preserving TypeScript. It does not guess placement, suppress a failed projection, or fall back to a different Result target.
- TypeScript backend projection returns only checker facts. tt constructs `result-return-nested` and its suggestion.
- CLI, server, mapper, and Engine expose the same located diagnostics and never use an unwind or empty answer as recovery.

## 9. Implementation order

Use the ordered, green plan below. Begin from `33acccc`; do not re-integrate TASK-280 or recreate old slice 1. Independent shipped-code prerequisites land first, the one-release migration behavior is then fixed as a contract, and language slices follow without reopening placement or ownership decisions.

Every item is a green commit with its tests. P0–P6 are independent shipped-code tasks, each with its own `docs/tasks/TASK-NNN-*.md` record against `33acccc`; each may merge to `main` alone and remains valuable if this proposal is abandoned. No language slice begins until all prerequisites are Complete on `main` or have been cherry-picked onto the implementation branch and the P-matrix gate below is green.

1. **P0 — Preserve the planner's existing failure return.** Scope: stop the codegen wrapper from converting Evaluation IR's already-fallible lowering result into `ice::bug!`, and carry it as a located diagnostic across every public client. Files/symbols: `src/codegen/core.rs::lowering_plan` (the wrapper at `:49-76` that converts `evaluation_ir::EvaluationFile::lowering_plan`'s existing `Result<LoweringPlan, EvaluationError>` at `src/evaluation_ir.rs:484` into `ice::bug!`; `LoweringPlan` itself is `src/evaluation_ir.rs:109`), `src/lib.rs::{analyze,compile_report}`, CLI, `src/server.rs`, `src/content_mapper.rs`, and Engine/Snapshot entry points. Tests: run the for-header, expression-boundary Result, and discarded-Result failures through `catch_unwind` at library level and through CLI/server/mapper/Engine; assert non-empty located diagnostics, not exit 101, a bug payload, or an empty vector. Green: no source input unwinds and every consumer reports the same tt code. **Ships to `main` alone: yes.**

2. **P1 — Close host projection and source-preservation crashes.** Scope: make C-style declaration-init Legal by hoisting only its initializer value while retaining `let i` in the header; make repeated for-test a located rejection; make discarded Result source preservation diagnose rather than ICE. Files/symbols: `src/program_syntax.rs::{owner_reach,HostContinuation}`, `src/program_syntax.rs::emit_result_region` (the ProgramSyntax projection at `:1078`, not the codegen printer of the same name at `src/codegen/core.rs:1832`), `src/evaluation_ir.rs`, `src/codegen/core.rs::lowering_plan`, and relevant diagnostics. Tests: statement and expression-boundary host cases, `for (let i = try n();;)`, `for (; try ready();)`, assignment-init guard, and `result { ... };`, all wrapped around `analyze` and verified as parseable output or located diagnostics. Green: Legal init executes once and retains declaration semantics; repeated test and discard never panic. **Ships to `main` alone: yes.**

3. **P2 — Repair expression-boundary Result hosting.** Scope: replace the #87 `MissingHost` path for a current Result containing value-form `try` with the current language's located placement result, without changing passthrough. Files/symbols: `src/program_syntax.rs::expected_exit_calls`, `src/program_syntax.rs::emit_result_region` (the ProgramSyntax projection at `:1078`, not `src/codegen/core.rs::emit_result_region`, the expression-boundary printer at `:1832`), `src/evaluation_ir.rs` target capability, and `src/codegen/core.rs::{lowering_plan,emit_result_region}`. Tests: the same Result input in statement-capable and expression-boundary owners through analyze/compile/server/Engine; include the already-correct expression-host match-tail guard. Green: no `MissingHost`, no unwind, and a stable located code at every host. **Ships to `main` alone: yes.**

4. **P3 — Repair concise-arrow propagation.** Scope: make ordinary, parenthesized, and pipeline-step concise arrows containing expression `try` lower to valid block-bodied arrows without moving the return target. Files/symbols: arrow/pipeline ownership in `src/program_syntax.rs`, `src/evaluation_ir.rs`, and corresponding codegen printers. Tests: default verification and `--no-verify` output parse, preserve evaluation order and lexical capture, and keep standalone concise `=> try x` behavior. Green: every accepted output parses and the arrow, never its enclosing function, owns failure. **Ships to `main` alone: yes.**

5. **P4 — Reject unsound function targets.** Scope: reject statement and expression `try` in constructors, generators, and async generators, including constructor positions before/inside/after `super`, while retaining nested ResultRegion and `using` legality. Files/symbols: `src/program_syntax.rs::EvaluationOwner`, `src/evaluation_ir.rs::TargetCapability`, `src/sema.rs` placement reporting, diagnostics. Tests: located `try-placement` for both forms; runtime guard that `new C() instanceof C`; generator guard proving no emitted program can produce `{value: Err, done: true}` or silently truncate `for...of`; nested Result and disposal acceptance tests. Green: unsafe inputs emit nothing and never rely on `ts2409`, TypeScript types, or consumer behavior as the signal. **Ships to `main` alone: yes.**

6. **P5 — Close shipped claimer gaps.** Scope: make `for` update, object/array spread, and local destructuring defaults either enter the existing placement protocol or receive the intended located rejection instead of `verify-failed`. Files/symbols: `src/parser/tries.rs`, AST/HIR traversal, ProgramSyntax owner projection, sema recovery. Tests: one claim/recovery test per syntactic edge plus passthrough controls for members/properties named `try`. Green: every candidate is either valid emitted TypeScript or a located `try-placement`, never “did not parse as tt try.” **Ships to `main` alone: yes.**

7. **P6 — Preserve placement reasons.** Scope: stop erasing `ExpressionBoundaryReason`, report static-block ownership accurately, and correct the stale `Place::ValueRegion` result-body comment at `src/sema.rs:252-253` to match the `Place::ResultRegion` behavior at `src/sema.rs:627-634`. Files/symbols: `src/lib.rs::try_target_errors`, `src/sema.rs` placement messages and comments, `src/program_syntax.rs::EvaluationOwner::StaticBlock`, diagnostic rendering. Tests: distinct reason/message assertions for repeated loop positions, parameter owners, static blocks, constructors, and isolated crossings. Green: all consumer surfaces retain the same typed reason and original try span, and sema documentation names the implemented place. **Ships to `main` alone: yes.**

**P-matrix — prerequisite placement gate, not a language slice.** Scope: before M0 or any L-slice, enumerate every §4.5 row at both a statement host and an expression-boundary host. Files/symbols: the placement integration tests around `src/program_syntax.rs`, `src/evaluation_ir.rs`, `src/sema.rs`, and the public `ttc::analyze` path. Tests: each row asserts a specific diagnostic code or parseable emitted TypeScript, with `catch_unwind` around `ttc::analyze`; include `using`/`await using`, constructor, generator/async generator, `for` declaration-init/test/update and `for-of` RHS, concise arrows, isolated crossings, and whole ResultRegion hosts. Green: no row panics and no row ends as `verify-failed` “did not parse as a tt `try`”; all P5 claimer cases must therefore be closed before the gate passes.

8. **M0 — Freeze the one-release crossing migration.** Scope: add the `try-crosses-value-region` compatibility diagnostic for the #87-accepted isolated-arm shape, its applicable nested-function extraction edit where mechanically proven safe, and the release-note/help contract; it is staged before language work but published only with the `<-` cutover. Files/symbols: nearest-scope/crossing analysis in `src/analysis` and `src/sema.rs`, diagnostics and edit spans, `docs/ai/tt.md`, snapshots/content mapper. Tests: preserve conditional evaluation, captures, argument order, and old function-level propagation after applying the edit; require located help without an edit when applicability cannot be proven. Green: the compatibility diagnostic fires on every #87-accepted isolated-arm shape, and every offered edit parses, type-checks, and preserves runtime behavior, captures, and evaluation order/count. The anti-retargeting assertion—that no affected program silently moves from a function target to a ResultRegion target—is not testable until nearest-scope propagation exists and is therefore an L2 exit criterion, re-run there against the M0 corpus. **Ships to `main` alone: no; it lands in the same release as L4's `<-` removal.**

9. **L0 — Repair Result completion in both existing printers.** Scope: implement ResultRegion call registration and `region.value` projection together, plus a real expression-host continuation, so result-owned return always wraps `Ok`. Files/symbols: `src/program_syntax.rs::expected_exit_calls`, `src/program_syntax.rs::emit_result_region` (the ProgramSyntax projection at `:1078`, not `src/codegen/core.rs::emit_result_region`, the expression-boundary printer at `:1832`), `src/codegen/core.rs::{emit_result_region,emit_result_region_continued,emit_body_with_exits,wrap_result_ok}`, and `docs/ai/tt.md` (the result-block `return` rule, cited by content rather than line number). Tests: the complete two-row §6.2 matrix, runtime assertions, output snapshots, source maps, and type checking. Green: each host independently satisfies its §6.2 condition; neither a raw arm nor raw body value returns from the user function; in the same commit, the language guide states that a result-owned `return x` completes the block with `Ok(x)` and a bare `return;` with `Ok(undefined)`. Constructor/generator policy is not part of this slice.

10. **L1 — Add scope and completion identity.** Scope: introduce stable `ResultRegionId`, Result-targeted exits, structural `is_async`, ownership stacks, labels, finally semantics, and validation shared by both printers. Files/symbols: `src/core_ir/mod.rs::{ExitTarget,ResultRegion}`, `src/core_ir/lower.rs`, ProgramSyntax projection, Core validation, codegen completion mapping. Tests: nested functions/results, invalid/stale target validation, await across nested boundaries, label collision, and finally override for success and failure. Green: every completion has exactly one live destination and both printers consume an identical semantic record.

11. **L2 — Cut over syntax and nearest-scope propagation.** Scope: parse statement-bodied Result blocks claimed by one nearest lexical tt `try`, lower inner propagation to ResultRegion, preserve #87 prefix-primary syntax, and retract the `<-` help at cutover. Files/symbols: `src/parser/tries.rs` and result parser, AST/HIR, resolve/analysis nearest-scope walk, Core lowering, `src/sema.rs:451-466`, `docs/ai/tt.md:65-77`. Tests: every §5 example, nested claim stops, ASI and TypeScript passthrough corpus, one-target validation, source maps, both host capabilities, and the full M0 compatibility corpus. Green: claimed inner try exits only its nearest ResultRegion; nested function try exits its function; no valid TypeScript bytes change; no M0-affected program silently moves from a function target to a ResultRegion target.

12. **L3 — Add control-flow and use diagnostics.** Scope: implement `result-no-success-value`, `result-value-discarded`, let-else completion, outward break/continue/label/yield crossing checks, and permanent isolated-region crossing behavior while retaining M0's one-release migration help. Files/symbols: resolve/analysis/sema CFG, `HostExit` capture, diagnostic registry/explain, server/content mapper. Tests: all abrupt completions, unreachable versus reachable fallthrough, discard variants, inline if-let/let-else, nested Decisions, and generator/yield crossings. Green: every path either produces one Result completion or one named diagnostic; no redundant primary diagnostics.

13. **L4 — Remove `<-` and the old tail surface.** Scope: delete ResultBind parsing/HIR/anchors, semicolon-free tail semantics, and `result-tail-semicolon`, while activating the M0 migration and applicable `= try` edits in the same release. Files/symbols: result parser, AST/HIR, `AnchorKind::ResultBind`, diagnostics/explain, mapper, semantic tokens, snapshots, docs. Tests: applicable edits for old binds, `a < -b` passthrough, identifier `result` corpus, anchor/source-map replacements, and one-release aliases. Green: no old syntax is emitted or silently accepted, every supported migration edit is valid, and all external diagnostic surfaces agree.

14. **L5 — Add typed nested-Result diagnostics.** Scope: implement the definite Result-shape checker query and `result-return-nested` without adding untyped guesses. Files/symbols: `src/engine/projection.rs`, `src/typescript/backend.rs`, `src/engine/semantics.rs`, diagnostic suggestions. Tests: definite Result, union/non-Result/unknown/generic cases, `return try` edit, server and Engine typed paths. Green: only checker-proven nested Results diagnose, and ordinary TypeScript errors remain TypeScript's responsibility.

15. **L6 — Complete public documentation and release gates.** Scope: update the AI language guide, design status, reference/examples, migration notes, and task records, then run the full repository gate. Files/symbols: `docs/ai/tt.md`, this design, user-facing English documentation, `docs/tasks`, fixtures. Tests: documentation examples compile or diagnose as shown; run `./scripts/ci`, including fmt, clippy, all tests, TypeScript verification, snapshots, server/mapper, and runtime integration. Green: the full gate passes from a clean worktree and the published docs describe no removed rule.

## 10. Non-goals

- Option propagation (`try` on `Option`).
- Implicit error conversion.
- Auto-wrapping `Ok` on ordinary function `return`.
- Auto-flattening a `Result` returned from a `result` block.
- Making `result { }` claimable with no inner `try`.
- Using `$tt_expr` to make loop-header or parameter-default `try` legal.
- IIFE fallbacks the evaluation protocol already forbids.
- Changing the `Result` representation (`kind` / `value` / `error`).
- Generator-owned and async-generator-owned function propagation is rejected as a current safety rule. A future generator Result protocol is outside this design and is not a condition attached to the rejection.
- `yield`/`yield*` crossing a ResultRegion is a named diagnostic and not lowered.
- Making decorators into function Result scopes is a non-goal; a whole nested ResultRegion remains subject only to the host's ordinary expression capability.
- A uniform variable environment across both Result printers is a non-goal.
- Scope opt-outs for `try-crosses-value-region` are a non-goal.

## 11. Decisions already taken in this discussion

Do not reopen these unless the language contract is shown to break.

1. **One `try`.** `<-` is the same operation with a different exit target.
   Keep the operation; drop the second spelling.
2. **`result` stays.** It is the inner Result scope, not a second unwrap
   syntax.
3. A useful result success is written with result-owned `return`; reachable fallthrough is `result-no-success-value`.
4. Result-owned `return x` means `Ok(x)`; ordinary function returns never auto-wrap.
5. Illegal hosts are located tt errors. Repeated and unmodeled-conditional edges are distinct classes.
6. The host evaluation protocol is the only expression-`try` lowering; parser hoists and TypeScript FlowNodes are not alternatives.
7. `<-` and the semicolon-free tail form are removed in one migration release.
8. Nearest lexical Result scope selects the failure target; functions and ResultRegions have distinct identities.
9. Function-targeted propagation through an isolated value region remains Legal, but propagation targeting an outer ResultRegion across that region is `try-crosses-value-region`.
10. `for` declaration init and `using`/`await using` are Legal. Repeated for-test/update positions are rejected.
11. Constructor-, generator-, async-generator-, and static-block-owned function propagation is rejected; nested ResultRegions remain Legal.
12. Result completion has one semantic representation and two host-selected printers; ResultRegion call registration and `region.value` projection are both required.

## 12. Current tree vs this proposal

`33acccc` already ships TASK-280 as PR #87: expression `try`, prefix-primary parsing, function-targeted Core propagation, and host-evaluation lowering are the baseline. Preserve that valid surface while repairing the independent defects in §9. The tree does not yet implement ResultRegion identity, Result-targeted propagation, statement-bodied Result syntax, Result-owned completion capture, `<-` removal, crossing/must-use diagnostics, or typed nested-Result detection.

The historical TASK-280 gate has been spent. At the syntax cutover, retract both the current sema help and the user guide rule directing inner Result propagation to `<-` (`src/sema.rs:451-466`, `docs/ai/tt.md:65-77`). A passing historical `cargo test` is necessary but insufficient because it caught none of the prerequisite crash, invalid-emit, public-client, constructor, or generator failures; the §9 obligations are required gates.
