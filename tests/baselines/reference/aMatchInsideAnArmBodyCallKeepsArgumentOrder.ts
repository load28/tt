//// [aMatchInsideAnArmBodyCallKeepsArgumentOrder.tt] ////
variant S { A(v: number), B(w: number), C }
declare const s: S;
declare function eff(): number;
declare function g(a: number, b: number): number;
const x = match (s) { A(v) => g(eff(), match (s) { B(w) => w, _ => 0 }), _ => 0 };


//// [aMatchInsideAnArmBodyCallKeepsArgumentOrder.ts]
type S =
  | { kind: "A"; v: number }
  | { kind: "B"; w: number }
  | { kind: "C" };
const S = {
  A: (v: number): S => ({ kind: "A", v }),
  B: (w: number): S => ({ kind: "B", w }),
  C: { kind: "C" } as const,
};
declare const s: S;
declare function eff(): number;
declare function g(a: number, b: number): number;
let $tt_v0$x: number;
{
  const $tt_m = s;
  switch ($tt_m.kind) {
    case "A": {
      const { v } = $tt_m;
      let $tt_v1$x: number;
      const $tt_v2$x = (g);
      const $tt_v3$x: number = (eff());
      {
        const $tt_m = s;
        switch ($tt_m.kind) {
          case "B": {
            const { w } = $tt_m;
            $tt_v1$x = w;
            break;
          }
          default: {
            $tt_v1$x = 0;
            break;
          }
        }
      }
      $tt_v0$x = $tt_v2$x($tt_v3$x, $tt_v1$x);
      break;
    }
    default: {
      $tt_v0$x = 0;
      break;
    }
  }
}
const x = $tt_v0$x;
