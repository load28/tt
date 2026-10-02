//// [aSlotCarriesTheDeclaredTypeOnlyWhenItIsTheReturnedValueS2.tt] ////
variant Shape { Circle(radius: number), Point }
declare const s: Shape;
export async function asy(): Promise<number> { return match (s) { Circle(radius) => 1, Point => 0 }; }


//// [aSlotCarriesTheDeclaredTypeOnlyWhenItIsTheReturnedValueS2.ts]
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
export async function asy(): Promise<number> { let $tt_v0: Awaited< Promise<number>>;
{
  const $tt_m = s;
  switch ($tt_m.kind) {
    case "Circle": {
      const { radius } = $tt_m;
      $tt_v0 = 1;
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
return $tt_v0; }
