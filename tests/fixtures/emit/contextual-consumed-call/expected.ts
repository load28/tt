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

let $tt_v0: number;
const $tt_v1: typeof consume = (consume);
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
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const consumed = $tt_v0;

{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      api.consume({ kind: "item", run: x => x + value }); break;
    }
    case "Empty": {
      api.consume(({ kind: "item", run: x => x }));
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}


let $tt_v6: (number) | (undefined);
const $tt_v5: typeof maybeConsume = (maybeConsume);
if ($tt_v5 != null) {
  {
    const $tt_m = state;
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        $tt_v6 = $tt_v5(({ kind: "item", run: x => x + value }));
        break;
      }
      case "Empty": {
        $tt_v6 = $tt_v5(({ kind: "item", run: x => x }));
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
} else {
  $tt_v6 = undefined;
}

const optional = $tt_v6;

let $tt_v7: Item;
const $tt_v8: typeof generic = (generic);
const $tt_v9 = $tt_v8<Item>;
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v7 = $tt_v9(({ kind: "item", run: x => x + value }));
      break;
    }
    case "Empty": {
      $tt_v7 = $tt_v9(({ kind: "item", run: x => x }));
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const instantiated = $tt_v7;

export { consumed, optional, instantiated };
