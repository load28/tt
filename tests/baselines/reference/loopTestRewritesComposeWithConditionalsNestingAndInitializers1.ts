//// [loopTestRewritesComposeWithConditionalsNestingAndInitializers1.tt] ////
declare const flag: boolean; declare function next(): unknown;
while (flag && match (next()) { is Error => false, _ => true }) { work(); }


//// [loopTestRewritesComposeWithConditionalsNestingAndInitializers1.ts]
declare const flag: boolean; declare function next(): unknown;
while (true) {
  let $tt_v2: boolean;
  
  if (!(flag)) break;
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
  $tt_v2 = $tt_v0;
  if (!($tt_v2)) break;
  { work(); }}
