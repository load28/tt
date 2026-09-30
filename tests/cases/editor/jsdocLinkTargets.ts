// @filename: shapes.ts
export type Shape = { kind: "Circle"; radius: number } | { kind: "Point" };
export const Shape = { Circle: (radius: number): Shape => ({ kind: "Circle", radius }), Point: { kind: "Point" } as const };
/** Area. See {@link perimeter}. */
export function area(s: Shape): number { return 1; }
export function perimeter(s: Shape): number { return 2; }
// @filename: main.ts
import { area, Shape } from "./shapes";
type S = { kind: "A" } | { kind: "B"; x: number };
/**
 * Scales a value.
 * @see {@link other}
 */
export function scale(n: number): number { return n; }
export function other() {}
export const r = (s: S) => s.kind === "A" ? /*local*/scale(1) : /*imported*/area(Shape.Point);
export const q = scale(/*args*/);
