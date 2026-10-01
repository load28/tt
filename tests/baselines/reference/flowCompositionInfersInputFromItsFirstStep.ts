//// [flowCompositionInfersInputFromItsFirstStep.tt] ////

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

const label = flow |> half |> Option.mapP(x => x + 1) |> Option.unwrapOrP(0) |> .toFixed(1);
const s: string = label(4);

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [flowCompositionInfersInputFromItsFirstStep.ts]
import { $tt_fl } from "./tt/runtime.js";

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

const label = $tt_fl($tt_fl($tt_fl(half, Option.mapP(x => x + 1)), Option.unwrapOrP(0)), (($tt_v) => ($tt_v).toFixed(1)));
const s: string = label(4);

export {};
