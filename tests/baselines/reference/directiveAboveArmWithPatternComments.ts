//// [directiveAboveArmWithPatternComments.tt] ////
// A `// @ts-ignore` above an arm governs that arm, as TypeScript applies a
// directive to the next line that is neither blank nor a `//` comment, also
// when the arm's pattern holds comments: they are written after the arm, so
// the directive still stands right above it. The error in the next arm is
// still reported, at its own position.
export variant Shape { Circle(r: number), Square(s: number), Point }

export function size(shape: Shape): number {
  return match (shape) {
    // @ts-ignore
    Circle(/* the radius */ r) /* read */ => r.radius,
    Square(/* the side */ s) => s.side,
    Point => 0,
  };
}


//// [directiveAboveArmWithPatternComments.ts]
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
// A `// @ts-ignore` above an arm governs that arm, as TypeScript applies a
// directive to the next line that is neither blank nor a `//` comment, also
// when the arm's pattern holds comments: they are written after the arm, so
// the directive still stands right above it. The error in the next arm is
// still reported, at its own position.
export type Shape =
  | { kind: "Circle"; r: number }
  | { kind: "Square"; s: number }
  | { kind: "Point" };
export const Shape = {
  Circle: (r: number): Shape => ({ kind: "Circle", r }),
  Square: (s: number): Shape => ({ kind: "Square", s }),
  Point: { kind: "Point" } as const,
};

export function size(shape: Shape): number {
  let $tt_v0: number;
  {
    const $tt_m = shape;
    switch ($tt_m.kind) {
      // @ts-ignore
      case "Circle": { const { r } = $tt_m; $tt_v0 = r.radius; break; } /* the radius */ /* read */
      case "Square": {
        const { s } = $tt_m;
        $tt_v0 = s.side;
        break;
      }
      /* the side */
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
}
