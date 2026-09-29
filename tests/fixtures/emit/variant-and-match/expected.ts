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
export type Shape =
  | { kind: "Circle"; radius: number }
  | { kind: "Rect"; width: number; height: number }
  | { kind: "Point" };
export const Shape = {
  Circle: (radius: number): Shape => ({ kind: "Circle", radius }),
  Rect: (width: number, height: number): Shape => ({ kind: "Rect", width, height }),
  Point: { kind: "Point" } as const,
};

declare const shape: Shape;

let $tt_v0: number;
{
  const $tt_m = shape;
  switch ($tt_m.kind) {
    case "Circle": {
      const { radius } = $tt_m;
      const $tt_a0 = { value: Math.PI * radius ** 2 };
      $tt_v0 = $tt_a0.value;
      break;
    }
    case "Rect": {
      const { width: w, height } = $tt_m;
      const $tt_a1 = { value: w * height };
      $tt_v0 = $tt_a1.value;
      break;
    }
    case "Point": {
      const $tt_a2 = { value: 0 };
      $tt_v0 = $tt_a2.value;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
export const area = $tt_v0;
