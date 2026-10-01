//// [typecheckExhaustiveMatchPasses.tt] ////

variant Shape { Circle(radius: number), Point }
const f = (s: Shape) => match (s) {
  Circle(radius) => radius,
  Point => 0,
};

export {};


//// [typecheckExhaustiveMatchPasses.ts]
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

type Shape =
  | { kind: "Circle"; radius: number }
  | { kind: "Point" };
const Shape = {
  Circle: (radius: number): Shape => ({ kind: "Circle", radius }),
  Point: { kind: "Point" } as const,
};
const f = (s: Shape) => {
  let $tt_v0: number;
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "Circle": {
        const { radius } = $tt_m;
        $tt_v0 = radius;
        break;
      }
      case "Point": {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
};

export {};
