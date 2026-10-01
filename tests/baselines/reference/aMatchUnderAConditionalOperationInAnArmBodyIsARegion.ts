//// [aMatchUnderAConditionalOperationInAnArmBodyIsARegion.tt] ////
variant S { A(v: number), B(w: number), C }
declare const s: S;
declare function eff(): number;
const y = match (s) { A(v) => eff() > 0 && match (s) { B(w) => w > 0, _ => false }, _ => false };


//// [aMatchUnderAConditionalOperationInAnArmBodyIsARegion.ts]
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
let $tt_v0$y: boolean;
{
  const $tt_m = s;
  switch ($tt_m.kind) {
    case "A": {
      const { v } = $tt_m;
      let $tt_v3$y: boolean;
      let $tt_v2$y: boolean;
      if ($tt_v2$y = eff() > 0) {
        let $tt_v1$y: boolean;
        {
          const $tt_m = s;
          switch ($tt_m.kind) {
            case "B": {
              const { w } = $tt_m;
              $tt_v1$y = w > 0;
              break;
            }
            default: {
              $tt_v1$y = false;
              break;
            }
          }
        }
        $tt_v3$y = $tt_v2$y && $tt_v1$y;
      } else {
        $tt_v3$y = $tt_v2$y;
      }
      $tt_v0$y = $tt_v3$y;
      break;
    }
    default: {
      $tt_v0$y = false;
      break;
    }
  }
}
const y = $tt_v0$y;
