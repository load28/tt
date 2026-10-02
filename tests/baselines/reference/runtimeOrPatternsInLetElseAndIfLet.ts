//// [runtimeOrPatternsInLetElseAndIfLet.tt] ////

variant Shape { Circle(r: number), Square(r: number), Dot }

function side(s: Shape): number {
  const Circle(r) | Square(r) = s else { return 0; };
  return r;
}

function tell(s: Shape): string {
  if let Circle(r) | Square(r) = s {
    return "sized " + r;
  } else {
    return "dot";
  }
}

console.log(side(Shape.Circle(3)));
console.log(side(Shape.Square(4)));
console.log(side(Shape.Dot));
console.log(tell(Shape.Square(5)));
console.log(tell(Shape.Dot));

export {};


//// [runtimeOrPatternsInLetElseAndIfLet.ts]

type Shape =
  | { kind: "Circle"; r: number }
  | { kind: "Square"; r: number }
  | { kind: "Dot" };
const Shape = {
  Circle: (r: number): Shape => ({ kind: "Circle", r }),
  Square: (r: number): Shape => ({ kind: "Square", r }),
  Dot: { kind: "Dot" } as const,
};

function side(s: Shape): number {
  const $tt_t0 = s;
  if ($tt_t0.kind !== "Circle" && $tt_t0.kind !== "Square") {
    return 0;
  }
  const { r } = $tt_t0;
  return r;
}

function tell(s: Shape): string {
  {
    const $tt_t1 = s;
    if ($tt_t1.kind === "Circle" || $tt_t1.kind === "Square") {
      const { r } = $tt_t1;
      return "sized " + r;
    } else {
      return "dot";
    }
  }
}

console.log(side(Shape.Circle(3)));
console.log(side(Shape.Square(4)));
console.log(side(Shape.Dot));
console.log(tell(Shape.Square(5)));
console.log(tell(Shape.Dot));

export {};
