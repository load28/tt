//// [pipelineThisAndOrder.tt] ////
// A pipeline step calls a method on its receiver with `this` bound, evaluates
// the piped value before the receiver, and an optional step short-circuits.
const log: string[] = [];
function note<T>(label: string, value: T): T {
  log.push(label);
  return value;
}
function flush(title: string, value: unknown) {
  console.log(`${title}: ${JSON.stringify(value)} after ${log.join(" ")}`);
  log.length = 0;
}
class Counter {
  count = 0;
  constructor(readonly name: string) {}
  add(n: number) {
    log.push(`add on ${this.name}`);
    this.count += n;
    return this.count;
  }
  twice(n: number) {
    return this.add(n) + this.add(n);
  }
}
const counter = new Counter("counter");
flush("member step", note("head", 2) |> note("receiver", counter).add);
flush("this in method", 3 |> counter.twice);
flush("computed member", 1 |> counter[note("key", "add" as const)]);
function find(present: boolean): Counter | undefined {
  log.push(`find ${present}`);
  return present ? counter : undefined;
}
flush("optional absent", note("head", 4) |> find(false)?.add);
flush("optional present", note("head", 5) |> find(true)?.add);
flush("optional tail then step", (undefined as string | undefined) |> ?.trim() |> String);
const composed = flow |> ((n: number) => note("double", n * 2)) |> String |> .padStart(3, "0");
flush("flow built", "nothing yet");
flush("flow called", composed(4));
flush("postfix chain", " a,b " |> .trim().split(","));
flush("nested pipelines", [3, 1, 2] |> ((xs: number[]) => xs.map((x) => x |> counter.add)) |> JSON.stringify);
flush("final count", counter.count);
export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [pipelineThisAndOrder.ts]
import { $tt_fl } from "./tt/runtime.js";
// A pipeline step calls a method on its receiver with `this` bound, evaluates
// the piped value before the receiver, and an optional step short-circuits.
const log: string[] = [];
function note<T>(label: string, value: T): T {
  log.push(label);
  return value;
}
function flush(title: string, value: unknown) {
  console.log(`${title}: ${JSON.stringify(value)} after ${log.join(" ")}`);
  log.length = 0;
}
class Counter {
  count = 0;
  constructor(readonly name: string) {}
  add(n: number) {
    log.push(`add on ${this.name}`);
    this.count += n;
    return this.count;
  }
  twice(n: number) {
    return this.add(n) + this.add(n);
  }
}
const counter = new Counter("counter");
flush("member step", (($tt_v, $tt_r) => $tt_r.add($tt_v))(note("head", 2), (note("receiver", counter))));
flush("this in method", counter.twice(3));
flush("computed member", counter[note("key", "add" as const)](1));
function find(present: boolean): Counter | undefined {
  log.push(`find ${present}`);
  return present ? counter : undefined;
}
flush("optional absent", (($tt_v, $tt_r) => $tt_r?.add($tt_v))(note("head", 4), (find(false))));
flush("optional present", (($tt_v, $tt_r) => $tt_r?.add($tt_v))(note("head", 5), (find(true))));
flush("optional tail then step", (($tt_v, $tt_f) => $tt_f($tt_v))((undefined as string | undefined)?.trim(), String));
const composed = $tt_fl($tt_fl(((n: number) => note("double", n * 2)), (($tt_f) => ($tt_v) => $tt_f($tt_v))(String)), (($tt_v) => ($tt_v).padStart(3, "0")));
flush("flow built", "nothing yet");
flush("flow called", composed(4));
flush("postfix chain", " a,b ".trim().split(","));
flush("nested pipelines", (($tt_v, $tt_r) => $tt_r.stringify($tt_v))(((xs: number[]) => xs.map((x) => (($tt_v, $tt_r) => $tt_r.add($tt_v))(x, (counter))))([3, 1, 2]), (JSON)));
flush("final count", counter.count);
export {};
