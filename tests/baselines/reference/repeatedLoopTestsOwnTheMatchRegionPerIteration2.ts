//// [repeatedLoopTestsOwnTheMatchRegionPerIteration2.tt] ////
for (; match (next()) { is Error => false, _ => true }; tick()) { work(); }


//// [repeatedLoopTestsOwnTheMatchRegionPerIteration2.ts]
for (; ; tick()) {
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
