//// [aHoistedComputedSymbolKeyKeepsItsUniqueSymbolType.tt] ////
variant O { S(v: number), N }
export function f(o: O) {
  const it = { *[Symbol.iterator]() { yield 1; }, v: match (o) { S(v) => v, N => 0 } };
  return [...it, it.v];
}


//// [aHoistedComputedSymbolKeyKeepsItsUniqueSymbolType.ts]
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
type O =
  | { kind: "S"; v: number }
  | { kind: "N" };
const O = {
  S: (v: number): O => ({ kind: "S", v }),
  N: { kind: "N" } as const,
};
export function f(o: O) {
  let $tt_v0: number;
  const $tt_v1: typeof Symbol.iterator = (Symbol.iterator);
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "S": {
        const { v } = $tt_m;
        $tt_v0 = v;
        break;
      }
      case "N": {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const it = { *[$tt_v1]() { yield 1; }, v: $tt_v0 };
  return [...it, it.v];
}
