//// [pipelineInsideMatchScrutineeArmAndTemplate.tt] ////
variant E { A(v: number), B }
const r = match (x |> norm) {
  A(v) => v |> double,
  B => 0,
};
const t = `n=${x |> f}`;


//// [pipelineInsideMatchScrutineeArmAndTemplate.ts]
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
let $tt_v0$r;
{
  const $tt_m = (($tt_v, $tt_f) => $tt_f($tt_v))(x, norm);
  switch ($tt_m.kind) {
    case "A": {
      const { v } = $tt_m;
      $tt_v0$r = (($tt_v, $tt_f) => $tt_f($tt_v))(v, double);
      break;
    }
    case "B": {
      $tt_v0$r = 0;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const r = $tt_v0$r;
const t = `n=${(($tt_v, $tt_f) => $tt_f($tt_v))(x, f)}`;
