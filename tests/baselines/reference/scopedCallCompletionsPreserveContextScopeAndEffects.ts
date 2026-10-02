//// [scopedCallCompletionsPreserveContextScopeAndEffects.tt] ////

variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function consume(item: Item) { trace.push("call:" + item.run(3)); }
for (const state of [State.Ready(4), State.Empty]) {
  trace.length = 0;
  consume(match (state) {
    Ready(value) => {
      const consume = value + 1;
      const local = () => consume;
      trace.push("arm");
      return {kind: "item", run: x => x + local()};
    },
    Empty => ({kind: "item", run: x => x - 1}),
  });
  trace.push("after");
  console.log(trace.join(","));
}
function throws(item: Item): void { trace.push("throws:" + item.run(2)); throw new Error("consumer"); }
trace.length = 0;
try {
  throws(match (State.Ready(5)) {
    Ready(value) => { trace.push("value"); return {kind: "item", run: x => x + value}; },
    Empty => ({kind: "item", run: x => x}),
  });
} catch { trace.push("caught"); }
console.log(trace.join(","));

export {};


//// [scopedCallCompletionsPreserveContextScopeAndEffects.ts]
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
function consume(item: Item) { trace.push("call:" + item.run(3)); }
for (const state of [State.Ready(4), State.Empty]) {
  trace.length = 0;
  const $tt_v1 = (consume);
  {
    const $tt_m = state;
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        const consume = value + 1;
      const local = () => consume;
      trace.push("arm");
        $tt_v1({kind: "item", run: x => x + local()});
        break;
      }
      case "Empty": {
        $tt_v1(({kind: "item", run: x => x - 1}));
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  
  trace.push("after");
  console.log(trace.join(","));
}
function throws(item: Item): void { trace.push("throws:" + item.run(2)); throw new Error("consumer"); }
trace.length = 0;
try {
  const $tt_v3 = (throws);
  {
    const $tt_m = State.Ready(5);
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        trace.push("value"); $tt_v3({kind: "item", run: x => x + value}); break;
      }
      case "Empty": {
        $tt_v3(({kind: "item", run: x => x}));
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  
} catch { trace.push("caught"); }
console.log(trace.join(","));

export {};
