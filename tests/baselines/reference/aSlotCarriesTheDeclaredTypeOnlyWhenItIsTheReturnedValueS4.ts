//// [aSlotCarriesTheDeclaredTypeOnlyWhenItIsTheReturnedValueS4.tt] ////
variant Shape { Circle(radius: number), Point }
declare const s: Shape;
export function pred(x: unknown): x is number { return match (s) { Circle(radius) => typeof x === "number", Point => false }; }


//// [aSlotCarriesTheDeclaredTypeOnlyWhenItIsTheReturnedValueS4.ts]
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
declare const s: Shape;
export function pred(x: unknown): x is number { let $tt_v0: boolean;
{
  const $tt_m = s;
  switch ($tt_m.kind) {
    case "Circle": {
      const { radius } = $tt_m;
      $tt_v0 = typeof x === "number";
      break;
    }
    case "Point": {
      $tt_v0 = false;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
return $tt_v0; }
