//// [anIfLetSubjectPipingAMatchIsOnePipeline.tt] ////
variant V { A(n: number), B }
const bump = (v: V): V => match (v) { A(n) => V.A(n + 1), B => V.B };
function f(x: V) {
  if let A(n) = match (x) { A(n) => x, B => V.A(0) } |> bump { return n; }
  return -1;
}
console.log(f(V.A(1)), f(V.B));


//// [anIfLetSubjectPipingAMatchIsOnePipeline.ts]
var $tt_show: (value: unknown) => string = function (value) {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
};
type V =
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const bump = (v: V): V => {
  let $tt_v0: V;
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v0 = V.A(n + 1);
        break;
      }
      case "B": {
        $tt_v0 = V.B;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
};
function f(x: V) {
  {
    let $tt_t0; do {
      let $tt_v1: V;
      let $tt_v3: V;
      {
        const $tt_m = x;
        switch ($tt_m.kind) {
          case "A": {
            const { n } = $tt_m;
            $tt_v3 = x;
            break;
          }
          case "B": {
            $tt_v3 = V.A(0);
            break;
          }
          default: {
            throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
      }
      $tt_v1 = bump($tt_v3);
      $tt_t0 = $tt_v1; break;
    } while (false);
    if ($tt_t0.kind === "A") {
      const { n } = $tt_t0;
      return n;
    }
  }
  return -1;
}
console.log(f(V.A(1)), f(V.B));
