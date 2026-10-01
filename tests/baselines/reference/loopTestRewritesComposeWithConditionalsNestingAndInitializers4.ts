//// [loopTestRewritesComposeWithConditionalsNestingAndInitializers4.tt] ////
for (let a = match (1) { 1 => 1, _ => 0 }; match (a) { 1 => true, _ => false }; a++) { use(a); }


//// [loopTestRewritesComposeWithConditionalsNestingAndInitializers4.ts]
{
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
  for (let a = $tt_v0; ; a++) {
    let $tt_v1: boolean;
    {
      const $tt_m = a;
      switch ($tt_m) {
        case 1: {
          $tt_v1 = true;
          break;
        }
        default: {
          $tt_v1 = false;
          break;
        }
      }
    }
    if (!($tt_v1)) break; { use(a); }
}}
