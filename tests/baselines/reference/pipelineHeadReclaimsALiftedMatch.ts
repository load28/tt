//// [pipelineHeadReclaimsALiftedMatch.tt] ////
variant E { A(v: number), B }
const a = match (e) { A(v) => v, B => 0, } |> double;


//// [pipelineHeadReclaimsALiftedMatch.ts]
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
let $tt_v0$a;
do {
  let $tt_v2;
  {
    const $tt_m = e;
    switch ($tt_m.kind) {
      case "A": {
        const { v } = $tt_m;
        $tt_v2 = v;
        break;
      }
      case "B": {
        $tt_v2 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v0$a = double($tt_v2);
  break;
} while (false);
const a = $tt_v0$a;
