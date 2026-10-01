//// [variantWithPayloadEmitsUnionTypeAndConstructors.tt] ////

variant Shape {
  Circle(radius: number),
  Rect(width: number, height: number),
  Point,
}


//// [variantWithPayloadEmitsUnionTypeAndConstructors.ts]

type Shape =
  | { kind: "Circle"; radius: number }
  | { kind: "Rect"; width: number; height: number }
  | { kind: "Point" };
const Shape = {
  Circle: (radius: number): Shape => ({ kind: "Circle", radius }),
  Rect: (width: number, height: number): Shape => ({ kind: "Rect", width, height }),
  Point: { kind: "Point" } as const,
};
