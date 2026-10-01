//// [repeatedLoopTestsOwnTheMatchRegionPerIteration1.tt] ////
while (match (next()) { is Error => false, _ => true }) { work(); }


//// [repeatedLoopTestsOwnTheMatchRegionPerIteration1.ts]
while (true) {
  let $tt_v0: boolean;
  {
    const $tt_m = next();
    do {
      if ($tt_m instanceof Error) {
        $tt_v0 = false;
        break;
      }
      $tt_v0 = true;
      break;
    } while (false);
  }
  if (!($tt_v0)) break; { work(); }}
