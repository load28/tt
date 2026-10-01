//// [matchArmSingleStatementIfKeepsSyntheticExitConditional.tt] ////
variant V { A(n: number), B }
const f = (v: V): number => match (v) {
A(n) => { if (n === 0) return 100; return n; },
B => -1,
};


//// [matchArmSingleStatementIfKeepsSyntheticExitConditional.ts]
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
type V =
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const f = (v: V): number => {
  let $tt_v0: number;
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        if (n === 0) { $tt_v0 = 100; break; } $tt_v0 = n; break;
      }
      case "B": {
        $tt_v0 = -1;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
};
