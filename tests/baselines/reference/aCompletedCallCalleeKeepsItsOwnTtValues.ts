//// [aCompletedCallCalleeKeepsItsOwnTtValues.tt] ////
const h = (x: number) => x + 1;
function f2(...a: unknown[]) { return a; }
variant V { A(n: number), B }
export function F(v: V) {
  return f2(((w: number) => w |> h)(match (v) { A(n) => n, B => -1 }), match (v) { A => 1, B => 2 });
}
console.log(F(V.A(1)));


//// [aCompletedCallCalleeKeepsItsOwnTtValues.ts]
function $tt_show(value: unknown): string {
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
}
const h = (x: number) => x + 1;
function f2(...a: unknown[]) { return a; }
type V =
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
export function F(v: V) {
  let $tt_v0: number;
  let $tt_v1: unknown[];
  const $tt_v3: typeof f2 = (f2);
  const $tt_v2 = ((void 0, (w: number) => (($tt_v, $tt_f) => $tt_f($tt_v))(w, h)));
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v0 = $tt_v2(n);
        break;
      }
      case "B": {
        $tt_v0 = $tt_v2(-1);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const $tt_v4 = ($tt_v0);
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v1 = $tt_v3($tt_v4, 1);
        break;
      }
      case "B": {
        $tt_v1 = $tt_v3($tt_v4, 2);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v1;
}
console.log(F(V.A(1)));
