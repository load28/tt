//// [shape.tt] ////
export variant Shape { Circle(r: number), Point }

//// [a.tt] ////
import { Shape } from ".\\shape.tt";
declare const s: Shape;
export const n = match (s) { Circle(r) => r, Point => 0 };


//// [a.ts]
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
import { Shape } from ".\\shape.js";
declare const s: Shape;
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
export const n = $tt_v0;

//// [shape.ts]
export type Shape =
  | { kind: "Circle"; r: number }
  | { kind: "Point" };
export const Shape = {
  Circle: (r: number): Shape => ({ kind: "Circle", r }),
  Point: { kind: "Point" } as const,
};
\ No newline at end of file
