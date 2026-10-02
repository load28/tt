interface Point { x: number; y: number }
/** The straight-line distance between two points. */
export function [|/*decl*/distance|](a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}
const /*value*/origin: Point = { x: 0, y: 0 };
export const d = /*call*/[|distance|](origin, { x: 3, y: /*args*/4 });
export const label = origin./*member*/x.toFixed(1);
const unused: string = 1;
