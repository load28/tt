//// [shape.tt] ////
export variant Shape { Circle(r: number), Point }

//// [a.tt] ////
import { Shape } from ".\\shape.tt";
declare const s: Shape;
export const n = match (s) { Circle(r) => r };


//// [shape.ts]
export type Shape =
  | { kind: "Circle"; r: number }
  | { kind: "Point" };
export const Shape = {
  Circle: (r: number): Shape => ({ kind: "Circle", r }),
  Point: { kind: "Point" } as const,
};
\ No newline at end of file
