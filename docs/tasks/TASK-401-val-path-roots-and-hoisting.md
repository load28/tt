# TASK-401: Judge only val-rooted paths and resolve hoisted declarations

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`val` forbids mutation through access paths rooted at a `val` binding (docs/ai/tt.md, `val` section). Plain `ttc` reported `val-mutation` for paths that are not rooted at the binding at all: `state.config.debug = true` and `this.config.debug = true` with `val const config`, and `q.o.x++` with `val const o`. It also reported `o.x = 2` inside `function w() { o.x = 2; var o = { x: 1 }; }`, where `o` is w's own hoisted `var`, not the outer `val` binding.

## Scope

- Included: The assignment-target collector of the `val` checker (`src/val/targets.rs`) and the scope model of the untyped `val` walk (`src/val/checker.rs`, `src/val.rs`): `var` hoisting to the enclosing function, script, class static block, or namespace body, and function declarations binding throughout their block.
- Excluded: `let`/`const`/`class` temporal dead zones. A use before a lexical declaration in the same block still resolves to an outer binding in the untyped model; such a use is itself a TypeScript error (TS2448/TS2454) and a runtime `ReferenceError`, and the typed path (`--check-types`) resolves every root by symbol identity already. No test targeted it and it was not reported.

## Decisions

### Decision 1: A property name is never the root of a write target

- **Context**: `targets::writes` tried every token index as the start of an assignment target. Starting at `config` in `state.config.debug = true` produced a target `config.debug` rooted at `config`, and the walk then treated that dotted identifier as a root because it was in the write set. `visit_ident` lets a dotted identifier through only when it is a write root, because the `.` of a spread (`[...o.p] = xs`) is also a `.` token.
- **Alternatives considered**: Dropping the write-set exception in `visit_ident` would lose the spread-target case (`[...o.p] = xs`, which is an ECMA-262 AssignmentRestElement whose DestructuringAssignmentTarget is `o.p`). Filtering by name would be a heuristic.
- **Decision and rationale**: In ECMA-262 a MemberExpression `a.b` / OptionalChain `a?.b` has an IdentifierName after the `.`/`?.`, which is not an IdentifierReference, so it can never start a LeftHandSideExpression. `reference` now refuses a start whose previous token is `?.` or a member `.` (a `.` that is not the last dot of `...`). The typed probe collection shares this collector, so it no longer collects these non-roots either.

### Decision 2: Model declaration instantiation instead of sequential declaration

- **Context**: The walk declared a binding when it reached the declaration's keyword, in the innermost frame. ECMA-262 FunctionDeclarationInstantiation (10.2.11) instantiates every VarDeclaredName of a function body (all `var` declarations outside nested functions, and top-level function declarations) at function entry; BlockDeclarationInstantiation (14.2.3) instantiates a block's function declarations at block entry (strict code: TypeScript modules and classes). GlobalDeclarationInstantiation/module instantiation does the same for the file.
- **Alternatives considered**: Declaring `var` into the innermost block (the old behaviour) makes `{ var o } o.x = 1` resolve to an outer `val`, and makes an earlier use resolve outward. Hoisting lexical declarations too was set aside (see Scope).
- **Decision and rationale**: `Checker::instantiate` runs when a scope is entered. It declares the block's statement-level function declarations and, when the scope is a `var` scope (a function body, an arrow's block body, a class `static` block, a namespace body, or the file), every `var`/`val var` declaration in it outside nested `var` scopes. The walk no longer declares `var` at the keyword. A `catch` block is not a `var` scope (ECMA-262 14.15; `var` in it belongs to the enclosing function).

### Decision 3: Match arms are blocks of their function

- **Context**: The walk models a `match` arm pattern like a parameter list, so an arm body looked like an arrow function body and would have become its own `var` scope.
- **Decision and rationale**: tt lowers a match arm into a `case` block of the enclosing function (docs/ai/tt.md: match never emits an IIFE or callback), so a `var` in an arm is hoisted to that function in the emitted TypeScript. The first top-level arrow of each entry of a `match ( ... ) { ... }` body is an arm arrow and does not open a `var` scope; an arrow inside the arm body still does.

## Work log

- 2026-09-27: Reproduced with `ttc --check`: the three member-rooted cases and the hoisted `var`/function cases each reported `val-mutation`; `[...o.p] = xs` and `({ a: o.p } = v)` correctly reported.
- 2026-09-27: Fixed `targets::reference`. Added `Checker::instantiate`, `nested_var_scope`, `var_scope_block`, `arm_arrows`, and split `function_body` out of `param_scope`. Checked `var` in blocks, `for (var ...)`, `catch`, nested functions, bare-parameter arrows, static blocks, `val var`, and match arms.
- 2026-09-27: Updated the `val` section of docs/ai/tt.md.
- 2026-09-27: Added `val_judges_only_paths_rooted_at_the_binding` and `val_resolves_hoisted_declarations_to_their_scope` (tests/compile/cases_07.rs); both fail on the previous compiler and pass now. Added `a_property_that_shares_a_val_binding_name_is_not_that_binding` (tests/native/cases_02.rs) as a typed-path contract; it passed before as well, since the typed verdict resolves the collected root by symbol.

## Issues and resolutions

### Issue 1: Match-arm bodies looked like function bodies

- **Symptom**: With the first implementation a `var` in a match arm was scoped to the arm, while the emitted `case` block hoists it to the function.
- **Cause**: `param_scope` treats `A(n) =>` as a parameter list.
- **Resolution**: Decision 3.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/val/targets.rs`, `src/val/checker.rs`, `src/val.rs`, `docs/ai/tt.md`, `tests/compile/cases_07.rs`, `tests/native/cases_02.rs`. Only paths rooted at a `val` binding are judged, and `var` and function declarations shadow an outer `val` over the scope ECMAScript gives them.
