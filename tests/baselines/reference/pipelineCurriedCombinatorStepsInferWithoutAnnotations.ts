//// [pipelineCurriedCombinatorStepsInferWithoutAnnotations.tt] ////

type TOption<T> = { kind: "Some"; value: T } | { kind: "None" };
const Option = {
  Some: <T>(value: T): TOption<T> => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
  mapP:
    <T, U>(f: (value: T) => U) =>
    (o: TOption<T>): TOption<U> =>
      o.kind === "Some" ? { kind: "Some", value: f(o.value) } : { kind: "None" },
  unwrapOrP:
    <T>(fallback: T) =>
    (o: TOption<T>): T =>
      o.kind === "Some" ? o.value : fallback,
};
const half = (n: number): TOption<number> =>
  n % 2 === 0 ? Option.Some(n / 2) : Option.None;

const label: string = half(4) |> Option.mapP(x => x + 1) |> Option.unwrapOrP(0) |> .toFixed(1);

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [pipelineCurriedCombinatorStepsInferWithoutAnnotations.ts]
import { $tt_ap } from "./tt/runtime.js";

type TOption<T> = { kind: "Some"; value: T } | { kind: "None" };
const Option = {
  Some: <T>(value: T): TOption<T> => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
  mapP:
    <T, U>(f: (value: T) => U) =>
    (o: TOption<T>): TOption<U> =>
      o.kind === "Some" ? { kind: "Some", value: f(o.value) } : { kind: "None" },
  unwrapOrP:
    <T>(fallback: T) =>
    (o: TOption<T>): T =>
      o.kind === "Some" ? o.value : fallback,
};
const half = (n: number): TOption<number> =>
  n % 2 === 0 ? Option.Some(n / 2) : Option.None;

const label: string = $tt_ap($tt_ap(half(4), Option.mapP(x => x + 1)), Option.unwrapOrP(0)).toFixed(1);

export {};
