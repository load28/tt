//// [directiveBetweenMatchArmsGovernsOneArm.tt] ////
// A `// @ts-ignore` between match arms governs the one arm on the line
// after it, as TypeScript applies a directive to the next line that is
// neither blank nor a `//` comment: the error in the arm after that one is
// still reported, at its own position.
export variant Shape { Circle(r: number), Square(s: number), Point }

export function size(shape: Shape): number {
  return match (shape) {
    // @ts-ignore
    Circle(r) => r.radius,
    Square(s) => s.side,
    Point => 0,
  };
}

export const inline = (n: number) => [
  match (n) {
    // @ts-ignore
    1 => n.one,
    _ => n.other,
  },
];


//// [directiveBetweenMatchArmsGovernsOneArm.ts]
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
// A `// @ts-ignore` between match arms governs the one arm on the line
// after it, as TypeScript applies a directive to the next line that is
// neither blank nor a `//` comment: the error in the arm after that one is
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
      case "Circle": { const { r } = $tt_m; $tt_v0 = r.radius;
        break;
      }
      case "Square": {
        const { s } = $tt_m;
        $tt_v0 = s.side;
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
}

export const inline = (n: number) => {
  let $tt_v1: number;
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 1: $tt_v1 = 0; break;
      default: $tt_v1 = 1; break;
    }
  }
  return [
  (
  // @ts-ignore
  $tt_v1 === 0 ? n.one
   : n.other),
];
};
