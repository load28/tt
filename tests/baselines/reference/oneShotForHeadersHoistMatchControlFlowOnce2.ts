//// [oneShotForHeadersHoistMatchControlFlowOnce2.tt] ////
for (let value = match (source) { is Number => 1, _ => 0 }; value < 2; value++) { use(value); }


//// [oneShotForHeadersHoistMatchControlFlowOnce2.ts]
{
  let $tt_v0: number;
  {
    const $tt_m = source;
    do {
      if ($tt_m instanceof Number) {
        $tt_v0 = 1;
        break;
      }
      $tt_v0 = 0;
      break;
    } while (false);
  }
  for (let value = $tt_v0; value < 2; value++) { use(value); }
}
