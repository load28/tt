//// [aBlockArmExitLeavesTheRegionFromInsideALoop.tt] ////

variant Pick { Scan(from: number), Zero }
declare const nothing: number;
function choose(p: Pick): number {
  return match (p) {
    Scan(from) => {
      for (const x of [from, from + 1, from + 2]) {
        if (x % 3 === 0) { return x; }
      }
      return -1;
    },
    Zero => 0,
  };
}
console.log(choose(Pick.Scan(2)), choose(Pick.Scan(4)), choose(Pick.Zero));

export {};


//// [aBlockArmExitLeavesTheRegionFromInsideALoop.ts]
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

type Pick =
  | { kind: "Scan"; from: number }
  | { kind: "Zero" };
const Pick = {
  Scan: (from: number): Pick => ({ kind: "Scan", from }),
  Zero: { kind: "Zero" } as const,
};
declare const nothing: number;
function choose(p: Pick): number {
  let $tt_v0: number;
  $tt_y_v0: {
    const $tt_m = p;
    switch ($tt_m.kind) {
      case "Scan": {
        const { from } = $tt_m;
        for (const x of [from, from + 1, from + 2]) {
        if (x % 3 === 0) { $tt_v0 = x; break $tt_y_v0; }
      }
        $tt_v0 = -1;
        break $tt_y_v0;
      }
      case "Zero": {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}
console.log(choose(Pick.Scan(2)), choose(Pick.Scan(4)), choose(Pick.Zero));

export {};
