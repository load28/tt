//// [literalWrappedCallArgumentsKeepTheirContext.tt] ////

variant State { Ready(value: number), Empty }
declare const state: State;
declare function consume(item: {kind: "item"; run: (x: number) => number}): void;
declare function consumeAll(items: ((x: number) => number)[]): void;
declare function nested(outer: {inner: {run: (x: number) => number}}): void;
consume({kind: "item", run: match (state) {
  Ready(value) => x => x + value,
  Empty => x => x,
}});
consumeAll([match (state) {
  Ready(value) => x => x + value,
  Empty => x => x,
}]);
nested({inner: {run: match (state) {
  Ready(value) => x => x + value,
  Empty => x => x,
}}});
const kept: void = consume({kind: "item", run: match (state) {
  Ready(value) => x => x + value,
  Empty => x => x,
}});

export {};


//// [literalWrappedCallArgumentsKeepTheirContext.ts]
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
declare const state: State;
declare function consume(item: {kind: "item"; run: (x: number) => number}): void;
declare function consumeAll(items: ((x: number) => number)[]): void;
declare function nested(outer: {inner: {run: (x: number) => number}}): void;
const $tt_v2: typeof consume = (consume);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v2({kind: "item", run: x => x + value});
      break;
    }
    case "Empty": {
      $tt_v2({kind: "item", run: x => x});
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}

const $tt_v4: typeof consumeAll = (consumeAll);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v4([x => x + value]);
      break;
    }
    case "Empty": {
      $tt_v4([x => x]);
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}

const $tt_v6: typeof nested = (nested);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v6({inner: {run: x => x + value}});
      break;
    }
    case "Empty": {
      $tt_v6({inner: {run: x => x}});
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}

let $tt_v7: void;
const $tt_v9: typeof consume = (consume);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      $tt_v7 = $tt_v9({kind: "item", run: x => x + value});
      break;
    }
    case "Empty": {
      $tt_v7 = $tt_v9({kind: "item", run: x => x});
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const kept: void = $tt_v7;

export {};
