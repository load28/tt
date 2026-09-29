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
/** A drawable shape. */
export type Shape =
  /** A circle around the origin. */
  | { kind: "Circle"; radius: number } // the common case
  // Rectangles are axis-aligned.
  /**
   * A rectangle.
   *
   * Its corner is the origin.
   */
  | {
      kind: "Rect";
      /** Width in pixels. */
      width: number;
      height: number; // in pixels
    }
  /** No area at all. */
  | { kind: "Point" }; // the last case
export const Shape = {
  /** A circle around the origin. */
  Circle: (radius: number): Shape => ({ kind: "Circle", radius }),
  /**
   * A rectangle.
   *
   * Its corner is the origin.
   */
  Rect: (
    /** Width in pixels. */
    width: number,
    height: number,
  ): Shape => ({ kind: "Rect", width, height }),
  /** No area at all. */
  Point: { kind: "Point" } as const,
};

export declare type Remote =
  /** A value the server sent. */
  | {
      kind: "Loaded";
      /** The payload. */
      value: string;
    }
  | { kind: "Pending" };
export declare const Remote: {
  /** A value the server sent. */
  readonly Loaded: (
    /** The payload. */
    value: string,
  ) => Remote;
  readonly Pending: { readonly kind: "Pending" };
};

declare const shape: Shape;

let $tt_v0: number;
{
  const $tt_m = shape;
  switch ($tt_m.kind) {
    case "Circle": {
      const { radius } = $tt_m;
      const $tt_a0 = { value: Math.PI * radius ** 2 }; $tt_v0 = $tt_a0.value;
      break;
    }
    case "Rect": {
      const { width, height } = $tt_m;
      const $tt_a1 = { value: width * height }; $tt_v0 = $tt_a1.value;
      break;
    }
    case "Point": {
      const $tt_a2 = { value: 0 }; $tt_v0 = $tt_a2.value;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
export const area = $tt_v0;
