//// [elidedAndCapturedInputsPreserveOrderIdentityAndBindings.tt] ////

variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: () => number};
const trace: string[] = [];
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
let shared = 1;
for (const state of [State.Ready(9), State.Empty]) {
  trace.length = 0;
  shared = 1;
  const items: Item[] = [{kind: "item", run: () => shared}, match (mark("subject", state)) {
    Ready(value) => (trace.push("arm"), shared = value, {kind: "item" as const, run: () => shared + 1}),
    Empty => ({kind: "item" as const, run: () => 0}),
  }];
  console.log(`${items[0].run()}:${items[1].run()}:${items[0] === items[0]}`, trace.join(","));
}
function pair(first: Item, second: Item): string { trace.push("call"); return `${first.run()}:${second.run()}`; }
trace.length = 0;
shared = 1;
const completed = pair({kind: "item" as const, run: () => shared}, match (mark("subject", State.Ready(4))) {
  Ready(value) => (trace.push("arm"), shared = value, {kind: "item", run: () => shared + 1}),
  Empty => ({kind: "item", run: () => 0}),
});
console.log(completed, trace.join(","));
trace.length = 0;
function effectful(): Item { trace.push("effectful"); return {kind: "item", run: () => 0}; }
pair(effectful(), match (mark<State>("subject", State.Empty)) {
  Ready(value) => ({kind: "item", run: () => value}),
  Empty => ({kind: "item", run: () => 0}),
});
console.log(trace.join(","));

export {};


//// [elidedAndCapturedInputsPreserveOrderIdentityAndBindings.ts]
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
type Item = {kind: "item"; run: () => number};
const trace: string[] = [];
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
let shared = 1;
for (const state of [State.Ready(9), State.Empty]) {
  trace.length = 0;
  shared = 1;
  let $tt_v0: Item;
  {
    const $tt_m = mark("subject", state);
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        $tt_v0 = (trace.push("arm"), shared = value, {kind: "item" as const, run: () => shared + 1});
        break;
      }
      case "Empty": {
        $tt_v0 = ({kind: "item" as const, run: () => 0});
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const items: Item[] = [{kind: "item", run: () => shared}, $tt_v0];
  console.log(`${items[0].run()}:${items[1].run()}:${items[0] === items[0]}`, trace.join(","));
}
function pair(first: Item, second: Item): string { trace.push("call"); return `${first.run()}:${second.run()}`; }
trace.length = 0;
shared = 1;
let $tt_v1: string;
const $tt_v2 = (pair);
const $tt_v3: Item = ({kind: "item" as const, run: () => shared});
{
  const $tt_m = mark("subject", State.Ready(4));
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v1 = $tt_v2($tt_v3, (trace.push("arm"), shared = value, {kind: "item", run: () => shared + 1}));
      break;
    }
    case "Empty": {
      $tt_v1 = $tt_v2($tt_v3, ({kind: "item", run: () => 0}));
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const completed = $tt_v1;
console.log(completed, trace.join(","));
trace.length = 0;
function effectful(): Item { trace.push("effectful"); return {kind: "item", run: () => 0}; }
const $tt_v5 = (pair);
const $tt_v6: Item = (effectful());
{
  const $tt_m = mark<State>("subject", State.Empty);
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v5($tt_v6, ({kind: "item", run: () => value}));
      break;
    }
    case "Empty": {
      $tt_v5($tt_v6, ({kind: "item", run: () => 0}));
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}

console.log(trace.join(","));

export {};
