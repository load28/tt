//// [loopTestRewritesComposeWithConditionalsNestingAndInitializers3.tt] ////
declare function a(): number; declare function b(): number;
while (match (a()) { 1 => true, _ => false } || match (b()) { 2 => true, _ => false }) { work(); }


//// [loopTestRewritesComposeWithConditionalsNestingAndInitializers3.ts]
declare function a(): number; declare function b(): number;
while (true) {
  let $tt_v0: boolean;
  let $tt_v2: boolean;
  {
    const $tt_m = a();
    switch ($tt_m) {
      case 1: {
        $tt_v0 = true;
        break;
      }
      default: {
        $tt_v0 = false;
        break;
      }
    }
  }if ($tt_v0) {
    $tt_v2 = $tt_v0;
  } else {
    let $tt_v1: boolean;
    {
      const $tt_m = b();
      switch ($tt_m) {
        case 2: {
          $tt_v1 = true;
          break;
        }
        default: {
          $tt_v1 = false;
          break;
        }
      }
    }
    $tt_v2 = $tt_v0 || $tt_v1;
  }
  
  if (!($tt_v2)) break; { work(); }}
