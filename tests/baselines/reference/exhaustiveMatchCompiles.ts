//// [exhaustiveMatchCompiles.tt] ////

variant Shape { Circle(radius: number), Rect(w: number, h: number), Point }
const f = (s: Shape) => match (s) {
  Circle(radius) => radius,
  Rect(w, h) => w * h,
  Point => 0,
};


//// [exhaustiveMatchCompiles.ts]
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
      case "Rect": {
        const { w, h } = $tt_m;
        $tt_v0 = w * h;
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
