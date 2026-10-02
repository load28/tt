//// [aGuardedAllWildcardTupleArmIsTestedByItsGuardAlone.tt] ////
variant T { A, B }
function f(a: T, b: T, cond: boolean): number {
  return match (a, b) {
    (A, _) => 1,
    (_, _) if cond => 2,
    _ => 3,
  };
}


//// [aGuardedAllWildcardTupleArmIsTestedByItsGuardAlone.ts]
type T =
  | { kind: "A" }
  | { kind: "B" };
const T = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
function f(a: T, b: T, cond: boolean): number {
  let $tt_v0: number;
  {
    const $tt_m0 = a;
    const $tt_m1 = b;
    do {
      if ($tt_m0.kind === "A") {
        $tt_v0 = 1;
        break;
      }
      if (cond) {
        $tt_v0 = 2;
        break;
      }
      $tt_v0 = 3;
      break;
    } while (false);
  }
  return $tt_v0;
}
