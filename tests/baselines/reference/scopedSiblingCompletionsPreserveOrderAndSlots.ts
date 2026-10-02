//// [scopedSiblingCompletionsPreserveOrderAndSlots.tt] ////

variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
function pair(first: Item, second: Item): number { trace.push("call"); return first.run(1) + second.run(2); }
let shared = 10;
for (const state of [State.Ready(4), State.Empty]) {
  trace.length = 0;
  shared = 10;
  const result = pair(
    match (mark("subject1", state)) {
      Ready(value) => (trace.push("arm1"), shared = value, {kind: "item" as const, run: (x: number) => x + shared}),
      Empty => ({kind: "item" as const, run: (x: number) => x}),
    },
    match (mark("subject2", shared > 5)) {
      true => (trace.push("arm2:big"), {kind: "item", run: x => x * shared}),
      false => (trace.push("arm2:small"), {kind: "item", run: x => x + shared}),
    },
  );
  console.log(result, trace.join(","));
}
function throwingSubject(): State { trace.push("boom"); throw new Error("subject"); }
trace.length = 0;
try {
  pair(
    match (State.Ready(1)) { Ready(value) => (trace.push("first"), {kind: "item" as const, run: (x: number) => x + value}), Empty => ({kind: "item" as const, run: (x: number) => x}) },
    match (throwingSubject()) { Ready(value) => ({kind: "item", run: x => x + value}), Empty => ({kind: "item", run: x => x}) },
  );
} catch { trace.push("caught"); }
console.log(trace.join(","));

export {};


//// [scopedSiblingCompletionsPreserveOrderAndSlots.ts]
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
function pair(first: Item, second: Item): number { trace.push("call"); return first.run(1) + second.run(2); }
let shared = 10;
for (const state of [State.Ready(4), State.Empty]) {
  trace.length = 0;
  shared = 10;
  let $tt_v0: Item;
  let $tt_v1: number;
  const $tt_v2 = (pair);
  {
    const $tt_m = mark("subject1", state);
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        $tt_v0 = (trace.push("arm1"), shared = value, {kind: "item" as const, run: (x: number) => x + shared});
        break;
      }
      case "Empty": {
        $tt_v0 = ({kind: "item" as const, run: (x: number) => x});
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  {
    const $tt_m = mark("subject2", shared > 5);
    switch ($tt_m) {
      case true: {
        $tt_v1 = $tt_v2($tt_v0, (trace.push("arm2:big"), {kind: "item", run: x => x * shared}));
        break;
      }
      case false: {
        $tt_v1 = $tt_v2($tt_v0, (trace.push("arm2:small"), {kind: "item", run: x => x + shared}));
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  const result = $tt_v1;
  console.log(result, trace.join(","));
}
function throwingSubject(): State { trace.push("boom"); throw new Error("subject"); }
trace.length = 0;
try {
  let $tt_v3: Item;
  const $tt_v5 = (pair);
  {
    const $tt_m = State.Ready(1);
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        $tt_v3 = (trace.push("first"), {kind: "item" as const, run: (x: number) => x + value});
        break;
      }
      case "Empty": {
        $tt_v3 = ({kind: "item" as const, run: (x: number) => x});
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  {
    const $tt_m = throwingSubject();
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        $tt_v5($tt_v3, ({kind: "item", run: x => x + value}));
        break;
      }
      case "Empty": {
        $tt_v5($tt_v3, ({kind: "item", run: x => x}));
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
