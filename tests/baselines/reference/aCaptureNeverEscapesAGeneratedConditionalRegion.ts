//// [aCaptureNeverEscapesAGeneratedConditionalRegion.tt] ////
declare const flag: boolean;
declare function id(v: number): number;
export const short = flag && id(match (flag) { true => 1, _ => 0 });


//// [aCaptureNeverEscapesAGeneratedConditionalRegion.ts]
declare const flag: boolean;
declare function id(v: number): number;
let $tt_v3: (number) | (false);
let $tt_v2: boolean;
if ($tt_v2 = flag) {
  let $tt_v0: number;
  const $tt_v1 = (id);
  {
    const $tt_m = flag;
    switch ($tt_m) {
      case true: {
        $tt_v0 = 1;
        break;
      }
      default: {
        $tt_v0 = 0;
        break;
      }
    }
  }
  $tt_v3 = $tt_v2 && $tt_v1($tt_v0);
} else {
  $tt_v3 = $tt_v2;
}

export const short = $tt_v3;
