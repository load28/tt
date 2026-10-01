//// [wildcardSatisfiesExhaustiveness.tt] ////

variant Shape { Circle(radius: number), Rect(w: number, h: number), Point }
const f = (s: Shape) => match (s) {
  Circle(radius) => radius,
  _ => 0,
};


//// [wildcardSatisfiesExhaustiveness.ts]

type Shape =
  | { kind: "Circle"; radius: number }
  | { kind: "Rect"; w: number; h: number }
  | { kind: "Point" };
const Shape = {
  Circle: (radius: number): Shape => ({ kind: "Circle", radius }),
  Rect: (w: number, h: number): Shape => ({ kind: "Rect", w, h }),
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
      default: {
        $tt_v0 = 0;
        break;
      }
    }
  }
  return $tt_v0;
};
