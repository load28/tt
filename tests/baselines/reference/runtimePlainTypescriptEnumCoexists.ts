//// [runtimePlainTypescriptEnumCoexists.tt] ////

enum Color { Red, Green, Blue }
variant Shape { Circle(radius: number), Point }

console.log(Color.Green);
console.log(Color[Color.Blue]);
console.log(JSON.stringify(Shape.Circle(1)));

export {};


//// [runtimePlainTypescriptEnumCoexists.ts]

enum Color { Red, Green, Blue }
type Shape =
  | { kind: "Circle"; radius: number }
  | { kind: "Point" };
const Shape = {
  Circle: (radius: number): Shape => ({ kind: "Circle", radius }),
  Point: { kind: "Point" } as const,
};

console.log(Color.Green);
console.log(Color[Color.Blue]);
console.log(JSON.stringify(Shape.Circle(1)));

export {};
