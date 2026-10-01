//// [optionalPostfixPreservesShortCircuitOrderAndMethodReceiver.tt] ////

const order: string[] = [];
const mark = (name: string, value: number): number => { order.push(name); return value; };
const key = (): "method" => { order.push("key"); return "method"; };
const live = {
  base: 10,
  method(value: number): number {
    order.push(this === live ? "this" : "lost-this");
    return this.base + value;
  },
};
const absent = (() => undefined as typeof live | undefined)();
const hit = (order.push("head-hit"), live) |> ?.[key()]?.(mark("arg", 2));
const miss = (order.push("head-miss"), absent) |> ?.[key()]?.(mark("skipped", 3));
const after = miss |> (value => { order.push("after"); return value ?? -1; });
console.log(hit, miss, after, order.join(","));

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [optionalPostfixPreservesShortCircuitOrderAndMethodReceiver.ts]
import { $tt_ap } from "./tt/runtime.js";

const order: string[] = [];
const mark = (name: string, value: number): number => { order.push(name); return value; };
const key = (): "method" => { order.push("key"); return "method"; };
const live = {
  base: 10,
  method(value: number): number {
    order.push(this === live ? "this" : "lost-this");
    return this.base + value;
  },
};
const absent = (() => undefined as typeof live | undefined)();
const hit = (order.push("head-hit"), live)?.[key()]?.(mark("arg", 2));
const miss = (order.push("head-miss"), absent)?.[key()]?.(mark("skipped", 3));
const after = $tt_ap(miss, (value => { order.push("after"); return value ?? -1; }));
console.log(hit, miss, after, order.join(","));

export {};
