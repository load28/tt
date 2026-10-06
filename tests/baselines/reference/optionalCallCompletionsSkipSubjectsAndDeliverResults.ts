//// [optionalCallCompletionsSkipSubjectsAndDeliverResults.tt] ////

variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
const present: ((item: Item) => number) | undefined = item => { trace.push("call"); return item.run(2); };
const absent: ((item: Item) => number) | undefined = undefined;
for (const consume of [present, absent]) {
  trace.length = 0;
  const result = consume?.(match (mark("subject", State.Ready(5))) {
    Ready(value) => (trace.push("arm"), {kind: "item", run: x => x + value}),
    Empty => ({kind: "item", run: x => x}),
  });
  console.log(result, trace.join(",") || "silent");
}
const holder = {
  base: 100,
  maybe(item: Item): number { trace.push("method"); return this.base + item.run(1); },
};
trace.length = 0;
const viaReceiver = holder.maybe?.(match (mark("subject", State.Ready(7))) {
  Ready(value) => ({kind: "item", run: x => x * value}),
  Empty => ({kind: "item", run: x => x}),
});
console.log(viaReceiver, trace.join(","));

export {};


//// [optionalCallCompletionsSkipSubjectsAndDeliverResults.ts]
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
const present: ((item: Item) => number) | undefined = item => { trace.push("call"); return item.run(2); };
const absent: ((item: Item) => number) | undefined = undefined;
for (const consume of [present, absent]) {
  trace.length = 0;
  let $tt_v2: (number) | (undefined);
  const $tt_v1: typeof consume = (consume);
  if ($tt_v1 != null) {
    {
      const $tt_m = mark("subject", State.Ready(5));
      switch ($tt_m.kind) {
        case "Ready": {
          const { value } = $tt_m;
          $tt_v2 = $tt_v1((trace.push("arm"), {kind: "item", run: x => x + value}));
          break;
        }
        case "Empty": {
          $tt_v2 = $tt_v1(({kind: "item", run: x => x}));
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
  } else {
    $tt_v2 = undefined;
  }
  
  const result = $tt_v2;
  console.log(result, trace.join(",") || "silent");
}
const holder = {
  base: 100,
  maybe(item: Item): number { trace.push("method"); return this.base + item.run(1); },
};
trace.length = 0;
let $tt_v5: (number) | (undefined);
const $tt_v4 = (holder.maybe);
if ($tt_v4 != null) {
  {
    const $tt_m = mark("subject", State.Ready(7));
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        $tt_v5 = $tt_v4.call(holder, ({kind: "item", run: x => x * value}));
        break;
      }
      case "Empty": {
        $tt_v5 = $tt_v4.call(holder, ({kind: "item", run: x => x}));
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
} else {
  $tt_v5 = undefined;
}

const viaReceiver = $tt_v5;
console.log(viaReceiver, trace.join(","));

export {};
