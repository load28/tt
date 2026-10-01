//// [aLoweringIsLaidOutFromTheLineItReplaces.tt] ////
variant E { A(v: number), B }
declare const e: E;
function f(): number {
  if (true) {
    const r = match (e) { A(v) => v, B => 0 };
    return r;
  }
  return 0;
}


//// [aLoweringIsLaidOutFromTheLineItReplaces.ts]
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
type E =
  | { kind: "A"; v: number }
  | { kind: "B" };
const E = {
  A: (v: number): E => ({ kind: "A", v }),
  B: { kind: "B" } as const,
};
declare const e: E;
function f(): number {
  if (true) {
    let $tt_v0: number;
    {
      const $tt_m = e;
      switch ($tt_m.kind) {
        case "A": {
          const { v } = $tt_m;
          $tt_v0 = v;
          break;
        }
        case "B": {
          $tt_v0 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    const r = $tt_v0;
    return r;
  }
  return 0;
}
