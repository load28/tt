type Item = { kind: "item"; run: (x: number) => number };
declare function pair(first: Item, second: Item): number;
declare function trio(first: Item, between: number, third: Item): number;
declare function make(): Item;
declare const made: Item;
type State =
  | { kind: "Ready"; value: number }
  | { kind: "Empty" };
const State = {
  Ready: (value: number): State => ({ kind: "Ready", value }),
  Empty: { kind: "Empty" } as const,
};
declare const state: State;

let $tt_v0;
const $tt_v1 = (pair);
const $tt_v2 = (make());
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v0 = $tt_v1($tt_v2, ({ kind: "item", run: x => x + value }));
      break;
    }
    case "Empty": {
      $tt_v0 = $tt_v1($tt_v2, ({ kind: "item", run: x => x }));
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}
const paired = $tt_v0;

let $tt_v3;
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v3 = ({ kind: "item" as const, run: (x: number) => x + value });
      break;
    }
    case "Empty": {
      $tt_v3 = ({ kind: "item" as const, run: (x: number) => x });
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}
const inert: Item[] = [{ kind: "item", run: x => x }, $tt_v3];

const $tt_v5 = (trio);
const $tt_v6 = (made);
const $tt_v7 = (7);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v5($tt_v6, $tt_v7, { kind: "item", run: x => x + value }); break;
    }
    case "Empty": {
      $tt_v5($tt_v6, $tt_v7, ({ kind: "item", run: x => x }));
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}


let $tt_v8;
const $tt_v9 = (pair);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v8 = ({ kind: "item" as const, run: (x: number) => x + value });
      break;
    }
    case "Empty": {
      $tt_v8 = ({ kind: "item" as const, run: (x: number) => x });
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}
const leading = $tt_v9($tt_v8, make());

export { paired, inert, leading };
