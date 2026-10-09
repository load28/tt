//// [scopedGenericCallInstantiatesBeforeArmTypeShadowing.tt] ////

variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function original<T extends Item>(item: T) { trace.push("original:" + item.run(3)); }
let consume = original;
consume<Item>(match (State.Ready(4)) {
  Ready(value) => {
    type Item = never;
    consume = () => { trace.push("replaced"); };
    trace.push("arm");
    return {kind: "item", run: x => x + value};
  },
  Empty => ({kind: "item", run: x => x}),
});
console.log(trace.join(","));

export {};


//// [scopedGenericCallInstantiatesBeforeArmTypeShadowing.ts]
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
function original<T extends Item>(item: T) { trace.push("original:" + item.run(3)); }
let consume = original;
const $tt_v1: typeof consume = (consume);
const $tt_v2 = $tt_v1<Item>;
{
  const $tt_m = State.Ready(4);
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      type Item = never;
    consume = () => { trace.push("replaced"); };
    trace.push("arm");
      $tt_v2({kind: "item", run: x => x + value});
      break;
    }
    case "Empty": {
      $tt_v2(({kind: "item", run: x => x}));
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}

console.log(trace.join(","));

export {};
