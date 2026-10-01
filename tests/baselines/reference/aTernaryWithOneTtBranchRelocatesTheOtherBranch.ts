//// [aTernaryWithOneTtBranchRelocatesTheOtherBranch.tt] ////
declare const flag: boolean;
export const pick = flag ? match (1) { 1 => 1, _ => 0 } : 9;


//// [aTernaryWithOneTtBranchRelocatesTheOtherBranch.ts]
declare const flag: boolean;
let $tt_v2: number;
if (flag) {
  {
    const $tt_m = 1;
    switch ($tt_m) {
      case 1: {
        $tt_v2 = 1;
        break;
      }
      default: {
        $tt_v2 = 0;
        break;
      }
    }
  }
} else {
  $tt_v2 = 9;
}

export const pick = $tt_v2;
