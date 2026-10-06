//// [uncapturedFunctionCreationKeepsDefaultsAndBodiesLazy.tt] ////

const trace: string[] = [];
const mark = <T,>(name: string, value: T): T => { trace.push(name); return value; };
function consume(callback: (value?: number) => number, item: {run: (x: number) => number}) {
  trace.push("call");
  return callback() + item.run(2);
}
const first = consume((value = mark("default", 5)) => { trace.push("body"); return value; }, match (mark<boolean>("subject", true)) {
  true => (trace.push("arm"), {run: x => x + 1}),
  false => ({run: x => x - 1}),
});
console.log(first, trace.join(","));
trace.length = 0;
const second = consume(function(value = mark("default", 5)) { trace.push("body"); return value; }, match (mark<boolean>("subject", false)) {
  true => ({run: x => x + 1}),
  false => (trace.push("arm"), {run: x => x - 1}),
});
console.log(second, trace.join(","));

export {};


//// [uncapturedFunctionCreationKeepsDefaultsAndBodiesLazy.ts]
function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
}

const trace: string[] = [];
const mark = <T,>(name: string, value: T): T => { trace.push(name); return value; };
function consume(callback: (value?: number) => number, item: {run: (x: number) => number}) {
  trace.push("call");
  return callback() + item.run(2);
}
let $tt_v0: number;
const $tt_v1: typeof consume = (consume);
{
  const $tt_m = mark<boolean>("subject", true);
  switch ($tt_m) {
    case true: $tt_v0 = 0; break;
    case false: $tt_v0 = 1; break;
    default: throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
  }
}
const first = $tt_v1((value = mark("default", 5)) => { trace.push("body"); return value; }, ($tt_v0 === 0 ? (trace.push("arm"), {run: x => x + 1}) : ({run: x => x - 1})));
console.log(first, trace.join(","));
trace.length = 0;
let $tt_v3: number;
const $tt_v4: typeof consume = (consume);
{
  const $tt_m = mark<boolean>("subject", false);
  switch ($tt_m) {
    case true: $tt_v3 = 0; break;
    case false: $tt_v3 = 1; break;
    default: throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
  }
}
const second = $tt_v4(function(value = mark("default", 5)) { trace.push("body"); return value; }, ($tt_v3 === 0 ? ({run: x => x + 1}) : (trace.push("arm"), {run: x => x - 1})));
console.log(second, trace.join(","));

export {};
