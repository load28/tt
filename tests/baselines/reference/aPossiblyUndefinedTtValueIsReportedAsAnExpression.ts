//// [aPossiblyUndefinedTtValueIsReportedAsAnExpression.tt] ////
variant V { A, B }
export function f(v: V) {
  return "n" in match (v) { A => v, B => {} };
}


//// [aPossiblyUndefinedTtValueIsReportedAsAnExpression.ts]
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
type V =
  | { kind: "A" }
  | { kind: "B" };
const V = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
export function f(v: V) {
  let $tt_v0: (V) | (undefined);
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v0 = v;
        break;
      }
      case "B": {
        
          $tt_v0 = undefined;
          break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return "n" in $tt_v0;
}
