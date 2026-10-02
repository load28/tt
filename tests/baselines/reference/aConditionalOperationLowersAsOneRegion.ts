//// [aConditionalOperationLowersAsOneRegion.tt] ////
declare const flag: boolean;
export const a = flag && match (1) { 1 => 1, _ => 0 };


//// [aConditionalOperationLowersAsOneRegion.ts]
declare const flag: boolean;
let $tt_v2: (number) | (false);
let $tt_v1: boolean;
if ($tt_v1 = flag) {
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
  $tt_v2 = $tt_v1 && $tt_v0;
} else {
  $tt_v2 = $tt_v1;
}

export const a = $tt_v2;
