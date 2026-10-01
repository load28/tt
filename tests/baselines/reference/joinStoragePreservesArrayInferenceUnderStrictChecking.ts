//// [joinStoragePreservesArrayInferenceUnderStrictChecking.tt] ////

variant Shape { Circle(radius: number), Point }
declare const s: Shape;
export const spread = match(s) { Circle(radius) => [radius], Point => [] };
spread.push(1);
export const n: number = spread.length;
export const block = match(s) { Circle(radius) => { return [radius]; }, Point => { return []; } };
block.push(2);
const invalid: string[] = spread;

export {};


//// [joinStoragePreservesArrayInferenceUnderStrictChecking.ts]
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
let $tt_v0: number[];
{
  const $tt_m = s;
  switch ($tt_m.kind) {
    case "Circle": {
      const { radius } = $tt_m;
      const $tt_a0 = { value: [radius] };
      $tt_v0 = $tt_a0.value;
      break;
    }
    case "Point": {
      const $tt_a1 = { value: [] };
      $tt_v0 = $tt_a1.value;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
export const spread = $tt_v0;
spread.push(1);
export const n: number = spread.length;
let $tt_v1: number[];
{
  const $tt_m = s;
  switch ($tt_m.kind) {
    case "Circle": {
      const { radius } = $tt_m;
      const $tt_a2 = { value: [radius] };
      $tt_v1 = $tt_a2.value; break;
    }
    case "Point": {
      const $tt_a3 = { value: [] };
      $tt_v1 = $tt_a3.value; break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
export const block = $tt_v1;
block.push(2);
const invalid: string[] = spread;

export {};
