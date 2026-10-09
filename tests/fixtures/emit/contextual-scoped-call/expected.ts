var $tt_show: (value: unknown) => string = function (value) {
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
};
type State =
  | { kind: "Ready"; value: number }
  | { kind: "Empty" };
const State = {
  Ready: (value: number): State => ({ kind: "Ready", value }),
  Empty: { kind: "Empty" } as const,
};
declare const state: State;
declare function consume(item: {run: (x: number) => number}): void;
{
  const $tt_v1: typeof consume = (consume);
  {
    const $tt_m = state;
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        const amount = value + 1; $tt_v1({run: x => x + amount}); break;
      }
      case "Empty": {
        $tt_v1(({run: x => x}));
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  
}
declare function generic<T>(item: T): void;
{
  const $tt_v3: typeof generic = (generic);
  const $tt_v4 = $tt_v3<{run: (x: number) => number}>;
  {
    const $tt_m = state;
    switch ($tt_m.kind) {
      case "Ready": {
        const { value } = $tt_m;
        $tt_v4(({run: x => x + value}));
        break;
      }
      case "Empty": {
        $tt_v4(({run: x => x}));
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  
}
