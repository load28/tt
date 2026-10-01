//// [tupleMatchWildcardElementAndOrPatternCoverTheProduct.tt] ////

variant Dir { North(), South, East, West }
variant Speed { Fast(), Slow }
const step = match (d, s) {
  (North | South, _) => 1,
  (East, Fast | Slow) => 2,
  (West, _) => 3,
};


//// [tupleMatchWildcardElementAndOrPatternCoverTheProduct.ts]
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

type Dir =
  | { kind: "North" }
  | { kind: "South" }
  | { kind: "East" }
  | { kind: "West" };
const Dir = {
  North: (): Dir => ({ kind: "North" }),
  South: { kind: "South" } as const,
  East: { kind: "East" } as const,
  West: { kind: "West" } as const,
};
type Speed =
  | { kind: "Fast" }
  | { kind: "Slow" };
const Speed = {
  Fast: (): Speed => ({ kind: "Fast" }),
  Slow: { kind: "Slow" } as const,
};
let $tt_v0$step: number;
{
  const $tt_m0 = d;
  const $tt_m1 = s;
  do {
    if (($tt_m0.kind === "North" || $tt_m0.kind === "South")) {
      $tt_v0$step = 1;
      break;
    }
    if ($tt_m0.kind === "East" && ($tt_m1.kind === "Fast" || $tt_m1.kind === "Slow")) {
      $tt_v0$step = 2;
      break;
    }
    if ($tt_m0.kind === "West") {
      $tt_v0$step = 3;
      break;
    }
    throw new Error("tt match: unexpected case " + "[" + $tt_show($tt_m0) + "," + $tt_show($tt_m1) + "]");
  } while (false);
}
const step = $tt_v0$step;
