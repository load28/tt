# TASK-392: Call member pipeline steps on their receiver

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`x |> f` means `f(x)` (docs/ai/tt.md). With an inert head, `1 |> obj.add` already compiled to `obj.add(1)`. Every other member step compiled to `$tt_ap(value, obj.add)`, which passes a detached function, so `this` was `undefined`: `[1].map(x => x |> obj.add)`, `1 |> obj.add |> obj.add`, `x |> this.m` in a class, and `flow |> obj.add` all threw `TypeError`. Method semantics depended on the shape of the head.

## Scope

- Included: Call steps of value pipelines and `flow` compositions whose step is a member reference: `obj.m`, `obj[k]`, `obj.#m`, `super.m`, `super[k]`, including parentheses and TypeScript wrappers (`!`, `as`, `satisfies`, type assertions) around it.
- Excluded: Optional-chain callees (`obj?.m`), whose short-circuit is part of the call they would be written in; they keep the previous form.

## Decisions

### Decision 1: Recognise the member callee from the TypeScript AST of the step

- **Context**: Only the step's own syntax says whether it is a property reference. The step is source text the program-syntax projection does not see.
- **Alternatives considered**: A token-level scan cannot decide precedence (`await obj.m` is not a member callee).
- **Decision and rationale**: `program_syntax::source_member_callee` parses the step with swc and reports the receiver and computed-key spans. A step containing `await` or `yield` is parsed again in an async generator module context (`Context::Module | CanBeModule | InAsync | InGenerator`, following `parse_await_expr` in the vendored parser); the receiver and key spans are the same text under either reading.

### Decision 2: Keep `f(x)` call semantics without a runtime helper

- **Context**: The value must be evaluated before the step (the existing `$tt_ap` order), the receiver must be evaluated where it is written (it may contain `await`), and TypeScript must still infer a generic method's type arguments from the argument.
- **Alternatives considered**: A runtime helper taking a receiver and an accessor instantiates generic methods with `unknown`, which would reject `const n: number = x |> gen.id`. An arrow that closes over the receiver cannot contain `await`.
- **Decision and rationale**: A value step is emitted as `(($tt_v, $tt_r) => ($tt_r.m)($tt_v))(value, (receiver))`, with `$tt_k` for a computed key and no receiver parameter for `super`. Arguments evaluate value, receiver, then key, and the call inside is an ordinary method call. The step span is recorded as relocated for the source-preservation check.

### Decision 3: Bind member steps of `flow` at composition time

- **Context**: `$tt_fl(f, g)` evaluates each step when the composition is built.
- **Decision and rationale**: A `flow` member step becomes `(($tt_r) => ($tt_r.m).bind($tt_r))((receiver))`, or `(super.m).bind(this)`. Receiver and property are read when the composition is built, as before; `bind` keeps the method's own type, including its type parameters.

## Work log

- 2026-09-27: Reproduced the `TypeError` cases with `ttc -p` and Node. Added `source_member_callee`, the planning table `member_apply_steps`, and the emitter forms.
- 2026-09-27: A step with `await` was not recognised; the swc error was `TopLevelAwaitInScript`. Added the second parse context per the vendored parser's rule.
- 2026-09-27: Verified with Node and with the repository TypeScript (`node_modules/.bin/tsc --strict`) that `this`, private methods, `super`, computed keys, generic inference, `await` receivers, and head-before-getter order hold. Added an integration test; it fails on the previous compiler.

## Issues and resolutions

### Issue 1: Member steps lost `this`

- **Symptom**: `TypeError: Cannot read properties of undefined (reading 'k')`.
- **Cause**: `$tt_ap(value, obj.add)` and `$tt_fl(f, obj.add)` pass the property value without its receiver.
- **Resolution**: Decisions 1–3.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.
- [x] `a_member_step_calls_the_method_on_its_receiver` fails with the previous `src/` and passes now.

## Result

Changed `src/program_syntax.rs`, `src/codegen/core/planning.rs`, `src/codegen/core/mod.rs`, `src/codegen/core/emitter/mod.rs`, `src/codegen/core/emitter/expression.rs`, `tests/integration.rs`, and `docs/ai/tt.md`.
