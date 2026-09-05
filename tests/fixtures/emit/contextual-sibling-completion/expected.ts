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

const $tt_v4 = (trio);
const $tt_v5 = (made);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v4($tt_v5, 7, { kind: "item", run: x => x + value }); break;
    }
    case "Empty": {
      $tt_v4($tt_v5, 7, ({ kind: "item", run: x => x }));
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}


let $tt_v6;
const $tt_v7 = (pair);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v6 = ({ kind: "item" as const, run: (x: number) => x + value });
      break;
    }
    case "Empty": {
      $tt_v6 = ({ kind: "item" as const, run: (x: number) => x });
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}
const leading = $tt_v7($tt_v6, make());

export { paired, leading };
