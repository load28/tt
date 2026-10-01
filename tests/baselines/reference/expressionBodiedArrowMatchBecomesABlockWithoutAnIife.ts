//// [expressionBodiedArrowMatchBecomesABlockWithoutAnIife.tt] ////
variant E { A, B }
const f = (e: E) => match (e) { A => 1, B => 2 };


//// [expressionBodiedArrowMatchBecomesABlockWithoutAnIife.ts]
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
  | { kind: "A" }
  | { kind: "B" };
const E = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const f = (e: E) => {
  let $tt_v0: number;
  {
    const $tt_m = e;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v0 = 1;
        break;
      }
      case "B": {
        $tt_v0 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
};
