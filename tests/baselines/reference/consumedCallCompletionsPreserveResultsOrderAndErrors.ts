//// [consumedCallCompletionsPreserveResultsOrderAndErrors.tt] ////

variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
const api = {
  base: 10,
  get consume() {
    trace.push("callee");
    return function(this: {base: number}, item: Item) { trace.push("call"); return this.base + item.run(3); };
  },
};
for (const state of [State.Ready(4), State.Empty]) {
  trace.length = 0;
  const result = mark("receiver", api).consume(match (mark("subject", state)) {
    Ready(value) => (trace.push("arm"), {kind: "item", run: x => x + value}),
    Empty => ({kind: "item", run: x => x}),
  }) + mark("after", 1);
  console.log(result, trace.join(","));
}
function throws(item: Item): number { trace.push("throws:" + item.run(1)); throw new Error("consumer"); }
trace.length = 0;
let unreached = "kept";
try {
  unreached = "lost:" + throws(match (State.Ready(2)) {
    Ready(value) => { trace.push("value"); return {kind: "item", run: x => x + value}; },
    Empty => ({kind: "item", run: x => x}),
  });
} catch { trace.push("caught"); }
console.log(unreached, trace.join(","));

export {};


//// [consumedCallCompletionsPreserveResultsOrderAndErrors.ts]
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

type State =
  | { kind: "Ready"; value: number }
  | { kind: "Empty" };
const State = {
  Ready: (value: number): State => ({ kind: "Ready", value }),
  Empty: { kind: "Empty" } as const,
};
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
const api = {
  base: 10,
  get consume() {
    trace.push("callee");
    return function(this: {base: number}, item: Item) { trace.push("call"); return this.base + item.run(3); };
  },
};
for (const state of [State.Ready(4), State.Empty]) {
  trace.length = 0;
  let $tt_v0: number;
  const $tt_v2 = (mark("receiver", api));
  {
    const $tt_m = mark("subject", state);
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        $tt_v0 = $tt_v2.consume((trace.push("arm"), {kind: "item", run: x => x + value}));
        break;
      }
      case "Empty": {
        $tt_v0 = $tt_v2.consume(({kind: "item", run: x => x}));
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const result = $tt_v0 + mark("after", 1);
  console.log(result, trace.join(","));
}
function throws(item: Item): number { trace.push("throws:" + item.run(1)); throw new Error("consumer"); }
trace.length = 0;
let unreached = "kept";
try {
  let $tt_v3: number;
  const $tt_v4 = (throws);
  {
    const $tt_m = State.Ready(2);
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        trace.push("value"); $tt_v3 = $tt_v4({kind: "item", run: x => x + value}); break;
      }
      case "Empty": {
        $tt_v3 = $tt_v4(({kind: "item", run: x => x}));
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  unreached = "lost:" + $tt_v3;
} catch { trace.push("caught"); }
console.log(unreached, trace.join(","));

export {};
