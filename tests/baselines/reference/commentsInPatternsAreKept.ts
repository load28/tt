//// [commentsInPatternsAreKept.tt] ////
// A comment written in a pattern, between a pattern and its guard or `=>`,
// between a match's scrutinee and its `{`, or in an `if let` or a let-else
// before its block reaches the output: the lowering rewrites that text into
// tests and destructuring, so each comment is written, in source order, on
// a line of its own after the arm (or the statement) it was written in.
// What the program does is unchanged.
export variant Shape { Circle(radius: number), Square(side: number), Point }
export variant Maybe { Some(value: number), None }
export variant Box { Full(item: Maybe), Empty }

export function size(shape: Shape): number {
  return match (shape) /* by kind */ {
    Circle(/* the radius */ radius: r) /* doubled */ => r * 2,
    Square(side /* a side */ : /* named */ s) => s,
    Point // no area
      => 0,
  };
}

export function guarded(shape: Shape): string {
  return match (shape) {
    Circle(radius: r) /* before the guard */ if r > 1 /* after the guard */ => "big",
    _ /* anything else */ => "other",
  };
}

export function nested(box: Box): number {
  return match (box) {
    Full(item: Some(/* deep */ value: v)) => v,
    _ => -1,
  };
}

export function literal(n: number): string {
  return `${match (n) { 1 /* one */ | /* or two */ 2 => "low", _ => "high" }}`;
}

export function pair(a: Shape, b: Shape): string {
  return match (a, b) {
    (Circle(radius: r) /* first */, /* second */ Point) => `circle ${r} and point`,
    _ => "other pair",
  };
}

export function chain(shape: Shape): number {
  if /* a */ let Circle(/* c */ radius: r) = shape /* then */ {
    return r;
  } else if let Square(// a line comment
    side: s) = shape {
    return s;
  } else {
    return 0;
  }
}

export function bound(shape: Shape): number {
  const /* k */ Square(side: s) /* l */ = shape /* m */ else { return -1; };
  return s;
}

const shapes = [Shape.Circle(2), Shape.Square(3), Shape.Point];
console.log(shapes.map(size).join(","));
console.log(shapes.map(guarded).join(","));
console.log([nested(Box.Full(Maybe.Some(4))), nested(Box.Full(Maybe.None)), nested(Box.Empty)].join(","));
console.log([1, 2, 3].map(literal).join(","));
console.log(pair(Shape.Circle(1), Shape.Point), pair(Shape.Point, Shape.Point));
console.log(shapes.map(chain).join(","), shapes.map(bound).join(","));


//// [commentsInPatternsAreKept.ts]
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
// A comment written in a pattern, between a pattern and its guard or `=>`,
// between a match's scrutinee and its `{`, or in an `if let` or a let-else
// before its block reaches the output: the lowering rewrites that text into
// tests and destructuring, so each comment is written, in source order, on
// a line of its own after the arm (or the statement) it was written in.
// What the program does is unchanged.
export type Shape =
  | { kind: "Circle"; radius: number }
  | { kind: "Square"; side: number }
  | { kind: "Point" };
export const Shape = {
  Circle: (radius: number): Shape => ({ kind: "Circle", radius }),
  Square: (side: number): Shape => ({ kind: "Square", side }),
  Point: { kind: "Point" } as const,
};
export type Maybe =
  | { kind: "Some"; value: number }
  | { kind: "None" };
export const Maybe = {
  Some: (value: number): Maybe => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};
export type Box =
  | { kind: "Full"; item: Maybe }
  | { kind: "Empty" };
export const Box = {
  Full: (item: Maybe): Box => ({ kind: "Full", item }),
  Empty: { kind: "Empty" } as const,
};

export function size(shape: Shape): number {
  let $tt_v0: number;
  {
    const $tt_m = shape;
    switch ($tt_m.kind) {
      /* by kind */
      case "Circle": {
        const { radius: r } = $tt_m;
        $tt_v0 = r * 2;
        break;
      }
      /* the radius */
      /* doubled */
      case "Square": {
        const { side: s } = $tt_m;
        $tt_v0 = s;
        break;
      }
      /* a side */
      /* named */
      case "Point": {
        $tt_v0 = 0;
        break;
      }
      // no area
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}

export function guarded(shape: Shape): string {
  let $tt_v1: string;
  {
    const $tt_m = shape;
    do {
      if ($tt_m.kind === "Circle") {
        const { radius: r } = $tt_m;
        if (r > 1 /* after the guard */) {
          $tt_v1 = "big";
          break;
        }
      }
      /* before the guard */
      $tt_v1 = "other";
      break;
      /* anything else */
    } while (false);
  }
  return $tt_v1;
}

export function nested(box: Box): number {
  let $tt_v2: number;
  {
    const $tt_m = box;
    do {
      if ($tt_m.kind === "Full" && $tt_m.item.kind === "Some") {
        const { value: v } = $tt_m.item;
        $tt_v2 = v;
        break;
      }
      /* deep */
      $tt_v2 = -1;
      break;
    } while (false);
  }
  return $tt_v2;
}

export function literal(n: number): string {
  let $tt_v3: number;
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 1: case 2: $tt_v3 = 0; break;
      default: $tt_v3 = 1; break;
    }
  }
  return `${($tt_v3 === 0 ? "low"
  /* one */
  /* or two */
   : "high")}`;
}

export function pair(a: Shape, b: Shape): string {
  let $tt_v4: string;
  {
    const $tt_m0 = a;
    const $tt_m1 = b;
    do {
      if ($tt_m0.kind === "Circle" && $tt_m1.kind === "Point") {
        const { radius: r } = $tt_m0;
        $tt_v4 = `circle ${r} and point`;
        break;
      }
      /* first */
      /* second */
      $tt_v4 = "other pair";
      break;
    } while (false);
  }
  return $tt_v4;
}

export function chain(shape: Shape): number {
  {
    const $tt_t0 = shape;
    if ($tt_t0.kind === "Circle") {
      const { radius: r } = $tt_t0;
      return r;
    }
    /* a */
    /* c */
    /* then */
    else {
      const $tt_t1 = shape;
      if ($tt_t1.kind === "Square") {
        const { side: s } = $tt_t1;
        return s;
      }
      // a line comment
      else {
        return 0;
      }
    }
  }
}

export function bound(shape: Shape): number {
  const $tt_t2 = shape;
  if ($tt_t2.kind !== "Square") {
    return -1;
  }
  const { side: s } = $tt_t2;
  /* k */
  /* l */
  /* m */
  return s;
}

const shapes = [Shape.Circle(2), Shape.Square(3), Shape.Point];
console.log(shapes.map(size).join(","));
console.log(shapes.map(guarded).join(","));
console.log([nested(Box.Full(Maybe.Some(4))), nested(Box.Full(Maybe.None)), nested(Box.Empty)].join(","));
console.log([1, 2, 3].map(literal).join(","));
console.log(pair(Shape.Circle(1), Shape.Point), pair(Shape.Point, Shape.Point));
console.log(shapes.map(chain).join(","), shapes.map(bound).join(","));
