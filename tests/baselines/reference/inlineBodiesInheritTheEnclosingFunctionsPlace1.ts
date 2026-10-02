//// [inlineBodiesInheritTheEnclosingFunctionsPlace1.tt] ////
variant E { A(x: number), B }
function f(e: E): Result<number, string> {
  if let A(x) = e {
    const n = try g(x);
    return Result.Ok(n);
  }
  return Result.Ok(0);
}


//// [inlineBodiesInheritTheEnclosingFunctionsPlace1.ts]
type E =
  | { kind: "A"; x: number }
  | { kind: "B" };
const E = {
  A: (x: number): E => ({ kind: "A", x }),
  B: { kind: "B" } as const,
};
function f(e: E): Result<number, string> {
  {
    const $tt_t0 = e;
    if ($tt_t0.kind === "A") {
      const { x } = $tt_t0;
      const $tt_t1 = g(x);
      if (!("value" in $tt_t1)) {
        return $tt_t1;
      }
      const n = $tt_t1.value;
    return Result.Ok(n);
    }
  }
  return Result.Ok(0);
}
