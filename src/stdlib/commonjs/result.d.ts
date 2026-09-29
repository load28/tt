import type { TOption } from "./option.js";
/** The success variant of a `Result`, carrying a `value`. */
export type TOk<T> = {
    kind: "Ok";
    value: T;
};
/** The failure variant of a `Result`, carrying an `error`. */
export type TErr<E> = {
    kind: "Err";
    error: E;
};
/** A success or failure value. */
export type TResult<T, E> = TOk<T> | TErr<E>;
/** The union of error types carried by a result type. */
export type TErrorOf<R> = R extends TErr<infer E> ? E : never;
/** Constructs `Ok(value)`. */
export declare const Ok: <T>(value: T) => TOk<T>;
/** Constructs `Err(error)`. */
export declare const Err: <E>(error: E) => TErr<E>;
/** True if `r` is `Ok` (narrows the type). */
export declare const isOk: <T, E>(r: TResult<T, E>) => r is TOk<T>;
/** True if `r` is `Err` (narrows the type). */
export declare const isErr: <T, E>(r: TResult<T, E>) => r is TErr<E>;
/** Applies `f` to the `Ok` value; leaves `Err` untouched. */
export declare const map: <T, E = never, U = never>(r: TResult<T, E>, f: (value: T) => U) => TResult<U, E>;
/** Applies `f` to the `Err` error; leaves `Ok` untouched. */
export declare const mapErr: <T = never, E = never, F = never>(r: TResult<T, E>, f: (error: E) => F) => TResult<T, F>;
/** Chains a computation and unions its error with the incoming errors. */
export declare const andThen: <T, U = never, F = never, R extends TResult<T, unknown> = TResult<T, never>>(r: R & TResult<T, unknown>, f: (value: T) => TResult<U, F>) => TResult<U, TErrorOf<R> | F>;
/** Recovers from `Err` with a computation returning a `Result`. */
export declare const orElse: <T = never, E = never, F = never>(r: TResult<T, E>, f: (error: E) => TResult<T, F>) => TResult<T, F>;
/** The `Ok` value, or `fallback` for `Err`. */
export declare const unwrapOr: <T, E>(r: TResult<T, E>, fallback: T) => T;
/** The `Ok` value, or the result of `f(error)` for `Err`. */
export declare const unwrapOrElse: <T, E>(r: TResult<T, E>, f: (error: E) => T) => T;
/** The `Ok` value; throws `Error(message)` for `Err`. */
export declare const expect: <T, E>(r: TResult<T, E>, message: string) => T;
/** The `Ok` value as an `Option` (drops the error). */
export declare const ok: <T = never, E = never>(r: TResult<T, E>) => TOption<T>;
/** The `Err` error as an `Option`. */
export declare const err: <T = never, E = never>(r: TResult<T, E>) => TOption<E>;
/** Runs `f`, capturing a thrown exception as `Err`. */
export declare const fromThrowable: <T>(f: () => T) => TResult<T, unknown>;
/** Awaits `p`, capturing a rejection as `Err`. */
export declare const fromPromise: <T>(p: Promise<T>) => Promise<TResult<T, unknown>>;
/** Flattens one level of nesting. */
export declare const flatten: <T, E = never>(r: TResult<TResult<T, E>, E>) => TResult<T, E>;
/** Swaps `Result<Option<T>, E>` into `Option<Result<T, E>>`. */
export declare const transpose: <T, E = never>(r: TResult<TOption<T>, E>) => TOption<TResult<T, E>>;
/** Collects all `Ok` values, or returns the first `Err`. */
export declare const collect: <T, E = never>(items: readonly TResult<T, E>[]) => TResult<T[], E>;
/** Curried `map` for pipelines. */
export declare const mapP: <T, U>(f: (value: T) => U) => <E = never>(r: TResult<T, E>) => TResult<U, E>;
/** Curried `mapErr` for pipelines. */
export declare const mapErrP: <E, F>(f: (error: E) => F) => <T = never>(r: TResult<T, E>) => TResult<T, F>;
/** Curried `andThen` for pipelines. */
export declare const andThenP: <T, U = never, F = never>(f: (value: T) => TResult<U, F>) => <R extends TResult<T, unknown>>(r: R) => TResult<U, TErrorOf<R> | F>;
/** Curried `orElse` for pipelines. */
export declare const orElseP: <E, U = never, F = never>(f: (error: E) => TResult<U, F>) => <T = never>(r: TResult<T, E>) => TResult<T | U, F>;
/** Curried `unwrapOr` for pipelines. */
export declare const unwrapOrP: <T, E>(fallback: T) => (r: TResult<T, E>) => T;
/** Curried `unwrapOrElse` for pipelines. */
export declare const unwrapOrElseP: <T, E>(f: (error: E) => T) => (r: TResult<T, E>) => T;
/** Curried `expect` for pipelines. */
export declare const expectP: <T, E>(message: string) => (r: TResult<T, E>) => T;
