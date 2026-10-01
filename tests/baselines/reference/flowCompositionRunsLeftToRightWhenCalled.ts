//// [flowCompositionRunsLeftToRightWhenCalled.tt] ////

const order: string[] = [];
const tap = <T,>(name: string) => (v: T): T => { order.push(name); return v; };
const f = flow |> tap<number>("s1") |> .toFixed(0) |> tap("s2");
console.log(order.join(","), "|", f(10), "|", order.join(","));

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [flowCompositionRunsLeftToRightWhenCalled.ts]
import { $tt_fl } from "./tt/runtime.js";

const order: string[] = [];
const tap = <T,>(name: string) => (v: T): T => { order.push(name); return v; };
const f = $tt_fl($tt_fl(tap<number>("s1"), (($tt_v) => ($tt_v).toFixed(0))), tap("s2"));
console.log(order.join(","), "|", f(10), "|", order.join(","));

export {};
