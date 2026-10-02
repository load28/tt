export type Shape = { kind: "Circle"; radius: number } | { kind: "Rect"; width: number } | { kind: "Point" };
export const Shape = {};
const limit = 1;
export function g(s: Shape) {
  let a;
  if (s.kind === "Circle") { const { radius } = s; a = radius; }
  else if (s.kind === "Rect") { const { width } = s; a = /*body*/; }
  let b;
  if (s.kind === "Circle") { const { radius } = s; b = radius; }
  else if (s.kind === "Rect") { const { width } = s; b = /*afterComma*/; }
  else { b = 0; }
  return [a, b];
}
