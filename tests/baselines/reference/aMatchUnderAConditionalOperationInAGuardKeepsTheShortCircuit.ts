//// [aMatchUnderAConditionalOperationInAGuardKeepsTheShortCircuit.tt] ////
variant S { A(v: number), B(w: number), C }
declare const s: S;
const x = match (s) { A(v) if v > 0 && match (s) { B(w) => w > 0, _ => false } => 1, _ => 0 };


//// [aMatchUnderAConditionalOperationInAGuardKeepsTheShortCircuit.ts]
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
let $tt_v0$x: number;
{
  const $tt_m = s;
  do {
    if ($tt_m.kind === "A") {
      const { v } = $tt_m;
      let $tt_v3: boolean;
      let $tt_v2: boolean;
      if ($tt_v2 = v > 0) {
        let $tt_v1: boolean;
        {
          const $tt_m = s;
          switch ($tt_m.kind) {
            case "B": {
              const { w } = $tt_m;
              $tt_v1 = w > 0;
              break;
            }
            default: {
              $tt_v1 = false;
              break;
            }
          }
        }
        $tt_v3 = $tt_v2 && $tt_v1;
      } else {
        $tt_v3 = $tt_v2;
      }
      if ($tt_v3) {
        $tt_v0$x = 1;
        break;
      }
    }
    $tt_v0$x = 0;
    break;
  } while (false);
}
const x = $tt_v0$x;
