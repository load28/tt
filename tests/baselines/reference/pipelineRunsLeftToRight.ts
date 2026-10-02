//// [pipelineRunsLeftToRight.tt] ////

const order: string[] = [];
const tap = <T,>(name: string) => (v: T): T => { order.push(name); return v; };
const out = (order.push("head"), 10) |> tap("s1") |> .toFixed(0) |> tap("s2");
console.log(order.join(","), out);

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [pipelineRunsLeftToRight.ts]
import { $tt_ap } from "./tt/runtime.js";

const order: string[] = [];
const tap = <T,>(name: string) => (v: T): T => { order.push(name); return v; };
const out = $tt_ap($tt_ap((order.push("head"), 10), tap("s1")).toFixed(0), tap("s2"));
console.log(order.join(","), out);

export {};
