import type { TResult } from "./result.js";
/** An optional value: either `Some` carrying a `value`, or `None`. */
export type TOption<T> = {
    kind: "Some";
    value: T;
} | {
    kind: "None";
};
/** Constructs `Some(value)`. */
export declare const Some: <T>(value: T) => TOption<T>;
/** The `None` value. */
export declare const None: {
    readonly kind: "None";
};
/** True if `o` is `Some` (narrows the type). */
export declare const isSome: <T>(o: TOption<T>) => o is {
    kind: "Some";
    value: T;
};
/** True if `o` is `None` (narrows the type). */
export declare const isNone: <T>(o: TOption<T>) => o is {
    kind: "None";
};
/** Applies `f` to the value inside `Some`; leaves `None` untouched. */
export declare const map: <T, U>(o: TOption<T>, f: (value: T) => U) => TOption<U>;
/** Chains a computation that itself returns an `Option`. */
export declare const andThen: <T, U = never>(o: TOption<T>, f: (value: T) => TOption<U>) => TOption<U>;
/** Returns `o` if it is `Some`, otherwise the fallback produced by `f`. */
export declare const orElse: <T = never>(o: TOption<T>, f: () => TOption<T>) => TOption<T>;
/** Keeps `Some` only when the predicate holds. */
export declare const filter: <T>(o: TOption<T>, pred: (value: T) => boolean) => TOption<T>;
/** The value inside `Some`, or `fallback` for `None`. */
export declare const unwrapOr: <T>(o: TOption<T>, fallback: T) => T;
/** The value inside `Some`, or the result of `f` for `None`. */
export declare const unwrapOrElse: <T>(o: TOption<T>, f: () => T) => T;
/** The value inside `Some`; throws `Error(message)` for `None`. */
export declare const expect: <T>(o: TOption<T>, message: string) => T;
/** Converts to a `Result`: `Some` becomes `Ok`, `None` becomes `Err(error)`. */
export declare const okOr: <T, E>(o: TOption<T>, error: E) => TResult<T, E>;
/** Wraps a nullable value: `null`/`undefined` become `None`. */
export declare const fromNullable: <T>(value: T | null | undefined) => TOption<T>;
/** Unwraps to a nullable value: `None` becomes `null`. */
export declare const toNullable: <T>(o: TOption<T>) => T | null;
/** Pairs two options: `Some` of the tuple only when both are `Some`. */
export declare const zip: <T, U>(a: TOption<T>, b: TOption<U>) => TOption<[T, U]>;
/** Flattens one level of nesting: `Some(Some(x))` becomes `Some(x)`. */
export declare const flatten: <T>(o: TOption<TOption<T>>) => TOption<T>;
/** Swaps `Option<Result<T, E>>` into `Result<Option<T>, E>`. */
export declare const transpose: <T, E>(o: TOption<TResult<T, E>>) => TResult<TOption<T>, E>;
/** Collects all `Some` values, or returns `None`. */
export declare const collect: <T>(items: readonly TOption<T>[]) => TOption<T[]>;
/** Curried `map` for pipelines. */
export declare const mapP: <T, U>(f: (value: T) => U) => (o: TOption<T>) => TOption<U>;
/** Curried `andThen` for pipelines. */
export declare const andThenP: <T, U = never>(f: (value: T) => TOption<U>) => (o: TOption<T>) => TOption<U>;
/** Curried `orElse` for pipelines. */
export declare const orElseP: <U = never>(f: () => TOption<U>) => <T = never>(o: TOption<T>) => TOption<T | U>;
/** Curried `filter` for pipelines. */
export declare const filterP: <T>(pred: (value: T) => boolean) => (o: TOption<T>) => TOption<T>;
/** Curried `unwrapOr` for pipelines. */
export declare const unwrapOrP: <T>(fallback: T) => (o: TOption<T>) => T;
/** Curried `unwrapOrElse` for pipelines. */
export declare const unwrapOrElseP: <T>(f: () => T) => (o: TOption<T>) => T;
/** Curried `expect` for pipelines. */
export declare const expectP: <T>(message: string) => (o: TOption<T>) => T;
/** Curried `okOr` for pipelines. */
export declare const okOrP: <T, E>(error: E) => (o: TOption<T>) => TResult<T, E>;
