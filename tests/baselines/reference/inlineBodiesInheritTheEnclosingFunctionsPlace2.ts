//// [inlineBodiesInheritTheEnclosingFunctionsPlace2.tt] ////
variant E { A(x: number), B }
function f(e: E): number {
  const Some(v) = find(e) else {
    const B() = e else { throw new Error("a"); };
    return 0;
  };
  return v;
}


//// [inlineBodiesInheritTheEnclosingFunctionsPlace2.ts]
type E =
  | { kind: "A"; x: number }
  | { kind: "B" };
const E = {
  A: (x: number): E => ({ kind: "A", x }),
  B: { kind: "B" } as const,
};
function f(e: E): number {
  const $tt_t0 = find(e);
  if ($tt_t0.kind !== "Some") {
    const $tt_t1 = e;
    if ($tt_t1.kind !== "B") {
      throw new Error("a");
    }
    
    return 0;
  }
  const { v } = $tt_t0;
  return v;
}
