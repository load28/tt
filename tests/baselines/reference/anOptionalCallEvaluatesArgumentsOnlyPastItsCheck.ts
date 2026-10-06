//// [anOptionalCallEvaluatesArgumentsOnlyPastItsCheck.tt] ////
declare const f: ((v: number, w: number) => number) | undefined;
declare function pre(): number;
export const r = f?.(pre(), match (1) { 1 => 1, _ => 0 });


//// [anOptionalCallEvaluatesArgumentsOnlyPastItsCheck.ts]
declare const f: ((v: number, w: number) => number) | undefined;
declare function pre(): number;
let $tt_v3: (number) | (undefined);
const $tt_v1: typeof f = (f);
if ($tt_v1 != null) {
  const $tt_v2: number = (pre());
  let $tt_v0: number;
  {
    const $tt_m = 1;
    switch ($tt_m) {
      case 1: {
        $tt_v0 = 1;
        break;
      }
      default: {
        $tt_v0 = 0;
        break;
      }
    }
  }
  $tt_v3 = $tt_v1($tt_v2, $tt_v0);
} else {
  $tt_v3 = undefined;
}

export const r = $tt_v3;
