//// [controlFlowArmCompletionsCallAtTheAuthoredExit.tt] ////

variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
function consume(item: Item): number { trace.push("call"); return item.run(2); }
for (const state of [State.Ready(1), State.Ready(5), State.Empty]) {
  trace.length = 0;
  const result = consume(match (mark("subject", state)) {
    Ready(value) => {
      for (const step of [1, 2]) {
        if (step === value) { trace.push("loop"); return {kind: "item", run: x => x + value}; }
      }
      trace.push("fallthrough");
      return {kind: "item", run: x => x - value};
    },
    Empty => ({kind: "item", run: x => x}),
  }) + mark("after", 10);
  console.log(result, trace.join(","));
}
function throwing(item: Item): number { trace.push("throwing"); throw new Error("consumer"); }
trace.length = 0;
try {
  throwing(match (State.Ready(1)) {
    Ready(value) => { if (value > 0) return {kind: "item", run: x => x}; trace.push("unreached"); return {kind: "item", run: x => x}; },
    Empty => ({kind: "item", run: x => x}),
  });
} catch { trace.push("caught"); }
console.log(trace.join(","));

export {};


//// [controlFlowArmCompletionsCallAtTheAuthoredExit.ts]
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
function consume(item: Item): number { trace.push("call"); return item.run(2); }
for (const state of [State.Ready(1), State.Ready(5), State.Empty]) {
  trace.length = 0;
  let $tt_v0: number;
  const $tt_v1: typeof consume = (consume);
  $tt_y_v0: {
    const $tt_m = mark("subject", state);
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        for (const step of [1, 2]) {
        if (step === value) { trace.push("loop"); $tt_v0 = $tt_v1({kind: "item", run: x => x + value}); break $tt_y_v0; }
      }
      trace.push("fallthrough");
        $tt_v0 = $tt_v1({kind: "item", run: x => x - value});
        break $tt_y_v0;
      }
      case "Empty": {
        $tt_v0 = $tt_v1(({kind: "item", run: x => x}));
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const result = $tt_v0 + mark("after", 10);
  console.log(result, trace.join(","));
}
function throwing(item: Item): number { trace.push("throwing"); throw new Error("consumer"); }
trace.length = 0;
try {
  const $tt_v3: typeof throwing = (throwing);
  {
    const $tt_m = State.Ready(1);
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        if (value > 0) { $tt_v3({kind: "item", run: x => x}); break; } trace.push("unreached"); $tt_v3({kind: "item", run: x => x}); break;
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
