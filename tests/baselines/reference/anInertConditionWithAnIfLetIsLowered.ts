//// [anInertConditionWithAnIfLetIsLowered.tt] ////
variant V { A(n: number), B }
export function f(k: number, v: V) {
  const u = ((x: number) => { if let A(n) = v { return n + x; } return 0; }) ? match (k) { 1 => "one", _ => "other" } : "none";
  return u;
}


//// [anInertConditionWithAnIfLetIsLowered.ts]
type V =
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
export function f(k: number, v: V) {
  let $tt_v2: string;
  if ((x: number) => { {
    const $tt_t0 = v;
    if ($tt_t0.kind === "A") {
      const { n } = $tt_t0;
      return n + x;
    }
  } return 0; }) {
    {
      const $tt_m = k;
      switch ($tt_m) {
        case 1: {
          $tt_v2 = "one";
          break;
        }
        default: {
          $tt_v2 = "other";
          break;
        }
      }
    }
  } else {
    $tt_v2 = "none";
  }
  
  const u = $tt_v2;
  return u;
}
