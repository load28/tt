//// [aMatchInAGuardTestWithAPipelineIsLoweredBeforeTheTest.tt] ////
variant S { A(v: number), B(w: number), C }
declare const s: S;
const z = match (s) { A(v) if match (s) { B(w) => w > 0, _ => false } |> Boolean => 1, _ => 0 };


//// [aMatchInAGuardTestWithAPipelineIsLoweredBeforeTheTest.ts]
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
let $tt_v0$z: number;
{
  const $tt_m = s;
  do {
    if ($tt_m.kind === "A") {
      const { v } = $tt_m;
      let $tt_v1: boolean;
      do {
        let $tt_v3: boolean | undefined;
        {
          const $tt_m = s;
          switch ($tt_m.kind) {
            case "B": {
              const { w } = $tt_m;
              $tt_v3 = w > 0;
              break;
            }
            default: {
              $tt_v3 = false;
              break;
            }
          }
        }
        $tt_v1 = Boolean($tt_v3);
        break;
      } while (false);
      if ($tt_v1) {
        $tt_v0$z = 1;
        break;
      }
    }
    $tt_v0$z = 0;
    break;
  } while (false);
}
const z = $tt_v0$z;
