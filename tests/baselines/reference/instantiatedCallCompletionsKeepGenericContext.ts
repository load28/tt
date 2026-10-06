//// [instantiatedCallCompletionsKeepGenericContext.tt] ////

variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
const trace: string[] = [];
function generic<T>(value: T): T { trace.push("call"); return value; }
const item = generic<Item>(match (State.Ready(3)) {
  Ready(value) => { trace.push("arm"); return {kind: "item", run: x => x + value}; },
  Empty => ({kind: "item", run: x => x}),
});
console.log(item.run(1), trace.join(","));

export {};


//// [instantiatedCallCompletionsKeepGenericContext.ts]
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
function generic<T>(value: T): T { trace.push("call"); return value; }
let $tt_v0: Item;
const $tt_v1: typeof generic = (generic);
const $tt_v2 = $tt_v1<Item>;
{
  const $tt_m = State.Ready(3);
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      trace.push("arm"); $tt_v0 = $tt_v2({kind: "item", run: x => x + value}); break;
    }
    case "Empty": {
      $tt_v0 = $tt_v2(({kind: "item", run: x => x}));
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const item = $tt_v0;
console.log(item.run(1), trace.join(","));

export {};
