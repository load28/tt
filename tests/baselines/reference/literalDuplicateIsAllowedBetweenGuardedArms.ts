//// [literalDuplicateIsAllowedBetweenGuardedArms.tt] ////
const v = match (x) { 1 if a => 1, 1 if b => 2, 1 => 3, _ => 4 };


//// [literalDuplicateIsAllowedBetweenGuardedArms.ts]
let $tt_v0$v: number;
{
  const $tt_m = x;
  do {
    if ($tt_m === 1) {
      if (a) {
        $tt_v0$v = 1;
        break;
      }
    }
    if ($tt_m === 1) {
      if (b) {
        $tt_v0$v = 2;
        break;
      }
    }
    if ($tt_m === 1) {
      $tt_v0$v = 3;
      break;
    }
    $tt_v0$v = 4;
    break;
  } while (false);
}
const v = $tt_v0$v;
\ No newline at end of file
