//// [orPatternCountsForExhaustiveness1.tt] ////

variant Dir { North(), South, East, West }
const f = (d: Dir) => match (d) {
  North | South => 1,
  East | West => 2,
};


//// [orPatternCountsForExhaustiveness1.ts]
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
const f = (d: Dir) => {
  let $tt_v0: number;
  {
    const $tt_m = d;
    switch ($tt_m.kind) {
      case "North": case "South": {
        $tt_v0 = 1;
        break;
      }
      case "East": case "West": {
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
