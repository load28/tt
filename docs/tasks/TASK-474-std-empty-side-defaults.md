# TASK-474: std combinators keep the side a callback never returns

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`const a = Result.andThen(r, (n) => Result.Ok(n + 1))` with `r: TResult<number, string>` typed `a` as `TResult<number, unknown>`, so `const a2: TResult<number, string> = a` failed. `docs/ai/tt.md` §@tt/std promises `TResult<T, E>` + `(T) => TResult<U, F>` → `TResult<U, E | F>` and says the constructors fit any slot. The same loss hit `orElse`/`orElseP` recoveries that return only `Ok`, `Err`-only callbacks (the ok side), `map` on an `Ok` value, `mapErr` on an `Err` value, `collect` of `Ok` values, and Option `andThen`/`orElse` callbacks that return only `None`.

## Scope

- Included: The type parameters of `src/stdlib/result.ts` and `src/stdlib/option.ts` (no runtime change), `docs/ai/tt.md`, and integration tests in `tests/integration/cases_02.rs`.
- Excluded: Parameters that always have an inference source (a callback's own return for `map`, a fallback value for `unwrapOr`). `async` callbacks: `andThen` takes a synchronous `TResult` step, and a `Promise` return stays a type error as before.

## Decisions

### Decision 1: Default a side with no inference source to `never`

- **Context**: `Result.Ok` returns `TOk<U>` alone. Inferring `U` and `F` from `TOk<number>` against `TResult<U, F>` (= `TOk<U> | TErr<F>`) finds a candidate only for `U`. The TypeScript handbook (Generics, "Generic Parameter Defaults") and the inference rule in the checker say a type parameter with no candidates takes its default when it has one, otherwise its constraint, which for an unconstrained parameter is `unknown`. `TErr<unknown>` is not assignable to `TErr<string>`, which is the reported failure.
- **Alternatives considered**:
  - Infer the callback's whole return type `S extends TResult<unknown, unknown>` and read the sides with conditional helpers (`TErrorOf<S>` and a new `TValueOf<S>`). Correct for `andThen`, but it changes every hover and signature into conditional types, needs a new exported helper, and does not help the input side (`map(Result.Ok(1), f)`), which has the same cause.
  - Reorder parameters so defaulted ones come last. Breaks explicit type arguments already in use (`Result.collect<number, string>(…)`, `Result.flatten<number, string>(…)`).
- **Decision and rationale**: Add `= never` defaults in place, which keeps explicit type argument positions valid. `never` is the exact type of a side no value supplies: `TResult<U, never>` is assignable to every `TResult<U, F>`, and `E | never` is `E`. Where TypeScript requires a default on every later parameter, the later ones also get `never`; they always have candidates when the call is well typed. `andThen`'s `R` gets `TResult<T, never>`, and it is always inferred from `r`.

### Decision 2: Curried forms take the incoming side on the returned function

- **Context**: `mapP`, `mapErrP` and `orElseP` fixed the incoming result's other side at the outer call, where only the callback is visible. `mapErrP(f)` left `T` as `unknown`, and `orElseP(f)` read `T` only from the recovery, so a recovery returning only `Err` gave `T = unknown`. A `never` default at the outer call would reject the input instead.
- **Alternatives considered**: Keep the outer parameters and rely on higher-order inference in pipelines. It does not apply to a direct `mapErrP(f)(r)` call, and it cannot recover a side the callback never names.
- **Decision and rationale**: The returned function is generic in the incoming side (`<E = never>` for `mapP`, `<T = never>` for `mapErrP` and `orElseP`), as `andThenP` already was with `R`. `orElseP` cannot compare the recovery's value type with a `T` it has not seen yet, so it returns `TResult<T | U, F>`; the data-first `orElse` still requires the recovery's value to be `T`. Option `orElseP` takes the same shape (`TOption<T | U>`).

## Work log

- 2026-09-28: Reproduced with a scratch project over the std sources and both the `tsc` on `PATH` (6.0.2) and the pinned `node_modules/typescript` (7.1.0-dev): 11 of 14 exactness checks failed, and so did the reported `andThen` assignment.
- 2026-09-28: Added the defaults and moved the curried input sides in `src/stdlib/result.ts` and `src/stdlib/option.ts`. Both compilers accepted every check: Ok-only, Err-only, a full `TResult` annotation, a `TOk | TErr` union return, `andThenP`/`orElseP`/`mapP`/`mapErrP`, `map`/`mapErr` on single-variant inputs, `collect`, Option `andThen`/`orElseP`, explicit type arguments, and a rejected `orElse` recovery of the wrong type.
- 2026-09-28: Added `std_result_combinators_keep_the_side_a_callback_never_returns` and `std_result_or_else_still_requires_the_recovered_value_type` to `tests/integration/cases_02.rs`. With the old std sources restored the first test fails with the reported `TResult<number, unknown>` error.
- 2026-09-28: Documented the rule in `docs/ai/tt.md` §@tt/std.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/stdlib/result.ts`, `src/stdlib/option.ts`, `tests/integration/cases_02.rs`, `docs/ai/tt.md`, this record, and `docs/tasks/INDEX.md`. A combinator side that no argument supplies is now `never`, so chained and recovered results keep their declared types.
