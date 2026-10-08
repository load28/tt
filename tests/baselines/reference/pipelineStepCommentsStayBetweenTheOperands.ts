//// [pipelineStepCommentsStayBetweenTheOperands.tt] ////
const needsString = (s: string) => s.length;
const f = (x: number) => x;
const g = (n: number) => (x: number) => x + n;
export const a = 1 /*c1*/ |> /*c2*/ f;
export const b = [1, 2]
  // keep me 1
  |> .map((x) => x * 2)
  // keep me 2
  |> .join(",");
export const q = flow
  // keep me 3
  |> f
  // keep me 4
  |> String;
export const p = 1
  // @ts-expect-error -- a number piped into a string step
  |> needsString;
export const c = 1
  // keep me 5
  |> g(1)
  // keep me 6
  |> g(2);

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [pipelineStepCommentsStayBetweenTheOperands.ts]
import { $tt_ap, $tt_fl } from "./tt/runtime.js";
const needsString = (s: string) => s.length;
const f = (x: number) => x;
const g = (n: number) => (x: number) => x + n;
export const a = f( /*c1*/  /*c2*/ 1);
export const b = [1, 2]
  // keep me 1
  .map((x) => x * 2)
  // keep me 2
  .join(",");
export const q = (($tt_g, $tt_f) => $tt_fl($tt_g, ($tt_v) => $tt_f($tt_v)))(
  // keep me 3
  f,
  // keep me 4
  String);
export const p = needsString(
  // @ts-expect-error -- a number piped into a string step
  1);
export const c = $tt_ap(g(1)(
  // keep me 5
  1),
  // keep me 6
  g(2));
