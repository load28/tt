//// [literalMatchWithAGuardBecomesAnIfChain.tt] ////
const v = match (code) { 200 if ok => 1, 200 => 2, _ => 3 };


//// [literalMatchWithAGuardBecomesAnIfChain.ts]
let $tt_v0$v: number;
{
  const $tt_m = code;
  do {
    if ($tt_m === 200) {
      if (ok) {
        $tt_v0$v = 1;
        break;
      }
    }
    if ($tt_m === 200) {
      $tt_v0$v = 2;
      break;
    }
    $tt_v0$v = 3;
    break;
  } while (false);
}
const v = $tt_v0$v;
\ No newline at end of file
