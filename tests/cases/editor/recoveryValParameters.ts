interface P { x: number }
function a(p: P): number {
  return p.x;
}
function b(): void {
  let c = 1;
  c =
}
function d(q: P): number {
  return /*q*/q.x;
}
export const n = a({ x: 1 }) + d({ x: 2 });
