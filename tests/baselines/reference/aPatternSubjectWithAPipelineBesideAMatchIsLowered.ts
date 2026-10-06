//// [aPatternSubjectWithAPipelineBesideAMatchIsLowered.tt] ////
variant V { A(n: number), B }
const wrap = (xs: unknown[]): V => (xs.length === 2 ? V.A(Number(xs[0]) + String(xs[1]).length) : V.B);
function f(k: number) {
  const A(n) = wrap([match (k) { 1 => 10, _ => 20 }, k |> String]) else { return -1; };
  if let A(n: m) = wrap([match (k) { 1 => 1, _ => 2 }, 100 |> String]) { return n + m; }
  return n;
}
console.log(f(1), f(25));


//// [aPatternSubjectWithAPipelineBesideAMatchIsLowered.ts]
type V =
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const wrap = (xs: unknown[]): V => (xs.length === 2 ? V.A(Number(xs[0]) + String(xs[1]).length) : V.B);
function f(k: number) {
  let $tt_t0; let $tt_v0: number;
  const $tt_v2 = (wrap);
  {
    const $tt_m = k;
    switch ($tt_m) {
      case 1: {
        $tt_v0 = 10;
        break;
      }
      default: {
        $tt_v0 = 20;
        break;
      }
    }
  }
  $tt_t0 = $tt_v2([$tt_v0, (($tt_v, $tt_f) => $tt_f($tt_v))(k, String)]);
  if ($tt_t0.kind !== "A") {
    return -1;
  }
  const { n } = $tt_t0;
  {
    let $tt_t1; let $tt_v3: number;
    const $tt_v5 = (wrap);
    {
      const $tt_m = k;
      switch ($tt_m) {
        case 1: {
          $tt_v3 = 1;
          break;
        }
        default: {
          $tt_v3 = 2;
          break;
        }
      }
    }
    $tt_t1 = $tt_v5([$tt_v3, String(100)]);
    if ($tt_t1.kind === "A") {
      const { n: m } = $tt_t1;
      return n + m;
    }
  }
  return n;
}
console.log(f(1), f(25));
