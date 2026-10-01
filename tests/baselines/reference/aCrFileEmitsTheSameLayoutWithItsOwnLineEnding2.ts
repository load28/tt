//// [aCrFileEmitsTheSameLayoutWithItsOwnLineEnding2.tt] ////
variant Shape { Circle(r: number), Point }
declare const s: Shape;
function f() {
  const a = match (s) {
    Circle(r) => r,
    Point => 0,
  };
  return a |> String;
}


//// [aCrFileEmitsTheSameLayoutWithItsOwnLineEnding2.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
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
  | { kind: "Circle"; r: number }
  | { kind: "Point" };
const Shape = {
  Circle: (r: number): Shape => ({ kind: "Circle", r }),
  Point: { kind: "Point" } as const,
};
declare const s: Shape;
function f() {
  let $tt_v0: number;
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "Circle": {
        const { r } = $tt_m;
        $tt_v0 = r;
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
  const a = $tt_v0;
  return $tt_ap(a, String);
}
