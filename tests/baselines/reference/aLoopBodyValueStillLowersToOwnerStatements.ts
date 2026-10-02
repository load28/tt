//// [aLoopBodyValueStillLowersToOwnerStatements.tt] ////
let n = 0;
while (n < 3) { const v = match (n) { 0 => 1, _ => 0 }; n = n + v; }


//// [aLoopBodyValueStillLowersToOwnerStatements.ts]
let n = 0;
while (n < 3) { let $tt_v0: number;
{
  const $tt_m = n;
  switch ($tt_m) {
    case 0: {
      $tt_v0 = 1;
      break;
    }
    default: {
      $tt_v0 = 0;
      break;
    }
  }
}
const v = $tt_v0; n = n + v; }
