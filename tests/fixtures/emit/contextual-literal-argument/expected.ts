type Item = { kind: string; run: (x: number) => number };
declare function consume(item: Item): number;
declare function consumeAll(runs: ((x: number) => number)[]): void;
declare function nested(outer: { inner: Item }): void;
declare function effect(): string;
type State =
  | { kind: "Ready"; value: number }
  | { kind: "Empty" };
const State = {
  Ready: (value: number): State => ({ kind: "Ready", value }),
  Empty: { kind: "Empty" } as const,
};
declare const state: State;

const $tt_v2 = (consume);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v2({ kind: "item", run: x => x + value });
      break;
    }
    case "Empty": {
      $tt_v2({ kind: "item", run: x => x });
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}


let $tt_v3;
const $tt_v5 = (consume);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v3 = $tt_v5({ kind: "item", run: x => x + value });
      break;
    }
    case "Empty": {
      $tt_v3 = $tt_v5({ kind: "item", run: x => x });
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}
const kept = $tt_v3;

const $tt_v7 = (consumeAll);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v7([x => x + value]);
      break;
    }
    case "Empty": {
      $tt_v7([x => x]);
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}


const $tt_v10 = (nested);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v10({ inner: { kind: "item", run: x => x + value } });
      break;
    }
    case "Empty": {
      $tt_v10({ inner: { kind: "item", run: x => x } });
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}


let $tt_v11;
const $tt_v13 = (consume);
const $tt_v12 = (effect());
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v11 = (x: number) => x + value;
      break;
    }
    case "Empty": {
      $tt_v11 = (x: number) => x;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}
$tt_v13({ kind: $tt_v12, run: $tt_v11 });

let $tt_v14;
const $tt_v16 = (consume);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v14 = x => x + value;
      break;
    }
    case "Empty": {
      $tt_v14 = x => x;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}
$tt_v16({ kind: "item", run: $tt_v14 as (x: number) => number });

export { kept };
