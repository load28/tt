//// [commentsBetweenMatchArms.tt] ////
// A comment written between two match arms reaches the output where the
// arms were written, and a `// @ts-expect-error` or `// @ts-ignore` there
// governs the arm it governs in the source: the line after it.
export variant Shape { Circle(r: number), Square(s: number), Point }

export function area(shape: Shape): number {
  return match (shape) {
    // the radius is doubled
    Circle(r) => r * 2,
    // @ts-expect-error a square reads a field it does not have
    Square(s) => s.side ?? s,
    /* a point has no area */
    Point => 0,
    // after the last arm
  };
}

export function guarded(shape: Shape): string {
  const label = match (shape) {
    Circle(r) if r > 1 => "big circle",
    // @ts-ignore the guard compares a number with a string
    Circle(r) if r === "small" => "never",
    Circle(r) => `circle ${r}`, // @ts-expect-error governs the next arm
    Square(s) => s.toFixed(1).missing ?? "square",
    _ => "point",
  };
  return label;
}

export function code(n: number): string {
  return String(
    match (n) {
      1 => "one",
      // two arms share a comment line
      2 | 3 => "few",
      /* a block comment before the wildcard */ _ => "many",
    },
  );
}

export function blockArm(shape: Shape): number {
  const value = match (shape) {
    Circle(r) => {
      return r;
    },
    // @ts-expect-error the block arm's first line reads a missing field
    Square(s) => { const side: number = s.edge; return side ?? s; },
    Point => -1,
  };
  return value;
}

export function pair(a: Shape, b: Shape): string {
  return match (a, b) {
    (Circle, Circle) => "two circles",
    // a tuple arm after a comment
    (Point, _) => "point first",
    _ => "other",
  };
}

console.log(area(Shape.Circle(2)), area(Shape.Square(3)), area(Shape.Point));
console.log(guarded(Shape.Circle(3)), guarded(Shape.Circle(1)), guarded(Shape.Square(2)), guarded(Shape.Point));
console.log(code(1), code(3), code(9));
console.log(blockArm(Shape.Circle(4)), blockArm(Shape.Square(5)), blockArm(Shape.Point));
console.log(pair(Shape.Circle(1), Shape.Circle(2)), pair(Shape.Point, Shape.Circle(1)), pair(Shape.Square(1), Shape.Point));


//// [commentsBetweenMatchArms.ts]
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
// A comment written between two match arms reaches the output where the
// arms were written, and a `// @ts-expect-error` or `// @ts-ignore` there
// governs the arm it governs in the source: the line after it.
export type Shape =
  | { kind: "Circle"; r: number }
  | { kind: "Square"; s: number }
  | { kind: "Point" };
export const Shape = {
  Circle: (r: number): Shape => ({ kind: "Circle", r }),
  Square: (s: number): Shape => ({ kind: "Square", s }),
  Point: { kind: "Point" } as const,
};

export function area(shape: Shape): number {
  let $tt_v0: number;
  {
    const $tt_m = shape;
    switch ($tt_m.kind) {
      // the radius is doubled
      case "Circle": {
        const { r } = $tt_m;
        $tt_v0 = r * 2;
        break;
      }
      // @ts-expect-error a square reads a field it does not have
      case "Square": { const { s } = $tt_m; $tt_v0 = s.side ?? s;
        break;
      }
      /* a point has no area */
      case "Point": {
        $tt_v0 = 0;
        break;
      }
      // after the last arm
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}

export function guarded(shape: Shape): string {
  let $tt_v1;
  {
    const $tt_m = shape;
    do {
      if ($tt_m.kind === "Circle") {
        const { r } = $tt_m;
        if (r > 1) {
          $tt_v1 = "big circle";
          break;
        }
      }
      // @ts-ignore the guard compares a number with a string
      if ($tt_m.kind === "Circle") { const { r } = $tt_m; if (r === "small") { $tt_v1 = "never";
          break;
        }
      }
      if ($tt_m.kind === "Circle") {
        const { r } = $tt_m;
        $tt_v1 = `circle ${r}`;
        break;
      }
      // @ts-expect-error governs the next arm
      if ($tt_m.kind === "Square") { const { s } = $tt_m; $tt_v1 = s.toFixed(1).missing ?? "square";
        break;
      }
      $tt_v1 = "point";
      break;
    } while (false);
  }
  const label = $tt_v1;
  return label;
}

export function code(n: number): string {
  let $tt_v2: number;
  const $tt_v3: typeof String = (String);
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 1: $tt_v2 = 0; break;
      case 2: case 3: $tt_v2 = 1; break;
      default: $tt_v2 = 2; break;
    }
  }
  return $tt_v3(
    ($tt_v2 === 0 ? "one" : 
    // two arms share a comment line
    $tt_v2 === 1 ? "few"
     : 
    /* a block comment before the wildcard */
    "many"
    ),
  );
}

export function blockArm(shape: Shape): number {
  let $tt_v4: number;
  {
    const $tt_m = shape;
    switch ($tt_m.kind) {
      case "Circle": {
        const { r } = $tt_m;
        $tt_v4 = r;
        break;
      }
      // @ts-expect-error the block arm's first line reads a missing field
      case "Square": { const { s } = $tt_m; const side: number = s.edge; $tt_v4 = side ?? s; break;
      }
      case "Point": {
        $tt_v4 = -1;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const value = $tt_v4;
  return value;
}

export function pair(a: Shape, b: Shape): string {
  let $tt_v5: string;
  {
    const $tt_m0 = a;
    const $tt_m1 = b;
    do {
      if ($tt_m0.kind === "Circle" && $tt_m1.kind === "Circle") {
        $tt_v5 = "two circles";
        break;
      }
      // a tuple arm after a comment
      if ($tt_m0.kind === "Point") {
        $tt_v5 = "point first";
        break;
      }
      $tt_v5 = "other";
      break;
    } while (false);
  }
  return $tt_v5;
}

console.log(area(Shape.Circle(2)), area(Shape.Square(3)), area(Shape.Point));
console.log(guarded(Shape.Circle(3)), guarded(Shape.Circle(1)), guarded(Shape.Square(2)), guarded(Shape.Point));
console.log(code(1), code(3), code(9));
console.log(blockArm(Shape.Circle(4)), blockArm(Shape.Square(5)), blockArm(Shape.Point));
console.log(pair(Shape.Circle(1), Shape.Circle(2)), pair(Shape.Point, Shape.Circle(1)), pair(Shape.Square(1), Shape.Point));
