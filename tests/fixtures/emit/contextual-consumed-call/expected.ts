type Item = { kind: "item"; run: (x: number) => number };
declare function consume(item: Item): number;
declare const api: { consume(item: Item): number };
declare const maybeConsume: ((item: Item) => number) | undefined;
declare function generic<T>(value: T): T;
type State =
  | { kind: "Ready"; value: number }
  | { kind: "Empty" };
const State = {
  Ready: (value: number): State => ({ kind: "Ready", value }),
  Empty: { kind: "Empty" } as const,
};
declare const state: State;

let $tt_v0;
const $tt_v1 = (consume);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v0 = $tt_v1(({ kind: "item", run: x => x + value }));
      break;
    }
    case "Empty": {
      $tt_v0 = $tt_v1(({ kind: "item", run: x => x }));
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}
const consumed = $tt_v0;

const $tt_v4 = (api);
const $tt_v3 = ($tt_v4.consume).bind($tt_v4);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v3({ kind: "item", run: x => x + value }); break;
    }
    case "Empty": {
      $tt_v3(({ kind: "item", run: x => x }));
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}


let $tt_v7;
const $tt_v6 = (maybeConsume);
if ($tt_v6 != null) {
  {
    const $tt_m = state;
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        $tt_v7 = $tt_v6(({ kind: "item", run: x => x + value }));
        break;
      }
      case "Empty": {
        $tt_v7 = $tt_v6(({ kind: "item", run: x => x }));
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
      }
    }
  }
} else {
  $tt_v7 = undefined;
}

const optional = $tt_v7;

let $tt_v8;
const $tt_v9 = (generic);
const $tt_v10 = $tt_v9<Item>;
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v8 = $tt_v10(({ kind: "item", run: x => x + value }));
      break;
    }
    case "Empty": {
      $tt_v8 = $tt_v10(({ kind: "item", run: x => x }));
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}
const instantiated = $tt_v8;

export { consumed, optional, instantiated };
