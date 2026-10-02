//// [tsconfig.json] ////
{
  "compilerOptions": {
    "target": "es2022",
    "module": "preserve",
    "moduleResolution": "bundler",
    "strict": true,
    "noEmit": true,
    "allowImportingTsExtensions": true
  }
}

//// [shapes.tt] ////
export variant Shape { Circle(radius: number), Rect(width: number, height: number) }

//// [area.tt] ////
import { Shape } from "./shapes.tt";
export function area(s: Shape): number {
  return match (s) {
    Circle(radius) => Math.PI * radius * radius,
    Rect(width, height) => width * height,
  };
}
export const perimeter = (s: Shape) => match (s) { Circle(radius) => 2 * Math.PI * radius };

//// [main.ts] ////
import { area } from "./area.tt";
import { Shape } from "./shapes.tt";
export const total: number = area(Shape.Circle(1)) + area(Shape.Rect(2, 3));


//// [main.ts]
import { area } from "./area.ts";
import { Shape } from "./shapes.ts";
export const total: number = area(Shape.Circle(1)) + area(Shape.Rect(2, 3));

//// [shapes.ts]
export type Shape =
  | { kind: "Circle"; radius: number }
  | { kind: "Rect"; width: number; height: number };
export const Shape = {
  Circle: (radius: number): Shape => ({ kind: "Circle", radius }),
  Rect: (width: number, height: number): Shape => ({ kind: "Rect", width, height }),
};
