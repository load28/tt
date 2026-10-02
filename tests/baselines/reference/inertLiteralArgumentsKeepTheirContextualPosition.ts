//// [inertLiteralArgumentsKeepTheirContextualPosition.tt] ////

variant State { Ready(value: number), Empty }
type Item = {kind: "item"; run: (x: number) => number};
declare const state: State;
declare function widget(props: {first: Item; second: Item}): number;
declare function tagged(first: Item, second: number[], third: Item): number;
const items: Item[] = [{kind: "item", run: x => x}, made];
const nested: {first: Item; second: Item} = {
  first: {kind: "item", run: x => x},
  second: made,
};
declare const made: Item;
const wrapped = widget({
  first: {kind: "item", run: x => x},
  second: match (state) {
    Ready(value) => (made),
    Empty => (made),
  },
});
const counted = tagged({kind: "item", run: x => x}, [1, 2], made);
export {items, nested, wrapped, counted};

export {};


//// [inertLiteralArgumentsKeepTheirContextualPosition.ts]
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
declare const state: State;
declare function widget(props: {first: Item; second: Item}): number;
declare function tagged(first: Item, second: number[], third: Item): number;
const items: Item[] = [{kind: "item", run: x => x}, made];
const nested: {first: Item; second: Item} = {
  first: {kind: "item", run: x => x},
  second: made,
};
declare const made: Item;
let $tt_v0: number;
const $tt_v2 = (widget);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v0 = $tt_v2({
  first: {kind: "item", run: x => x},
  second: (made),
});
      break;
    }
    case "Empty": {
      $tt_v0 = $tt_v2({
  first: {kind: "item", run: x => x},
  second: (made),
});
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const wrapped = $tt_v0;
const counted = tagged({kind: "item", run: x => x}, [1, 2], made);
export {items, nested, wrapped, counted};

export {};
