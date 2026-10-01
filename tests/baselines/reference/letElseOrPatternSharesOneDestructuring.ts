//// [letElseOrPatternSharesOneDestructuring.tt] ////
variant E { A(x: number), B(x: number), C }
function f(e: E): number {
  const A(x) | B(x) = e else { return 0; };
  return x;
}


//// [letElseOrPatternSharesOneDestructuring.ts]
type E =
  | { kind: "A"; x: number }
  | { kind: "B"; x: number }
  | { kind: "C" };
const E = {
  A: (x: number): E => ({ kind: "A", x }),
  B: (x: number): E => ({ kind: "B", x }),
  C: { kind: "C" } as const,
};
function f(e: E): number {
  const $tt_t0 = e;
  if ($tt_t0.kind !== "A" && $tt_t0.kind !== "B") {
    return 0;
  }
  const { x } = $tt_t0;
  return x;
}
