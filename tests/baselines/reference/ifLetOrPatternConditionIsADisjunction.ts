//// [ifLetOrPatternConditionIsADisjunction.tt] ////
variant E { A(x: number), B(x: number), C }
function g(e: E): number {
  if let A(x) | B(x) = e {
    return x;
  }
  return -1;
}


//// [ifLetOrPatternConditionIsADisjunction.ts]
type E =
  | { kind: "A"; x: number }
  | { kind: "B"; x: number }
  | { kind: "C" };
const E = {
  A: (x: number): E => ({ kind: "A", x }),
  B: (x: number): E => ({ kind: "B", x }),
  C: { kind: "C" } as const,
};
function g(e: E): number {
  {
    const $tt_t0 = e;
    if ($tt_t0.kind === "A" || $tt_t0.kind === "B") {
      const { x } = $tt_t0;
      return x;
    }
  }
  return -1;
}
