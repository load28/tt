//// [aMaterializedPipelineAccumulatorUsesADirectCall.tt] ////
variant E { A(value: number), B }
const value = match (E.A(1)) { A(value) => value, B => 0 } |> String;


//// [aMaterializedPipelineAccumulatorUsesADirectCall.ts]
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
  | { kind: "A"; value: number }
  | { kind: "B" };
const E = {
  A: (value: number): E => ({ kind: "A", value }),
  B: { kind: "B" } as const,
};
let $tt_v0$value: string;
do {
  let $tt_v2: number;
  {
    const $tt_m = E.A(1);
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v2 = value;
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
  $tt_v0$value = String($tt_v2);
  break;
} while (false);
const value = $tt_v0$value;
