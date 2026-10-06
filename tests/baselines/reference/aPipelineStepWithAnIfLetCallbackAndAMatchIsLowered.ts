//// [aPipelineStepWithAnIfLetCallbackAndAMatchIsLowered.tt] ////
variant V { A(n: number), B }
const pick = (f: (v: V) => number, fallback: number) => (vs: V[]) => vs.map((v) => f(v) || fallback);
function f(vs: V[], mode: number) {
  return vs |> pick((v) => { if let A(n) = v { return n; } return 0; }, match (mode) { 1 => -1, _ => 9 });
}
console.log(f([V.A(3), V.B], 1).join(","), f([V.B], 2).join(","));


//// [aPipelineStepWithAnIfLetCallbackAndAMatchIsLowered.ts]
type V =
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const pick = (f: (v: V) => number, fallback: number) => (vs: V[]) => vs.map((v) => f(v) || fallback);
function f(vs: V[], mode: number) {
  let $tt_v0: number[];
  do {
    const $tt_v4: V[] = vs;
    let $tt_v1: number;
    const $tt_v2: typeof pick = (pick);
    const $tt_v3: (v: V) => number = ((v) => { {
      const $tt_t0 = v;
      if ($tt_t0.kind === "A") {
        const { n } = $tt_t0;
        return n;
      }
    } return 0; });
    {
      const $tt_m = mode;
      switch ($tt_m) {
        case 1: {
          $tt_v1 = -1;
          break;
        }
        default: {
          $tt_v1 = 9;
          break;
        }
      }
    }
    $tt_v0 = $tt_v2($tt_v3, $tt_v1)($tt_v4);
    break;
  } while (false);
  return $tt_v0;
}
console.log(f([V.A(3), V.B], 1).join(","), f([V.B], 2).join(","));
