type S = { kind: "A" } | { kind: "B"; x: number };
class Cls { constructor(a: number, b: string) {} }
declare function two(a: number, b: string): number;
const half = (n: number) => n / 2;
export function f(s: S, m: number) {
  const p = two(1, "q/*stored*/");
  const q = new Cls(1, /*constructed*/);
  console.log(half(m), m/*piped*/);
  return [p, q];
}
